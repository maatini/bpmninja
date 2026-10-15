//! Condition evaluator for gateway routing.
//!
//! Evaluates simple expressions like `"x == 5"`, `"amount > 100"`, or
//! bare `"flag"` (truthy check) against a variable map.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::Value;

/// Strict grammar for conditional events; gateway parsing remains compatible.
pub fn validate_event_condition(expression: &str) -> crate::domain::EngineResult<()> {
    compile_event_condition(expression).map(|_| ())
}

pub(crate) fn compile_event_condition(
    expression: &str,
) -> crate::domain::EngineResult<CompiledCondition> {
    let invalid = || {
        crate::domain::EngineError::InvalidDefinition(
            "Conditional event requires a variable or a single comparison with a JSON literal"
                .into(),
        )
    };
    let expression = expression.trim();
    if expression.is_empty() || expression.len() > 4096 {
        return Err(invalid());
    }
    let name_end = expression
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(expression.len());
    let variable = &expression[..name_end];
    if !variable.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return Err(invalid());
    }
    let tail = expression[name_end..].trim();
    if tail.is_empty() {
        return Ok(CompiledCondition::Truthy(variable.into()));
    }
    for (text, operator) in [
        ("==", Operator::Eq),
        ("!=", Operator::Ne),
        (">=", Operator::Ge),
        ("<=", Operator::Le),
        (">", Operator::Gt),
        ("<", Operator::Lt),
    ] {
        if let Some(right) = tail.strip_prefix(text) {
            let right = right.trim();
            let rhs = serde_json::from_str::<Value>(right)
                .ok()
                .or_else(|| {
                    (right.len() >= 2
                        && right.starts_with('\'')
                        && right.ends_with('\'')
                        && !right[1..right.len() - 1].contains(['\'', '\\', '\n', '\r']))
                    .then(|| Value::String(right[1..right.len() - 1].into()))
                })
                .filter(|value| !value.is_array() && !value.is_object())
                .ok_or_else(invalid)?;
            return Ok(CompiledCondition::Comparison {
                variable: variable.into(),
                operator,
                rhs,
            });
        }
    }
    Err(invalid())
}

/// Evaluates a simple condition expression against token variables.
///
/// Supported forms:
/// - `"variable == value"` / `"variable != value"`
/// - `"variable > value"` / `"variable < value"` / `"variable >= value"` / `"variable <= value"`
/// - `"variable"` (truthy check: non-null, non-false, non-zero, non-empty-string)
///
/// Returns `false` if the variable is missing or the expression is malformed.
pub fn evaluate_condition(expr: &str, variables: &HashMap<String, Value>) -> bool {
    CompiledCondition::parse(expr).evaluate(variables)
}

/// Parsed condition kept with a deployed definition, never serialized.
#[derive(Debug, Clone)]
pub(crate) enum CompiledCondition {
    Empty,
    Truthy(String),
    Comparison {
        variable: String,
        operator: Operator,
        rhs: Value,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Operator {
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
}

impl CompiledCondition {
    pub(crate) fn parse(expr: &str) -> Self {
        let expr = expr.trim();
        if expr.is_empty() {
            return Self::Empty;
        }
        for (op, operator) in [
            ("==", Operator::Eq),
            ("!=", Operator::Ne),
            (">=", Operator::Ge),
            ("<=", Operator::Le),
            (">", Operator::Gt),
            ("<", Operator::Lt),
        ] {
            if let Some(idx) = expr.find(op) {
                return Self::Comparison {
                    variable: expr[..idx].trim().to_string(),
                    operator,
                    rhs: parse_rhs(expr[idx + op.len()..].trim()),
                };
            }
        }
        Self::Truthy(expr.to_string())
    }

    pub(crate) fn evaluate(&self, variables: &HashMap<String, Value>) -> bool {
        match self {
            Self::Empty => false,
            Self::Truthy(expr) => {
                match variables.get(expr) {
                    Some(Value::Bool(b)) => *b,
                    Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
                    Some(Value::String(s)) => !s.is_empty(),
                    Some(Value::Null) | None => false,
                    // Arrays and objects are truthy
                    Some(_) => true,
                }
            }
            Self::Comparison {
                variable,
                operator,
                rhs,
            } => {
                let Some(lhs) = variables.get(variable) else {
                    return false;
                };
                match operator {
                    Operator::Eq => values_eq(lhs, rhs),
                    Operator::Ne => !values_eq(lhs, rhs),
                    Operator::Gt => values_cmp(lhs, rhs) == Some(Ordering::Greater),
                    Operator::Lt => values_cmp(lhs, rhs) == Some(Ordering::Less),
                    Operator::Ge => values_cmp(lhs, rhs).is_some_and(|o| o != Ordering::Less),
                    Operator::Le => values_cmp(lhs, rhs).is_some_and(|o| o != Ordering::Greater),
                }
            }
        }
    }
}

/// Parses a right-hand-side string into a `serde_json::Value`.
fn parse_rhs(s: &str) -> Value {
    // Strip surrounding quotes (single or double) for string comparison.
    // Use char boundaries to avoid panics on multi-byte UTF-8 input.
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() >= 2 {
            let start = s.char_indices().nth(1).map(|(i, _)| i).unwrap_or(1);
            let end = s
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(s.len());
            if start <= end {
                return Value::String(s[start..end].to_string());
            }
        }
        return Value::String(String::new());
    }
    // Boolean literals
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }
    // Null
    if s == "null" {
        return Value::Null;
    }
    // Try number
    if let Ok(n) = s.parse::<i64>() {
        return Value::Number(n.into());
    }
    if let Ok(n) = s.parse::<f64>() {
        // Rust clippy warning let_and_return fix
        if let Some(n) = serde_json::Number::from_f64(n) {
            return Value::Number(n);
        }
    }
    // Fallback: treat as plain string
    Value::String(s.to_string())
}

/// Equality comparison for JSON values.
fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a
            .as_f64()
            .zip(b.as_f64())
            .is_some_and(|(x, y)| (x - y).abs() < f64::EPSILON),
        _ => a == b,
    }
}

/// Ordering comparison for JSON values (numbers only).
fn values_cmp(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let fa = a.as_f64()?;
            let fb = b.as_f64()?;
            fa.partial_cmp(&fb)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_equality() {
        let mut vars = HashMap::new();
        vars.insert("status".to_string(), json!("approved"));
        vars.insert("count".to_string(), json!(5));

        assert!(evaluate_condition("status == 'approved'", &vars));
        assert!(evaluate_condition("status == \"approved\"", &vars));
        assert!(!evaluate_condition("status == 'rejected'", &vars));
        assert!(evaluate_condition("status != 'rejected'", &vars));

        assert!(evaluate_condition("count == 5", &vars));
        assert!(!evaluate_condition("count == 6", &vars));
    }

    #[test]
    fn test_numeric_comparisons() {
        let mut vars = HashMap::new();
        vars.insert("amount".to_string(), json!(150.5));
        vars.insert("score".to_string(), json!(10));

        assert!(evaluate_condition("amount > 100", &vars));
        assert!(evaluate_condition("amount >= 150.5", &vars));
        assert!(evaluate_condition("amount < 200", &vars));
        assert!(evaluate_condition("amount <= 150.5", &vars));
        assert!(!evaluate_condition("amount > 200", &vars));

        assert!(evaluate_condition("score > 5", &vars));
        assert!(evaluate_condition("score < 15", &vars));
    }

    #[test]
    fn test_values_eq_numbers_vs_default() {
        // Catches: delete match arm (Number, Number), replace < with <=
        // Numbers use epsilon-based comparison, not default ==
        let a = json!(1.0);
        let b = json!(1);
        assert!(values_eq(&a, &b));

        let a = json!(0.1 + 0.2);
        let b = json!(0.3);
        // f64 epsilon comparison should catch near-equal floats
        assert!(values_eq(&a, &b));

        // Different numbers must not be equal
        let a = json!(1.0);
        let b = json!(2.0);
        assert!(!values_eq(&a, &b));

        // Edge case: diff exactly at EPSILON boundary → should NOT be equal
        // (catches: replace < with <= in values_eq)
        let x = 1.0_f64;
        let y = x + f64::EPSILON;
        let a = serde_json::Value::from(x);
        let b = serde_json::Value::from(y);
        assert!(
            !values_eq(&a, &b),
            "Numbers differing by exactly EPSILON should NOT be equal"
        );
    }

    #[test]
    fn test_parse_rhs_quoted_strings() {
        // Catches: replace && with || in parse_rhs (line 66)
        // Both start AND end quote must match for stripping
        let val = parse_rhs("\"hello\"");
        assert_eq!(val, json!("hello"));

        let val = parse_rhs("'world'");
        assert_eq!(val, json!("world"));

        // Mismatched quotes: should NOT strip
        let val = parse_rhs("\"hello'");
        assert_eq!(val, json!("\"hello'"));

        let val = parse_rhs("'hello\"");
        assert_eq!(val, json!("'hello\""));
    }

    #[test]
    fn test_truthy() {
        let mut vars = HashMap::new();
        vars.insert("is_valid".to_string(), json!(true));
        vars.insert("is_empty".to_string(), json!(false));
        vars.insert("null_val".to_string(), json!(null));
        vars.insert("name".to_string(), json!("Bob"));
        vars.insert("empty_str".to_string(), json!(""));
        vars.insert("num".to_string(), json!(1));
        vars.insert("zero".to_string(), json!(0));

        assert!(evaluate_condition("is_valid", &vars));
        assert!(!evaluate_condition("is_empty", &vars));
        assert!(!evaluate_condition("null_val", &vars));
        assert!(evaluate_condition("name", &vars));
        assert!(!evaluate_condition("empty_str", &vars));
        assert!(evaluate_condition("num", &vars));
        assert!(!evaluate_condition("zero", &vars));
        assert!(!evaluate_condition("missing_var", &vars));
    }
}

#[cfg(test)]
mod event_condition_tests {
    use super::*;
    #[test]
    fn strict_conditions_handle_json_strings_and_reject_unsupported_expressions() {
        let vars = HashMap::from([("status".into(), Value::String("x == y\nnext".into()))]);
        assert!(
            compile_event_condition(r#"status == "x == y\nnext""#)
                .unwrap()
                .evaluate(&vars)
        );
        assert!(
            compile_event_condition("status != 'other > value'")
                .unwrap()
                .evaluate(&vars)
        );
        for expression in [
            "",
            "a && b",
            "a >",
            "a === 1",
            "a == unquoted",
            "a > []",
            "1name",
            "f()",
            "a.b",
            "a == 1 == 1",
        ] {
            assert!(
                validate_event_condition(expression).is_err(),
                "{expression}"
            );
        }
        for expression in [
            "flag",
            "amount >= 1.5",
            "status == 'approved'",
            "flag == true",
            "value == null",
        ] {
            assert!(validate_event_condition(expression).is_ok(), "{expression}");
        }
    }
}
