use crate::domain::data::evaluate_mappings;
use crate::domain::{DataScope, EngineResult, ProcessDefinition, Token};

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
    if let Some(mappings) = def.data_mappings.get(node) {
        let inputs = evaluate_mappings(&mappings.inputs, &token.variables)?;
        token.data_scope = Some(Box::new(DataScope {
            node_id: node.to_string(),
            original_variables: token.variables.clone(),
            local_names: inputs.keys().cloned().collect(),
        }));
        token.variables.extend(inputs);
    }
    Ok(())
}

/// Operates on a candidate token; the caller commits only after end listeners succeed.
pub(crate) fn leave(token: &mut Token, def: &ProcessDefinition, node: &str) -> EngineResult<()> {
    let Some(scope) = token.data_scope.as_ref() else {
        return Ok(());
    };
    if scope.node_id != node {
        return Ok(());
    }
    let mappings = def.data_mappings.get(node).ok_or_else(|| {
        crate::domain::EngineError::DataMappingError(format!(
            "mapping for active node '{node}' is missing"
        ))
    })?;
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
