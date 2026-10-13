use super::CamundaError;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
const TYPES: &[&str] = &[
    "Null", "Boolean", "String", "Short", "Integer", "Long", "Double", "Date", "Json",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TypedVariable {
    #[serde(default)]
    value: Value,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    value_info: Option<HashMap<String, Value>>,
}

fn inferred(value: &Value) -> &'static str {
    match value {
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::String(_) => "String",
        Value::Number(number)
            if number
                .as_i64()
                .is_some_and(|value| i32::try_from(value).is_ok()) =>
        {
            "Integer"
        }
        Value::Number(number) if number.as_i64().is_some() => "Long",
        Value::Number(_) => "Double",
        _ => "Json",
    }
}
fn matches(value: &Value, kind: &str) -> bool {
    if value.is_null() {
        return true;
    }
    match kind {
        "Null" => false,
        "Boolean" => value.is_boolean(),
        "String" => value.is_string(),
        "Short" => value
            .as_i64()
            .is_some_and(|value| i16::try_from(value).is_ok()),
        "Integer" => value
            .as_i64()
            .is_some_and(|value| i32::try_from(value).is_ok()),
        "Long" => value.as_i64().is_some(),
        "Double" => value.is_number(),
        "Date" => value.as_str().is_some_and(|value| {
            chrono::DateTime::parse_from_rfc3339(value).is_ok()
                || chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f%z").is_ok()
        }),
        "Json" => true,
        _ => false,
    }
}
type Variables = (HashMap<String, Value>, HashMap<String, String>);
pub(super) fn decode_variables(
    variables: HashMap<String, TypedVariable>,
) -> Result<Variables, CamundaError> {
    let mut values = HashMap::new();
    let mut types = HashMap::new();
    for (name, variable) in variables {
        let kind = variable
            .kind
            .unwrap_or_else(|| inferred(&variable.value).into());
        if !TYPES.contains(&kind.as_str()) {
            return Err(CamundaError::bad(format!(
                "Variable {name}: unsupported type {kind}"
            )));
        }
        if variable.value_info.is_some_and(|info| {
            info.iter().any(|(key, value)| {
                !value.is_null()
                    && !(key == "transient" && value == &Value::Bool(false))
                    && !(kind == "Json"
                        && key == "serializationDataFormat"
                        && value == "application/json")
            })
        }) {
            return Err(CamundaError::bad(format!(
                "Variable {name}: unsupported valueInfo option"
            )));
        }
        let value = if kind == "Json" {
            if let Some(serialized) = variable.value.as_str() {
                serde_json::from_str(serialized).map_err(|_| {
                    CamundaError::bad(format!("Variable {name}: invalid serialized JSON"))
                })?
            } else {
                variable.value
            }
        } else {
            variable.value
        };
        if !matches(&value, &kind) {
            return Err(CamundaError::bad(format!(
                "Variable {name}: value does not match type {kind}"
            )));
        }
        let value = if kind == "Double" && !value.is_null() {
            Value::from(
                value
                    .as_f64()
                    .ok_or_else(|| CamundaError::bad("Invalid Double"))?,
            )
        } else {
            value
        };
        types.insert(name.clone(), kind);
        values.insert(name, value);
    }
    Ok((values, types))
}
pub(super) fn encode_variable(value: &Value, hint: Option<&str>) -> Value {
    let kind = hint
        .filter(|kind| TYPES.contains(kind) && matches(value, kind))
        .unwrap_or_else(|| inferred(value));
    let encoded = if kind == "Json" && !value.is_null() {
        Value::String(value.to_string())
    } else if kind == "Double" {
        value.as_f64().map(Value::from).unwrap_or(Value::Null)
    } else {
        value.clone()
    };
    json!({"type": kind, "value": encoded, "valueInfo": if kind == "Json" { json!({"serializationDataFormat":"application/json"}) } else { json!({}) }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_types_ranges_and_transient_metadata() {
        for variable in [
            json!({"type":"Short","value":32768}),
            json!({"type":"Date","value":"not-a-date"}),
            json!({"type":"Json","value":"broken"}),
            json!({"type":"Float","value":1.5}),
            json!({"type":"String","value":"x","valueInfo":{"transient":true}}),
        ] {
            let variable: TypedVariable = serde_json::from_value(variable).unwrap();
            assert!(decode_variables(HashMap::from([("bad".into(), variable)])).is_err());
        }
    }

    #[test]
    fn stale_hints_fall_back_and_double_values_use_floating_json() {
        assert_eq!(
            encode_variable(&json!("a string"), Some("Long"))["type"],
            "String"
        );
        assert_eq!(
            encode_variable(&json!(5), Some("Double"))["value"].to_string(),
            "5.0"
        );
        assert_eq!(
            encode_variable(&Value::Null, Some("String"))["type"],
            "String"
        );
        let variable =
            serde_json::from_value(json!({"type":"Double","value":5,"valueInfo":null})).unwrap();
        let (values, types) =
            decode_variables(HashMap::from([("value".into(), variable)])).unwrap();
        assert_eq!(values["value"].to_string(), "5.0");
        assert_eq!(types["value"], "Double");
    }
}
