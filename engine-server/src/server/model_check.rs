//! Deploy-time model checking (V1) via `rust_bpmn_analyzer`.
//!
//! Checks the deployed BPMN for safeness, option to complete (no deadlocks /
//! livelocks), proper completion and dead activities, using partial order
//! reduction. Mode per `MODEL_CHECK` (`off` | `warn` | `enforce`, default
//! `warn`): `warn` reports findings in the deploy response, `enforce` rejects
//! the deploy.
//!
//! State-space exploration cannot be cancelled once started, so at most one
//! analysis runs at a time (others are reported as skipped) and a deploy never
//! waits longer than `MODEL_CHECK_TIMEOUT_MS` (then: inconclusive).

use std::sync::Arc;

use rust_bpmn_analyzer::{AmpleSetConfig, Property};
use serde::Serialize;
use tokio::sync::Semaphore;

/// Default wall-clock budget for one analysis.
pub(crate) const DEFAULT_MODEL_CHECK_TIMEOUT_MS: u64 = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelCheckMode {
    Off,
    #[default]
    Warn,
    Enforce,
}

impl ModelCheckMode {
    /// Parses `off` / `warn` / `enforce` (case-insensitive); anything else → `Warn`.
    pub(crate) fn parse(value: Option<&str>) -> Self {
        match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            Some("off") | Some("false") | Some("0") => Self::Off,
            Some("enforce") => Self::Enforce,
            _ => Self::Warn,
        }
    }

    pub(crate) fn from_env() -> Self {
        Self::parse(std::env::var("MODEL_CHECK").ok().as_deref())
    }
}

pub(crate) fn timeout_ms_from_env() -> u64 {
    std::env::var("MODEL_CHECK_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|ms| *ms > 0)
        .unwrap_or(DEFAULT_MODEL_CHECK_TIMEOUT_MS)
}

/// Model checking settings + the single-analysis guard (part of `AppState`).
pub(crate) struct ModelChecker {
    pub(crate) mode: ModelCheckMode,
    timeout: std::time::Duration,
    guard: Arc<Semaphore>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ModelCheckStatus {
    Passed,
    Violations,
    Skipped,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Violation {
    pub property: &'static str,
    pub elements: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ModelCheckReport {
    pub status: ModelCheckStatus,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ModelCheckReport {
    fn without_result(status: ModelCheckStatus, reason: impl Into<String>) -> Self {
        Self {
            status,
            violations: Vec::new(),
            reason: Some(reason.into()),
        }
    }

    /// Human-readable summary for logs and `enforce` errors.
    pub(crate) fn summary(&self) -> String {
        self.violations
            .iter()
            .map(|v| {
                if v.elements.is_empty() {
                    v.property.to_string()
                } else {
                    format!("{} ({})", v.property, v.elements.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn property_name(property: &Property) -> &'static str {
    match property {
        Property::Safeness => "safeness",
        Property::OptionToComplete => "option_to_complete",
        Property::ProperCompletion => "proper_completion",
        Property::NoDeadActivities => "no_dead_activities",
    }
}

/// Rewrites self-closing elements (`<startEvent id="s"/>`) into start/end
/// pairs. The analyzer's reader only registers events and gateways written in
/// the long form (as bpmn-js does); self-closing ones are silently dropped and
/// later crash it. The rewrite does not change the XML's meaning.
fn expand_empty_elements(xml: &str) -> Result<String, String> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut writer = quick_xml::Writer::new(Vec::with_capacity(xml.len() + xml.len() / 4));
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let written = match event {
            Event::Eof => break,
            Event::Empty(e) => {
                let end = e.to_end().into_owned();
                writer
                    .write_event(Event::Start(e))
                    .and_then(|()| writer.write_event(Event::End(end)))
            }
            other => writer.write_event(other),
        };
        written.map_err(|e| e.to_string())?;
    }
    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

/// Runs the analyzer synchronously (call from a blocking thread).
fn analyze(xml: &str) -> ModelCheckReport {
    if bpmn_parser::parse_bpmn_xml(xml).is_ok_and(|def| !def.event_subprocesses.is_empty()) {
        return ModelCheckReport::without_result(
            ModelCheckStatus::Skipped,
            "Event subprocess bodies are structurally validated; this analyzer does not model subscriptions, interruption, or repeated activations",
        );
    }
    let xml = match expand_empty_elements(xml) {
        Ok(xml) => xml,
        Err(e) => {
            return ModelCheckReport::without_result(
                ModelCheckStatus::Skipped,
                format!("could not prepare XML for analysis: {e}"),
            );
        }
    };
    let collaboration = match rust_bpmn_analyzer::read_bpmn_from_string(&xml) {
        Ok(c) => c,
        Err(e) => {
            return ModelCheckReport::without_result(
                ModelCheckStatus::Skipped,
                format!(
                    "model contains elements the analyzer does not support: {}",
                    e.unsupported_elements.join(", ")
                ),
            );
        }
    };
    let properties = vec![
        Property::Safeness,
        Property::OptionToComplete,
        Property::ProperCompletion,
        Property::NoDeadActivities,
    ];
    let checked =
        rust_bpmn_analyzer::run_with_por(&collaboration, properties, AmpleSetConfig::default());
    let violations: Vec<Violation> = checked
        .result
        .property_results
        .iter()
        .filter(|r| !r.fulfilled)
        .map(|r| Violation {
            property: property_name(&r.property),
            elements: r.problematic_elements.clone(),
        })
        .collect();
    ModelCheckReport {
        status: if violations.is_empty() {
            ModelCheckStatus::Passed
        } else {
            ModelCheckStatus::Violations
        },
        violations,
        reason: None,
    }
}

impl ModelChecker {
    pub(crate) fn new(mode: ModelCheckMode, timeout_ms: u64) -> Self {
        Self {
            mode,
            timeout: std::time::Duration::from_millis(timeout_ms),
            guard: Arc::new(Semaphore::new(1)),
        }
    }

    /// Checks `xml`. `None` when model checking is off.
    pub(crate) async fn check(&self, xml: &str) -> Option<ModelCheckReport> {
        if self.mode == ModelCheckMode::Off {
            return None;
        }
        let Ok(permit) = Arc::clone(&self.guard).try_acquire_owned() else {
            return Some(ModelCheckReport::without_result(
                ModelCheckStatus::Skipped,
                "another model check is still running",
            ));
        };
        let xml = xml.to_owned();
        // The permit moves into the thread: it is released when the analysis
        // really ends, even if the deploy stopped waiting earlier.
        let handle = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            analyze(&xml)
        });
        let report = match tokio::time::timeout(self.timeout, handle).await {
            Ok(Ok(report)) => report,
            Ok(Err(join_err)) => ModelCheckReport::without_result(
                ModelCheckStatus::Skipped,
                format!("analyzer failed: {join_err}"),
            ),
            Err(_) => ModelCheckReport::without_result(
                ModelCheckStatus::Inconclusive,
                format!("analysis exceeded {} ms", self.timeout.as_millis()),
            ),
        };
        Some(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_subprocess_semantics_are_reported_as_skipped() {
        let xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"><message id="m" name="alert"/><process id="p" isExecutable="true"><startEvent id="s"/><endEvent id="e"/><sequenceFlow id="f" sourceRef="s" targetRef="e"/><subProcess id="h" triggeredByEvent="true"><startEvent id="hs"><messageEventDefinition messageRef="m"/></startEvent><endEvent id="he"/><sequenceFlow id="hf" sourceRef="hs" targetRef="he"/></subProcess></process></definitions>"#;
        assert!(bpmn_parser::parse_bpmn_xml(xml).is_ok());
        assert_eq!(analyze(xml).status, ModelCheckStatus::Skipped);
    }

    #[test]
    fn expands_self_closing_elements() {
        let out = expand_empty_elements(r#"<a><b id="x"/><c>t</c></a>"#).unwrap();
        assert_eq!(out, r#"<a><b id="x"></b><c>t</c></a>"#);
    }

    #[test]
    fn parses_modes() {
        assert_eq!(ModelCheckMode::parse(None), ModelCheckMode::Warn);
        assert_eq!(ModelCheckMode::parse(Some("OFF")), ModelCheckMode::Off);
        assert_eq!(
            ModelCheckMode::parse(Some(" enforce ")),
            ModelCheckMode::Enforce
        );
        assert_eq!(ModelCheckMode::parse(Some("bogus")), ModelCheckMode::Warn);
    }
}
