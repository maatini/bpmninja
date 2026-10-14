//! Serializable data mappings. Expressions support variable references only.
use super::{EngineError, EngineResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MappingValue {
    Literal(Value),
    Variable(String),
    List(Vec<MappingValue>),
    Map(HashMap<String, MappingValue>),
}

impl MappingValue {
    pub fn evaluate(&self, variables: &HashMap<String, Value>) -> EngineResult<Value> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Variable(name) => variables.get(name).cloned().ok_or_else(|| {
                EngineError::DataMappingError(format!("Data mapping: variable '{name}' is missing"))
            }),
            Self::List(values) => values
                .iter()
                .map(|v| v.evaluate(variables))
                .collect::<EngineResult<Vec<_>>>()
                .map(Value::Array),
            Self::Map(values) => values
                .iter()
                .map(|(k, v)| Ok((k.clone(), v.evaluate(variables)?)))
                .collect::<EngineResult<serde_json::Map<_, _>>>()
                .map(Value::Object),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataMapping {
    pub target: String,
    pub value: MappingValue,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActivityMappings {
    #[serde(default)]
    pub inputs: Vec<DataMapping>,
    #[serde(default)]
    pub outputs: Vec<DataMapping>,
    #[serde(default)]
    pub call_inputs: Option<Vec<DataMapping>>,
    #[serde(default)]
    pub call_outputs: Option<Vec<DataMapping>>,
}

/// Persisted per activation, including each multi-instance token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataScope {
    pub node_id: String,
    pub original_variables: HashMap<String, Value>,
    pub local_names: Vec<String>,
}

pub fn evaluate_mappings(
    mappings: &[DataMapping],
    source: &HashMap<String, Value>,
) -> EngineResult<HashMap<String, Value>> {
    let mut result = HashMap::new();
    for mapping in mappings {
        if mapping.target.trim().is_empty() || result.contains_key(&mapping.target) {
            return Err(EngineError::DataMappingError(format!(
                "Data mapping: empty or duplicate target '{}'",
                mapping.target
            )));
        }
        result.insert(mapping.target.clone(), mapping.value.evaluate(source)?);
    }
    Ok(result)
}
