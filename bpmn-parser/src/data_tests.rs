use crate::parse_bpmn_xml;
use engine_core::domain::MappingValue;
use serde_json::json;
use std::collections::HashMap;

fn xml(declarations: &str, activity_data: &str) -> String {
    format!(
        r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn" xmlns:bpmninja="https://bpmninja.dev/schema/bpmn">
    <process id="data" isExecutable="true">{declarations}<startEvent id="start"/>
    <serviceTask id="act" camunda:topic="mapped">{activity_data}</serviceTask><endEvent id="end"/>
    <sequenceFlow id="f1" sourceRef="start" targetRef="act"/><sequenceFlow id="f2" sourceRef="act" targetRef="end"/>
    </process></definitions>"#
    )
}

#[test]
fn x11_defaults_declarations_and_nested_camunda_parameters() {
    let xml = xml(
        r#"<property id="customer" name="customerId" bpmninja:default="7"/><dataObject id="order" name="order"/>"#,
        r#"<extensionElements><camunda:inputOutput>
        <camunda:inputParameter name="id">${customerId}</camunda:inputParameter>
        <camunda:inputParameter name="options"><camunda:map>
          <camunda:entry key="labels"><camunda:list><camunda:value>a &amp; b</camunda:value><camunda:value>${customerId}</camunda:value></camunda:list></camunda:entry>
        </camunda:map></camunda:inputParameter>
        <camunda:outputParameter name="receipt">${result}</camunda:outputParameter>
        </camunda:inputOutput></extensionElements>"#,
    );
    let def = parse_bpmn_xml(&xml).unwrap();
    assert_eq!(def.variable_defaults["customerId"], json!(7));
    assert!(!def.variable_defaults.contains_key("order"));
    let mappings = &def.data_mappings["act"];
    assert_eq!(
        mappings.inputs[0].value,
        MappingValue::Variable("customerId".into())
    );
    assert_eq!(
        mappings.inputs[1]
            .value
            .evaluate(&HashMap::from([("customerId".into(), json!(7))]))
            .unwrap(),
        json!({"labels": ["a & b", 7]})
    );
    let roundtrip: engine_core::domain::ProcessDefinition =
        serde_json::from_str(&serde_json::to_string(&def).unwrap()).unwrap();
    assert_eq!(roundtrip.data_mappings, def.data_mappings);
}

#[test]
fn x11_standard_io_associations_resolve_ids_to_variable_names() {
    let def = parse_bpmn_xml(&xml(r#"<dataObject id="source" name="customerId"/><dataObjectReference id="alias" dataObjectRef="source"/><property id="receipt" name="receipt"/>"#,
        r#"<ioSpecification><dataInput id="in" name="id"/><dataOutput id="out" name="result"/><inputSet><dataInputRefs>in</dataInputRefs></inputSet><outputSet><dataOutputRefs>out</dataOutputRefs></outputSet></ioSpecification>
        <dataInputAssociation><sourceRef>alias</sourceRef><targetRef>in</targetRef></dataInputAssociation>
        <dataOutputAssociation><sourceRef>out</sourceRef><targetRef>receipt</targetRef></dataOutputAssociation>"#)).unwrap();
    assert_eq!(def.data_mappings["act"].inputs[0].target, "id");
    assert_eq!(def.data_mappings["act"].outputs[0].target, "receipt");
}

#[test]
fn x11_task_property_is_an_association_endpoint() {
    let def = parse_bpmn_xml(&xml(r#"<property id="source" name="customerId"/>"#,
        r#"<property id="local" name="id"/><dataInputAssociation><sourceRef>source</sourceRef><targetRef>local</targetRef></dataInputAssociation>"#)).unwrap();
    assert_eq!(def.data_mappings["act"].inputs[0].target, "id");
}

#[test]
fn x11_default_extension_resolves_namespace_aliases() {
    let document = xml(
        r#"<property id="source" name="source" bpmninja:default="[1,2]"/>"#,
        "",
    )
    .replace("bpmninja:", "ninja:")
    .replace("xmlns:bpmninja", "xmlns:ninja");
    assert_eq!(
        parse_bpmn_xml(&document).unwrap().variable_defaults["source"],
        json!([1, 2])
    );
}

#[test]
fn x11_call_in_out_are_explicit_transfers() {
    let xml = xml("", r#"<extensionElements><camunda:in source="id" target="childId"/><camunda:out sourceExpression="${answer}" target="receipt"/></extensionElements>"#)
        .replace("<serviceTask id=\"act\" camunda:topic=\"mapped\">", "<callActivity id=\"act\" calledElement=\"child\">")
        .replace("</serviceTask>", "</callActivity>");
    let def = parse_bpmn_xml(&xml).unwrap();
    let mappings = &def.data_mappings["act"];
    assert_eq!(mappings.call_inputs.as_ref().unwrap()[0].target, "childId");
    assert_eq!(mappings.call_outputs.as_ref().unwrap()[0].target, "receipt");
}

#[test]
fn x11_rejects_unsupported_or_ambiguous_data_semantics() {
    for contents in [
        r#"<extensionElements><camunda:inputOutput><camunda:inputParameter name="x">${a + b}</camunda:inputParameter></camunda:inputOutput></extensionElements>"#,
        r#"<extensionElements><camunda:inputOutput><camunda:inputParameter name="x"><camunda:script scriptFormat="javascript">1</camunda:script></camunda:inputParameter></camunda:inputOutput></extensionElements>"#,
        r#"<extensionElements><camunda:inputOutput><camunda:inputParameter name="x">a</camunda:inputParameter><camunda:inputParameter name="x">b</camunda:inputParameter></camunda:inputOutput></extensionElements>"#,
        r#"<dataInputAssociation><sourceRef>missing</sourceRef><targetRef>missing</targetRef></dataInputAssociation>"#,
        r#"<dataOutputAssociation><targetRef>missing</targetRef></dataOutputAssociation>"#,
        r#"<dataInputAssociation><sourceRef>missing</sourceRef><targetRef>missing</targetRef><transformation>1</transformation></dataInputAssociation>"#,
        r#"<extensionElements><camunda:in source="x" target="y"/></extensionElements>"#,
        r#"<ioSpecification><dataInput id="same"/><dataOutput id="same"/></ioSpecification>"#,
    ] {
        assert!(
            parse_bpmn_xml(&xml("", contents)).is_err(),
            "accepted: {contents}"
        );
    }
    assert!(
        parse_bpmn_xml(&xml(
            r#"<property id="source" bpmninja:default="invalid-json"/>"#,
            ""
        ))
        .is_err()
    );
}
