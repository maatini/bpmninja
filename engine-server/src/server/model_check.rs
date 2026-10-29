//! Deploy-time model checking via `rust_bpmn_analyzer`.
//!
//! The Cargo feature `model-check` is off by default, so a normal build does
//! not compile the git dependency. At runtime `MODEL_CHECK` defaults to `off`.
//!
//! `warn` always finishes the deploy: findings are logged and returned in the
//! response. `enforce` rejects only a confirmed violation, or a binary that
//! was not built with the feature (otherwise enforce would silently do
//! nothing). A timeout, an analyzer panic, or a busy checker never fails a
//! deploy in `warn`, and does not fail one in `enforce` either — the response
//! records `inconclusive`, `failed`, or `skipped`.
//!
//! At most one analysis runs at a time. The work is `spawn_blocking`, so the
//! async runtime is not blocked. The permit stays with that thread until the
//! analysis actually ends, including after the deploy has stopped waiting.

use std::time::Duration;

use serde::Serialize;
#[cfg(any(test, feature = "model-check"))]
use std::sync::Arc;
#[cfg(any(test, feature = "model-check"))]
use tokio::sync::Semaphore;

/// Default wall-clock budget for one analysis. A second deploy does not wait
/// for the in-flight permit; it is reported as skipped.
pub(crate) const DEFAULT_MODEL_CHECK_TIMEOUT_MS: u64 = 2_000;

/// Runtime switch read from `MODEL_CHECK`.
///
/// Default is [`Off`](Self::Off). `Warn` and `Enforce` analyze a model only
/// when this binary was built with `--features model-check`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelCheckMode {
    /// Do not analyze. The deploy response has no `model_check` field.
    #[default]
    Off,
    /// Deploy succeeds. Findings are logged and included in the response.
    Warn,
    /// Confirmed violations reject the deploy. See the module docs for the
    /// cases that still deploy.
    Enforce,
}

impl ModelCheckMode {
    /// `off`/`false`/`0` → off, `warn`/`true`/`on`/`1` → warn, `enforce` →
    /// enforce. Anything else, including a missing value, is off.
    pub(crate) fn parse(value: Option<&str>) -> Self {
        match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            Some("warn" | "true" | "on" | "1") => Self::Warn,
            Some("enforce") => Self::Enforce,
            _ => Self::Off,
        }
    }

    pub(crate) fn from_env() -> Self {
        let raw = std::env::var("MODEL_CHECK").ok();
        let mode = Self::parse(raw.as_deref());
        if let Some(value) = raw.as_deref() {
            let normalized = value.trim().to_ascii_lowercase();
            let known = matches!(
                normalized.as_str(),
                "" | "off" | "false" | "0" | "warn" | "true" | "on" | "1" | "enforce"
            );
            if !known {
                tracing::warn!(value, "unknown MODEL_CHECK value, using off");
            }
        }
        mode
    }
}

pub(crate) fn timeout_ms_from_env() -> u64 {
    parse_timeout_ms(std::env::var("MODEL_CHECK_TIMEOUT_MS").ok().as_deref())
}

pub(crate) fn parse_timeout_ms(value: Option<&str>) -> u64 {
    value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .unwrap_or(DEFAULT_MODEL_CHECK_TIMEOUT_MS)
}

/// Settings plus the single-analysis guard. One instance lives in `AppState`.
pub(crate) struct ModelChecker {
    pub(crate) mode: ModelCheckMode,
    timeout: Duration,
    #[cfg(any(test, feature = "model-check"))]
    guard: Arc<Semaphore>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCheckStatus {
    Passed,
    Violations,
    /// Unsupported elements, or another analysis still holds the permit.
    Skipped,
    /// The deploy stopped waiting. The analysis may still be running.
    Inconclusive,
    /// The analyzer panicked or the blocking task failed.
    Failed,
    /// `MODEL_CHECK` is on, but the binary has no analyzer.
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Violation {
    pub property: &'static str,
    pub elements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModelCheckReport {
    pub status: ModelCheckStatus,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ModelCheckReport {
    fn with_reason(status: ModelCheckStatus, reason: impl Into<String>) -> Self {
        Self {
            status,
            violations: Vec::new(),
            reason: Some(reason.into()),
        }
    }

    /// Text used in logs and in the `error` string of an enforce rejection.
    pub(crate) fn summary(&self) -> String {
        self.violations
            .iter()
            .map(|violation| {
                if violation.elements.is_empty() {
                    violation.property.to_string()
                } else {
                    format!("{} ({})", violation.property, violation.elements.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub(crate) fn error_detail(&self) -> String {
        let findings = self.summary();
        if !findings.is_empty() {
            return findings;
        }
        self.reason
            .clone()
            .unwrap_or_else(|| "model check rejected the deploy".to_string())
    }
}

impl ModelChecker {
    pub(crate) fn new(mode: ModelCheckMode, timeout_ms: u64) -> Self {
        let timeout_ms = if timeout_ms == 0 {
            DEFAULT_MODEL_CHECK_TIMEOUT_MS
        } else {
            timeout_ms
        };
        Self {
            mode,
            timeout: Duration::from_millis(timeout_ms),
            #[cfg(any(test, feature = "model-check"))]
            guard: Arc::new(Semaphore::new(1)),
        }
    }

    pub(crate) fn timeout_ms(&self) -> u64 {
        u64::try_from(self.timeout.as_millis()).unwrap_or(u64::MAX)
    }

    /// Enforce rejects confirmed violations and a missing analyzer.
    ///
    /// Timeouts, panics, and a busy checker do not reject: those deploys
    /// succeed and carry the report. Warn never rejects.
    pub(crate) fn rejects_deploy(&self, report: &ModelCheckReport) -> bool {
        if self.mode != ModelCheckMode::Enforce {
            return false;
        }
        matches!(
            report.status,
            ModelCheckStatus::Violations | ModelCheckStatus::Unavailable
        )
    }

    /// `None` when model checking is off.
    pub(crate) async fn check(&self, xml: &str) -> Option<ModelCheckReport> {
        if self.mode == ModelCheckMode::Off {
            None
        } else {
            self.check_enabled(xml).await
        }
    }

    #[cfg(not(feature = "model-check"))]
    async fn check_enabled(&self, _xml: &str) -> Option<ModelCheckReport> {
        tracing::warn!(
            mode = ?self.mode,
            "MODEL_CHECK is enabled but engine-server was built without the model-check feature"
        );
        Some(ModelCheckReport::with_reason(
            ModelCheckStatus::Unavailable,
            "engine-server was built without the model-check Cargo feature",
        ))
    }

    #[cfg(feature = "model-check")]
    async fn check_enabled(&self, xml: &str) -> Option<ModelCheckReport> {
        Some(self.run_blocking(xml, analyze_bpmn).await)
    }

    /// Runs `analyze` on the blocking pool behind a one-permit semaphore.
    ///
    /// A panic inside `analyze` becomes [`ModelCheckStatus::Failed`]. A wait
    /// longer than the configured budget becomes
    /// [`ModelCheckStatus::Inconclusive`] while the permit stays held until
    /// the blocking call returns.
    #[cfg(any(test, feature = "model-check"))]
    pub(crate) async fn run_blocking<F>(&self, xml: &str, analyze: F) -> ModelCheckReport
    where
        F: FnOnce(&str) -> ModelCheckReport + Send + 'static,
    {
        let Ok(permit) = Arc::clone(&self.guard).try_acquire_owned() else {
            return ModelCheckReport::with_reason(
                ModelCheckStatus::Skipped,
                "another model check is still running",
            );
        };
        let xml = xml.to_owned();
        let timeout = self.timeout;
        let timeout_ms = self.timeout_ms();
        let handle = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| analyze(&xml))) {
                Ok(report) => report,
                Err(_) => {
                    ModelCheckReport::with_reason(ModelCheckStatus::Failed, "analyzer panicked")
                }
            }
        });
        match tokio::time::timeout(timeout, handle).await {
            Ok(Ok(report)) => report,
            Ok(Err(join_err)) => ModelCheckReport::with_reason(
                ModelCheckStatus::Failed,
                format!("analyzer task failed: {join_err}"),
            ),
            Err(_) => ModelCheckReport::with_reason(
                ModelCheckStatus::Inconclusive,
                format!("analysis exceeded {timeout_ms} ms"),
            ),
        }
    }
}

pub(crate) fn log_report(definition_name: &str, report: &ModelCheckReport) {
    match report.status {
        ModelCheckStatus::Passed => {
            tracing::debug!(definition = definition_name, "model check passed");
        }
        ModelCheckStatus::Violations => {
            tracing::warn!(
                definition = definition_name,
                findings = %report.summary(),
                "model check found violations"
            );
        }
        ModelCheckStatus::Skipped
        | ModelCheckStatus::Inconclusive
        | ModelCheckStatus::Failed
        | ModelCheckStatus::Unavailable => {
            tracing::warn!(
                definition = definition_name,
                status = ?report.status,
                reason = report.reason.as_deref().unwrap_or(""),
                "model check did not produce a verdict"
            );
        }
    }
}

#[cfg(feature = "model-check")]
fn property_name(property: &rust_bpmn_analyzer::Property) -> &'static str {
    match property {
        rust_bpmn_analyzer::Property::Safeness => "safeness",
        rust_bpmn_analyzer::Property::OptionToComplete => "option_to_complete",
        rust_bpmn_analyzer::Property::ProperCompletion => "proper_completion",
        rust_bpmn_analyzer::Property::NoDeadActivities => "no_dead_activities",
    }
}

/// The analyzer's reader drops self-closing events and gateways and can panic
/// later. Expanding them does not change the model.
#[cfg(feature = "model-check")]
fn expand_empty_elements(xml: &str) -> Result<String, String> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut writer = quick_xml::Writer::new(Vec::with_capacity(xml.len() + xml.len() / 4));
    loop {
        let event = reader.read_event().map_err(|error| error.to_string())?;
        let written = match event {
            Event::Eof => break,
            Event::Empty(start) => {
                let end = start.to_end().into_owned();
                writer
                    .write_event(Event::Start(start))
                    .and_then(|()| writer.write_event(Event::End(end)))
            }
            other => writer.write_event(other),
        };
        written.map_err(|error| error.to_string())?;
    }
    String::from_utf8(writer.into_inner()).map_err(|error| error.to_string())
}

#[cfg(feature = "model-check")]
fn analyze_bpmn(xml: &str) -> ModelCheckReport {
    let xml = match expand_empty_elements(xml) {
        Ok(xml) => xml,
        Err(error) => {
            return ModelCheckReport::with_reason(
                ModelCheckStatus::Skipped,
                format!("could not prepare XML for analysis: {error}"),
            );
        }
    };
    let collaboration = match rust_bpmn_analyzer::read_bpmn_from_string(&xml) {
        Ok(collaboration) => collaboration,
        Err(error) => {
            return ModelCheckReport::with_reason(
                ModelCheckStatus::Skipped,
                format!(
                    "model contains elements the analyzer does not support: {}",
                    error.unsupported_elements.join(", ")
                ),
            );
        }
    };
    let properties = vec![
        rust_bpmn_analyzer::Property::Safeness,
        rust_bpmn_analyzer::Property::OptionToComplete,
        rust_bpmn_analyzer::Property::ProperCompletion,
        rust_bpmn_analyzer::Property::NoDeadActivities,
    ];
    let checked = rust_bpmn_analyzer::run_with_por(
        &collaboration,
        properties,
        rust_bpmn_analyzer::AmpleSetConfig::default(),
    );
    let violations: Vec<Violation> = checked
        .result
        .property_results
        .iter()
        .filter(|result| !result.fulfilled)
        .map(|result| Violation {
            property: property_name(&result.property),
            elements: result.problematic_elements.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tracing_subscriber::prelude::*;

    fn passed_report() -> ModelCheckReport {
        ModelCheckReport {
            status: ModelCheckStatus::Passed,
            violations: Vec::new(),
            reason: None,
        }
    }

    fn report(status: ModelCheckStatus) -> ModelCheckReport {
        ModelCheckReport {
            status,
            violations: vec![Violation {
                property: "option_to_complete",
                elements: vec!["Join".to_string()],
            }],
            reason: Some("detail".to_string()),
        }
    }

    #[test]
    fn parses_modes_and_defaults_to_off() {
        assert_eq!(ModelCheckMode::default(), ModelCheckMode::Off);
        assert_eq!(ModelCheckMode::parse(None), ModelCheckMode::Off);
        assert_eq!(ModelCheckMode::parse(Some("")), ModelCheckMode::Off);
        assert_eq!(ModelCheckMode::parse(Some("OFF")), ModelCheckMode::Off);
        assert_eq!(ModelCheckMode::parse(Some("false")), ModelCheckMode::Off);
        assert_eq!(ModelCheckMode::parse(Some("0")), ModelCheckMode::Off);
        assert_eq!(ModelCheckMode::parse(Some(" warn ")), ModelCheckMode::Warn);
        assert_eq!(ModelCheckMode::parse(Some("true")), ModelCheckMode::Warn);
        assert_eq!(ModelCheckMode::parse(Some("on")), ModelCheckMode::Warn);
        assert_eq!(ModelCheckMode::parse(Some("1")), ModelCheckMode::Warn);
        assert_eq!(
            ModelCheckMode::parse(Some(" enforce ")),
            ModelCheckMode::Enforce
        );
        assert_eq!(ModelCheckMode::parse(Some("bogus")), ModelCheckMode::Off);
    }

    #[test]
    fn parses_timeout_with_fallback() {
        assert_eq!(parse_timeout_ms(None), DEFAULT_MODEL_CHECK_TIMEOUT_MS);
        assert_eq!(parse_timeout_ms(Some("0")), DEFAULT_MODEL_CHECK_TIMEOUT_MS);
        assert_eq!(
            parse_timeout_ms(Some(" -5 ")),
            DEFAULT_MODEL_CHECK_TIMEOUT_MS
        );
        assert_eq!(
            parse_timeout_ms(Some("nope")),
            DEFAULT_MODEL_CHECK_TIMEOUT_MS
        );
        assert_eq!(parse_timeout_ms(Some(" 1500 ")), 1_500);
    }

    #[test]
    fn summary_lists_findings_and_falls_back_to_the_reason() {
        let violations = ModelCheckReport {
            status: ModelCheckStatus::Violations,
            violations: vec![
                Violation {
                    property: "option_to_complete",
                    elements: vec!["Join".to_string()],
                },
                Violation {
                    property: "safeness",
                    elements: Vec::new(),
                },
            ],
            reason: None,
        };
        assert_eq!(violations.summary(), "option_to_complete (Join); safeness");
        assert_eq!(violations.error_detail(), violations.summary());

        let unavailable = ModelCheckReport::with_reason(
            ModelCheckStatus::Unavailable,
            "engine-server was built without the model-check Cargo feature",
        );
        assert_eq!(
            unavailable.error_detail(),
            "engine-server was built without the model-check Cargo feature"
        );
    }

    #[test]
    fn enforce_rejects_violations_and_a_missing_analyzer_only() {
        let warn = ModelChecker::new(ModelCheckMode::Warn, 1_000);
        let enforce = ModelChecker::new(ModelCheckMode::Enforce, 1_000);
        let off = ModelChecker::new(ModelCheckMode::Off, 1_000);
        let cases = [
            (ModelCheckStatus::Passed, false),
            (ModelCheckStatus::Violations, true),
            (ModelCheckStatus::Skipped, false),
            (ModelCheckStatus::Inconclusive, false),
            (ModelCheckStatus::Failed, false),
            (ModelCheckStatus::Unavailable, true),
        ];
        for (status, enforce_rejects) in cases {
            let report = report(status);
            assert!(
                !warn.rejects_deploy(&report),
                "warn must not reject {status:?}"
            );
            assert!(
                !off.rejects_deploy(&report),
                "off must not reject {status:?}"
            );
            assert_eq!(
                enforce.rejects_deploy(&report),
                enforce_rejects,
                "{status:?}"
            );
        }
    }

    #[tokio::test]
    async fn off_does_not_analyze() {
        let checker = ModelChecker::new(ModelCheckMode::Off, 1_000);
        assert!(checker.check("<definitions/>").await.is_none());
    }

    #[cfg(not(feature = "model-check"))]
    #[tokio::test]
    async fn warn_without_the_feature_is_unavailable_and_does_not_reject() {
        let checker = ModelChecker::new(ModelCheckMode::Warn, 1_000);
        let report = checker
            .check("<definitions/>")
            .await
            .expect("warn returns a report");
        assert_eq!(report.status, ModelCheckStatus::Unavailable);
        assert!(!checker.rejects_deploy(&report));
        let enforce = ModelChecker::new(ModelCheckMode::Enforce, 1_000);
        assert!(enforce.rejects_deploy(&report));
    }

    #[tokio::test]
    async fn timeout_is_inconclusive_and_holds_the_permit() {
        let checker = Arc::new(ModelChecker::new(ModelCheckMode::Warn, 50));
        let release = Arc::new(AtomicBool::new(false));
        let release_bg = Arc::clone(&release);
        let checker_bg = Arc::clone(&checker);
        let started = Instant::now();
        let first = tokio::spawn(async move {
            checker_bg
                .run_blocking("slow", move |_| {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !release_bg.load(Ordering::SeqCst) && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    passed_report()
                })
                .await
        });

        let first_report = tokio::time::timeout(Duration::from_secs(2), first)
            .await
            .expect("checker timeout did not return")
            .expect("blocking task join");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "deploy waited {:?}",
            started.elapsed()
        );
        assert_eq!(first_report.status, ModelCheckStatus::Inconclusive);
        assert!(!checker.rejects_deploy(&first_report));

        let skipped = checker.run_blocking("other", |_| passed_report()).await;
        assert_eq!(skipped.status, ModelCheckStatus::Skipped);
        release.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    #[tokio::test]
    async fn analyzer_panic_is_isolated_and_releases_the_permit() {
        let checker = ModelChecker::new(ModelCheckMode::Warn, 2_000);
        let failed = checker
            .run_blocking("xml", |_| panic!("analyzer bug"))
            .await;
        assert_eq!(failed.status, ModelCheckStatus::Failed);
        assert_eq!(failed.reason.as_deref(), Some("analyzer panicked"));
        assert!(!checker.rejects_deploy(&failed));

        let next = checker.run_blocking("xml", |_| passed_report()).await;
        assert_eq!(next.status, ModelCheckStatus::Passed);
    }

    #[tokio::test]
    async fn only_one_analysis_runs_at_a_time() {
        let checker = Arc::new(ModelChecker::new(ModelCheckMode::Warn, 5_000));
        let entered = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let entered_bg = Arc::clone(&entered);
        let release_bg = Arc::clone(&release);
        let checker_bg = Arc::clone(&checker);
        let first = tokio::spawn(async move {
            checker_bg
                .run_blocking("first", move |_| {
                    entered_bg.store(true, Ordering::SeqCst);
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !release_bg.load(Ordering::SeqCst) && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    passed_report()
                })
                .await
        });

        let wait_until = Instant::now() + Duration::from_secs(2);
        while !entered.load(Ordering::SeqCst) {
            assert!(Instant::now() < wait_until, "analysis did not start");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let second = checker.run_blocking("second", |_| passed_report()).await;
        assert_eq!(second.status, ModelCheckStatus::Skipped);
        assert!(
            second
                .reason
                .as_deref()
                .unwrap_or("")
                .contains("another model check"),
            "{second:?}"
        );

        release.store(true, Ordering::SeqCst);
        let first_report = tokio::time::timeout(Duration::from_secs(2), first)
            .await
            .expect("first analysis did not finish")
            .expect("join");
        assert_eq!(first_report.status, ModelCheckStatus::Passed);
    }

    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S> tracing_subscriber::Layer<S> for Capture
    where
        S: tracing::Subscriber,
    {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct Visit(String);
            impl tracing::field::Visit for Visit {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    use std::fmt::Write;
                    let _ = write!(self.0, "{}={:?} ", field.name(), value);
                }
            }
            let mut visit = Visit(String::new());
            event.record(&mut visit);
            let line = format!("{} {}", event.metadata().level(), visit.0);
            self.0.lock().expect("log capture").push(line);
        }
    }

    #[test]
    fn violations_are_logged_for_warn_mode() {
        let report = ModelCheckReport {
            status: ModelCheckStatus::Violations,
            violations: vec![Violation {
                property: "option_to_complete",
                elements: vec!["Join".to_string()],
            }],
            reason: None,
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(Capture(Arc::clone(&events)));
        tracing::subscriber::with_default(subscriber, || log_report("DeadlockProcess", &report));
        let recorded = events.lock().expect("log capture").join("\n");
        assert!(
            recorded.contains("WARN") && recorded.contains("option_to_complete"),
            "{recorded}"
        );
        assert!(recorded.contains("Join"), "{recorded}");
        assert!(recorded.contains("DeadlockProcess"), "{recorded}");
    }

    #[cfg(feature = "model-check")]
    #[test]
    fn expands_self_closing_elements() {
        let out = expand_empty_elements(r#"<a><b id="x"/><c>t</c></a>"#).expect("xml");
        assert_eq!(out, r#"<a><b id="x"></b><c>t</c></a>"#);
    }

    /// XOR split into an AND join: one token can never arrive. The analyzer
    /// reports that as `option_to_complete` (deadlock / stuck execution,
    /// including livelock-style states that cannot reach an end).
    #[cfg(feature = "model-check")]
    const DEADLOCK_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Deadlock" targetNamespace="http://bpmn.io/schema/bpmn">
  <bpmn:process id="DeadlockProcess" isExecutable="true">
    <bpmn:startEvent id="Start" />
    <bpmn:exclusiveGateway id="Split" />
    <bpmn:userTask id="A" />
    <bpmn:userTask id="B" />
    <bpmn:parallelGateway id="Join" />
    <bpmn:endEvent id="End" />
    <bpmn:sequenceFlow id="F1" sourceRef="Start" targetRef="Split" />
    <bpmn:sequenceFlow id="FA" sourceRef="Split" targetRef="A" />
    <bpmn:sequenceFlow id="FB" sourceRef="Split" targetRef="B" />
    <bpmn:sequenceFlow id="F2" sourceRef="A" targetRef="Join" />
    <bpmn:sequenceFlow id="F3" sourceRef="B" targetRef="Join" />
    <bpmn:sequenceFlow id="F4" sourceRef="Join" targetRef="End" />
  </bpmn:process>
</bpmn:definitions>"#;

    #[cfg(feature = "model-check")]
    const SOUND_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Sound" targetNamespace="http://bpmn.io/schema/bpmn">
  <bpmn:process id="SoundProcess" isExecutable="true">
    <bpmn:startEvent id="Start" />
    <bpmn:userTask id="Task" />
    <bpmn:endEvent id="End" />
    <bpmn:sequenceFlow id="F1" sourceRef="Start" targetRef="Task" />
    <bpmn:sequenceFlow id="F2" sourceRef="Task" targetRef="End" />
  </bpmn:process>
</bpmn:definitions>"#;

    #[cfg(feature = "model-check")]
    #[test]
    fn clean_model_passes_and_deadlock_is_a_finding() {
        let clean = analyze_bpmn(SOUND_BPMN);
        assert_eq!(clean.status, ModelCheckStatus::Passed, "{clean:?}");

        let deadlock = analyze_bpmn(DEADLOCK_BPMN);
        assert_eq!(
            deadlock.status,
            ModelCheckStatus::Violations,
            "{deadlock:?}"
        );
        assert!(
            deadlock
                .violations
                .iter()
                .any(|violation| violation.property == "option_to_complete"),
            "deadlock/livelock must violate option_to_complete: {deadlock:?}"
        );
    }
}
