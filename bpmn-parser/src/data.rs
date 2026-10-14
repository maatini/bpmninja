//! X11 extraction uses the original XML so nested values and reference scopes are retained.
use engine_core::domain::{ActivityMappings, BpmnElement, DataMapping, MappingValue, ProcessDefinition};
use engine_core::error::{EngineError, EngineResult};
use quick_xml::events::Event;
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug)]
struct Node {
    name: String,
    attrs: HashMap<String, String>,
    text: String,
    children: Vec<Node>,
}
impl Node {
    fn attr(&self, name: &str) -> Option<&str> { self.attrs.get(name).map(String::as_str) }
    fn children(&self, name: &str) -> impl Iterator<Item=&Node> { self.children.iter().filter(move |n| n.name == name) }
    fn id(&self) -> EngineResult<&str> { self.attr("id").ok_or_else(|| invalid("data element requires an id")) }
}
fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidDefinition(format!("Data mapping: {}", message.into()))
}

fn tree(xml: &str) -> EngineResult<Node> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut stack = vec![Node::default()];
    loop {
        match reader.read_event().map_err(|e| invalid(e.to_string()))? {
            Event::Start(e) => {
                let mut node = Node { name: e.local_name().as_ref().to_string(), ..Default::default() };
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| invalid(e.to_string()))?;
                    // Attribute prefixes are preserved for extension defaults.
                    let name = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                    let value = attr.decode_and_unescape_value(reader.decoder()).map_err(|e| invalid(e.to_string()))?.into_owned();
                    node.attrs.insert(name, value);
                }
                stack.push(node);
            }
            Event::End(_) => {
                let node = stack.pop().ok_or_else(|| invalid("unbalanced XML"))?;
                stack.last_mut().ok_or_else(|| invalid("unbalanced XML"))?.children.push(node);
            }
            Event::Text(e) => stack.last_mut().unwrap().text.push_str(&e.decode().map_err(|e| invalid(e.to_string()))?),
            Event::CData(e) => stack.last_mut().unwrap().text.push_str(&e.decode().map_err(|e| invalid(e.to_string()))?),
            Event::GeneralRef(e) => {
                let reference = e.decode().map_err(|e| invalid(e.to_string()))?;
                let escaped = format!("&{reference};");
                stack.last_mut().unwrap().text.push_str(&quick_xml::escape::unescape(&escaped).map_err(|e| invalid(e.to_string()))?);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(stack.pop().unwrap())
}

fn reference(text: &str) -> EngineResult<String> {
    let name = text.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(invalid(format!("unsupported variable expression '{text}' (use ${{variable}})")));
    }
    Ok(name.into())
}
fn expression(text: &str) -> EngineResult<MappingValue> {
    let trimmed = text.trim();
    if trimmed.starts_with("${") || trimmed.starts_with("#{") {
        let inner = trimmed.get(2..).and_then(|s| s.strip_suffix('}')).ok_or_else(|| invalid("unterminated expression"))?;
        return Ok(MappingValue::Variable(reference(inner)?));
    }
    if text.contains("${") || text.contains("#{") {
        return Err(invalid("expression interpolation is not supported"));
    }
    Ok(MappingValue::Literal(serde_json::Value::String(text.into())))
}
fn value(node: &Node) -> EngineResult<MappingValue> {
    if node.children.is_empty() { return expression(&node.text); }
    if !node.text.trim().is_empty() || node.children.len() != 1 { return Err(invalid("parameter requires one value")); }
    let child = &node.children[0];
    match child.name.as_str() {
        "list" => {
            if child.children.iter().any(|n| n.name != "value") { return Err(invalid("list accepts value children only")); }
            Ok(MappingValue::List(child.children.iter().map(value).collect::<EngineResult<_>>()?))
        }
        "map" => {
            let mut values = HashMap::new();
            for entry in &child.children {
                if entry.name != "entry" { return Err(invalid("map accepts entry children only")); }
                let key = entry.attr("key").ok_or_else(|| invalid("map entry requires key"))?;
                if values.insert(key.into(), value(entry)?).is_some() { return Err(invalid("duplicate map key")); }
            }
            Ok(MappingValue::Map(values))
        }
        _ => Err(invalid(format!("unsupported parameter value '{}'", child.name))),
    }
}

fn add_mapping(mappings: &mut Vec<DataMapping>, target: &str, value: MappingValue) -> EngineResult<()> {
    if target.trim().is_empty() || mappings.iter().any(|m| m.target == target) {
        return Err(invalid(format!("empty or duplicate target '{target}'")));
    }
    mappings.push(DataMapping { target: target.into(), value });
    Ok(())
}

fn declarations(scope: &Node, def: &mut ProcessDefinition) -> EngineResult<HashMap<String, String>> {
    let mut refs = HashMap::new();
    let mut names = HashSet::new();
    for declaration in scope.children.iter().filter(|n| matches!(n.name.as_str(), "property" | "dataObject")) {
        let id = declaration.id()?;
        let name = declaration.attr("name").filter(|n| !n.trim().is_empty()).unwrap_or(id);
        if !names.insert(name) || refs.insert(id.into(), name.into()).is_some() { return Err(invalid("duplicate data declaration")); }
        def.data_variables.push(name.into());
        // Explicit extension: declarations themselves do not specify JSON defaults.
        if let Some(default) = declaration.attr("bpmninja:default") {
            let default = serde_json::from_str(default).map_err(|e| invalid(format!("invalid JSON default on '{id}': {e}")))?;
            def.variable_defaults.insert(name.into(), default);
        }
    }
    for alias in scope.children("dataObjectReference") {
        let target = alias.attr("dataObjectRef").ok_or_else(|| invalid("dataObjectReference requires dataObjectRef"))?;
        let name = refs.get(target).cloned().ok_or_else(|| invalid(format!("unknown dataObjectRef '{target}'")))?;
        if refs.insert(alias.id()?.into(), name).is_some() { return Err(invalid("duplicate data reference id")); }
    }
    Ok(refs)
}

fn activity(node: &Node, refs: &HashMap<String, String>) -> EngineResult<ActivityMappings> {
    let mut result = ActivityMappings::default();
    let mut inputs = HashMap::new();
    let mut outputs = HashMap::new();
    for io in node.children("ioSpecification") {
        for item in &io.children {
            if matches!(item.name.as_str(), "dataInput" | "dataOutput") {
                if item.attr("isCollection") == Some("true") { return Err(invalid("collection ioSpecification is not supported")); }
                let id = item.id()?;
                let name = item.attr("name").filter(|s| !s.trim().is_empty()).unwrap_or(id);
                let map = if item.name == "dataInput" { &mut inputs } else { &mut outputs };
                if refs.contains_key(id) || inputs_and_outputs_conflict(id, map) { return Err(invalid("duplicate IO reference")); }
                map.insert(id.into(), name.into());
            } else if !matches!(item.name.as_str(), "inputSet" | "outputSet") {
                return Err(invalid(format!("unsupported ioSpecification child '{}'", item.name)));
            }
        }
    }
    if inputs.keys().any(|id| outputs.contains_key(id)) { return Err(invalid("ambiguous IO id")); }
    for association in node.children.iter().filter(|n| matches!(n.name.as_str(), "dataInputAssociation" | "dataOutputAssociation")) {
        if association.children.iter().any(|n| !matches!(n.name.as_str(), "sourceRef" | "targetRef")) {
            return Err(invalid("association assignment/transformation is not supported"));
        }
        let sources: Vec<_> = association.children("sourceRef").collect();
        let targets: Vec<_> = association.children("targetRef").collect();
        if sources.len() != 1 || targets.len() != 1 { return Err(invalid("association requires one sourceRef and targetRef")); }
        let source = sources[0].text.trim();
        let target = targets[0].text.trim();
        let (source_map, target_map, mappings) = if association.name == "dataInputAssociation" {
            (refs, &inputs, &mut result.inputs)
        } else { (&outputs, refs, &mut result.outputs) };
        let source_name = source_map.get(source).ok_or_else(|| invalid(format!("unknown association source '{source}'")))?;
        let target_name = target_map.get(target).ok_or_else(|| invalid(format!("unknown association target '{target}'")))?;
        add_mapping(mappings, target_name, MappingValue::Variable(source_name.clone()))?;
    }
    for extensions in node.children("extensionElements") {
        for extension in &extensions.children {
            match extension.name.as_str() {
                "inputOutput" => {
                    for parameter in &extension.children {
                        let target = parameter.attr("name").ok_or_else(|| invalid("parameter requires name"))?;
                        let mappings = match parameter.name.as_str() {
                            "inputParameter" => &mut result.inputs,
                            "outputParameter" => &mut result.outputs,
                            _ => return Err(invalid("inputOutput accepts inputParameter/outputParameter only")),
                        };
                        add_mapping(mappings, target, value(parameter)?)?;
                    }
                }
                "in" | "out" => {
                    if node.name != "callActivity" { return Err(invalid("in/out mappings require callActivity")); }
                    if extension.attr("variables").is_some() || extension.attr("local").is_some() || extension.attr("businessKey").is_some() {
                        return Err(invalid("in/out supports source/sourceExpression and target only"));
                    }
                    let target = extension.attr("target").ok_or_else(|| invalid("in/out requires target"))?;
                    let mapped = match (extension.attr("source"), extension.attr("sourceExpression")) {
                        (Some(source), None) => MappingValue::Variable(reference(source)?),
                        (None, Some(source)) => {
                            if !source.trim().starts_with("${") && !source.trim().starts_with("#{") { return Err(invalid("sourceExpression requires a variable reference")); }
                            expression(source)?
                        }
                        _ => return Err(invalid("in/out requires exactly one source or sourceExpression")),
                    };
                    let mappings = if extension.name == "in" { &mut result.call_inputs } else { &mut result.call_outputs };
                    add_mapping(mappings.get_or_insert_with(Vec::new), target, mapped)?;
                }
                _ => {}
            }
        }
    }
    // Presence of either direction opts into explicit transfer on both sides.
    if result.call_inputs.is_some() || result.call_outputs.is_some() {
        result.call_inputs.get_or_insert_with(Vec::new);
        result.call_outputs.get_or_insert_with(Vec::new);
    }
    Ok(result)
}
fn inputs_and_outputs_conflict(id: &str, map: &HashMap<String, String>) -> bool { map.contains_key(id) }

fn apply_scope(scope: &Node, def: &mut ProcessDefinition, inherited: &HashMap<String, String>, root: bool) -> EngineResult<()> {
    let own = declarations(scope, def)?;
    if !root && (!own.is_empty() || scope.children("ioSpecification").next().is_some()) {
        return Err(invalid("subprocess data declarations require runtime scopes (X6)"));
    }
    let mut refs = inherited.clone();
    refs.extend(own);
    for node in &scope.children {
        if node.name == "subProcess" {
            apply_scope(node, def, &refs, false)?;
        }
        let has_data = node.children.iter().any(|n| matches!(n.name.as_str(), "dataInputAssociation" | "dataOutputAssociation" | "ioSpecification")) ||
            node.children("extensionElements").any(|e| e.children.iter().any(|n| matches!(n.name.as_str(), "inputOutput" | "in" | "out")));
        if !has_data { continue; }
        let id = node.id()?;
        if !matches!(def.nodes.get(id), Some(BpmnElement::UserTask(_) | BpmnElement::ServiceTask { .. } | BpmnElement::ScriptTask { .. } | BpmnElement::SendTask { .. } | BpmnElement::CallActivity { .. })) {
            return Err(invalid(format!("data mappings on '{}' require a supported task or Call Activity", node.name)));
        }
        def.data_mappings.insert(id.into(), activity(node, &refs)?);
    }
    if scope.children("ioSpecification").next().is_some() || scope.children("dataInputAssociation").next().is_some() || scope.children("dataOutputAssociation").next().is_some() {
        return Err(invalid("process/subprocess IO mappings are not supported"));
    }
    if scope.children("extensionElements").any(|e| e.children.iter().any(|n| matches!(n.name.as_str(), "inputOutput" | "in" | "out"))) {
        return Err(invalid("process/subprocess extension mappings are not supported"));
    }
    Ok(())
}

pub(crate) fn apply(xml: &str, def: &mut ProcessDefinition) -> EngineResult<()> {
    let document = tree(xml)?;
    let definitions = document.children.first().ok_or_else(|| invalid("missing definitions"))?;
    let process = definitions.children("process").find(|n| n.attr("id") == Some(def.id.as_str())).ok_or_else(|| invalid("missing process"))?;
    apply_scope(process, def, &HashMap::new(), true)
}
