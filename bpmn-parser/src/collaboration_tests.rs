use crate::{parse_bpmn_processes, parse_bpmn_xml};
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
