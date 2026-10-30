use crate::{
    CollaborationDocument, parse_bpmn_collaboration, parse_bpmn_processes, parse_bpmn_xml,
};
use engine_core::domain::BpmnElement;

fn pool(id: &str, executable: &str, start: &str) -> String {
    format!(
        r#"<process id="{id}" {executable}><startEvent id="{id}_s">{start}</startEvent><endEvent id="{id}_e"/><sequenceFlow id="{id}_f" sourceRef="{id}_s" targetRef="{id}_e"/></process>"#
    )
}

fn document(pools: &str) -> String {
    format!(
        r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"><message id="order" name="Order"/>{pools}</definitions>"#
    )
}

#[test]
fn executable_pools_keep_order_and_shared_message_references() {
    let xml = document(&format!(
        "{}{}{}",
        pool("external", r#"isExecutable="false""#, ""),
        pool("sender", r#"isExecutable="true""#, ""),
        pool(
            "receiver",
            r#"isExecutable="true""#,
            r#"<messageEventDefinition messageRef="order"/>"#,
        ),
    ));
    let definitions = parse_bpmn_processes(&xml).unwrap();
    assert_eq!(
        definitions
            .iter()
            .map(|d| d.id.as_str())
            .collect::<Vec<_>>(),
        ["sender", "receiver"]
    );
    assert!(!definitions[0].nodes.contains_key("receiver_s"));
    assert!(
        matches!(&definitions[1].nodes["receiver_s"], BpmnElement::MessageStartEvent { message_name } if message_name == "Order")
    );
    assert_eq!(parse_bpmn_xml(&xml).unwrap().id, "sender");
}

#[test]
fn invalid_later_executable_pool_fails_the_whole_parse() {
    let invalid = r#"<process id="invalid" isExecutable="true"><startEvent id="bad"><conditionalEventDefinition><condition/></conditionalEventDefinition></startEvent></process>"#;
    let xml = document(&format!(
        "{}{invalid}",
        pool("valid", r#"isExecutable="true""#, "")
    ));
    assert!(parse_bpmn_xml(&xml).is_ok());
    assert!(parse_bpmn_processes(&xml).is_err());
    // Unsupported semantics in an explicitly non-executable pool are ignored.
    assert!(
        parse_bpmn_processes(&xml.replace(
            r#"id="invalid" isExecutable="true""#,
            r#"id="invalid" isExecutable="false""#
        ))
        .is_ok()
    );
}

#[test]
fn collection_requires_explicit_execution_but_legacy_fallback_remains() {
    for executable in ["", r#"isExecutable="false""#] {
        let xml = document(&pool("legacy", executable, ""));
        assert!(parse_bpmn_processes(&xml).is_err());
        assert_eq!(parse_bpmn_xml(&xml).unwrap().id, "legacy");
    }
    assert!(parse_bpmn_processes(&document("")).is_err());
}

#[test]
fn collection_rejects_duplicate_and_empty_process_ids() {
    for other in ["one", ""] {
        let xml = document(&format!(
            "{}{}",
            pool("one", r#"isExecutable="true""#, ""),
            pool(other, r#"isExecutable="false""#, "")
        ));
        assert!(parse_bpmn_processes(&xml).is_err());
    }
}

fn collaboration_xml() -> String {
    document(&format!(
        r#"<process id="sender" isExecutable="true"><startEvent id="ss"/><sendTask id="send" messageRef="order"/><endEvent id="se"/><sequenceFlow id="sf1" sourceRef="ss" targetRef="send"/><sequenceFlow id="sf2" sourceRef="send" targetRef="se"/></process>{}
        <collaboration id="collaboration"><participant id="seller" processRef="sender"/><participant id="buyer" processRef="receiver"/><participant id="external"/>
        <messageFlow id="delivery" sourceRef="send" targetRef="receiver_s" messageRef="order"/></collaboration>"#,
        pool(
            "receiver",
            r#"isExecutable="true""#,
            r#"<messageEventDefinition messageRef="order"/>"#
        ),
    ))
}

fn rejected(xml: &str, reason: &str) {
    let error = parse_bpmn_collaboration(xml).unwrap_err().to_string();
    assert!(error.contains(reason), "expected {reason:?}, got {error}");
}

#[test]
fn collaboration_routes_preserve_pool_element_and_message_identity() {
    let parsed = parse_bpmn_collaboration(&collaboration_xml()).unwrap();
    assert_eq!(parsed.processes.len(), 2);
    let collaboration = &parsed.collaborations[0];
    assert_eq!(collaboration.id, "collaboration");
    assert_eq!(collaboration.participants[2].process_id, None);
    assert!(!collaboration.participants[2].executable);
    let flow = &collaboration.message_flows[0];
    assert_eq!(flow.message_id, "order");
    assert_eq!(flow.message_name, "Order");
    assert_eq!(flow.source.process_id, "sender");
    assert_eq!(flow.source.participant_id, "seller");
    assert_eq!(flow.source.element_id, "send");
    assert_eq!(flow.target.process_id, "receiver");
    assert_eq!(flow.target.participant_id, "buyer");
    assert_eq!(flow.target.element_id, "receiver_s");
    let restored: CollaborationDocument =
        serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(parsed.collaborations, restored.collaborations);
    assert_eq!(restored.processes[1].id, "receiver");
}

#[test]
fn collaboration_infers_optional_flow_message_but_never_endpoint_messages() {
    let xml = collaboration_xml();
    assert!(
        parse_bpmn_collaboration(&xml.replace(
            r#"targetRef="receiver_s" messageRef="order""#,
            r#"targetRef="receiver_s""#
        ))
        .is_ok()
    );
    rejected(
        &xml.replace(
            r#"<sendTask id="send" messageRef="order"/>"#,
            r#"<sendTask id="send" name="Order"/>"#,
        ),
        "requires messageRef",
    );
    rejected(
        &xml.replace(
            r#"targetRef="receiver_s" messageRef="order""#,
            r#"targetRef="receiver_s" messageRef="other""#,
        ),
        "differs from its endpoints",
    );
}

#[test]
fn collaboration_validates_message_declarations_and_identity() {
    let xml = collaboration_xml();
    rejected(
        &xml.replace(r#"messageRef="order""#, r#"messageRef="missing""#),
        "unknown message",
    );
    // Identical names do not make distinct BPMN message declarations equivalent.
    let different = xml
        .replace(
            "</definitions>",
            r#"<message id="other" name="Order"/></definitions>"#,
        )
        .replace(
            r#"<messageEventDefinition messageRef="order"/>"#,
            r#"<messageEventDefinition messageRef="other"/>"#,
        );
    rejected(&different, "different messages");
    rejected(
        &xml.replace(r#"name="Order""#, r#"name=" ""#),
        "requires name",
    );
    rejected(&xml.replace(r#"<sendTask id="send" messageRef="order"/>"#, r#"<sendTask id="send" messageRef="other"><messageEventDefinition messageRef="order"/></sendTask>"#), "conflicting message references");
}

#[test]
fn collaboration_rejects_unresolved_or_ambiguous_pool_references() {
    let xml = collaboration_xml();
    rejected(
        &xml.replace(r#"processRef="receiver""#, r#"processRef="missing""#),
        "unknown process",
    );
    rejected(
        &xml.replace(
            r#"<participant id="external"/>"#,
            r#"<participant id="external" processRef="receiver"/>"#,
        ),
        "multiple participants",
    );
    rejected(
        &xml.replace(r#"<participant id="buyer" processRef="receiver"/>"#, ""),
        "no participant",
    );
    rejected(
        &xml.replace(
            r#"id="receiver" isExecutable="true""#,
            r#"id="receiver" isExecutable="false""#,
        ),
        "non-executable pool",
    );
    rejected(
        &xml.replace(r#"targetRef="receiver_s""#, r#"targetRef="external""#),
        "participant and nested endpoints",
    );
    rejected(
        &xml.replace(r#"targetRef="receiver_s""#, r#"targetRef="missing""#),
        "must reference a top-level",
    );
}

#[test]
fn collaboration_rejects_wrong_direction_and_same_pool_routes() {
    let xml = collaboration_xml();
    rejected(
        &xml.replace(
            r#"sourceRef="send" targetRef="receiver_s""#,
            r#"sourceRef="receiver_s" targetRef="send""#,
        ),
        "unsupported sourceRef kind",
    );
    rejected(
        &xml.replace(
            r#"targetRef="receiver_s" messageRef"#,
            r#"targetRef="ss" messageRef"#,
        ),
        "unsupported targetRef kind",
    );
    let same_pool = xml.replace(r#"<endEvent id="se"/>"#, r#"<endEvent id="se"/><intermediateCatchEvent id="local"><messageEventDefinition messageRef="order"/></intermediateCatchEvent><sequenceFlow id="local_exit" sourceRef="local" targetRef="se"/>"#)
        .replace(r#"targetRef="receiver_s" messageRef="order""#, r#"targetRef="local" messageRef="order""#);
    rejected(&same_pool, "must cross participant processes");
}

#[test]
fn collaboration_ids_are_unique_across_pools_and_metadata() {
    let xml = collaboration_xml();
    rejected(
        &xml.replace(r#"id="receiver_e""#, r#"id="send""#),
        "duplicate document ID",
    );
    rejected(
        &xml.replace(r#"id="buyer""#, r#"id="seller""#),
        "duplicate document ID",
    );
    rejected(
        &xml.replace(r#"id="delivery""#, r#"id="order""#),
        "duplicate document ID",
    );
    rejected(
        &xml.replace(r#"processRef="receiver""#, r#"processRef="tns:receiver""#),
        "unqualified local ID",
    );
    rejected(
        &xml.replace(
            r#"targetRef="receiver_s" messageRef="order""#,
            r#"targetRef=" receiver_s " messageRef="order""#,
        ),
        "unqualified local ID",
    );
}

#[test]
fn collaboration_accepts_throw_receive_catch_and_boundary_endpoints() {
    let xml = collaboration_xml();
    let throw = xml.replace(r#"<sendTask id="send" messageRef="order"/>"#, r#"<intermediateThrowEvent id="send"><messageEventDefinition messageRef="order"/></intermediateThrowEvent>"#);
    assert!(parse_bpmn_collaboration(&throw).is_ok());
    let receive = xml.replace(
        r#"<startEvent id="receiver_s"><messageEventDefinition messageRef="order"/></startEvent>"#,
        r#"<receiveTask id="receiver_s" instantiate="true" messageRef="order"/>"#,
    );
    assert!(parse_bpmn_collaboration(&receive).is_ok());
    for endpoint in [
        r#"<receiveTask id="wait" messageRef="order"/>"#,
        r#"<intermediateCatchEvent id="wait"><messageEventDefinition messageRef="order"/></intermediateCatchEvent>"#,
        r#"<userTask id="work"/><sequenceFlow id="work_exit" sourceRef="work" targetRef="receiver_e"/><boundaryEvent id="wait" attachedToRef="work"><messageEventDefinition messageRef="order"/></boundaryEvent>"#,
    ] {
        let input = xml
            .replace(
                r#"<endEvent id="receiver_e"/>"#,
                &format!(r#"{endpoint}<endEvent id="receiver_e"/><sequenceFlow id="wait_exit" sourceRef="wait" targetRef="receiver_e"/>"#),
            )
            .replace(
                r#"targetRef="receiver_s" messageRef="order""#,
                r#"targetRef="wait" messageRef="order""#,
            );
        assert!(parse_bpmn_collaboration(&input).is_ok(), "{input}");
    }
}

#[test]
fn collaboration_rejects_loop_and_nested_scope_routes() {
    let xml = collaboration_xml();
    let looped = xml.replace(r#"<sendTask id="send" messageRef="order"/>"#, r#"<sendTask id="send" messageRef="order"><standardLoopCharacteristics><loopCondition>again</loopCondition></standardLoopCharacteristics></sendTask>"#);
    rejected(&looped, "standardLoopCharacteristics");
    // Nur die äußeren Sequenzflüsse zeigen auf den Sub-Prozess. Der Send-Task
    // bleibt verschachtelt und ist deshalb kein Top-Level-Endpunkt.
    let nested = xml
        .replace(
            r#"<sendTask id="send" messageRef="order"/>"#,
            r#"<subProcess id="nested"><startEvent id="ns"/><sendTask id="send" messageRef="order"/><endEvent id="ne"/><sequenceFlow id="nf1" sourceRef="ns" targetRef="send"/><sequenceFlow id="nf2" sourceRef="send" targetRef="ne"/></subProcess>"#,
        )
        .replace(
            r#"<sequenceFlow id="sf1" sourceRef="ss" targetRef="send"/>"#,
            r#"<sequenceFlow id="sf1" sourceRef="ss" targetRef="nested"/>"#,
        )
        .replace(
            r#"<sequenceFlow id="sf2" sourceRef="send" targetRef="se"/>"#,
            r#"<sequenceFlow id="sf2" sourceRef="nested" targetRef="se"/>"#,
        );
    rejected(&nested, "participant and nested endpoints");
}
