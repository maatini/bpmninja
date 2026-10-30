use crate::parser::parse_bpmn_xml;
use engine_core::model::{BpmnElement, EngineError, ListenerEvent};

#[test]
fn parse_simple_bpmn() {
    let xml = r#"
        <bpmn:definitions id="def1" xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start1" />
                <serviceTask id="svc1" data-handler="my_handler" />
                <userTask id="ut1" data-assignee="alice" />
                <endEvent id="end1" />
                <sequenceFlow id="f1" sourceRef="start1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="ut1" />
                <sequenceFlow id="f3" sourceRef="ut1" targetRef="end1" />
            </process>
        </bpmn:definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert_eq!(def.id, "proc1");
    assert!(def.nodes.contains_key("start1"));
    assert!(def.nodes.contains_key("svc1"));
    assert!(def.nodes.contains_key("ut1"));
    assert!(def.nodes.contains_key("end1"));

    assert_eq!(def.next_node("start1"), Some("svc1"));
    assert_eq!(def.next_node("svc1"), Some("ut1"));
    assert_eq!(def.next_node("ut1"), Some("end1"));

    match def.nodes.get("svc1").unwrap() {
        BpmnElement::ServiceTask { topic, .. } => assert_eq!(topic, "my_handler"),
        _ => panic!("Expected ServiceTask"),
    }

    match def.nodes.get("ut1").unwrap() {
        BpmnElement::UserTask(a) => assert_eq!(a, "alice"),
        _ => panic!("Expected UserTask"),
    }
}

#[test]
fn parse_conditional_flows() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start1" />
                <exclusiveGateway id="gw1" />
                <endEvent id="end1" />
                <endEvent id="end2" />
                
                <sequenceFlow id="f1" sourceRef="start1" targetRef="gw1" />
                
                <sequenceFlow id="f2" sourceRef="gw1" targetRef="end1">
                    <conditionExpression xsi:type="tFormalExpression">amount &gt; 100</conditionExpression>
                </sequenceFlow>
                
                <sequenceFlow id="f3" sourceRef="gw1" targetRef="end2" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();

    // Gateway must be parsed as ExclusiveGateway, NOT ServiceTask
    match def.nodes.get("gw1").unwrap() {
        BpmnElement::ExclusiveGateway { default } => assert_eq!(*default, None),
        other => panic!("Expected ExclusiveGateway, got {:?}", other),
    }

    let flows = def.next_nodes("gw1");
    assert_eq!(flows.len(), 2);

    let flow1 = flows.iter().find(|f| f.target == "end1").unwrap();
    assert_eq!(flow1.condition, Some("amount > 100".to_string()));

    let flow2 = flows.iter().find(|f| f.target == "end2").unwrap();
    assert_eq!(flow2.condition, None);
}

#[test]
fn reject_invalid_condition_expression() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start1" />
                <exclusiveGateway id="gw1" />
                <userTask id="ut1" data-assignee="alice" />
                <userTask id="ut2" data-assignee="bob" />
                <endEvent id="end1" />
                <sequenceFlow id="f1" sourceRef="start1" targetRef="gw1" />
                <sequenceFlow id="f2" sourceRef="gw1" targetRef="ut1">
                    <conditionExpression xsi:type="tFormalExpression">${a &amp;&amp;}</conditionExpression>
                </sequenceFlow>
                <sequenceFlow id="f3" sourceRef="gw1" targetRef="ut2" />
                <sequenceFlow id="f4" sourceRef="ut1" targetRef="end1" />
                <sequenceFlow id="f5" sourceRef="ut2" targetRef="end1" />
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err();
    assert!(
        matches!(err, EngineError::InvalidDefinition(_)),
        "expected InvalidDefinition, got: {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("gw1") && msg.contains("ut1") && msg.contains("position"),
        "error should name source, target and position: {msg}"
    );
}

#[test]
fn reject_invalid_join_condition_expression() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start1" />
                <complexGateway id="cgw">
                    <activationCondition xsi:type="tFormalExpression">tokens &gt;</activationCondition>
                </complexGateway>
                <endEvent id="end1" />
                <sequenceFlow id="f1" sourceRef="start1" targetRef="cgw" />
                <sequenceFlow id="f2" sourceRef="cgw" targetRef="end1" />
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err();
    assert!(
        matches!(err, EngineError::InvalidDefinition(_)),
        "expected InvalidDefinition, got: {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("complex gateway 'cgw'") && msg.contains("position"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_exclusive_gateway_with_default() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start1" />
                <exclusiveGateway id="gw1" default="f3" />
                <userTask id="ut1" data-assignee="alice" />
                <userTask id="ut2" data-assignee="bob" />
                <endEvent id="end1" />
                
                <sequenceFlow id="f1" sourceRef="start1" targetRef="gw1" />
                <sequenceFlow id="f2" sourceRef="gw1" targetRef="ut1">
                    <conditionExpression xsi:type="tFormalExpression">x &gt; 0</conditionExpression>
                </sequenceFlow>
                <sequenceFlow id="f3" sourceRef="gw1" targetRef="ut2" />
                <sequenceFlow id="f4" sourceRef="ut1" targetRef="end1" />
                <sequenceFlow id="f5" sourceRef="ut2" targetRef="end1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();

    // Default attribute must resolve flow "f3" → target "ut2"
    match def.nodes.get("gw1").unwrap() {
        BpmnElement::ExclusiveGateway { default } => {
            assert_eq!(default.as_deref(), Some("ut2"));
        }
        other => panic!("Expected ExclusiveGateway, got {:?}", other),
    }

    // User tasks must be parsed correctly
    match def.nodes.get("ut1").unwrap() {
        BpmnElement::UserTask(a) => assert_eq!(a, "alice"),
        other => panic!("Expected UserTask, got {:?}", other),
    }
    match def.nodes.get("ut2").unwrap() {
        BpmnElement::UserTask(a) => assert_eq!(a, "bob"),
        other => panic!("Expected UserTask, got {:?}", other),
    }
}

#[test]
fn parse_timer_start() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start1">
                    <timerEventDefinition>
                        <timeDuration>PT60S</timeDuration>
                    </timerEventDefinition>
                </startEvent>
                <endEvent id="end1" />
                <sequenceFlow id="f1" sourceRef="start1" targetRef="end1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("start1").unwrap() {
        BpmnElement::TimerStartEvent(engine_core::timer_definition::TimerDefinition::Duration(
            d,
        )) => assert_eq!(d.as_secs(), 60),
        _ => panic!("Expected TimerStartEvent"),
    }
}

/// Regression test: bpmn-js generates interleaved elements, e.g.
/// `startEvent`, `sequenceFlow`, `serviceTask`, `sequenceFlow`, `endEvent`.
/// quick-xml 0.31 rejected this as "duplicate field `sequenceFlow`".
/// Fixed by upgrading to quick-xml 0.37 with `overlapped-lists` feature.
#[test]
fn parse_interleaved_bpmn_js_output() {
    let xml = r#"
        <bpmn2:definitions id="Definitions_1" xmlns:bpmn2="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <bpmn2:process id="Process_1" isExecutable="true">
                <bpmn2:startEvent id="StartEvent_1" />
                <bpmn2:sequenceFlow id="Flow_1" sourceRef="StartEvent_1" targetRef="ServiceTask_1" />
                <bpmn2:serviceTask id="ServiceTask_1" data-handler="validate" />
                <bpmn2:sequenceFlow id="Flow_2" sourceRef="ServiceTask_1" targetRef="UserTask_1" />
                <bpmn2:userTask id="UserTask_1" data-assignee="admin" />
                <bpmn2:sequenceFlow id="Flow_3" sourceRef="UserTask_1" targetRef="EndEvent_1" />
                <bpmn2:endEvent id="EndEvent_1" />
            </bpmn2:process>
        </bpmn2:definitions>
    "#;

    let def = parse_bpmn_xml(xml).expect("should parse interleaved BPMN XML");
    assert_eq!(def.id, "Process_1");
    assert_eq!(def.nodes.len(), 4);
    assert_eq!(def.flows.len(), 3);
    assert_eq!(def.next_node("StartEvent_1"), Some("ServiceTask_1"));
    assert_eq!(def.next_node("ServiceTask_1"), Some("UserTask_1"));
    assert_eq!(def.next_node("UserTask_1"), Some("EndEvent_1"));
}

#[test]
fn test_parse_execution_listeners_and_scripts() {
    let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Definitions_1">
  <bpmn:process id="Process_1" isExecutable="true">
    <bpmn:extensionElements>
      <bpmn:executionListener event="start">
        <bpmn:script scriptFormat="rhai">
          print("Process Started");
        </bpmn:script>
      </bpmn:executionListener>
    </bpmn:extensionElements>
    
    <bpmn:startEvent id="Start_1" />
    <bpmn:sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_1" />
    
    <bpmn:serviceTask id="Task_1">
      <bpmn:extensionElements>
        <bpmn:executionListener event="end">
          <bpmn:script scriptFormat="rhai">
            print("Task Ended");
          </bpmn:script>
        </bpmn:executionListener>
      </bpmn:extensionElements>
    </bpmn:serviceTask>
    
    <bpmn:sequenceFlow id="Flow_2" sourceRef="Task_1" targetRef="End_1" />
    <bpmn:endEvent id="End_1" />
  </bpmn:process>
</bpmn:definitions>
"#;
    let p = parse_bpmn_xml(xml).expect("Should parse");

    let mut process_listeners = p.listeners.get("Process_1").cloned().unwrap_or_default();
    process_listeners.sort_by_key(|l| match l.event {
        ListenerEvent::Start => 1,
        ListenerEvent::End => 2,
    });

    assert_eq!(process_listeners.len(), 1);
    assert!(matches!(process_listeners[0].event, ListenerEvent::Start));
    assert_eq!(process_listeners[0].script, "print(\"Process Started\");");

    let task_listeners = p.listeners.get("Task_1").cloned().unwrap_or_default();
    assert_eq!(task_listeners.len(), 1);
    assert!(matches!(task_listeners[0].event, ListenerEvent::End));
    assert_eq!(task_listeners[0].script, "print(\"Task Ended\");");
}

// ---- WP-5: Additional test coverage ----

#[test]
fn parse_parallel_gateway() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <parallelGateway id="fork" />
                <serviceTask id="a" data-topic="task_a" />
                <serviceTask id="b" data-topic="task_b" />
                <parallelGateway id="join" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="fork" />
                <sequenceFlow id="f2" sourceRef="fork" targetRef="a" />
                <sequenceFlow id="f3" sourceRef="fork" targetRef="b" />
                <sequenceFlow id="f4" sourceRef="a" targetRef="join" />
                <sequenceFlow id="f5" sourceRef="b" targetRef="join" />
                <sequenceFlow id="f6" sourceRef="join" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert!(matches!(
        def.nodes.get("fork").unwrap(),
        BpmnElement::ParallelGateway
    ));
    assert!(matches!(
        def.nodes.get("join").unwrap(),
        BpmnElement::ParallelGateway
    ));
    assert_eq!(def.next_nodes("fork").len(), 2);
}

#[test]
fn parse_inclusive_gateway() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <inclusiveGateway id="gw1" />
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="gw1" />
                <sequenceFlow id="f2" sourceRef="gw1" targetRef="e1">
                    <conditionExpression xsi:type="tFormalExpression">x &gt; 0</conditionExpression>
                </sequenceFlow>
                <sequenceFlow id="f3" sourceRef="gw1" targetRef="e2">
                    <conditionExpression xsi:type="tFormalExpression">y &gt; 0</conditionExpression>
                </sequenceFlow>
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert!(matches!(
        def.nodes.get("gw1").unwrap(),
        BpmnElement::InclusiveGateway
    ));
    assert_eq!(def.next_nodes("gw1").len(), 2);
}

#[test]
fn parse_message_start_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <message id="msg1" name="OrderReceived" />
            <process id="proc1">
                <startEvent id="s1">
                    <messageEventDefinition messageRef="msg1" />
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("s1").unwrap() {
        BpmnElement::MessageStartEvent { message_name } => {
            assert_eq!(message_name, "OrderReceived");
        }
        other => panic!("Expected MessageStartEvent, got {:?}", other),
    }
}

#[test]
fn parse_message_catch_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <message id="msg1" name="PaymentConfirmed" />
            <process id="proc1">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait_payment">
                    <messageEventDefinition messageRef="msg1" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait_payment" />
                <sequenceFlow id="f2" sourceRef="wait_payment" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("wait_payment").unwrap() {
        BpmnElement::MessageCatchEvent { message_name } => {
            assert_eq!(message_name, "PaymentConfirmed");
        }
        other => panic!("Expected MessageCatchEvent, got {:?}", other),
    }
}

#[test]
fn parse_error_end_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <error id="err1" errorCode="VALIDATION_FAILED" />
            <process id="proc1">
                <startEvent id="s1" />
                <endEvent id="e1">
                    <errorEventDefinition errorRef="err1" />
                </endEvent>
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("e1").unwrap() {
        BpmnElement::ErrorEndEvent { error_code } => {
            assert_eq!(error_code, "VALIDATION_FAILED");
        }
        other => panic!("Expected ErrorEndEvent, got {:?}", other),
    }
}

#[test]
fn parse_timer_catch_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait">
                    <timerEventDefinition>
                        <timeDuration>PT30S</timeDuration>
                    </timerEventDefinition>
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait" />
                <sequenceFlow id="f2" sourceRef="wait" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("wait").unwrap() {
        BpmnElement::TimerCatchEvent(engine_core::timer_definition::TimerDefinition::Duration(
            d,
        )) => assert_eq!(d.as_secs(), 30),
        other => panic!("Expected TimerCatchEvent, got {:?}", other),
    }
}

#[test]
fn parse_boundary_timer_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <serviceTask id="task1" data-topic="long_task" />
                <boundaryEvent id="timeout" attachedToRef="task1" cancelActivity="true">
                    <timerEventDefinition>
                        <timeDuration>PT5M</timeDuration>
                    </timerEventDefinition>
                </boundaryEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="task1" />
                <sequenceFlow id="f2" sourceRef="task1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="timeout" targetRef="e2" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("timeout").unwrap() {
        BpmnElement::BoundaryTimerEvent {
            attached_to,
            timer,
            cancel_activity,
        } => {
            assert_eq!(attached_to, "task1");
            match timer {
                engine_core::timer_definition::TimerDefinition::Duration(d) => {
                    assert_eq!(d.as_secs(), 300)
                }
                _ => panic!("Expected Duration timer"),
            }
            assert!(*cancel_activity);
        }
        other => panic!("Expected BoundaryTimerEvent, got {:?}", other),
    }
}

#[test]
fn parse_boundary_error_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <error id="err1" errorCode="PAYMENT_FAILED" />
            <process id="proc1">
                <startEvent id="s1" />
                <serviceTask id="task1" data-topic="charge" />
                <boundaryEvent id="on_error" attachedToRef="task1">
                    <errorEventDefinition errorRef="err1" />
                </boundaryEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="task1" />
                <sequenceFlow id="f2" sourceRef="task1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="on_error" targetRef="e2" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("on_error").unwrap() {
        BpmnElement::BoundaryErrorEvent {
            attached_to,
            error_code,
        } => {
            assert_eq!(attached_to, "task1");
            assert_eq!(error_code.as_deref(), Some("PAYMENT_FAILED"));
        }
        other => panic!("Expected BoundaryErrorEvent, got {:?}", other),
    }
}

#[test]
fn parse_picks_executable_process() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="collab_proc" isExecutable="false">
                <startEvent id="cs1" />
                <endEvent id="ce1" />
                <sequenceFlow id="cf1" sourceRef="cs1" targetRef="ce1" />
            </process>
            <process id="main_proc" isExecutable="true">
                <startEvent id="s1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert_eq!(def.id, "main_proc", "Should pick the executable process");
}

#[test]
fn parse_duration_rejects_invalid_input() {
    // P1D (days not supported), empty, garbage — all must fail
    let template = |dur: &str| {
        format!(
            r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1">
                    <timerEventDefinition>
                        <timeDuration>{}</timeDuration>
                    </timerEventDefinition>
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>"#,
            dur
        )
    };

    // Valid inputs must still work
    assert!(parse_bpmn_xml(&template("PT5S")).is_ok());
    assert!(parse_bpmn_xml(&template("PT1H30M")).is_ok());
    assert!(parse_bpmn_xml(&template("PT0S")).is_ok());

    // Invalid inputs must now fail
    // P1D is now supported
    assert!(
        parse_bpmn_xml(&template("P1D")).is_ok(),
        "P1D should be accepted"
    );
    assert!(
        parse_bpmn_xml(&template("")).is_err(),
        "Empty duration should be rejected"
    );
    assert!(
        parse_bpmn_xml(&template("HELLO")).is_err(),
        "Garbage should be rejected"
    );
    assert!(
        parse_bpmn_xml(&template("PT")).is_err(),
        "PT without value should be rejected"
    );
}

#[test]
fn parse_event_based_gateway() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <message id="msg1" name="Approval" />
            <process id="proc1">
                <startEvent id="s1" />
                <eventBasedGateway id="ebgw" />
                <intermediateCatchEvent id="wait_msg">
                    <messageEventDefinition messageRef="msg1" />
                </intermediateCatchEvent>
                <intermediateCatchEvent id="wait_timer">
                    <timerEventDefinition>
                        <timeDuration>PT30S</timeDuration>
                    </timerEventDefinition>
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="ebgw" />
                <sequenceFlow id="f2" sourceRef="ebgw" targetRef="wait_msg" />
                <sequenceFlow id="f3" sourceRef="ebgw" targetRef="wait_timer" />
                <sequenceFlow id="f4" sourceRef="wait_msg" targetRef="e1" />
                <sequenceFlow id="f5" sourceRef="wait_timer" targetRef="e2" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert!(
        matches!(
            def.nodes.get("ebgw").unwrap(),
            BpmnElement::EventBasedGateway
        ),
        "Expected EventBasedGateway"
    );
    assert_eq!(def.next_nodes("ebgw").len(), 2);
    // Verify the targets are correct event types
    assert!(matches!(
        def.nodes.get("wait_msg").unwrap(),
        BpmnElement::MessageCatchEvent { .. }
    ));
    assert!(matches!(
        def.nodes.get("wait_timer").unwrap(),
        BpmnElement::TimerCatchEvent(_)
    ));
}

#[test]
fn reject_event_subprocess() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <error id="err1" errorCode="VALIDATION_FAILED" />
            <process id="proc1">
                <startEvent id="s1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
                <subProcess id="sub1" triggeredByEvent="true">
                    <startEvent id="sub_s1">
                        <errorEventDefinition errorRef="err1" />
                    </startEvent>
                    <endEvent id="sub_e1" />
                    <sequenceFlow id="sub_f1" sourceRef="sub_s1" targetRef="sub_e1" />
                </subProcess>
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("event sub-process 'sub1'") && err.contains("triggeredByEvent"),
        "event sub-process must be rejected: {err}"
    );
    assert!(
        err.contains("errorEventDefinition on startEvent 'sub_s1'"),
        "error start inside the event sub-process is also unsupported: {err}"
    );
}

#[test]
fn parse_regular_subprocess_accepted() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <endEvent id="sub_e1" />
                    <sequenceFlow id="sub_f1" sourceRef="sub_s1" targetRef="sub_e1" />
                </subProcess>
            </process>
        </definitions>
    "#;

    // Regular embedded sub-processes are now fully supported.
    let result = parse_bpmn_xml(xml);
    assert!(
        result.is_ok(),
        "Regular embedded sub-process should be parsed"
    );
}

#[test]
fn parse_time_date() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1">
                    <timerEventDefinition>
                        <timeDate>2026-04-06T14:30:00Z</timeDate>
                    </timerEventDefinition>
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("s1").unwrap() {
        BpmnElement::TimerStartEvent(
            engine_core::timer_definition::TimerDefinition::AbsoluteDate(dt),
        ) => {
            assert_eq!(dt.to_rfc3339(), "2026-04-06T14:30:00+00:00");
        }
        _ => panic!("Expected TimerStartEvent with AbsoluteDate"),
    }
}

#[test]
fn parse_time_cycle_cron() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1">
                    <timerEventDefinition>
                        <timeCycle>*/5 * * * *</timeCycle>
                    </timerEventDefinition>
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("s1").unwrap() {
        BpmnElement::TimerStartEvent(
            engine_core::timer_definition::TimerDefinition::CronCycle {
                expression,
                max_repetitions,
            },
        ) => {
            assert_eq!(expression, "*/5 * * * *");
            assert_eq!(*max_repetitions, None);
        }
        _ => panic!("Expected TimerStartEvent with CronCycle"),
    }
}

#[test]
fn parse_time_cycle_repeating_interval() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1">
                    <timerEventDefinition>
                        <timeCycle>R3/PT10H</timeCycle>
                    </timerEventDefinition>
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("s1").unwrap() {
        BpmnElement::TimerStartEvent(
            engine_core::timer_definition::TimerDefinition::RepeatingInterval {
                repetitions,
                interval,
            },
        ) => {
            assert_eq!(*repetitions, Some(3));
            assert_eq!(interval.as_secs(), 36000);
        }
        _ => panic!("Expected TimerStartEvent with RepeatingInterval"),
    }
}
#[test]
fn parse_time_cycle_repeating_interval_compact() {
    // Compact format without slash: R3PT30S (as produced by bpmn-js properties panel)
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1">
                    <timerEventDefinition>
                        <timeCycle>R3PT30S</timeCycle>
                    </timerEventDefinition>
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("s1").unwrap() {
        BpmnElement::TimerStartEvent(
            engine_core::timer_definition::TimerDefinition::RepeatingInterval {
                repetitions,
                interval,
            },
        ) => {
            assert_eq!(*repetitions, Some(3));
            assert_eq!(interval.as_secs(), 30);
        }
        _ => panic!("Expected TimerStartEvent with RepeatingInterval"),
    }
}

#[test]
fn parse_script_task() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
          <bpmn:process id="Process_1" isExecutable="true">
            <bpmn:startEvent id="StartEvent_1"/>
            <bpmn:scriptTask id="ScriptTask_1" bpmn:scriptFormat="rhai">
              <bpmn:script>let x = 1;</bpmn:script>
            </bpmn:scriptTask>
            <bpmn:endEvent id="EndEvent_1"/>
            <bpmn:sequenceFlow id="f1" sourceRef="StartEvent_1" targetRef="ScriptTask_1"/>
            <bpmn:sequenceFlow id="f2" sourceRef="ScriptTask_1" targetRef="EndEvent_1"/>
          </bpmn:process>
        </bpmn:definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    let node = def.get_node("ScriptTask_1").unwrap();
    assert!(matches!(node, BpmnElement::ScriptTask { script, .. } if script == "let x = 1;"));
}

#[test]
fn parse_terminate_end_event() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
          <bpmn:process id="Process_1" isExecutable="true">
            <bpmn:startEvent id="StartEvent_1"/>
            <bpmn:endEvent id="EndEvent_1">
              <bpmn:terminateEventDefinition />
            </bpmn:endEvent>
            <bpmn:sequenceFlow id="f1" sourceRef="StartEvent_1" targetRef="EndEvent_1"/>
          </bpmn:process>
        </bpmn:definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    let node = def.get_node("EndEvent_1").unwrap();
    assert!(matches!(node, BpmnElement::TerminateEndEvent));
}

#[test]
fn parse_send_task() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
          <bpmn:message id="Message_1" name="OrderShipped" />
          <bpmn:process id="Process_1" isExecutable="true">
            <bpmn:startEvent id="StartEvent_1"/>
            <bpmn:sendTask id="SendTask_1">
                <bpmn:messageEventDefinition bpmn:messageRef="Message_1" />
            </bpmn:sendTask>
            <bpmn:endEvent id="EndEvent_1"/>
            <bpmn:sequenceFlow id="f1" sourceRef="StartEvent_1" targetRef="SendTask_1"/>
            <bpmn:sequenceFlow id="f2" sourceRef="SendTask_1" targetRef="EndEvent_1"/>
          </bpmn:process>
        </bpmn:definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    let node = def.get_node("SendTask_1").unwrap();
    assert!(
        matches!(node, BpmnElement::SendTask { message_name, .. } if message_name == "OrderShipped")
    );
}

#[test]
fn parse_intermediate_message_throw() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
          <bpmn:message id="Message_1" name="OrderShipped" />
          <bpmn:process id="Process_1" isExecutable="true">
            <bpmn:startEvent id="StartEvent_1"/>
            <bpmn:intermediateThrowEvent id="Throw_1">
              <bpmn:messageEventDefinition bpmn:messageRef="Message_1" />
            </bpmn:intermediateThrowEvent>
            <bpmn:endEvent id="EndEvent_1"/>
            <bpmn:sequenceFlow id="f1" sourceRef="StartEvent_1" targetRef="Throw_1"/>
            <bpmn:sequenceFlow id="f2" sourceRef="Throw_1" targetRef="EndEvent_1"/>
          </bpmn:process>
        </bpmn:definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    let node = def.get_node("Throw_1").unwrap();
    assert!(
        matches!(node, BpmnElement::SendTask { message_name, .. } if message_name == "OrderShipped")
    );
}

#[test]
fn test_parse_complex_gateway() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
          <bpmn:process id="Process_1" isExecutable="true">
            <bpmn:startEvent id="StartEvent_1"/>
            <bpmn:complexGateway id="Complex_1" default="Flow_3">
                <bpmn:activationCondition xsi:type="bpmn:tFormalExpression">
                    token_count &gt;= 2
                </bpmn:activationCondition>
            </bpmn:complexGateway>
            <bpmn:endEvent id="EndEvent_1"/>
            <bpmn:endEvent id="EndEvent_2"/>
            <bpmn:sequenceFlow id="Flow_1" sourceRef="StartEvent_1" targetRef="Complex_1"/>
            <bpmn:sequenceFlow id="Flow_2" sourceRef="Complex_1" targetRef="EndEvent_1"/>
            <bpmn:sequenceFlow id="Flow_3" sourceRef="Complex_1" targetRef="EndEvent_2"/>
          </bpmn:process>
        </bpmn:definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    let node = def.get_node("Complex_1").unwrap();

    match node {
        BpmnElement::ComplexGateway {
            default,
            join_condition,
        } => {
            assert_eq!(default.as_deref(), Some("EndEvent_2")); // Flow_3 points to EndEvent_2
            assert_eq!(join_condition.as_deref(), Some("token_count >= 2"));
        }
        _ => panic!("Expected ComplexGateway"),
    }
}

#[test]
fn parse_escalation_events() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <escalation id="Esc_1" name="ReviewNeeded" escalationCode="ESC_REVIEW" />
            <process id="proc1">
                <startEvent id="start" />
                <intermediateThrowEvent id="esc_throw">
                    <escalationEventDefinition escalationRef="Esc_1" />
                </intermediateThrowEvent>
                <endEvent id="esc_end">
                    <escalationEventDefinition escalationRef="Esc_1" />
                </endEvent>
                <serviceTask id="task1" data-topic="work" />
                <boundaryEvent id="boundary_esc" attachedToRef="task1" cancelActivity="false">
                    <escalationEventDefinition escalationRef="Esc_1" />
                </boundaryEvent>
                <endEvent id="end" />
                <endEvent id="handler_end" />
                <sequenceFlow id="f1" sourceRef="start" targetRef="task1" />
                <sequenceFlow id="f2" sourceRef="task1" targetRef="esc_throw" />
                <sequenceFlow id="f3" sourceRef="esc_throw" targetRef="esc_end" />
                <sequenceFlow id="f4" sourceRef="boundary_esc" targetRef="handler_end" />
            </process>
        </bpmn:definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();

    // Check escalation throw event
    let throw = def.nodes.get("esc_throw").expect("esc_throw should exist");
    match throw {
        BpmnElement::EscalationThrowEvent { escalation_code } => {
            assert_eq!(escalation_code, "ESC_REVIEW");
        }
        _ => panic!("Expected EscalationThrowEvent, got {:?}", throw),
    }

    // Check escalation end event
    let end = def.nodes.get("esc_end").expect("esc_end should exist");
    match end {
        BpmnElement::EscalationEndEvent { escalation_code } => {
            assert_eq!(escalation_code, "ESC_REVIEW");
        }
        _ => panic!("Expected EscalationEndEvent, got {:?}", end),
    }

    // Check boundary escalation event
    let boundary = def
        .nodes
        .get("boundary_esc")
        .expect("boundary_esc should exist");
    match boundary {
        BpmnElement::BoundaryEscalationEvent {
            attached_to,
            escalation_code,
            cancel_activity,
        } => {
            assert_eq!(attached_to, "task1");
            assert_eq!(escalation_code.as_deref(), Some("ESC_REVIEW"));
            assert!(!cancel_activity);
        }
        _ => panic!("Expected BoundaryEscalationEvent, got {:?}", boundary),
    }
}

#[test]
fn parse_compensation_events() {
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="start" />
                <serviceTask id="task1" data-topic="work" />
                <boundaryEvent id="boundary_comp" attachedToRef="task1">
                    <compensateEventDefinition />
                </boundaryEvent>
                <serviceTask id="comp_handler" data-topic="undo_work" />
                <intermediateThrowEvent id="comp_throw">
                    <compensateEventDefinition activityRef="task1" />
                </intermediateThrowEvent>
                <endEvent id="comp_end">
                    <compensateEventDefinition />
                </endEvent>
                <endEvent id="end" />
                <sequenceFlow id="f1" sourceRef="start" targetRef="task1" />
                <sequenceFlow id="f2" sourceRef="task1" targetRef="comp_throw" />
                <sequenceFlow id="f3" sourceRef="comp_throw" targetRef="comp_end" />
                <sequenceFlow id="f4" sourceRef="boundary_comp" targetRef="comp_handler" />
                <sequenceFlow id="f5" sourceRef="comp_handler" targetRef="end" />
            </process>
        </bpmn:definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();

    // Check compensation boundary event
    let boundary = def
        .nodes
        .get("boundary_comp")
        .expect("boundary_comp should exist");
    match boundary {
        BpmnElement::BoundaryCompensationEvent { attached_to } => {
            assert_eq!(attached_to, "task1");
        }
        _ => panic!("Expected BoundaryCompensationEvent, got {:?}", boundary),
    }

    // Check compensation throw event
    let throw = def
        .nodes
        .get("comp_throw")
        .expect("comp_throw should exist");
    match throw {
        BpmnElement::CompensationThrowEvent { activity_ref } => {
            assert_eq!(activity_ref.as_deref(), Some("task1"));
        }
        _ => panic!("Expected CompensationThrowEvent, got {:?}", throw),
    }

    // Check compensation end event
    let end = def.nodes.get("comp_end").expect("comp_end should exist");
    match end {
        BpmnElement::CompensationEndEvent { activity_ref } => {
            assert!(activity_ref.is_none());
        }
        _ => panic!("Expected CompensationEndEvent, got {:?}", end),
    }
}

#[test]
fn test_camunda_namespace_script_parsed() {
    // bpmn-js/Camunda Modeler export BPMN with camunda: namespace prefix on
    // executionListener and script elements. Verify these are parsed correctly.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:camunda="http://camunda.org/schema/1.0/bpmn"
             id="Definitions_1">
  <process id="TestProcess" isExecutable="true">
    <startEvent id="start"/>
    <serviceTask id="task1">
      <extensionElements>
        <camunda:executionListener event="end">
          <camunda:script scriptFormat="rhai">let result = 42;</camunda:script>
        </camunda:executionListener>
      </extensionElements>
    </serviceTask>
    <endEvent id="end"/>
    <sequenceFlow id="f1" sourceRef="start" targetRef="task1"/>
    <sequenceFlow id="f2" sourceRef="task1" targetRef="end"/>
  </process>
</definitions>"#;

    let def = parse_bpmn_xml(xml).expect("Should parse BPMN with camunda: namespace");
    let listeners = def.listeners.get("task1").cloned().unwrap_or_default();
    // camunda:executionListener must be recognized (namespace prefix stripped by quick-xml)
    assert_eq!(
        listeners.len(),
        1,
        "camunda:executionListener should be parsed; got listeners: {:?}",
        listeners
    );
    assert!(matches!(listeners[0].event, ListenerEvent::End));
    assert_eq!(listeners[0].script, "let result = 42;");
}

#[test]
fn test_call_activity_parsed() {
    // Verify that <callActivity calledElement="..."> is correctly parsed.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Definitions_1">
  <process id="ParentProcess" isExecutable="true">
    <startEvent id="start"/>
    <callActivity id="call1" calledElement="ChildProcess"/>
    <endEvent id="end"/>
    <sequenceFlow id="f1" sourceRef="start" targetRef="call1"/>
    <sequenceFlow id="f2" sourceRef="call1" targetRef="end"/>
  </process>
</definitions>"#;

    let def = parse_bpmn_xml(xml).expect("Should parse BPMN with callActivity");
    let node = def.nodes.get("call1").expect("call1 should exist");
    match node {
        BpmnElement::CallActivity { called_element } => {
            assert_eq!(called_element, "ChildProcess");
        }
        _ => panic!("Expected CallActivity, got {:?}", node),
    }
}

#[test]
fn parse_camunda_assignee_and_topic() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <userTask id="ut1" camunda:assignee="alice" />
                <serviceTask id="st1" camunda:topic="charge-card" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="ut1" />
                <sequenceFlow id="f2" sourceRef="ut1" targetRef="st1" />
                <sequenceFlow id="f3" sourceRef="st1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("ut1").unwrap() {
        BpmnElement::UserTask(a) => assert_eq!(a, "alice"),
        other => panic!("Expected UserTask, got {:?}", other),
    }
    match def.nodes.get("st1").unwrap() {
        BpmnElement::ServiceTask { topic, .. } => assert_eq!(topic, "charge-card"),
        other => panic!("Expected ServiceTask, got {:?}", other),
    }
}

#[test]
fn parse_data_attributes_win_over_camunda() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <userTask id="ut1" data-assignee="desktop" camunda:assignee="camunda" />
                <serviceTask id="st1" data-topic="data-topic" camunda:topic="camunda-topic" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="ut1" />
                <sequenceFlow id="f2" sourceRef="ut1" targetRef="st1" />
                <sequenceFlow id="f3" sourceRef="st1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("ut1").unwrap() {
        BpmnElement::UserTask(a) => assert_eq!(a, "desktop"),
        other => panic!("Expected UserTask, got {:?}", other),
    }
    match def.nodes.get("st1").unwrap() {
        BpmnElement::ServiceTask { topic, .. } => assert_eq!(topic, "data-topic"),
        other => panic!("Expected ServiceTask, got {:?}", other),
    }
}

#[test]
fn parse_send_task_message_ref_attribute() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <message id="Message_1" name="OrderShipped" />
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <sendTask id="send1" messageRef="Message_1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="send1" />
                <sequenceFlow id="f2" sourceRef="send1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("send1").unwrap() {
        BpmnElement::SendTask { message_name, .. } => assert_eq!(message_name, "OrderShipped"),
        other => panic!("Expected SendTask, got {:?}", other),
    }
}

#[test]
fn reject_empty_timer_event_definition() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1">
                    <timerEventDefinition />
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("timerEventDefinition") && msg.contains("s1"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_timer_formal_expression() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait">
                    <timerEventDefinition>
                        <timeDuration xsi:type="bpmn:tFormalExpression">PT45S</timeDuration>
                    </timerEventDefinition>
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait" />
                <sequenceFlow id="f2" sourceRef="wait" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("wait").unwrap() {
        BpmnElement::TimerCatchEvent(engine_core::timer_definition::TimerDefinition::Duration(
            d,
        )) => assert_eq!(d.as_secs(), 45),
        other => panic!("Expected TimerCatchEvent Duration, got {:?}", other),
    }
}

#[test]
fn parse_receive_task_as_message_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <message id="Message_1" name="OrderReceived" />
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <receiveTask id="recv1" messageRef="Message_1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="recv1" />
                <sequenceFlow id="f2" sourceRef="recv1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("recv1").unwrap() {
        BpmnElement::MessageCatchEvent { message_name } => {
            assert_eq!(message_name, "OrderReceived")
        }
        other => panic!("Expected MessageCatchEvent, got {:?}", other),
    }
}

#[test]
fn parse_manual_task_as_user_task() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <manualTask id="man1" camunda:assignee="clerk" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="man1" />
                <sequenceFlow id="f2" sourceRef="man1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("man1").unwrap() {
        BpmnElement::UserTask(a) => assert_eq!(a, "clerk"),
        other => panic!("Expected UserTask, got {:?}", other),
    }
}

#[test]
fn reject_multi_instance_loop_characteristics() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <userTask id="ut1">
                    <multiInstanceLoopCharacteristics isSequential="false">
                        <loopCardinality>3</loopCardinality>
                    </multiInstanceLoopCharacteristics>
                </userTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="ut1" />
                <sequenceFlow id="f2" sourceRef="ut1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Multi-instance is only supported for service tasks")
            && msg.contains("userTask 'ut1'"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_sequential_multi_instance_on_user_task() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <userTask id="ut1">
                    <multiInstanceLoopCharacteristics isSequential="true">
                        <loopCardinality>3</loopCardinality>
                    </multiInstanceLoopCharacteristics>
                </userTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="ut1" />
                <sequenceFlow id="f2" sourceRef="ut1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let msg = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        msg.contains("Multi-instance is only supported for service tasks")
            && msg.contains("userTask 'ut1'"),
        "unexpected error: {msg}"
    );
}

#[test]
fn accept_parallel_multi_instance_loop_cardinality() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1" data-topic="notify">
                    <multiInstanceLoopCharacteristics isSequential="false">
                        <loopCardinality>3</loopCardinality>
                    </multiInstanceLoopCharacteristics>
                </serviceTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("svc1").unwrap() {
        BpmnElement::ServiceTask {
            topic,
            multi_instance: Some(mi),
        } => {
            assert_eq!(topic, "notify");
            assert!(!mi.is_sequential);
            assert_eq!(mi.loop_cardinality.as_deref(), Some("3"));
            assert_eq!(mi.collection, None);
            assert_eq!(mi.element_variable, None);
        }
        other => panic!("Expected ServiceTask with multi-instance, got {other:?}"),
    }
}

#[test]
fn accept_parallel_multi_instance_collection() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1" data-topic="notify">
                    <multiInstanceLoopCharacteristics isSequential="false"
                        collection="orders" elementVariable="order" />
                </serviceTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("svc1").unwrap() {
        BpmnElement::ServiceTask {
            multi_instance: Some(mi),
            ..
        } => {
            assert!(!mi.is_sequential);
            assert_eq!(mi.collection.as_deref(), Some("orders"));
            assert_eq!(mi.element_variable.as_deref(), Some("order"));
            assert_eq!(mi.loop_cardinality, None);
        }
        other => panic!("Expected ServiceTask with multi-instance, got {other:?}"),
    }
}

#[test]
fn accept_sequential_multi_instance() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1" data-topic="notify">
                    <multiInstanceLoopCharacteristics isSequential="true">
                        <loopCardinality>2</loopCardinality>
                    </multiInstanceLoopCharacteristics>
                </serviceTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("svc1").unwrap() {
        BpmnElement::ServiceTask {
            topic,
            multi_instance: Some(mi),
        } => {
            assert_eq!(topic, "notify");
            assert!(mi.is_sequential);
            assert_eq!(mi.loop_cardinality.as_deref(), Some("2"));
            assert_eq!(mi.collection, None);
        }
        other => panic!("Expected ServiceTask with sequential multi-instance, got {other:?}"),
    }
}

#[test]
fn accept_sequential_multi_instance_collection() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1" data-topic="notify">
                    <multiInstanceLoopCharacteristics isSequential="true"
                        collection="orders" elementVariable="order" />
                </serviceTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("svc1").unwrap() {
        BpmnElement::ServiceTask {
            multi_instance: Some(mi),
            ..
        } => {
            assert!(mi.is_sequential);
            assert_eq!(mi.collection.as_deref(), Some("orders"));
            assert_eq!(mi.element_variable.as_deref(), Some("order"));
            assert_eq!(mi.loop_cardinality, None);
        }
        other => panic!("Expected ServiceTask with sequential multi-instance, got {other:?}"),
    }
}

#[test]
fn reject_multi_instance_without_source() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1">
                    <multiInstanceLoopCharacteristics isSequential="false" />
                </serviceTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let msg = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        msg.contains("requires either collection or loopCardinality") && msg.contains("svc1"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_multi_instance_with_both_sources() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1">
                    <multiInstanceLoopCharacteristics isSequential="false"
                        collection="orders">
                        <loopCardinality>2</loopCardinality>
                    </multiInstanceLoopCharacteristics>
                </serviceTask>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let msg = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        msg.contains("either collection or loopCardinality, not both") && msg.contains("svc1"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_multi_instance_invalid_cardinality() {
    let invalid_values = ["abc", "0", "-1"];
    for value in invalid_values {
        let xml = format!(
            r#"
            <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <process id="proc1" isExecutable="true">
                    <startEvent id="s1" />
                    <serviceTask id="svc1">
                        <multiInstanceLoopCharacteristics isSequential="false">
                            <loopCardinality>{value}</loopCardinality>
                        </multiInstanceLoopCharacteristics>
                    </serviceTask>
                    <endEvent id="e1" />
                    <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                    <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
                </process>
            </definitions>
        "#
        );
        let msg = parse_bpmn_xml(&xml).unwrap_err().to_string();
        assert!(
            msg.contains("must be a positive integer") && msg.contains("svc1"),
            "unexpected error for '{value}': {msg}"
        );
    }
}

#[test]
fn reject_generic_task() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <task id="t1" name="Do something" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="t1" />
                <sequenceFlow id="f2" sourceRef="t1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("task id='t1'"), "unexpected error: {msg}");
}

#[test]
fn reject_empty_script_task() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <scriptTask id="sc1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sc1" />
                <sequenceFlow id="f2" sourceRef="sc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("scriptTask") && msg.contains("sc1"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_catch_without_trigger() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait" />
                <sequenceFlow id="f2" sourceRef="wait" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("intermediateCatchEvent") && msg.contains("wait"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_business_rule_task_with_topic() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <businessRuleTask id="br1" camunda:topic="decide" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="br1" />
                <sequenceFlow id="f2" sourceRef="br1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("br1").unwrap() {
        BpmnElement::ServiceTask { topic, .. } => assert_eq!(topic, "decide"),
        other => panic!("Expected ServiceTask, got {:?}", other),
    }
}

#[test]
fn reject_business_rule_task_without_topic() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <businessRuleTask id="br1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="br1" />
                <sequenceFlow id="f2" sourceRef="br1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("businessRuleTask") && msg.contains("br1"),
        "unexpected error: {msg}"
    );
}

#[test]
fn flatten_subprocess_keeps_send_inclusive_event_and_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <message id="Message_1" name="InnerMsg" />
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <sendTask id="sub_send" messageRef="Message_1" />
                    <inclusiveGateway id="sub_or" />
                    <eventBasedGateway id="sub_eb" />
                    <intermediateCatchEvent id="sub_wait">
                        <messageEventDefinition messageRef="Message_1" />
                    </intermediateCatchEvent>
                    <intermediateCatchEvent id="sub_timer">
                        <timerEventDefinition>
                            <timeDuration>PT1S</timeDuration>
                        </timerEventDefinition>
                    </intermediateCatchEvent>
                    <endEvent id="sub_e1" />
                    <endEvent id="sub_e2" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_send" />
                    <sequenceFlow id="sf2" sourceRef="sub_send" targetRef="sub_or" />
                    <sequenceFlow id="sf3" sourceRef="sub_or" targetRef="sub_eb" />
                    <sequenceFlow id="sf4" sourceRef="sub_or" targetRef="sub_e2" />
                    <sequenceFlow id="sf5" sourceRef="sub_eb" targetRef="sub_wait" />
                    <sequenceFlow id="sf6" sourceRef="sub_eb" targetRef="sub_timer" />
                    <sequenceFlow id="sf7" sourceRef="sub_wait" targetRef="sub_e1" />
                    <sequenceFlow id="sf8" sourceRef="sub_timer" targetRef="sub_e1" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    assert!(
        matches!(
            def.nodes.get("sub_send").unwrap(),
            BpmnElement::SendTask { message_name, .. } if message_name == "InnerMsg"
        ),
        "sendTask inside subProcess must be flattened"
    );
    assert!(matches!(
        def.nodes.get("sub_or").unwrap(),
        BpmnElement::InclusiveGateway
    ));
    assert!(matches!(
        def.nodes.get("sub_eb").unwrap(),
        BpmnElement::EventBasedGateway
    ));
    assert!(matches!(
        def.nodes.get("sub_wait").unwrap(),
        BpmnElement::MessageCatchEvent { .. }
    ));
}

#[test]
fn flatten_subprocess_start_event_is_pass_through() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <scriptTask id="sub_script">
                        <script>let x = 1;</script>
                    </scriptTask>
                    <endEvent id="sub_e1" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_script" />
                    <sequenceFlow id="sf2" sourceRef="sub_script" targetRef="sub_e1" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();

    assert!(
        matches!(
            def.nodes.get("sub1"),
            Some(BpmnElement::EmbeddedSubProcess { start_node_id }) if start_node_id == "sub_script"
        ),
        "EmbeddedSubProcess must enter at the start event's target"
    );
    assert!(
        !def.nodes.contains_key("sub_s1"),
        "internal start event must not be materialised as a node"
    );
    assert!(
        def.nodes.values().all(|element| !matches!(
            element,
            BpmnElement::ServiceTask { topic, .. } if topic == "noop"
        )),
        "no synthetic noop service task may be created"
    );
    assert!(
        def.next_nodes("sub_s1").is_empty(),
        "flows out of the internal start event must not remain as edges"
    );
    assert_eq!(def.next_node("sub_script"), Some("sub_e1"));
}

#[test]
fn flatten_subprocess_start_event_with_multiple_flows_becomes_parallel_gateway() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <endEvent id="sub_e1" />
                    <endEvent id="sub_e2" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_e1" />
                    <sequenceFlow id="sf2" sourceRef="sub_s1" targetRef="sub_e2" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();

    assert!(
        matches!(def.nodes.get("sub_s1"), Some(BpmnElement::ParallelGateway)),
        "an implicit split at the internal start event must become a parallel gateway"
    );
    assert!(
        matches!(
            def.nodes.get("sub1"),
            Some(BpmnElement::EmbeddedSubProcess { start_node_id }) if start_node_id == "sub_s1"
        ),
        "EmbeddedSubProcess must enter at the fork gateway"
    );
    assert_eq!(def.next_nodes("sub_s1").len(), 2);
}

#[test]
fn flatten_subprocess_without_start_event_is_rejected() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <endEvent id="sub_e1" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    assert!(
        err.to_string().contains("has no startEvent"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_subprocess_with_multiple_start_events_is_rejected() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <startEvent id="sub_s2" />
                    <endEvent id="sub_e1" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_e1" />
                    <sequenceFlow id="sf2" sourceRef="sub_s2" targetRef="sub_e1" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err();
    assert!(
        err.to_string().contains("must have exactly one startEvent"),
        "unexpected error: {err}"
    );
}

const SUB_PROCESS_DEFAULT_GATEWAY_PREFIX: &str = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
"#;

const SUB_PROCESS_DEFAULT_GATEWAY_SUFFIX: &str = r#"
                    <scriptTask id="sub_default">
                        <script>let x = 1;</script>
                    </scriptTask>
                    <scriptTask id="sub_cond">
                        <script>let y = 1;</script>
                    </scriptTask>
                    <endEvent id="sub_e1" />
                    <endEvent id="sub_e2" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_gw" />
                    <sequenceFlow id="sf_cond" sourceRef="sub_gw" targetRef="sub_cond">
                        <conditionExpression xsi:type="tFormalExpression">x == 1</conditionExpression>
                    </sequenceFlow>
                    <sequenceFlow id="sf_default" sourceRef="sub_gw" targetRef="sub_default" />
                    <sequenceFlow id="sf2" sourceRef="sub_cond" targetRef="sub_e1" />
                    <sequenceFlow id="sf3" sourceRef="sub_default" targetRef="sub_e2" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
"#;

#[test]
fn flatten_subprocess_resolves_exclusive_gateway_default() {
    let xml = format!(
        "{SUB_PROCESS_DEFAULT_GATEWAY_PREFIX}<exclusiveGateway id=\"sub_gw\" default=\"sf_default\" />{SUB_PROCESS_DEFAULT_GATEWAY_SUFFIX}"
    );
    let def = parse_bpmn_xml(&xml).unwrap();

    match def.nodes.get("sub_gw") {
        Some(BpmnElement::ExclusiveGateway { default }) => assert_eq!(
            default.as_deref(),
            Some("sub_default"),
            "the default flow id must be resolved to its target node"
        ),
        other => panic!("expected ExclusiveGateway, got {other:?}"),
    }
}

#[test]
fn flatten_subprocess_resolves_complex_gateway_default() {
    let xml = format!(
        "{SUB_PROCESS_DEFAULT_GATEWAY_PREFIX}<complexGateway id=\"sub_gw\" default=\"sf_default\" />{SUB_PROCESS_DEFAULT_GATEWAY_SUFFIX}"
    );
    let def = parse_bpmn_xml(&xml).unwrap();

    match def.nodes.get("sub_gw") {
        Some(BpmnElement::ComplexGateway { default, .. }) => assert_eq!(
            default.as_deref(),
            Some("sub_default"),
            "the default flow id must be resolved to its target node"
        ),
        other => panic!("expected ComplexGateway, got {other:?}"),
    }
}

#[test]
fn flatten_subprocess_rejects_unknown_default_flow() {
    let xml = format!(
        "{SUB_PROCESS_DEFAULT_GATEWAY_PREFIX}<exclusiveGateway id=\"sub_gw\" default=\"missing_flow\" />{SUB_PROCESS_DEFAULT_GATEWAY_SUFFIX}"
    );
    let err = parse_bpmn_xml(&xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("sub_gw") && msg.contains("sub1") && msg.contains("missing_flow"),
        "error must name sub-process, gateway and flow: {msg}"
    );
    assert!(
        matches!(err, engine_core::error::EngineError::InvalidDefinition(_)),
        "unknown default flow must be InvalidDefinition: {err:?}"
    );
}

#[test]
fn parse_link_events_resolve_throw_to_catch_target() {
    // Camunda-typisches XML: der Catch hat keinen eingehenden, genau einen ausgehenden Flow.
    let xml = r#"
        <bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <bpmn:process id="proc_link" isExecutable="true">
                <bpmn:startEvent id="s1" />
                <bpmn:intermediateThrowEvent id="throw1">
                    <bpmn:linkEventDefinition name="Jump" />
                </bpmn:intermediateThrowEvent>
                <bpmn:intermediateCatchEvent id="catch1">
                    <bpmn:linkEventDefinition name="Jump" />
                </bpmn:intermediateCatchEvent>
                <bpmn:scriptTask id="after">
                    <bpmn:script>let jumped = true;</bpmn:script>
                </bpmn:scriptTask>
                <bpmn:endEvent id="e1" />
                <bpmn:sequenceFlow id="f1" sourceRef="s1" targetRef="throw1" />
                <bpmn:sequenceFlow id="f2" sourceRef="catch1" targetRef="after" />
                <bpmn:sequenceFlow id="f3" sourceRef="after" targetRef="e1" />
            </bpmn:process>
        </bpmn:definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();

    assert!(
        matches!(
            def.nodes.get("throw1"),
            Some(BpmnElement::LinkThrowEvent { target }) if target == "after"
        ),
        "throw must carry the catch successor as target"
    );
    assert!(
        !def.nodes.contains_key("catch1"),
        "link catch must not be materialised"
    );
    assert!(
        def.next_nodes("catch1").is_empty(),
        "flows out of the link catch must be dropped"
    );
    assert_eq!(
        def.next_node("throw1"),
        Some("after"),
        "throw must be wired to its jump target for graph validation"
    );
    assert!(matches!(
        def.nodes.get("after"),
        Some(BpmnElement::ScriptTask { .. })
    ));
}

#[test]
fn parse_link_pair_inside_sub_process() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_link_sub" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s" />
                    <intermediateThrowEvent id="sub_throw">
                        <linkEventDefinition name="InnerJump" />
                    </intermediateThrowEvent>
                    <intermediateCatchEvent id="sub_catch">
                        <linkEventDefinition name="InnerJump" />
                    </intermediateCatchEvent>
                    <scriptTask id="sub_after">
                        <script>let inner_jumped = true;</script>
                    </scriptTask>
                    <endEvent id="sub_e" />
                    <sequenceFlow id="sf1" sourceRef="sub_s" targetRef="sub_throw" />
                    <sequenceFlow id="sf2" sourceRef="sub_catch" targetRef="sub_after" />
                    <sequenceFlow id="sf3" sourceRef="sub_after" targetRef="sub_e" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();

    assert!(
        matches!(
            def.nodes.get("sub_throw"),
            Some(BpmnElement::LinkThrowEvent { target }) if target == "sub_after"
        ),
        "link pair inside a sub-process must resolve within that sub-process"
    );
    assert!(!def.nodes.contains_key("sub_catch"));
    assert_eq!(def.next_node("sub_throw"), Some("sub_after"));
    assert!(
        matches!(
            def.nodes.get("sub1"),
            Some(BpmnElement::EmbeddedSubProcess { start_node_id }) if start_node_id == "sub_throw"
        ),
        "sub-process entry must not change"
    );
}

#[test]
fn reject_link_throw_in_sub_process_targeting_outer_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_cross" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s" />
                    <intermediateThrowEvent id="sub_throw">
                        <linkEventDefinition name="OuterJump" />
                    </intermediateThrowEvent>
                    <endEvent id="sub_e" />
                    <sequenceFlow id="sf1" sourceRef="sub_s" targetRef="sub_throw" />
                    <sequenceFlow id="sf3" sourceRef="sub_s" targetRef="sub_e" />
                </subProcess>
                <intermediateCatchEvent id="outer_catch">
                    <linkEventDefinition name="OuterJump" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="outer_catch" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("unknown link name 'OuterJump'") && err.contains("scope 'sub1'"),
        "cross-scope link must be rejected, got: {err}"
    );
}

#[test]
fn reject_link_throw_at_top_level_targeting_inner_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_cross" isExecutable="true">
                <startEvent id="s1" />
                <intermediateThrowEvent id="outer_throw">
                    <linkEventDefinition name="InnerJump" />
                </intermediateThrowEvent>
                <subProcess id="sub1">
                    <startEvent id="sub_s" />
                    <intermediateCatchEvent id="sub_catch">
                        <linkEventDefinition name="InnerJump" />
                    </intermediateCatchEvent>
                    <endEvent id="sub_e" />
                    <sequenceFlow id="sf1" sourceRef="sub_s" targetRef="sub_e" />
                    <sequenceFlow id="sf2" sourceRef="sub_catch" targetRef="sub_e" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="outer_throw" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("unknown link name 'InnerJump'") && err.contains("scope 'proc_cross'"),
        "cross-scope link must be rejected, got: {err}"
    );
}

#[test]
fn reject_duplicate_link_name_in_same_scope() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_dup" isExecutable="true">
                <startEvent id="s1" />
                <intermediateCatchEvent id="catch1">
                    <linkEventDefinition name="Jump" />
                </intermediateCatchEvent>
                <intermediateCatchEvent id="catch2">
                    <linkEventDefinition name="Jump" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
                <sequenceFlow id="f2" sourceRef="catch1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="catch2" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("duplicate link name 'Jump'") && err.contains("scope 'proc_dup'"),
        "unexpected error: {err}"
    );
}

#[test]
fn reject_link_throw_without_matching_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_missing" isExecutable="true">
                <startEvent id="s1" />
                <intermediateThrowEvent id="throw1">
                    <linkEventDefinition name="Nowhere" />
                </intermediateThrowEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="throw1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("unknown link name 'Nowhere'") && err.contains("scope 'proc_missing'"),
        "unexpected error: {err}"
    );
}

#[test]
fn reject_link_catch_with_two_outgoing_flows() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_fork" isExecutable="true">
                <startEvent id="s1" />
                <intermediateThrowEvent id="throw1">
                    <linkEventDefinition name="Jump" />
                </intermediateThrowEvent>
                <intermediateCatchEvent id="catch1">
                    <linkEventDefinition name="Jump" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="throw1" />
                <sequenceFlow id="f2" sourceRef="catch1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="catch1" targetRef="e2" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("exactly one outgoing sequence flow") && err.contains("catch1"),
        "unexpected error: {err}"
    );
}

#[test]
fn reject_link_throw_with_outgoing_flow() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_throw_flow" isExecutable="true">
                <startEvent id="s1" />
                <intermediateThrowEvent id="throw1">
                    <linkEventDefinition name="Jump" />
                </intermediateThrowEvent>
                <intermediateCatchEvent id="catch1">
                    <linkEventDefinition name="Jump" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="throw1" />
                <sequenceFlow id="f2" sourceRef="throw1" targetRef="e2" />
                <sequenceFlow id="f3" sourceRef="catch1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("must not have outgoing sequence flows") && err.contains("throw1"),
        "unexpected error: {err}"
    );
}

#[test]
fn reject_cancel_end_event_in_regular_sub_process() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_cancel" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s" />
                    <endEvent id="sub_cancel_end">
                        <cancelEventDefinition />
                    </endEvent>
                    <sequenceFlow id="sf1" sourceRef="sub_s" targetRef="sub_cancel_end" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("cancel end events require a transaction sub-process")
            && err.contains("sub_cancel_end"),
        "unexpected error: {err}"
    );
}

#[test]
fn reject_cancel_boundary_event() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_cancel_boundary" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1" data-topic="work" />
                <boundaryEvent id="cancel_boundary" attachedToRef="svc1">
                    <cancelEventDefinition />
                </boundaryEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="cancel_boundary" targetRef="e2" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("cancel end events require a transaction sub-process")
            && err.contains("cancel_boundary"),
        "unexpected error: {err}"
    );
}

#[test]
fn reject_transaction_sub_process() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_tx" isExecutable="true">
                <startEvent id="s1" />
                <transaction id="tx1">
                    <startEvent id="tx_s" />
                    <endEvent id="tx_e" />
                    <sequenceFlow id="tx_f1" sourceRef="tx_s" targetRef="tx_e" />
                </transaction>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="tx1" />
                <sequenceFlow id="f2" sourceRef="tx1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("transaction sub-processes are not supported")
            && err.contains("cancel events cannot be deployed")
            && err.contains("tx1"),
        "unexpected error: {err}"
    );
}

#[test]
fn bpmn_without_cancel_events_stays_valid() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc_plain" isExecutable="true">
                <startEvent id="s1" />
                <serviceTask id="svc1" data-topic="work" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="svc1" />
                <sequenceFlow id="f2" sourceRef="svc1" targetRef="e1" />
            </process>
        </definitions>
    "#;
    let def = parse_bpmn_xml(xml).unwrap();
    assert!(matches!(
        def.nodes.get("svc1"),
        Some(BpmnElement::ServiceTask { .. })
    ));
}

// --- Signal-Events ---

#[test]
fn parse_signal_start_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="GlobalStart" />
            <process id="proc1">
                <startEvent id="s1">
                    <signalEventDefinition signalRef="sig1" />
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("s1").unwrap() {
        BpmnElement::SignalStartEvent { signal_name } => {
            assert_eq!(signal_name, "GlobalStart");
        }
        other => panic!("Expected SignalStartEvent, got {:?}", other),
    }
}

#[test]
fn parse_signal_catch_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="PaymentConfirmed" />
            <process id="proc1">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait_signal">
                    <signalEventDefinition signalRef="sig1" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait_signal" />
                <sequenceFlow id="f2" sourceRef="wait_signal" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("wait_signal").unwrap() {
        BpmnElement::SignalCatchEvent { signal_name } => {
            assert_eq!(signal_name, "PaymentConfirmed");
        }
        other => panic!("Expected SignalCatchEvent, got {:?}", other),
    }
}

#[test]
fn parse_signal_throw_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="NotifyAll" />
            <process id="proc1">
                <startEvent id="s1" />
                <intermediateThrowEvent id="throw_signal">
                    <signalEventDefinition signalRef="sig1" />
                </intermediateThrowEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="throw_signal" />
                <sequenceFlow id="f2" sourceRef="throw_signal" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("throw_signal").unwrap() {
        BpmnElement::SignalThrowEvent { signal_name } => {
            assert_eq!(signal_name, "NotifyAll");
        }
        other => panic!("Expected SignalThrowEvent, got {:?}", other),
    }
}

#[test]
fn parse_signal_end_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="Finished" />
            <process id="proc1">
                <startEvent id="s1" />
                <endEvent id="e1">
                    <signalEventDefinition signalRef="sig1" />
                </endEvent>
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("e1").unwrap() {
        BpmnElement::SignalEndEvent { signal_name } => {
            assert_eq!(signal_name, "Finished");
        }
        other => panic!("Expected SignalEndEvent, got {:?}", other),
    }
}

#[test]
fn parse_boundary_signal_event() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="Abort" />
            <process id="proc1">
                <startEvent id="s1" />
                <serviceTask id="task1" data-topic="charge" />
                <boundaryEvent id="on_signal" attachedToRef="task1" cancelActivity="false">
                    <signalEventDefinition signalRef="sig1" />
                </boundaryEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="task1" />
                <sequenceFlow id="f2" sourceRef="task1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="on_signal" targetRef="e2" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("on_signal").unwrap() {
        BpmnElement::BoundarySignalEvent {
            attached_to,
            signal_name,
            cancel_activity,
        } => {
            assert_eq!(attached_to, "task1");
            assert_eq!(signal_name, "Abort");
            assert!(!*cancel_activity);
        }
        other => panic!("Expected BoundarySignalEvent, got {:?}", other),
    }
}

#[test]
fn parse_signal_event_without_ref_is_rejected() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait_signal">
                    <signalEventDefinition />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait_signal" />
                <sequenceFlow id="f2" sourceRef="wait_signal" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let msg = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        msg.contains("no signalRef") && msg.contains("wait_signal"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_signal_event_with_unknown_ref_is_rejected() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1">
                <startEvent id="s1" />
                <intermediateThrowEvent id="throw_signal">
                    <signalEventDefinition signalRef="missing_signal" />
                </intermediateThrowEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="throw_signal" />
                <sequenceFlow id="f2" sourceRef="throw_signal" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let msg = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        msg.contains("unknown signal") && msg.contains("missing_signal"),
        "unexpected error: {msg}"
    );
}

#[test]
fn flatten_subprocess_keeps_signal_events() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="InnerSignal" />
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <intermediateCatchEvent id="sub_wait">
                        <signalEventDefinition signalRef="sig1" />
                    </intermediateCatchEvent>
                    <intermediateThrowEvent id="sub_throw">
                        <signalEventDefinition signalRef="sig1" />
                    </intermediateThrowEvent>
                    <endEvent id="sub_e1" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_wait" />
                    <sequenceFlow id="sf2" sourceRef="sub_wait" targetRef="sub_throw" />
                    <sequenceFlow id="sf3" sourceRef="sub_throw" targetRef="sub_e1" />
                </subProcess>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert!(
        matches!(
            def.nodes.get("sub_wait"),
            Some(BpmnElement::SignalCatchEvent { signal_name }) if signal_name == "InnerSignal"
        ),
        "signal catch inside subProcess must be flattened"
    );
    assert!(
        matches!(
            def.nodes.get("sub_throw"),
            Some(BpmnElement::SignalThrowEvent { signal_name }) if signal_name == "InnerSignal"
        ),
        "signal throw inside subProcess must be flattened"
    );
    assert!(matches!(
        def.nodes.get("sub1"),
        Some(BpmnElement::EmbeddedSubProcess { start_node_id }) if start_node_id == "sub_wait"
    ));
}

#[test]
fn event_based_gateway_accepts_signal_catch_targets() {
    let xml = r#"
        <definitions id="def1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <signal id="sig1" name="First" />
            <signal id="sig2" name="Second" />
            <process id="proc1">
                <startEvent id="s1" />
                <eventBasedGateway id="gw" />
                <intermediateCatchEvent id="wait_first">
                    <signalEventDefinition signalRef="sig1" />
                </intermediateCatchEvent>
                <intermediateCatchEvent id="wait_second">
                    <signalEventDefinition signalRef="sig2" />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="gw" />
                <sequenceFlow id="f2" sourceRef="gw" targetRef="wait_first" />
                <sequenceFlow id="f3" sourceRef="gw" targetRef="wait_second" />
                <sequenceFlow id="f4" sourceRef="wait_first" targetRef="e1" />
                <sequenceFlow id="f5" sourceRef="wait_second" targetRef="e2" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    assert!(matches!(
        def.nodes.get("wait_first"),
        Some(BpmnElement::SignalCatchEvent { .. })
    ));
    assert!(matches!(
        def.nodes.get("wait_second"),
        Some(BpmnElement::SignalCatchEvent { .. })
    ));
}

#[test]
fn parse_conditional_intermediate_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait_cond">
                    <conditionalEventDefinition>
                        <conditionExpression xsi:type="tFormalExpression">${amount &gt; 100}</conditionExpression>
                    </conditionalEventDefinition>
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait_cond" />
                <sequenceFlow id="f2" sourceRef="wait_cond" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("wait_cond").unwrap() {
        BpmnElement::ConditionalCatchEvent { condition } => {
            assert_eq!(condition, "${amount > 100}");
        }
        other => panic!("Expected ConditionalCatchEvent, got {other:?}"),
    }
}

#[test]
fn parse_boundary_conditional_interrupting_and_non_interrupting() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <userTask id="ut1" />
                <boundaryEvent id="bound_int" attachedToRef="ut1">
                    <conditionalEventDefinition>
                        <conditionExpression xsi:type="tFormalExpression">x &gt; 10</conditionExpression>
                    </conditionalEventDefinition>
                </boundaryEvent>
                <boundaryEvent id="bound_nonint" attachedToRef="ut1" cancelActivity="false">
                    <conditionalEventDefinition>
                        <conditionExpression xsi:type="tFormalExpression">x &gt; 20</conditionExpression>
                    </conditionalEventDefinition>
                </boundaryEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <endEvent id="e3" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="ut1" />
                <sequenceFlow id="f2" sourceRef="ut1" targetRef="e1" />
                <sequenceFlow id="f3" sourceRef="bound_int" targetRef="e2" />
                <sequenceFlow id="f4" sourceRef="bound_nonint" targetRef="e3" />
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("bound_int").unwrap() {
        BpmnElement::BoundaryConditionalEvent {
            attached_to,
            condition,
            cancel_activity,
        } => {
            assert_eq!(attached_to, "ut1");
            assert_eq!(condition, "x > 10");
            assert!(*cancel_activity);
        }
        other => panic!("Expected BoundaryConditionalEvent, got {other:?}"),
    }
    match def.nodes.get("bound_nonint").unwrap() {
        BpmnElement::BoundaryConditionalEvent {
            condition,
            cancel_activity,
            ..
        } => {
            assert_eq!(condition, "x > 20");
            assert!(!*cancel_activity);
        }
        other => panic!("Expected BoundaryConditionalEvent, got {other:?}"),
    }
}

#[test]
fn reject_conditional_catch_without_condition() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait_cond">
                    <conditionalEventDefinition />
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait_cond" />
                <sequenceFlow id="f2" sourceRef="wait_cond" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("conditionExpression") && msg.contains("wait_cond"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_invalid_conditional_condition() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <intermediateCatchEvent id="wait_cond">
                    <conditionalEventDefinition>
                        <conditionExpression xsi:type="tFormalExpression">${x &amp;&amp;}</conditionExpression>
                    </conditionalEventDefinition>
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="wait_cond" />
                <sequenceFlow id="f2" sourceRef="wait_cond" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("invalid condition") && msg.contains("wait_cond"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_conditional_start_event() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1">
                    <conditionalEventDefinition>
                        <conditionExpression xsi:type="tFormalExpression">x &gt; 0</conditionExpression>
                    </conditionalEventDefinition>
                </startEvent>
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="e1" />
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("conditional start events are not supported")
            && msg.contains("no global variable scope")
            && msg.contains("s1"),
        "unexpected error: {msg}"
    );
}

#[test]
fn reject_event_based_gateway_with_conditional_catch() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <eventBasedGateway id="gw" />
                <intermediateCatchEvent id="wait_cond">
                    <conditionalEventDefinition>
                        <conditionExpression xsi:type="tFormalExpression">x &gt; 0</conditionExpression>
                    </conditionalEventDefinition>
                </intermediateCatchEvent>
                <intermediateCatchEvent id="wait_timer">
                    <timerEventDefinition>
                        <timeDuration>PT10S</timeDuration>
                    </timerEventDefinition>
                </intermediateCatchEvent>
                <endEvent id="e1" />
                <endEvent id="e2" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="gw" />
                <sequenceFlow id="f2" sourceRef="gw" targetRef="wait_cond" />
                <sequenceFlow id="f3" sourceRef="gw" targetRef="wait_timer" />
                <sequenceFlow id="f4" sourceRef="wait_cond" targetRef="e1" />
                <sequenceFlow id="f5" sourceRef="wait_timer" targetRef="e2" />
            </process>
        </definitions>
    "#;

    let err = parse_bpmn_xml(xml).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("EventBasedGateway") && msg.contains("conditional catch"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_conditional_catch_in_subprocess() {
    let xml = r#"
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <process id="proc1" isExecutable="true">
                <startEvent id="s1" />
                <endEvent id="e1" />
                <sequenceFlow id="f1" sourceRef="s1" targetRef="sub1" />
                <sequenceFlow id="f2" sourceRef="sub1" targetRef="e1" />
                <subProcess id="sub1">
                    <startEvent id="sub_s1" />
                    <intermediateCatchEvent id="sub_cond">
                        <conditionalEventDefinition>
                            <conditionExpression xsi:type="tFormalExpression">x &gt; 0</conditionExpression>
                        </conditionalEventDefinition>
                    </intermediateCatchEvent>
                    <endEvent id="sub_e1" />
                    <sequenceFlow id="sf1" sourceRef="sub_s1" targetRef="sub_cond" />
                    <sequenceFlow id="sf2" sourceRef="sub_cond" targetRef="sub_e1" />
                </subProcess>
            </process>
        </definitions>
    "#;

    let def = parse_bpmn_xml(xml).unwrap();
    match def.nodes.get("sub_cond").unwrap() {
        BpmnElement::ConditionalCatchEvent { condition } => {
            assert_eq!(condition, "x > 0");
        }
        other => panic!("Expected ConditionalCatchEvent, got {other:?}"),
    }
}

// --- Deploy lehnt ununterstützte Elemente gesammelt ab -------------------------

fn process_with(body: &str) -> String {
    format!(
        r#"<definitions id="d" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL">
             <signal id="sig" name="go" />
             <message id="msg" name="m" />
             <process id="p" isExecutable="true">
               <startEvent id="s" />
               <endEvent id="e" />
               <sequenceFlow id="f" sourceRef="s" targetRef="e" />
               {body}
             </process>
           </definitions>"#
    )
}

fn rejection_of(body: &str) -> String {
    parse_bpmn_xml(&process_with(body))
        .expect_err("definition must be rejected")
        .to_string()
}

fn assert_rejected(body: &str, needles: &[&str]) {
    let err = rejection_of(body);
    for needle in needles {
        assert!(err.contains(needle), "missing {needle:?} in {err}");
    }
}

#[test]
fn reject_ad_hoc_sub_process() {
    assert_rejected(
        r#"<bpmn:adHocSubProcess id="adhoc" />"#,
        &["adHocSubProcess", "adhoc", "is not supported"],
    );
}

#[test]
fn reject_standard_loop_characteristics() {
    assert_rejected(
        r#"<userTask id="ut"><standardLoopCharacteristics /></userTask>"#,
        &["standardLoopCharacteristics", "'ut'", "is not supported"],
    );
}

#[test]
fn reject_transaction_names_id() {
    assert_rejected(
        r#"<transaction id="tx1"><startEvent id="ts"/><endEvent id="te"/><sequenceFlow id="tf" sourceRef="ts" targetRef="te"/></transaction>"#,
        &[
            "transaction sub-processes are not supported",
            "cancel events cannot be deployed",
            "tx1",
        ],
    );
}

#[test]
fn instantiating_receive_task_is_a_message_start() {
    for flag in ["true", "1"] {
        let xml = format!(
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <message id="msg" name="Order" />
                <process id="p" isExecutable="true">
                  <receiveTask id="recv" instantiate="{flag}" messageRef="msg" />
                  <endEvent id="e" />
                  <sequenceFlow id="f" sourceRef="recv" targetRef="e" />
                </process>
              </definitions>"#
        );
        let def = parse_bpmn_xml(&xml).unwrap_or_else(|err| panic!("{flag}: {err}"));
        assert!(
            matches!(
                def.nodes.get("recv"),
                Some(BpmnElement::MessageStartEvent { message_name }) if message_name == "Order"
            ),
            "{flag}"
        );
    }
}

#[test]
fn reject_instantiating_receive_with_incoming_flow() {
    let xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
        <message id="msg" name="Order" />
        <process id="p" isExecutable="true">
          <startEvent id="s" />
          <receiveTask id="recv" instantiate="true" messageRef="msg" />
          <endEvent id="e" />
          <sequenceFlow id="f1" sourceRef="s" targetRef="recv" />
          <sequenceFlow id="f2" sourceRef="recv" targetRef="e" />
        </process>
      </definitions>"#;
    let err = parse_bpmn_xml(xml).unwrap_err().to_string();
    assert!(
        err.contains("incoming sequence flows") && err.contains("recv"),
        "{err}"
    );
}

#[test]
fn reject_instantiating_receive_inside_subprocess() {
    assert_rejected(
        r#"<subProcess id="sp"><receiveTask id="recv" instantiate="true" messageRef="msg" /></subProcess>"#,
        &["instantiating receiveTask", "'recv'", "sub-process"],
    );
}

#[test]
fn reject_instantiating_event_based_gateway() {
    assert_rejected(
        r#"<eventBasedGateway id="gw" instantiate="true" />"#,
        &["instantiating or parallel eventBasedGateway", "'gw'"],
    );
}

#[test]
fn reject_parallel_event_based_gateway() {
    assert_rejected(
        r#"<eventBasedGateway id="gw" eventGatewayType="Parallel" />"#,
        &["instantiating or parallel eventBasedGateway", "'gw'"],
    );
}

#[test]
fn reject_message_end_event() {
    let err = rejection_of(
        r#"<endEvent id="msgEnd"><messageEventDefinition messageRef="msg" /></endEvent>"#,
    );
    assert!(
        err.contains("messageEventDefinition on endEvent 'msgEnd'"),
        "{err}"
    );
}

#[test]
fn reject_multiple_event_definitions() {
    assert_rejected(
        r#"<intermediateCatchEvent id="c">
             <messageEventDefinition messageRef="msg" />
             <timerEventDefinition><timeDuration>PT1M</timeDuration></timerEventDefinition>
           </intermediateCatchEvent>"#,
        &["multiple event definitions on intermediateCatchEvent 'c'"],
    );
}

#[test]
fn reject_error_start_event() {
    assert_rejected(
        r#"<startEvent id="errStart"><errorEventDefinition /></startEvent>"#,
        &["errorEventDefinition on startEvent 'errStart'"],
    );
}

#[test]
fn reject_escalation_start_event() {
    assert_rejected(
        r#"<startEvent id="escStart"><escalationEventDefinition /></startEvent>"#,
        &["escalationEventDefinition on startEvent 'escStart'"],
    );
}

#[test]
fn reject_compensation_start_event() {
    assert_rejected(
        r#"<startEvent id="compStart"><compensateEventDefinition /></startEvent>"#,
        &["compensateEventDefinition on startEvent 'compStart'"],
    );
}

#[test]
fn reject_link_start_event() {
    assert_rejected(
        r#"<startEvent id="linkStart"><linkEventDefinition name="L" /></startEvent>"#,
        &["linkEventDefinition on startEvent 'linkStart'"],
    );
}

#[test]
fn reject_multiple_event_definition_on_start() {
    assert_rejected(
        r#"<startEvent id="multi"><multipleEventDefinition /></startEvent>"#,
        &["multipleEventDefinition on startEvent 'multi'"],
    );
}

#[test]
fn reject_parallel_multiple_event_definition_on_start() {
    assert_rejected(
        r#"<startEvent id="pm"><parallelMultipleEventDefinition /></startEvent>"#,
        &["parallelMultipleEventDefinition on startEvent 'pm'"],
    );
}

#[test]
fn reject_cancel_start_event() {
    assert_rejected(
        r#"<startEvent id="cStart"><cancelEventDefinition /></startEvent>"#,
        &["cancelEventDefinition on startEvent 'cStart'"],
    );
}

#[test]
fn reject_timer_on_end_event() {
    assert_rejected(
        r#"<endEvent id="timerEnd"><timerEventDefinition><timeDuration>PT1S</timeDuration></timerEventDefinition></endEvent>"#,
        &["timerEventDefinition on endEvent 'timerEnd'"],
    );
}

#[test]
fn reject_conditional_on_end_event() {
    assert_rejected(
        r#"<endEvent id="condEnd"><conditionalEventDefinition><conditionExpression>x &gt; 0</conditionExpression></conditionalEventDefinition></endEvent>"#,
        &["conditionalEventDefinition on endEvent 'condEnd'"],
    );
}

#[test]
fn reject_link_on_end_event() {
    assert_rejected(
        r#"<endEvent id="linkEnd"><linkEventDefinition name="L" /></endEvent>"#,
        &["linkEventDefinition on endEvent 'linkEnd'"],
    );
}

#[test]
fn reject_error_on_intermediate_catch() {
    assert_rejected(
        r#"<intermediateCatchEvent id="badCatch"><errorEventDefinition /></intermediateCatchEvent>"#,
        &["errorEventDefinition on intermediateCatchEvent 'badCatch'"],
    );
}

#[test]
fn reject_terminate_on_intermediate_catch() {
    assert_rejected(
        r#"<intermediateCatchEvent id="badCatch"><terminateEventDefinition /></intermediateCatchEvent>"#,
        &["terminateEventDefinition on intermediateCatchEvent 'badCatch'"],
    );
}

#[test]
fn reject_compensation_on_intermediate_catch() {
    assert_rejected(
        r#"<intermediateCatchEvent id="badCatch"><compensateEventDefinition /></intermediateCatchEvent>"#,
        &["compensateEventDefinition on intermediateCatchEvent 'badCatch'"],
    );
}

#[test]
fn reject_escalation_on_intermediate_catch() {
    assert_rejected(
        r#"<intermediateCatchEvent id="badCatch"><escalationEventDefinition /></intermediateCatchEvent>"#,
        &["escalationEventDefinition on intermediateCatchEvent 'badCatch'"],
    );
}

#[test]
fn reject_cancel_on_intermediate_catch() {
    assert_rejected(
        r#"<intermediateCatchEvent id="badCatch"><cancelEventDefinition /></intermediateCatchEvent>"#,
        &["cancelEventDefinition on intermediateCatchEvent 'badCatch'"],
    );
}

#[test]
fn reject_timer_on_intermediate_throw() {
    assert_rejected(
        r#"<intermediateThrowEvent id="badThrow"><timerEventDefinition><timeDuration>PT1S</timeDuration></timerEventDefinition></intermediateThrowEvent>"#,
        &["timerEventDefinition on intermediateThrowEvent 'badThrow'"],
    );
}

#[test]
fn reject_error_on_intermediate_throw() {
    assert_rejected(
        r#"<intermediateThrowEvent id="badThrow"><errorEventDefinition /></intermediateThrowEvent>"#,
        &["errorEventDefinition on intermediateThrowEvent 'badThrow'"],
    );
}

#[test]
fn reject_terminate_on_intermediate_throw() {
    assert_rejected(
        r#"<intermediateThrowEvent id="badThrow"><terminateEventDefinition /></intermediateThrowEvent>"#,
        &["terminateEventDefinition on intermediateThrowEvent 'badThrow'"],
    );
}

#[test]
fn reject_conditional_on_intermediate_throw() {
    assert_rejected(
        r#"<intermediateThrowEvent id="badThrow"><conditionalEventDefinition><conditionExpression>x &gt; 1</conditionExpression></conditionalEventDefinition></intermediateThrowEvent>"#,
        &["conditionalEventDefinition on intermediateThrowEvent 'badThrow'"],
    );
}

#[test]
fn reject_link_on_boundary_event() {
    assert_rejected(
        r#"<boundaryEvent id="b1" attachedToRef="s"><linkEventDefinition name="L" /></boundaryEvent>"#,
        &["linkEventDefinition on boundaryEvent 'b1'"],
    );
}

#[test]
fn reject_terminate_on_boundary_event() {
    assert_rejected(
        r#"<boundaryEvent id="b1" attachedToRef="s"><terminateEventDefinition /></boundaryEvent>"#,
        &["terminateEventDefinition on boundaryEvent 'b1'"],
    );
}

#[test]
fn reject_timer_start_inside_embedded_subprocess() {
    assert_rejected(
        r#"<subProcess id="sp">
             <startEvent id="ss"><timerEventDefinition><timeDuration>PT1S</timeDuration></timerEventDefinition></startEvent>
             <endEvent id="se" />
             <sequenceFlow id="sf" sourceRef="ss" targetRef="se" />
           </subProcess>"#,
        &[
            "timerEventDefinition on startEvent 'ss'",
            "subProcess 'sp'",
            "pass-through",
        ],
    );
}

#[test]
fn reject_signal_start_inside_embedded_subprocess() {
    assert_rejected(
        r#"<subProcess id="sp">
             <startEvent id="ss"><signalEventDefinition signalRef="sig" /></startEvent>
             <endEvent id="se" />
             <sequenceFlow id="sf" sourceRef="ss" targetRef="se" />
           </subProcess>"#,
        &[
            "signalEventDefinition on startEvent 'ss'",
            "subProcess 'sp'",
        ],
    );
}

#[test]
fn reject_message_start_inside_embedded_subprocess() {
    assert_rejected(
        r#"<subProcess id="sp">
             <startEvent id="ss"><messageEventDefinition messageRef="msg" /></startEvent>
             <endEvent id="se" />
             <sequenceFlow id="sf" sourceRef="ss" targetRef="se" />
           </subProcess>"#,
        &[
            "messageEventDefinition on startEvent 'ss'",
            "subProcess 'sp'",
        ],
    );
}

#[test]
fn unsupported_elements_are_all_reported() {
    let err = rejection_of(
        r#"<adHocSubProcess id="adhoc" />
           <userTask id="ut"><standardLoopCharacteristics /></userTask>
           <transaction id="tx1" />
           <endEvent id="msgEnd"><messageEventDefinition messageRef="msg" /></endEvent>
           <receiveTask id="recv" instantiate="true" messageRef="msg" />"#,
    );
    for needle in [
        "adHocSubProcess 'adhoc'",
        "standardLoopCharacteristics 'ut'",
        "transaction 'tx1'",
        "messageEventDefinition on endEvent 'msgEnd'",
    ] {
        assert!(err.contains(needle), "missing {needle} in {err}");
    }
    assert!(
        err.contains("Unsupported BPMN element(s) in process 'p'"),
        "{err}"
    );
}

#[test]
fn unsupported_element_error_includes_source_line() {
    let xml = process_with(r#"<adHocSubProcess id="adhoc" />"#);
    let marker = xml
        .find("<adHocSubProcess")
        .expect("fixture contains the element");
    let expected = xml[..marker].bytes().filter(|&byte| byte == b'\n').count() + 1;
    let err = parse_bpmn_xml(&xml).unwrap_err().to_string();
    assert!(
        err.contains(&format!("line {expected}")),
        "expected line {expected} in {err}"
    );
}

#[test]
fn visual_and_data_elements_still_deploy() {
    let xml = r#"<definitions id="d" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
         <collaboration id="c"><participant id="pool" processRef="p" /></collaboration>
         <process id="p" isExecutable="true">
           <laneSet id="ls"><lane id="lane1"><flowNodeRef>s</flowNodeRef></lane></laneSet>
           <startEvent id="s" />
           <endEvent id="e" />
           <sequenceFlow id="f" sourceRef="s" targetRef="e" />
           <dataObject id="do1" />
           <dataObjectReference id="dor1" dataObjectRef="do1" />
           <dataStoreReference id="ds1" />
           <textAnnotation id="ta"><text>note</text></textAnnotation>
           <association id="as" sourceRef="s" targetRef="ta" />
           <group id="g" />
         </process>
       </definitions>"#;
    let def = parse_bpmn_xml(xml).expect("visual and data elements stay valid");
    assert!(matches!(def.nodes.get("s"), Some(BpmnElement::StartEvent)));
}

#[test]
fn unsupported_elements_outside_the_deployed_process_are_ignored() {
    let xml = r#"<definitions id="d" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
        <process id="p" isExecutable="true">
          <startEvent id="s" />
          <endEvent id="e" />
          <sequenceFlow id="f" sourceRef="s" targetRef="e" />
        </process>
        <process id="other" isExecutable="false">
          <adHocSubProcess id="adhoc" />
          <startEvent id="os"><errorEventDefinition /></startEvent>
        </process>
      </definitions>"#;
    let def = parse_bpmn_xml(xml).expect("only the executable process is checked");
    assert_eq!(def.id, "p");
}

#[test]
fn non_executable_process_is_still_checked_when_it_is_the_only_one() {
    let err = parse_bpmn_xml(
        r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
            <process id="only" isExecutable="false">
              <adHocSubProcess id="adhoc" />
              <startEvent id="s" /><endEvent id="e" />
              <sequenceFlow id="f" sourceRef="s" targetRef="e" />
            </process>
          </definitions>"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("process 'only'") && err.contains("adhoc"),
        "{err}"
    );
}

#[test]
fn supported_constructs_still_deploy() {
    let samples = [
        (
            "signal start",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <signal id="sig1" name="Go" />
                <process id="p" isExecutable="true">
                  <startEvent id="s"><signalEventDefinition signalRef="sig1" /></startEvent>
                  <endEvent id="e" />
                  <sequenceFlow id="f" sourceRef="s" targetRef="e" />
                </process>
              </definitions>"#,
            "s",
        ),
        (
            "conditional catch",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <process id="p" isExecutable="true">
                  <startEvent id="s" />
                  <intermediateCatchEvent id="c">
                    <conditionalEventDefinition><conditionExpression>x &gt; 0</conditionExpression></conditionalEventDefinition>
                  </intermediateCatchEvent>
                  <endEvent id="e" />
                  <sequenceFlow id="f1" sourceRef="s" targetRef="c" />
                  <sequenceFlow id="f2" sourceRef="c" targetRef="e" />
                </process>
              </definitions>"#,
            "c",
        ),
        (
            "link throw",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <process id="p" isExecutable="true">
                  <startEvent id="s" />
                  <intermediateThrowEvent id="throw1"><linkEventDefinition name="Jump" /></intermediateThrowEvent>
                  <intermediateCatchEvent id="catch1"><linkEventDefinition name="Jump" /></intermediateCatchEvent>
                  <endEvent id="e" />
                  <sequenceFlow id="f1" sourceRef="s" targetRef="throw1" />
                  <sequenceFlow id="f2" sourceRef="catch1" targetRef="e" />
                </process>
              </definitions>"#,
            "throw1",
        ),
        (
            "sequential multi-instance service task",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <process id="p" isExecutable="true">
                  <startEvent id="s" />
                  <serviceTask id="svc" data-topic="work">
                    <multiInstanceLoopCharacteristics isSequential="true">
                      <loopCardinality>2</loopCardinality>
                    </multiInstanceLoopCharacteristics>
                  </serviceTask>
                  <endEvent id="e" />
                  <sequenceFlow id="f1" sourceRef="s" targetRef="svc" />
                  <sequenceFlow id="f2" sourceRef="svc" targetRef="e" />
                </process>
              </definitions>"#,
            "svc",
        ),
        (
            "receive task",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <message id="Message_1" name="OrderReceived" />
                <process id="p" isExecutable="true">
                  <startEvent id="s" />
                  <receiveTask id="recv" messageRef="Message_1" />
                  <endEvent id="e" />
                  <sequenceFlow id="f1" sourceRef="s" targetRef="recv" />
                  <sequenceFlow id="f2" sourceRef="recv" targetRef="e" />
                </process>
              </definitions>"#,
            "recv",
        ),
        (
            "exclusive event-based gateway",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <message id="Message_1" name="M" />
                <process id="p" isExecutable="true">
                  <startEvent id="s" />
                  <eventBasedGateway id="gw" eventGatewayType="Exclusive" />
                  <intermediateCatchEvent id="wait_msg"><messageEventDefinition messageRef="Message_1" /></intermediateCatchEvent>
                  <intermediateCatchEvent id="wait_timer"><timerEventDefinition><timeDuration>PT10S</timeDuration></timerEventDefinition></intermediateCatchEvent>
                  <endEvent id="e1" />
                  <endEvent id="e2" />
                  <sequenceFlow id="f1" sourceRef="s" targetRef="gw" />
                  <sequenceFlow id="f2" sourceRef="gw" targetRef="wait_msg" />
                  <sequenceFlow id="f3" sourceRef="gw" targetRef="wait_timer" />
                  <sequenceFlow id="f4" sourceRef="wait_msg" targetRef="e1" />
                  <sequenceFlow id="f5" sourceRef="wait_timer" targetRef="e2" />
                </process>
              </definitions>"#,
            "gw",
        ),
        (
            "embedded subprocess",
            r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
                <process id="p" isExecutable="true">
                  <startEvent id="s" />
                  <subProcess id="sp">
                    <startEvent id="ss" />
                    <endEvent id="se" />
                    <sequenceFlow id="sf" sourceRef="ss" targetRef="se" />
                  </subProcess>
                  <endEvent id="e" />
                  <sequenceFlow id="f1" sourceRef="s" targetRef="sp" />
                  <sequenceFlow id="f2" sourceRef="sp" targetRef="e" />
                </process>
              </definitions>"#,
            "sp",
        ),
    ];

    for (label, xml, node) in samples {
        let def = parse_bpmn_xml(xml).unwrap_or_else(|err| panic!("{label} should deploy: {err}"));
        assert!(def.nodes.contains_key(node), "{label}: missing node {node}");
    }
}

#[test]
fn element_matrix_matches_parser_support_lists() {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::parser::{
        REJECTED_ATTRIBUTES, REJECTED_EVENT_DEFINITIONS, SUPPORTED_EVENT_DEFINITIONS,
        UNSUPPORTED_ELEMENTS,
    };

    let doc = include_str!("../../docs/knowledge-base/modules/bpmn-parser/element-matrix.md");
    let fence = "```support-index\n";
    let start = doc.find(fence).expect("support-index fence") + fence.len();
    let rest = &doc[start..];
    let end = rest.find("\n```").expect("closing fence");
    let index = &rest[..end];

    let words =
        |raw: &str| -> BTreeSet<String> { raw.split_whitespace().map(str::to_string).collect() };

    let mut supported = BTreeMap::<String, BTreeSet<String>>::new();
    let mut unsupported = BTreeSet::new();
    let mut rejected_attributes = BTreeSet::new();
    let mut rejected_events = BTreeSet::new();

    for line in index.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .unwrap_or_else(|| panic!("index line needs a colon: {line}"));
        match key.trim() {
            "unsupported-elements" => unsupported = words(value),
            "rejected-attributes" => rejected_attributes = words(value),
            "rejected-events" => rejected_events = words(value),
            kind => {
                supported.insert(kind.to_string(), words(value));
            }
        }
    }

    let code_unsupported: BTreeSet<String> = UNSUPPORTED_ELEMENTS
        .iter()
        .map(|item| (*item).to_string())
        .collect();
    assert_eq!(unsupported, code_unsupported);

    let code_attributes: BTreeSet<String> = REJECTED_ATTRIBUTES
        .iter()
        .map(|item| (*item).to_string())
        .collect();
    assert_eq!(rejected_attributes, code_attributes);

    let code_rejected: BTreeSet<String> = REJECTED_EVENT_DEFINITIONS
        .iter()
        .map(|(kind, definition)| format!("{kind}/{definition}"))
        .collect();
    assert_eq!(rejected_events, code_rejected);

    assert_eq!(
        supported.len(),
        SUPPORTED_EVENT_DEFINITIONS.len(),
        "event kinds in the matrix index and the parser diverged"
    );
    for (kind, definitions) in SUPPORTED_EVENT_DEFINITIONS {
        let documented = supported
            .get(*kind)
            .unwrap_or_else(|| panic!("matrix index is missing {kind}"));
        let code: BTreeSet<String> = definitions.iter().map(|item| (*item).to_string()).collect();
        assert_eq!(documented, &code, "{kind}");
        for (rejected_kind, rejected_def) in REJECTED_EVENT_DEFINITIONS {
            if rejected_kind == kind {
                assert!(
                    !code.contains(*rejected_def),
                    "{rejected_def} is both supported and rejected on {kind}"
                );
            }
        }
    }
}
