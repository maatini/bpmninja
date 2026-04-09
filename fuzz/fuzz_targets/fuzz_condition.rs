//! Fuzz target: Condition evaluator
//!
//! Feeds arbitrary expression strings and variable maps into `evaluate_condition()`
//! to discover panics in the comparison/parsing logic.
//! The evaluator must return `true` or `false` for any input — never panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use std::collections::HashMap;
use serde_json::Value;

/// Structured input for the condition evaluator fuzzer.
/// Using `Arbitrary` gives us better coverage than raw bytes.
#[derive(Arbitrary, Debug)]
struct ConditionInput {
    expression: String,
    /// Up to 8 variables with fuzzed keys and JSON-compatible values
    variables: Vec<(String, FuzzValue)>,
}

/// A simplified JSON value that `Arbitrary` can derive automatically.
#[derive(Arbitrary, Debug)]
enum FuzzValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl From<FuzzValue> for Value {
    fn from(v: FuzzValue) -> Self {
        match v {
            FuzzValue::Null => Value::Null,
            FuzzValue::Bool(b) => Value::Bool(b),
            FuzzValue::Int(i) => Value::Number(i.into()),
            FuzzValue::Float(f) => {
                // NaN/Inf are not valid JSON numbers — fall back to null
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            FuzzValue::Str(s) => Value::String(s),
        }
    }
}

fuzz_target!(|input: ConditionInput| {
    let variables: HashMap<String, Value> = input
        .variables
        .into_iter()
        .take(8) // Limit variable count to avoid degenerate cases
        .map(|(k, v)| (k, v.into()))
        .collect();

    // Must never panic — only return true/false
    let _ = engine_core::evaluate_condition(&input.expression, &variables);
});
