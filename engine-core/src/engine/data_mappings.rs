use std::collections::HashMap;

use serde_json::Value;

use crate::domain::data::evaluate_mappings;
use crate::domain::{BpmnElement, DataMapping, EngineResult, ProcessDefinition, Token};

pub(crate) fn enter(token: &mut Token, def: &ProcessDefinition, node: &str) -> EngineResult<()> {
    if token
        .data_scope
        .as_ref()
        .is_some_and(|scope| scope.node_id != node)
    {
        token.discard_data_scope();
    }
    if token.data_scope.is_some() {
        return Ok(());
    }
    // Multi-Instance teilt sich einen Token. Inputs gelten pro Item-Snapshot.
    if is_multi_instance_service(def, node) {
        return Ok(());
    }
    if let Some(mappings) = def.data_mappings.get(node) {
        let inputs = evaluate_mappings(&mappings.inputs, &token.variables)?;
        token.data_scope = Some(crate::domain::DataScope {
            node_id: node.to_string(),
            original_variables: token.variables.clone(),
            local_names: inputs.keys().cloned().collect(),
        });
        token.variables.extend(inputs);
    }
    Ok(())
}

/// Arbeitet auf einem Kandidaten. Der Aufrufer übernimmt ihn erst nach den End-Listenern.
pub(crate) fn leave(token: &mut Token, def: &ProcessDefinition, node: &str) -> EngineResult<()> {
    let Some(scope) = token.data_scope.as_ref() else {
        return Ok(());
    };
    if scope.node_id != node {
        return Ok(());
    }
    let Some(mappings) = def.data_mappings.get(node) else {
        return Err(crate::domain::EngineError::DataMappingError(format!(
            "Data mapping: activation of '{node}' has no mappings"
        )));
    };
    let outputs = evaluate_mappings(&mappings.outputs, &token.variables)?;
    let mut variables = if mappings.outputs.is_empty() {
        token.process_variables()
    } else {
        scope.original_variables.clone()
    };
    variables.extend(outputs);
    token.variables = variables;
    token.data_scope = None;
    Ok(())
}

pub(crate) fn is_multi_instance_service(def: &ProcessDefinition, node: &str) -> bool {
    matches!(
        def.get_node(node),
        Some(BpmnElement::ServiceTask {
            multi_instance: Some(_),
            ..
        })
    )
}

/// Hängt Aktivitätseingaben an einen Item-Snapshot. Der gemeinsame Token bleibt unberührt.
pub(crate) fn apply_inputs(
    def: &ProcessDefinition,
    node: &str,
    mut variables: HashMap<String, Value>,
) -> EngineResult<HashMap<String, Value>> {
    if let Some(mappings) = def.data_mappings.get(node) {
        let inputs = evaluate_mappings(&mappings.inputs, &variables)?;
        variables.extend(inputs);
    }
    Ok(variables)
}

/// Schreibt die Ausgänge einer Multi-Instance-Aktivierung auf den gemeinsamen Token.
///
/// Lokale Eingaben bleiben im Snapshot. Ohne Output-Mappings gelten die
/// Worker-Variablen wie bisher, abzüglich der lokalen Eingabenamen.
pub(crate) fn publish_multi_instance_outputs(
    token: &mut Token,
    def: &ProcessDefinition,
    node: &str,
    snapshot: &HashMap<String, Value>,
    worker: &HashMap<String, Value>,
) -> EngineResult<()> {
    let Some(mappings) = def.data_mappings.get(node) else {
        for (key, value) in worker {
            token.variables.insert(key.clone(), value.clone());
        }
        return Ok(());
    };
    if mappings.outputs.is_empty() {
        for (key, value) in worker {
            if is_input_target(&mappings.inputs, key) {
                continue;
            }
            token.variables.insert(key.clone(), value.clone());
        }
        return Ok(());
    }
    let mut source = snapshot.clone();
    source.extend(
        worker
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    let outputs = evaluate_mappings(&mappings.outputs, &source)?;
    for (key, value) in outputs {
        token.variables.insert(key, value);
    }
    Ok(())
}

pub(crate) fn apply_call_transfer(token: &mut Token, transferred: &HashMap<String, Value>) {
    token.variables.extend(transferred.clone());
    if let Some(scope) = &mut token.data_scope {
        scope.original_variables.extend(transferred.clone());
    }
}

fn is_input_target(inputs: &[DataMapping], name: &str) -> bool {
    inputs.iter().any(|mapping| mapping.target == name)
}
