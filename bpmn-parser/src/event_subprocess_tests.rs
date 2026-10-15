use crate::parse_bpmn_xml;
use engine_core::domain::EventSubProcessTrigger;

fn xml(start: &str, flags: &str, body: &str) -> String {
    format!(
        r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
    <message id="m" name="alert"/><signal id="s" name="stop"/><error id="e" errorCode="FAIL"/><escalation id="x" escalationCode="WARN"/>
    <process id="p" isExecutable="true"><startEvent id="start"/><userTask id="main"/><endEvent id="end"/>
    <sequenceFlow id="f1" sourceRef="start" targetRef="main"/><sequenceFlow id="f2" sourceRef="main" targetRef="end"/>
    <subProcess id="handler" triggeredByEvent="true"><startEvent id="hs" {flags}>{start}</startEvent>
    {body}<endEvent id="he"/><sequenceFlow id="h1" sourceRef="hs" targetRef="work"/><sequenceFlow id="h2" sourceRef="work" targetRef="he"/>
    </subProcess></process></definitions>"#
    )
}
#[test]
fn x9a_parses_all_five_starts_without_creating_another_process_start() {
    for (start, flags) in [
        (
            r#"<messageEventDefinition messageRef="m"/>"#,
            r#"isInterrupting="false""#,
        ),
        (
            r#"<signalEventDefinition signalRef="s"/>"#,
            r#"isInterrupting="true""#,
        ),
        (
            r#"<timerEventDefinition><timeDuration>PT1S</timeDuration></timerEventDefinition>"#,
            r#"isInterrupting="false""#,
        ),
        (r#"<errorEventDefinition errorRef="e"/>"#, ""),
        (
            r#"<escalationEventDefinition escalationRef="x"/>"#,
            r#"isInterrupting="false""#,
        ),
    ] {
        let def = parse_bpmn_xml(&xml(start, flags, "<userTask id=\"work\"/>")).unwrap();
        assert_eq!(def.start_event().unwrap().0, "start");
        assert_eq!(def.event_subprocesses[0].start_node, "hs");
        assert!(def.nodes.contains_key("work"));
        let roundtrip: engine_core::domain::ProcessDefinition =
            serde_json::from_value(serde_json::to_value(&def).unwrap()).unwrap();
        assert_eq!(roundtrip.event_subprocesses, def.event_subprocesses);
        assert_eq!(roundtrip.start_event().unwrap().0, "start");
    }
    let def = parse_bpmn_xml(&xml(
        r#"<errorEventDefinition errorRef="e"/>"#,
        "",
        "<userTask id=\"work\"/>",
    ))
    .unwrap();
    assert_eq!(
        def.event_subprocesses[0].trigger,
        EventSubProcessTrigger::Error(Some("FAIL".into()))
    );
}
#[test]
fn x9a_rejects_illegal_starts_and_scopes() {
    for (start, flags) in [
        (
            r#"<errorEventDefinition errorRef="e"/>"#,
            r#"isInterrupting="false""#,
        ),
        (r#"<conditionalEventDefinition/>"#, ""),
        (r#"<compensateEventDefinition/>"#, ""),
        (r#"<signalEventDefinition signalRef="missing"/>"#, ""),
        (r#"<messageEventDefinition/>"#, ""),
        (
            r#"<messageEventDefinition messageRef="m"/><signalEventDefinition signalRef="s"/>"#,
            "",
        ),
        ("", ""),
    ] {
        assert!(parse_bpmn_xml(&xml(start, flags, "<userTask id=\"work\"/>")).is_err());
    }
    let document = xml(
        r#"<messageEventDefinition messageRef="m"/>"#,
        "",
        "<userTask id=\"work\"/>",
    );
    assert!(
        parse_bpmn_xml(
            &document
                .replace(
                    "<subProcess id=\"handler\"",
                    "<subProcess id=\"outer\"><subProcess id=\"handler\""
                )
                .replace("</subProcess>", "</subProcess></subProcess>")
        )
        .is_err()
    );
    assert!(
        parse_bpmn_xml(&document.replace(
            "</process>",
            r#"<sequenceFlow id="bad" sourceRef="main" targetRef="handler"/></process>"#
        ))
        .is_err()
    );
    assert!(
        parse_bpmn_xml(&document.replace(
            "<userTask id=\"work\"/>",
            "<property id=\"local\"/><userTask id=\"work\"/>"
        ))
        .is_err()
    );
}
#[test]
fn x9a_handler_tasks_retain_x11_mappings() {
    let task = r#"<serviceTask id="work" camunda:topic="mapped"><extensionElements><camunda:inputOutput><camunda:inputParameter name="id">${customerId}</camunda:inputParameter><camunda:outputParameter name="receipt">${result}</camunda:outputParameter></camunda:inputOutput></extensionElements></serviceTask>"#;
    let def = parse_bpmn_xml(&xml(
        r#"<messageEventDefinition messageRef="m"/>"#,
        r#"isInterrupting="false""#,
        task,
    ))
    .unwrap();
    assert_eq!(def.data_mappings["work"].inputs[0].target, "id");
    assert_eq!(def.data_mappings["work"].outputs[0].target, "receipt");
}
