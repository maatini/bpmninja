//! Serialisierbare Daten-Mappings. Ausdrücke sind Variablenreferenzen, keine Skripte.
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
            Self::List(values) => {
                let mut items = Vec::with_capacity(values.len());
                for value in values {
                    items.push(value.evaluate(variables)?);
                }
                Ok(Value::Array(items))
            }
            Self::Map(values) => {
                let mut object = serde_json::Map::new();
                for (key, value) in values {
                    object.insert(key.clone(), value.evaluate(variables)?);
                }
                Ok(Value::Object(object))
            }
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

/// Persistierter Aktivierungszustand, einmal pro Token.
///
/// Multi-Instance-Service-Tasks teilen sich einen Token. Deren lokale Inputs
/// leben nur im Task-Snapshot, nicht in diesem Scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataScope {
    pub node_id: String,
    pub original_variables: HashMap<String, Value>,
    pub local_names: Vec<String>,
}

/// Wertet eine Mapping-Phase gegen genau einen Snapshot aus.
///
/// Schreibziele beeinflussen sich nicht gegenseitig. Ein Fehler verwirft die
/// ganze Phase; `source` bleibt unverändert.
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
