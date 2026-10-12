//! Script execution for BPMN execution listeners (start / end scripts).
//!
//! Uses the Rhai scripting engine with configurable hardening:
//! - `max_operations` limits CPU-bound loops
//! - `max_memory` approximates heap budget via string/array/map size caps
//!   (Rhai has no `set_max_memory` API; limits are derived from the budget)
//! - `timeout` aborts runaway scripts via `spawn_blocking` + `tokio::time::timeout`
//! - [`ScriptRunner`] shares one configured engine and caches compiled scripts

use std::collections::HashMap;

use rhai::Dynamic;
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{EngineError, EngineResult};
use crate::domain::{ListenerEvent, ProcessDefinition, Token};

// ---------------------------------------------------------------------------
// ScriptConfig — configurable resource limits for Rhai execution
// ---------------------------------------------------------------------------

/// Resource limits applied to every Rhai script evaluation.
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Maximum number of Rhai operations per evaluation (default: 50 000).
    pub max_operations: u64,
    /// Approximate heap budget in bytes (default: 2 MiB).
    ///
    /// Rhai exposes no total-heap limit; this value drives
    /// `set_max_string_size` / `set_max_array_size` / `set_max_map_size`.
    pub max_memory: usize,
    /// Hard wall-clock timeout in milliseconds (default: 1 000).
    pub timeout_ms: u64,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            max_operations: 50_000,
            max_memory: 2 * 1024 * 1024, // 2 MiB
            timeout_ms: 1_000,           // 1 second
        }
    }
}

impl ScriptConfig {
    /// Reads configuration from environment variables, falling back to defaults.
    ///
    /// | Variable                    | Default  |
    /// |-----------------------------|----------|
    /// | `RHAI_MAX_OPERATIONS`       | 50 000   |
    /// | `RHAI_MAX_MEMORY_BYTES`     | 2097152  |
    /// | `RHAI_TIMEOUT_MS`           | 1000     |
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("RHAI_MAX_OPERATIONS")
            && let Ok(n) = v.parse()
        {
            cfg.max_operations = n;
        }
        if let Ok(v) = std::env::var("RHAI_MAX_MEMORY_BYTES")
            && let Ok(n) = v.parse()
        {
            cfg.max_memory = n;
        }
        if let Ok(v) = std::env::var("RHAI_TIMEOUT_MS")
            && let Ok(n) = v.parse()
        {
            cfg.timeout_ms = n;
        }
        cfg
    }

    /// Creates a Rhai engine configured with the resource limits from this config.
    ///
    /// `max_memory` is mapped onto Rhai's collection size caps (there is no
    /// native heap-budget API). At the default 2 MiB budget this yields the
    /// historical caps (64 KiB strings, 10 000 array/map entries).
    pub fn build_engine(&self) -> rhai::Engine {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(self.max_operations);

        let (max_string, max_array, max_map) = self.derived_collection_limits();
        engine.set_max_string_size(max_string);
        engine.set_max_array_size(max_array);
        engine.set_max_map_size(max_map);
        engine
    }

    /// Derives string/array/map size caps from `max_memory`.
    ///
    /// Scaling is linear against the 2 MiB default so that lowering
    /// `RHAI_MAX_MEMORY_BYTES` actually hardens the sandbox.
    pub fn derived_collection_limits(&self) -> (usize, usize, usize) {
        const DEFAULT_MEMORY: usize = 2 * 1024 * 1024;
        const DEFAULT_STRING: usize = 64 * 1024;
        const DEFAULT_ARRAY: usize = 10_000;
        const DEFAULT_MAP: usize = 10_000;

        let budget = self.max_memory.max(1);
        // Scale relative to the documented 2 MiB default; never exceed defaults.
        let scale = (budget as f64 / DEFAULT_MEMORY as f64).min(1.0);

        let max_string = ((DEFAULT_STRING as f64) * scale).round() as usize;
        let max_array = ((DEFAULT_ARRAY as f64) * scale).round() as usize;
        let max_map = ((DEFAULT_MAP as f64) * scale).round() as usize;

        (
            max_string.clamp(1, DEFAULT_STRING),
            max_array.clamp(1, DEFAULT_ARRAY),
            max_map.clamp(1, DEFAULT_MAP),
        )
    }
}

// ---------------------------------------------------------------------------
// execute_script_safe — the single hardened entry point
// ---------------------------------------------------------------------------

/// Result of a successful script evaluation: the mutated variables.
pub type ScriptResult = HashMap<String, Value>;

/// Maximum number of compiled scripts kept in a [`ScriptRunner`] cache.
/// Scripts come from deployed definitions, so the set is normally small; the
/// cap only guards against unbounded growth (the cache is cleared when full).
const MAX_CACHED_SCRIPTS: usize = 1024;

/// Executes Rhai scripts with one shared, pre-configured engine and a cache of
/// compiled scripts.
///
/// Building a `rhai::Engine` registers all standard packages and compiling a
/// script parses it — both used to happen on every evaluation. The engine is
/// immutable after construction and `Send + Sync` (Rhai `sync` feature), so
/// one instance serves all evaluations. It is built on first use, so workflow
/// engines that never run a script do not pay for it.
pub struct ScriptRunner {
    config: ScriptConfig,
    engine: std::sync::OnceLock<std::sync::Arc<rhai::Engine>>,
    compiled: dashmap::DashMap<String, std::sync::Arc<rhai::AST>>,
}

impl ScriptRunner {
    pub fn new(config: ScriptConfig) -> Self {
        Self {
            config,
            engine: std::sync::OnceLock::new(),
            compiled: dashmap::DashMap::new(),
        }
    }

    pub fn config(&self) -> &ScriptConfig {
        &self.config
    }

    /// Evaluates `script` inside the hardened sandbox.
    ///
    /// Compilation and evaluation run on a blocking thread (`spawn_blocking`)
    /// so the Tokio runtime is never blocked. A wall-clock timeout aborts the
    /// evaluation if `ScriptConfig::timeout_ms` elapses.
    ///
    /// On success the returned `HashMap` contains the full variable scope after
    /// evaluation — callers merge this back into the token.
    pub async fn execute(
        &self,
        script: &str,
        variables: &HashMap<String, Value>,
    ) -> EngineResult<ScriptResult> {
        let engine = std::sync::Arc::clone(
            self.engine
                .get_or_init(|| std::sync::Arc::new(self.config.build_engine())),
        );
        let cached = self.compiled.get(script).map(|ast| ast.value().clone());
        let source = script.to_owned();
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let vars = variables.clone();

        let start = std::time::Instant::now();
        let handle = tokio::task::spawn_blocking(move || {
            let (ast, newly_compiled) = match cached {
                Some(ast) => (ast, false),
                None => {
                    let ast = engine
                        .compile(&source)
                        .map_err(|e| EngineError::ScriptError(e.to_string()))?;
                    (std::sync::Arc::new(ast), true)
                }
            };

            let mut scope = rhai::Scope::new();
            for (k, v) in &vars {
                scope.push_dynamic(k, rhai::serde::to_dynamic(v).unwrap_or(Dynamic::UNIT));
            }

            engine
                .eval_ast_with_scope::<()>(&mut scope, &ast)
                .map_err(|e| EngineError::ScriptError(e.to_string()))?;

            let mut result = vars;
            for (k, _, v) in scope.iter_raw() {
                if let Ok(json_val) = rhai::serde::from_dynamic(v) {
                    result.insert(k.to_string(), json_val);
                }
            }
            Ok((result, newly_compiled.then_some(ast)))
        });

        let result = match tokio::time::timeout(timeout, handle).await {
            Ok(Ok(Ok((result, new_ast)))) => {
                if let Some(ast) = new_ast {
                    if self.compiled.len() >= MAX_CACHED_SCRIPTS {
                        self.compiled.clear();
                    }
                    self.compiled.insert(script.to_owned(), ast);
                }
                Ok(result)
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(join_err)) => Err(EngineError::ScriptError(format!(
                "Script thread panicked: {join_err}"
            ))),
            Err(_elapsed) => Err(EngineError::ScriptError(format!(
                "Script execution timed out after {}ms",
                self.config.timeout_ms
            ))),
        };

        let elapsed = start.elapsed().as_secs_f64();
        metrics::histogram!("bpmn_script_execution_duration_seconds").record(elapsed);
        if result.is_err() {
            metrics::counter!("bpmn_errors_total", "type" => "script").increment(1);
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn cached_scripts(&self) -> usize {
        self.compiled.len()
    }
}

/// Evaluates a Rhai script inside a hardened sandbox (one-off).
///
/// Builds a fresh engine per call; the workflow engine uses a shared
/// [`ScriptRunner`] instead. Kept for callers that only have a config.
pub async fn execute_script_safe(
    config: &ScriptConfig,
    script: &str,
    variables: &HashMap<String, Value>,
) -> EngineResult<ScriptResult> {
    ScriptRunner::new(config.clone())
        .execute(script, variables)
        .await
}

// ---------------------------------------------------------------------------
// Listener helpers (updated to use execute_script_safe)
// ---------------------------------------------------------------------------

/// Executes all scripts of a given `ListenerEvent` on the specified node.
///
/// Mutates token variables based on script output and appends audit entries.
pub async fn run_node_scripts(
    runner: &ScriptRunner,
    _instance_id: Uuid,
    token: &mut Token,
    def: &ProcessDefinition,
    node_id: &str,
    event: ListenerEvent,
    audit_log: &mut Vec<String>,
) -> EngineResult<()> {
    if let Some(listeners) = def.listeners.get(node_id) {
        for l in listeners {
            if l.event == event {
                let result =
                    runner
                        .execute(&l.script, &token.variables)
                        .await
                        .map_err(|e| match e {
                            EngineError::ScriptError(msg) => EngineError::ScriptError(format!(
                                "Knoten '{}' ({}-Listener): {}",
                                node_id,
                                match event {
                                    ListenerEvent::Start => "start",
                                    ListenerEvent::End => "end",
                                },
                                msg
                            )),
                            other => other,
                        })?;
                token.variables = result;

                let event_name = match event {
                    ListenerEvent::Start => "start",
                    ListenerEvent::End => "end",
                };
                tracing::info!(
                    "Instance {_instance_id}: executed {event_name} script on node '{node_id}'"
                );
                audit_log.push(format!("📜 Executed {event_name} script on '{node_id}'"));
            }
        }
    }
    Ok(())
}

/// Runs End-event scripts and merges the resulting variables back
/// into the instance state.
///
/// This is a convenience wrapper around `run_node_scripts` that also
/// updates the `variables` field on the `ProcessInstance`.
pub async fn run_end_scripts(
    runner: &ScriptRunner,
    instance_id: Uuid,
    token: &mut Token,
    def: &ProcessDefinition,
    node_id: &str,
    instance_audit_log: &mut Vec<String>,
    instance_variables: &mut HashMap<String, Value>,
) -> EngineResult<()> {
    let mut end_audits = Vec::new();
    run_node_scripts(
        runner,
        instance_id,
        token,
        def,
        node_id,
        ListenerEvent::End,
        &mut end_audits,
    )
    .await?;
    instance_audit_log.append(&mut end_audits);
    *instance_variables = token.variables.clone();
    Ok(())
}
