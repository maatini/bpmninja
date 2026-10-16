# Graph Report - bpmninja  (2026-10-16)

## Corpus Check
- 410 files · ~538,921 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 18 file(s) not represented in the graph (top: (none) 8, .toml 2, .xml 2)

## Summary
- 4656 nodes · 10443 edges · 271 communities (209 shown, 62 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 299 edges (avg confidence: 0.89)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `2b11ed23`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- NatsPersistence
- InMemoryPersistence
- hashmap
- tauri.ts
- FailSaveXmlPersistence
- parse_bpmn_xml
- camunda.rs
- WorkflowPersistence
- retry_queue.rs
- Button
- model_check.rs
- InstrumentedPersistence
- use-engine-events.ts
- build_mutation_metrics.py
- stress_tests.rs
- desktop-tauri/package.json
- AppError
- conditional_boundaries.rs
- tests/helpers.rs
- bpmn-parser/src/models.rs
- persistence — Gotchas
- dependencies
- WorkflowEngine
- bpmn-ninja-external-task-client/package.json
- log_buffer.rs
- condition.rs
- agents
- bpmn-parser — Gotchas
- domain/mod.rs
- BpmnElement
- sse_consumer.rs
- react
- use-toast.ts
- server/state.rs
- engine-server — Gotchas
- instance.rs
- server/tasks.rs
- index.ts
- InstanceDetailDialog.tsx
- Token
- cn
- .record_history_event
- BPMNinja External Task Client Skill
- engine/loops.rs
- server/events.rs
- scripts
- parser.rs
- TimerDueIndex
- compilerOptions
- Critical Rules
- serialize
- WorkflowEngine
- compilerOptions
- components.json
- HistoryEventType
- persistence.rs
- tests/data_mappings.rs
- fuzz_history_diff.rs
- TaskService
- devDependencies
- compilerOptions
- ProcessDefinitionBuilder
- server/files.rs
- BPMNinja Quality Skill
- api-spec/package.json
- external-task-client — Gotchas
- journal.rs
- call_errors.rs
- conditional_catches.rs
- history/mod.rs
- BPMNinja Desktop Skill
- BPMNinja Server Skill
- http_load.py
- timer.rs
- tauri.conf.json
- .persist_instance
- runner.rs
- Value
- .reconcile_event_subprocesses
- api_helpers.rs
- history.rs
- client.rs
- commands/instances.rs
- server/deploy.rs
- persistence-nats/src/tests.rs
- CLAUDE.md — BPMNinja Project Guide
- bpmninja — Architecture Documentation
- DataViewer.tsx
- tests/loops.rs
- signals.rs
- BPMNinja Parser Skill
- InstanceStatus
- Component Responsibilities
- desktop-tauri — Dependencies
- persistence_ops.rs
- e2e_call_activity.rs
- value
- BPMNinja — Priorisierte Verbesserungen
- commands/monitoring.rs
- Shared Patterns
- HistoryEntry
- e2e_migration.rs
- e2e_service_tasks.rs
- BPMNinja Engine Skill
- nats_bench.rs
- `ExternalTaskClient`
- @bpmninja/external-task-client
- ExternalTaskClient.test.ts
- ConditionPropertiesProvider.ts
- engine-server/src/main.rs
- WorkflowEngine
- EngineError
- EventSubProcessState
- desktop-tauri — Gotchas
- assert-typescript-peer.mjs
- Key Architectural Decisions
- X2 / X9b — Conditional events
- tests/task_handler.rs
- .handle_next_action
- MonitoringPage.tsx
- api-spec/index.md
- BPMN 2.0.2 Element Matrix
- tests/event_subprocess.rs
- commands/tasks.rs
- DataMappingsPropertiesProvider.ts
- e2e_suspend_resume.rs
- e2e_gateways.rs
- Test Metrics
- engine_bench.rs
- commands/deploy.rs
- Data Flows
- Public API Surface
- Overview
- InstanceState
- e2e_file_variables.rs
- scripts
- app.spec.ts
- bpmn-js-properties-panel
- parallel.rs
- UI/Desktop Agent (Frontend)
- Dependencies
- variables.rs
- definitions
- X9 — Event subprocesses
- migration.rs
- json
- .handle_signal_throw_event
- BPMNinja Desktop (Tauri + React + bpmn-js)
- test_mutation_metrics.py
- Maintenance
- engine-core/index.md
- @bpmn-io/properties-panel
- BPMNinja
- e2e_history.rs
- e2e_instances_page.rs
- e2e_monitoring.rs
- e2e_start_errors.rs
- Task Memory Protocol & Context Resets
- Dependency Management
- Server Agent
- scripting.rs
- UI/Desktop Agent (Tauri Rust Backend)
- knowledge-base/index.md
- data_tests.rs
- commands/files.rs
- desktop-tauri/tsconfig.json
- LoadTest
- engine-server — Interfaces
- server/messages.rs
- audit.rs
- engine-core
- Graph Maintenance & Metriken
- Parser Agent
- RTK - Rust Token Killer (Google Antigravity)
- EventSubprocessPropertiesProvider.ts
- event_subprocess_tests.rs
- BPMN Model Checking and State-Space Reduction
- WorkflowEngine
- package.json
- fuzz/index.md
- boundary.rs
- X2a — Intermediate conditional catches
- persistence — Responsibilities
- ALLOCATOR
- Global Agent Directives & Workflow
- Rust Best Practices for BPMNinja
- Versioning Convention
- completion.rs
- Server allocator and release optimization experiments
- AGENTS.md
- copilot-instructions.md
- e2e_lifecycle.rs
- Execution Engine Agent
- Workflow: graphify
- super
- ProcessDefinition
- 02_GRAPH_FIRST_NAVIGATION.md
- ORCHESTRATOR_AGENT.md
- bpmninja-desktop/scripts/oracle.sh
- bpmninja-engine/scripts/oracle.sh
- bpmninja-parser/scripts/oracle.sh
- bpmninja-quality/scripts/oracle.sh
- bpmninja-server/scripts/oracle.sh
- vite-env.d.ts
- commit-and-push-quality-metrics.sh
- commit-and-push-quality-metrics_test.sh
- agent-orchestrator
- bpmninja-desktop
- X11 — Data defaults and activity mappings
- instance_pagination.rs
- X2c — Explicit top-level conditional starts
- e2e_deploy.rs
- ExternalTaskClient
- inmemorypersistence
- persistence — Interfaces
- inclusive_join.rs
- logs.rs
- retry.ts
- ActorType
- truncate_value_for_diff
- persistence — Dependencies
- ScriptPropertiesProvider.ts
- constants.rs
- Data defaults and mappings (X11)
- vite.config.ts
- WorkflowEngine
- history_batch.rs
- X2b — Conditional boundary catches
- 6. REST API (engine-server)
- 5.2 Implementations
- Intermediate conditional catches (X2a)
- e2e_versioning.rs
- engine-core — Dependencies
- .resume_catch
- call_activity_parallel.rs
- 8. Concurrency & Locking (K1)
- persist_lock.rs
- Event subprocess runtime (X9a/X9b)
- Camunda-7-Worker-Fassade
- Conditional boundary catches (X2b)
- BucketEntryDetail
- SSE-Zustandsupdates
- server/mod.rs
- architecture/index.md
- Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)
- correlate_message
- 4. Execution Architecture

## God Nodes (most connected - your core abstractions)
1. `Value` - 164 edges
2. `ProcessDefinition` - 163 edges
3. `Token` - 118 edges
4. `cn()` - 93 edges
5. `parse_bpmn_xml()` - 83 edges
6. `WorkflowEngine` - 83 edges
7. `NextAction` - 65 edges
8. `InMemoryPersistence` - 59 edges
9. `WorkflowPersistence` - 58 edges
10. `ProcessDefinitionBuilder` - 57 edges

## Surprising Connections (you probably didn't know these)
- `Invariants` --references--> `TaskService`  [INFERRED]
  docs/knowledge-base/modules/external-task-client/responsibility.md → bpmn-ninja-external-task-client/src/TaskService.ts
- `In-Process-Task-Handler (`engine/task_handler.rs`)` --references--> `TaskHandler`  [INFERRED]
  docs/knowledge-base/modules/engine-core/interfaces.md → bpmn-ninja-external-task-client/src/types.ts
- `UI Patterns` --references--> `stateBadgeClass()`  [INFERRED]
  .agent/rules/TAURI_REACT_UI_AGENT.md → desktop-tauri/src/features/instances/InstanceStateUtils.ts
- `⚠️ Playwright tests use specific selectors` --references--> `InstancesPage()`  [INFERRED]
  docs/knowledge-base/modules/desktop-tauri/gotchas.md → desktop-tauri/src/features/instances/InstancesPage.tsx
- `persistence-nats (Production)` --references--> `definitions()`  [INFERRED]
  docs/knowledge-base/modules/persistence/responsibility.md → desktop-tauri/src/features/modeler/properties/EventSubprocessPropertiesProvider.ts

## Import Cycles
- 2-file cycle: `engine-server/src/server/camunda.rs -> engine-server/src/server/camunda/variables.rs -> engine-server/src/server/camunda.rs`

## Communities (271 total, 62 thin omitted)

### Community 1 - "InMemoryPersistence"
Cohesion: 0.07
Nodes (10): InMemoryPersistence, test_get_storage_info_arithmetic(), test_query_history_filters(), fetch(), MAPPED, post(), serve(), x11_camunda_worker_receives_local_inputs_and_completes_mapped_outputs() (+2 more)

### Community 3 - "tauri.ts"
Cohesion: 0.08
Nodes (45): 1. Model Layer (`model.rs`), 2. Engine Core (`engine/` submodule), 3. External Tasks (`service_task.rs`), 4. Condition Evaluator (`condition.rs`), 5. Script Runner (`script_runner.rs`), 6. Persistence (`persistence.rs`), 7. Error Handling (`error.rs`), 8. History Module (`history.rs`) (+37 more)

### Community 4 - "FailSaveXmlPersistence"
Cohesion: 0.07
Nodes (10): deploy(), deploy_save_bpmn_xml_failure_is_5xx_and_not_listed(), deploy_without_persistence_is_200_and_listed(), FailSaveXmlPersistence, isolated_config(), list_definitions(), MINIMAL_BPMN_XML, serve() (+2 more)

### Community 5 - "parse_bpmn_xml"
Cohesion: 0.05
Nodes (77): parse_bpmn_xml(), embedded_subprocess_enters_at_start_successor_without_noop_task(), flatten_subprocess_keeps_send_inclusive_event_and_catch(), parse_boundary_error_event(), parse_boundary_timer_event(), parse_business_rule_task_with_topic(), parse_camunda_assignee_and_topic(), parse_compensation_events() (+69 more)

### Community 6 - "camunda.rs"
Cohesion: 0.20
Nodes (19): body(), bpmn_error(), CamundaError, complete(), CompleteRequest, duration(), ErrorRequest, extend() (+11 more)

### Community 8 - "retry_queue.rs"
Cohesion: 0.06
Nodes (28): InstanceStore, create_retry_queue(), create_retry_queue_with_capacity(), DEFAULT_RETRY_QUEUE_CAPACITY, execute_job(), INITIAL_BACKOFF_MS, INLINE_BACKOFF_MS, INLINE_RETRIES (+20 more)

### Community 9 - "Button"
Cohesion: 0.13
Nodes (45): Button, DialogContent, DialogDescription, DialogFooter(), DialogHeader(), DialogOverlay, DialogTitle, Input (+37 more)

### Community 10 - "model_check.rs"
Cohesion: 0.07
Nodes (40): AppBuildConfig, analyze(), DEFAULT_MODEL_CHECK_TIMEOUT_MS, expand_empty_elements(), expands_self_closing_elements(), ModelChecker, ModelCheckMode, Enforce (+32 more)

### Community 12 - "use-engine-events.ts"
Cohesion: 0.22
Nodes (8): createEngineEventBatcher(), EngineEventPayload, EngineEventScope, EngineEventType, matchesEngineEvent(), change, instance, @tauri-apps/api

### Community 13 - "build_mutation_metrics.py"
Cohesion: 0.34
Nodes (10): build_metrics(), count_from_outcomes_list(), count_from_summary_dict(), count_txt_lines(), counts_from_txt_dir(), find_outcomes_path(), _label(), main() (+2 more)

### Community 14 - "stress_tests.rs"
Cohesion: 0.06
Nodes (24): correctness_inclusive_gateway_partial_match(), correctness_mixed_xor_and_parallel(), drain_service_tasks(), history_query_limit_and_offset(), linear_definition(), parallel_definition(), race_concurrent_fetch_same_task(), stress_concurrent_fetch_and_lock() (+16 more)

### Community 15 - "desktop-tauri/package.json"
Cohesion: 0.05
Nodes (39): @types/node, typescript, license, name, private, type, version, autoprefixer (+31 more)

### Community 16 - "AppError"
Cohesion: 0.16
Nodes (26): delete_instance(), get_instance(), list_instances(), ListInstancesQuery, migrate_instance(), MigrateInstanceRequest, move_token(), MoveTokenRequest (+18 more)

### Community 17 - "conditional_boundaries.rs"
Cohesion: 0.17
Nodes (23): boundary_cancelled_activation_cannot_register_again(), boundary_claim_and_native_completion_race_has_one_continuation(), boundary_claim_fences_an_inflight_removed_task_token(), boundary_completion_outputs_do_not_interrupt_completed_activity(), boundary_declaration_order_stops_after_interrupting_winner(), boundary_failed_end_listener_keeps_subscription_and_discards_writes(), boundary_initial_and_later_true_interrupt_once(), boundary_initial_claim_skips_script_body_and_script_outputs_close_activation() (+15 more)

### Community 18 - "tests/helpers.rs"
Cohesion: 0.06
Nodes (18): exclusive_gateway_takes_matching_path(), exclusive_gateway_uses_default_when_no_match(), inclusive_gateway_forks_multiple_paths(), inclusive_gateway_single_match_no_fork(), test_same_gateway_type_detects_mismatch(), xor_gateway_negative_x_routes_to_user_task_2(), xor_gateway_positive_x_routes_to_user_task_1(), xor_gateway_user_task_merges_variables() (+10 more)

### Community 19 - "bpmn-parser/src/models.rs"
Cohesion: 0.17
Nodes (42): BpmnBoundaryEvent, BpmnCallActivity, BpmnCompensateEventDefinition, BpmnComplexGateway, BpmnConditionalEventDefinition, BpmnConditionalExpression, BpmnConditionExpression, BpmnDefinitions (+34 more)

### Community 20 - "persistence — Gotchas"
Cohesion: 0.15
Nodes (13): ⚠️ Adding a new entity type, ⚠️ Bucket handles are cached, ⚠️ Completed instances are archived, not deleted, ⚠️ docker-compose: NATS auth + loopback host ports, ⚠️ History entries are append-only, ⚠️ In-memory fallback is opt-in (dev only), ⚠️ In-memory persistence loses data on restart, ⚠️ KV `keys()` errors must propagate (+5 more)

### Community 21 - "dependencies"
Cohesion: 0.06
Nodes (35): dependencies, @base-ui/react, @bpmn-io/properties-panel, bpmn-js, bpmn-js-properties-panel, camunda-bpmn-moddle, class-variance-authority, clsx (+27 more)

### Community 22 - "WorkflowEngine"
Cohesion: 0.12
Nodes (4): WorkflowEngine, WorkflowEngine, EngineStats, InstancePage

### Community 23 - "bpmn-ninja-external-task-client/package.json"
Cohesion: 0.06
Nodes (33): author, dependencies, pino, pino-pretty, description, devDependencies, tsx, @types/node (+25 more)

### Community 24 - "log_buffer.rs"
Cohesion: 0.08
Nodes (11): append_entry_to_file(), COMPACT_AFTER, compact_file(), InnerBuffer, load_entries_from_file(), LogBuffer, LogEntry, MAX_ENTRIES (+3 more)

### Community 25 - "condition.rs"
Cohesion: 0.10
Nodes (18): compile_event_condition(), CompiledCondition, Comparison, Empty, Truthy, evaluate_condition(), Operator, Eq (+10 more)

### Community 27 - "agents"
Cohesion: 0.06
Nodes (31): agents, engine, orchestrator, parser, persistence, quality, server, ui (+23 more)

### Community 28 - "bpmn-parser — Gotchas"
Cohesion: 0.06
Nodes (27): bpmn-parser — Dependencies, Dependency Graph, Inbound, Outbound, ⚠️ Adding a new BpmnElement variant, bpmn-parser — Gotchas, ⚠️ Camunda namespace handling, ⚠️ InclusiveGateway has no join metadata in the model (+19 more)

### Community 30 - "BpmnElement"
Cohesion: 0.05
Nodes (37): BpmnElement, BoundaryCompensationEvent, BoundaryConditionalEvent, BoundaryErrorEvent, BoundaryEscalationEvent, BoundaryMessageEvent, BoundarySignalEvent, BoundaryTimerEvent (+29 more)

### Community 31 - "sse_consumer.rs"
Cohesion: 0.10
Nodes (19): connect_and_consume(), ConsumeOutcome, StreamEnded, UrlChanged, forwards_multiline_payload_only_at_message_boundary(), run_consumer(), sleep_or_url_change(), spawn() (+11 more)

### Community 32 - "react"
Cohesion: 0.10
Nodes (38): App(), Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle, Separator (+30 more)

### Community 33 - "use-toast.ts"
Cohesion: 0.11
Nodes (25): Toast, ToastAction, ToastActionElement, ToastClose, ToastDescription, ToastProps, ToastTitle, toastVariants (+17 more)

### Community 34 - "server/state.rs"
Cohesion: 0.08
Nodes (14): api_key_from_env(), AppState, cors_origins_from_env(), cors_unset_is_allowlist_not_any(), DEFAULT_CORS_ORIGINS, DEFAULT_MAX_UPLOAD_BYTES, max_upload_bytes_from_env(), MAX_XML_BYTES (+6 more)

### Community 35 - "engine-server — Gotchas"
Cohesion: 0.12
Nodes (16): ⚠️ Adding a new route, ⚠️ Body size limit is 10 MiB for XML/JSON, ⚠️ CORS is an allowlist (not `Any` by default), ⚠️ Deploy XML save failure is HTTP 500, engine-server — Gotchas, ⚠️ fetchAndLock long polling is notification-driven, ⚠️ In-memory fallback is opt-in (dev), ⚠️ `lockDuration` / `newDuration` are milliseconds at the HTTP boundary (+8 more)

### Community 36 - "instance.rs"
Cohesion: 0.15
Nodes (18): ActiveToken, append_audit_log_enforces_limit(), append_audit_log_exactly_at_limit_does_not_trim(), append_audit_log_under_limit(), CompensationRecord, EventSubscription, file_variable_names_returns_correct_list(), file_variable_names_returns_empty_when_no_files() (+10 more)

### Community 37 - "server/tasks.rs"
Cohesion: 0.19
Nodes (21): lock_duration_ms_to_secs(), bpmn_error(), BpmnErrorRequest, complete_service_task(), complete_task(), CompleteRequest, CompleteServiceTaskRequest, extend_lock() (+13 more)

### Community 38 - "index.ts"
Cohesion: 0.17
Nodes (16): DEFAULTS, noopLogger(), randomId(), BpmnErrorRequest, ClientConfig, CompleteRequest, ExtendLockRequest, ExternalTask (+8 more)

### Community 39 - "InstanceDetailDialog.tsx"
Cohesion: 0.13
Nodes (22): TabsContent, TabsList, TabsTrigger, InstanceDetailDialog(), InstanceDetailHeader(), HistoryTab(), HistoryTabProps, extractEmoji() (+14 more)

### Community 40 - "Token"
Cohesion: 0.09
Nodes (28): Token, resolve_next_target(), execute_complex_gateway(), execute_exclusive_gateway(), execute_inclusive_gateway(), execute_parallel_gateway(), WorkflowEngine, WorkflowEngine (+20 more)

### Community 41 - "cn"
Cohesion: 0.12
Nodes (38): AccordionContent, AccordionItem, AccordionTrigger, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter() (+30 more)

### Community 42 - ".record_history_event"
Cohesion: 0.17
Nodes (3): verify_lock_ownership(), WorkflowEngine, WorkflowEngine

### Community 43 - "BPMNinja External Task Client Skill"
Cohesion: 0.09
Nodes (21): API Contract (BPMNinja Engine, Rust/Axum), Architecture, BPMNinja External Task Client Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Configuration & Example, Critical Implementation Details, Domain Rules & Patterns (+13 more)

### Community 44 - "engine/loops.rs"
Cohesion: 0.13
Nodes (23): MultiInstanceDef, strip_expression(), condition_holds(), count_multi_instance_iteration(), instance_elements(), is_boundary_of(), local_variables(), LOOP_COUNTER (+15 more)

### Community 45 - "server/events.rs"
Cohesion: 0.06
Nodes (24): MAX_MESSAGES, NatsLogSink, STREAM_NAME, SUBJECT, auth_middleware(), extract_api_key(), HEALTH_PATH, READY_PATH (+16 more)

### Community 46 - "scripts"
Cohesion: 0.08
Nodes (23): env, ENGINE_API_URL, NATS_URL, packages, $schema, build, dev, engine:docker (+15 more)

### Community 47 - "parser.rs"
Cohesion: 0.19
Nodes (19): add_listeners(), EVENT_KINDS, first_attr(), flatten_subprocess(), nonempty_attr(), parse_iso8601_duration(), parse_repeating_interval(), parse_timer_definition() (+11 more)

### Community 49 - "compilerOptions"
Cohesion: 0.09
Nodes (22): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, jsx, lib, module, moduleDetection, moduleResolution (+14 more)

### Community 50 - "Critical Rules"
Cohesion: 0.09
Nodes (23): ⚠️ Backward-compatible re-exports, ⚠️ `BpmnElement` is a closed enum — must be exhaustive, ⚠️ Call-Activity tracking is `outstanding_calls`, not only `WaitingOnCallActivity`, Critical Rules, ⚠️ Event channel capacity, ⚠️ Exclusive gateway takes unconditional non-default flows, ⚠️ Executor history is buffered per batch, ⚠️ History snapshots (+15 more)

### Community 51 - "serialize"
Cohesion: 0.26
Nodes (4): get_pending_timers(), process_timers(), ProcessTimersResponse, NatsInfo

### Community 52 - "WorkflowEngine"
Cohesion: 0.05
Nodes (29): WorkflowEngine, complete_removes_task_from_topic_index(), deploy_single_task(), deploy_two_topic_instance(), fetch_only_requested_topic_leaves_other_index_untouched(), long_poll_wakes_when_failed_task_is_unlocked(), long_poll_wakes_when_task_is_created(), NO_FALLBACK (+21 more)

### Community 53 - "compilerOptions"
Cohesion: 0.09
Nodes (21): compilerOptions, declaration, declarationMap, esModuleInterop, forceConsistentCasingInFileNames, lib, module, moduleResolution (+13 more)

### Community 54 - "components.json"
Cohesion: 0.09
Nodes (21): aliases, components, hooks, lib, ui, utils, iconLibrary, menuAccent (+13 more)

### Community 57 - "HistoryEventType"
Cohesion: 0.08
Nodes (25): HistoryEventType, BranchCompleted, CallActivityCompleted, CallActivityStarted, CompensationTriggered, Error, EscalationThrown, EventSubprocessActivated (+17 more)

### Community 58 - "persistence.rs"
Cohesion: 0.24
Nodes (5): BucketEntry, BucketInfo, CompletedInstanceQuery, HistoryQuery, StorageInfo

### Community 59 - "tests/data_mappings.rs"
Cohesion: 0.11
Nodes (38): activity(), add_mapping(), apply(), apply_scope(), declarations(), expression(), invalid(), Node (+30 more)

### Community 60 - "fuzz_history_diff.rs"
Cohesion: 0.10
Nodes (10): ConditionInput, FuzzValue, Bool, Float, Int, Null, Str, arb_instance_state() (+2 more)

### Community 61 - "TaskService"
Cohesion: 0.14
Nodes (11): TaskService, Logger, RetryOptions, Environment-Driven Configuration, engine-server — Dependencies, Inbound, Key Environment Variables, Outbound (+3 more)

### Community 62 - "devDependencies"
Cohesion: 0.10
Nodes (20): devDependencies, autoprefixer, esbuild, eslint, @eslint/js, eslint-plugin-react-hooks, eslint-plugin-react-refresh, globals (+12 more)

### Community 63 - "compilerOptions"
Cohesion: 0.10
Nodes (19): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, lib, module, moduleDetection, moduleResolution, noEmit (+11 more)

### Community 64 - "ProcessDefinitionBuilder"
Cohesion: 0.10
Nodes (15): conditions_are_compiled_on_deploy_and_restore(), ProcessDefinitionBuilder, validate_loops(), ExecutionListener, ListenerEvent, End, Start, ScopeEventListener (+7 more)

### Community 65 - "server/files.rs"
Cohesion: 0.19
Nodes (8): content_disposition_attachment(), content_disposition_encodes_quotes_and_utf8(), content_disposition_strips_header_injection_and_paths(), content_disposition_uses_quoted_ascii_and_rfc5987(), delete_instance_file(), get_instance_file(), rfc5987_encode(), upload_instance_file()

### Community 66 - "BPMNinja Quality Skill"
Cohesion: 0.11
Nodes (18): BPMNinja Quality Skill, CI/CD, CI/CD Workflows, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Configuration, Documentation, Documentation (+10 more)

### Community 67 - "api-spec/package.json"
Cohesion: 0.11
Nodes (18): dependencies, @typespec/compiler, @typespec/http, @typespec/openapi, @typespec/openapi3, @typespec/rest, description, @typespec/compiler (+10 more)

### Community 68 - "external-task-client — Gotchas"
Cohesion: 0.07
Nodes (24): Architecture, Dev Dependencies, external-task-client — Dependencies, Inbound, Outbound, ⚠️ `bpmnError()` is not retried, ⚠️ Default-Port ist 8081, ⚠️ ESM only (+16 more)

### Community 69 - "journal.rs"
Cohesion: 0.17
Nodes (21): CachedSnapshot, choose_node(), decode_head(), DELTA, encode_head(), failure(), FULL, Head (+13 more)

### Community 70 - "call_errors.rs"
Cohesion: 0.12
Nodes (4): call_activity_propagates_error_to_matching_boundary_event(), call_activity_propagates_error_to_wildcard_boundary_event(), call_activity_unhandled_error_becomes_incident(), build_child_error_process()

### Community 71 - "conditional_catches.rs"
Cohesion: 0.23
Nodes (18): catch_waits_then_consumes_once_and_initial_true_passes(), competing_conditional_alternatives_cancel_losers_before_claim_recovery(), conditional_end_listener_failure_is_atomic_and_cancels_waits(), definition(), gateway_conditional_or_message_wins_once(), gateway_jobs_recover_original_ids_and_only_one_racing_winner(), legacy_gateway_waits_upgrade_or_reject_ambiguous_restore(), loop_revisit_while_true_creates_a_fresh_wait() (+10 more)

### Community 72 - "history/mod.rs"
Cohesion: 0.15
Nodes (13): calculate_diff(), calculate_diff_from_snapshot(), DiffSnapshot, format_file_human_text(), test_calculate_diff(), test_calculate_diff_empty(), test_calculate_diff_file_upload_human_text(), test_calculate_diff_only_removed() (+5 more)

### Community 73 - "BPMNinja Desktop Skill"
Cohesion: 0.11
Nodes (17): BPMNinja Desktop Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Cross-Stack, Domain Rules & Patterns, Evolution Log, Key Configuration Files, React Frontend (+9 more)

### Community 74 - "BPMNinja Server Skill"
Cohesion: 0.11
Nodes (17): agent-orchestrator, agent-orchestrator, BPMNinja Server Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Domain Rules & Patterns, engine-server, engine-server (+9 more)

### Community 76 - "http_load.py"
Cohesion: 0.14
Nodes (7): Client, main(), percentile(), rss_reader(), run_phase(), worker(), workflow()

### Community 77 - "timer.rs"
Cohesion: 0.06
Nodes (39): attach(), conditional_expression(), event_subprocess(), extract(), invalid(), local_attribute(), ParsedHandler, referenced() (+31 more)

### Community 78 - "tauri.conf.json"
Cohesion: 0.11
Nodes (17): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+9 more)

### Community 80 - "runner.rs"
Cohesion: 0.16
Nodes (6): execute_script_safe(), MAX_CACHED_SCRIPTS, run_end_scripts(), run_node_scripts(), ScriptConfig, ScriptRunner

### Community 81 - "Value"
Cohesion: 0.08
Nodes (21): FileReference, Value, BpmnErrorRequest, BucketEntriesQuery, CompletedInstancesQuery, CompleteRequest, CompleteServiceTaskRequest, CorrelateMessageRequest (+13 more)

### Community 82 - ".reconcile_event_subprocesses"
Cohesion: 0.13
Nodes (4): WorkflowEngine, inline_retry!, WorkflowEngine, WorkflowEngine

### Community 83 - "api_helpers.rs"
Cohesion: 0.30
Nodes (11): api_delete(), api_get(), api_post(), api_post_no_body(), api_put(), auth_headers(), blank_key_sends_no_headers(), empty_key_sends_no_headers() (+3 more)

### Community 84 - "history.rs"
Cohesion: 0.28
Nodes (6): CompletedInstancesQuery, get_completed_instance(), get_instance_history(), get_instance_history_entry(), list_completed_instances(), ServerHistoryQuery

### Community 85 - "client.rs"
Cohesion: 0.13
Nodes (4): HandleCache, KV_BUCKETS, NatsPersistence, OBJECT_STORES

### Community 86 - "commands/instances.rs"
Cohesion: 0.33
Nodes (13): delete_instance(), get_completed_instance(), get_instance_details(), get_instance_history(), list_instances(), migrate_instance(), move_token(), query_completed_instances() (+5 more)

### Community 87 - "server/deploy.rs"
Cohesion: 0.29
Nodes (9): DefinitionInfo, delete_all_definitions(), delete_definition(), DeleteDefinitionQuery, deploy_definition(), DeployRequest, DeployResponse, get_definition_xml() (+1 more)

### Community 88 - "persistence-nats/src/tests.rs"
Cohesion: 0.29
Nodes (17): connect_nats(), nats_required(), nats_url(), setup_nats_test(), test_definition_instance_restore_roundtrip(), test_history_append_and_load(), test_history_batch_is_ordered_and_deduplicated(), test_instance_journal_roundtrip_and_checkpointing() (+9 more)

### Community 89 - "CLAUDE.md — BPMNinja Project Guide"
Cohesion: 0.12
Nodes (16): Agenten-System, Architektur-Regeln, CLAUDE.md — BPMNinja Project Guide, Claude Workflow (unbedingt einhalten), Coding-Konventionen, Git & Release Workflow, graphify, KI-Verhaltensrichtlinien (Behavioral Guidelines) (+8 more)

### Community 90 - "bpmninja — Architecture Documentation"
Cohesion: 0.13
Nodes (15): 10. Code Statistics, 1. Workspace Overview, 2.1 Module Structure, 2.2 WorkflowEngine — Component Breakdown (K2), 2. engine-core — Core Architecture, 3.1 ProcessInstance (after K4 refactoring), 3.2 BPMN Element Types, 3. Data Model (+7 more)

### Community 91 - "DataViewer.tsx"
Cohesion: 0.15
Nodes (17): DataFormat, DataViewer(), DataViewerProps, decodeBase64Text(), detectFormat(), detectMimeType(), EXT_MIME, FORMAT_LABELS (+9 more)

### Community 92 - "tests/loops.rs"
Cohesion: 0.28
Nodes (16): bpmn_error_boundary_on_parallel_service_tasks_cancels_siblings(), completion_condition_cancels_remaining_instances(), details(), interrupting_boundary_cancels_all_instances(), invalid_collection_fails_the_step(), mi(), parallel_call_activities_complete_parent(), parallel_collection_collects_outputs_in_order() (+8 more)

### Community 93 - "signals.rs"
Cohesion: 0.21
Nodes (9): broadcast_resumes_every_waiting_instance(), interrupting_signal_boundary_cancels_task(), non_interrupting_signal_boundary_keeps_task(), signal_catch_survives_restart(), signals_and_messages_do_not_mix(), state_of(), throw_and_end_events_broadcast_after_their_batch(), user_task_with_signal_boundary() (+1 more)

### Community 94 - "BPMNinja Parser Skill"
Cohesion: 0.14
Nodes (13): BPMNinja Parser Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Domain Rules & Patterns, Evolution Log, Key Types, Relationship to engine-core, Scope & File Map (+5 more)

### Community 95 - "InstanceStatus"
Cohesion: 0.09
Nodes (19): EngineEvent, DefinitionChanged, EventSubprocessChanged, InstanceChanged, TaskChanged, InstanceDelta, InstanceStatus, Completed (+11 more)

### Community 96 - "Component Responsibilities"
Cohesion: 0.14
Nodes (14): bpmn-parser, Component Responsibilities, desktop-tauri, Element Handlers (engine-core/src/engine/handlers/), engine-server, Executor (engine-core/src/engine/executor/), external-task-client, Gateway Router (engine-core/src/engine/gateway.rs) (+6 more)

### Community 97 - "desktop-tauri — Dependencies"
Cohesion: 0.11
Nodes (14): Architecture Diagram, desktop-tauri — Dependencies, Inbound, Key npm Dependencies, Outbound, desktop-tauri, desktop-tauri — Interfaces, SSE Event Flow (+6 more)

### Community 98 - "persistence_ops.rs"
Cohesion: 0.14
Nodes (5): InstancePayload, payload_accepts_exact_limit_and_rejects_the_next_byte(), PERSIST_LOCK_STRIPES, PersistLocks, serialization_bounds_escaped_json_bytes()

### Community 99 - "e2e_call_activity.rs"
Cohesion: 0.32
Nodes (12): call_activity_eltern_abschluss_nach_kind(), call_activity_eltern_wartet_auf_kind(), call_activity_kind_ist_unterinstanz(), call_activity_mit_kind_service_task(), call_activity_variablen_propagation(), CHILD_BPMN, CHILD_SERVICE_BPMN, deploy() (+4 more)

### Community 100 - "value"
Cohesion: 0.19
Nodes (6): start_server(), stress_http_concurrent_deployments(), stress_http_concurrent_starts(), start_server(), test_update_variables_mid_execution_persists(), USER_TASK_BPMN_XML

### Community 101 - "BPMNinja — Priorisierte Verbesserungen"
Cohesion: 0.10
Nodes (20): Appendix — Größen-Snapshot, BPMNinja — Priorisierte Verbesserungen, Details, Empfohlener Aktionsplan (phasiert), Executive Summary, Kritische Issues (Kurzreferenz), Medium-term Improvements, Performance-Hotspots (+12 more)

### Community 102 - "commands/monitoring.rs"
Cohesion: 0.36
Nodes (8): get_api_key(), get_api_url(), get_bucket_entries(), get_bucket_entry_detail(), get_log_entries(), get_monitoring_data(), set_api_key(), set_api_url()

### Community 103 - "Shared Patterns"
Cohesion: 0.20
Nodes (10): Broadcast Events (SSE Push), Camunda-Compatible Service Task API, Fail-Closed Durability (Server), Fault-Tolerant Persistence Retry, Fine-Grained Per-Instance Locking, Hexagonal Architecture (Ports & Adapters), Lock-Free Wait-State Queues, Shared Patterns (+2 more)

### Community 104 - "HistoryEntry"
Cohesion: 0.18
Nodes (5): HistoryDiff, HistoryEntry, test_with_diff_empty_does_not_set(), test_with_diff_non_empty_sets_description(), VariableDiff

### Community 105 - "e2e_migration.rs"
Cohesion: 0.36
Nodes (11): BPMN_V1, BPMN_V2_COMPAT, BPMN_V2_RENAMED, deploy(), deploy_and_start(), migrate_instance_same_node_ids_returns_204(), migrate_instance_unknown_target_returns_404(), migrate_instance_updates_definition_key() (+3 more)

### Community 106 - "e2e_service_tasks.rs"
Cohesion: 0.26
Nodes (11): bpmn_error_routes_to_boundary_event(), complete_nonexistent_task_returns_404(), complete_wrong_worker_returns_conflict(), ERROR_BOUNDARY_BPMN, extend_lock_succeeds(), extend_lock_wrong_worker_returns_conflict(), fail_service_task_decrements_retries(), get_service_tasks_returns_list() (+3 more)

### Community 107 - "BPMNinja Engine Skill"
Cohesion: 0.17
Nodes (11): BPMNinja Engine Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Domain Rules & Patterns, Evolution Log, Scope & File Map, Step 1 – Generate, Step 2 – Surrogate Verification (Self-Critique) (+3 more)

### Community 108 - "nats_bench.rs"
Cohesion: 0.11
Nodes (4): bench_nats(), nats_url(), sample_instance(), script_chain()

### Community 109 - "`ExternalTaskClient`"
Cohesion: 0.17
Nodes (12): API Reference, `bpmnError(errorCode: string): Promise<void>`, `complete(variables?: Record<string, unknown>): Promise<void>`, `constructor(config?: ClientConfig)`, `extendLock(additionalDurationMs: number): Promise<void>`, `ExternalTaskClient`, `failure(errorMessage: string, errorDetails?: string, retries?: number): Promise<void>`, `start(): void` (+4 more)

### Community 110 - "@bpmninja/external-task-client"
Cohesion: 0.15
Nodes (12): @bpmninja/external-task-client, Configuration, Example, Features, Graceful Shutdown, Installation, License, Per-topic override: (+4 more)

### Community 111 - "ExternalTaskClient.test.ts"
Cohesion: 0.35
Nodes (6): createMockTask(), createMockFetch(), mockFetchResponse, setupGlobalFetchMock(), createMockLogger(), vitest

### Community 112 - "ConditionPropertiesProvider.ts"
Cohesion: 0.35
Nodes (9): ConditionTypeEntry(), CustomConditionGroup(), CustomPropertiesProvider, ExpressionEntry(), getConditionType(), isDefaultFlow(), SCRIPT_LANGUAGES, ScriptBodyEntry() (+1 more)

### Community 114 - "WorkflowEngine"
Cohesion: 0.29
Nodes (5): queue_cancelled_activations(), queue_lifecycle(), token_valid(), validate_definition(), WorkflowEngine

### Community 115 - "EngineError"
Cohesion: 0.06
Nodes (41): EngineError, AlreadyCompleted, DataMappingError, DefinitionHasInstances, ExecutionLimitExceeded, InstanceSuspended, InvalidDefinition, NoMatchingCondition (+33 more)

### Community 116 - "EventSubProcessState"
Cohesion: 0.27
Nodes (8): BoundaryActivation, BoundaryTask, Message, Service, User, ConditionalWait, EventSubProcessState, EventWaitGroup

### Community 117 - "desktop-tauri — Gotchas"
Cohesion: 0.17
Nodes (12): ⚠️ API-Key headers (`BPMNINJA_API_KEY`), ⚠️ Custom properties (ConditionPropertiesProvider), desktop-tauri — Gotchas, ⚠️ Dialog accessibility (Radix UI), ⚠️ Identifier, CSP, no dummy updater, ⚠️ Playwright tests use specific selectors, ⚠️ Rerunning the desktop app in dev mode, ⚠️ SSE connection lifecycle (+4 more)

### Community 118 - "assert-typescript-peer.mjs"
Cohesion: 0.22
Nodes (7): cmp(), declaredNumeric, lock, parseVer(), pkg, root, satisfies()

### Community 119 - "Key Architectural Decisions"
Cohesion: 0.18
Nodes (11): ADR-001: Hexagonal Architecture (Ports & Adapters), ADR-002: Token-Based Execution Model, ADR-003: Sub-Process Flattening at Parse Time, ADR-004: Lock-Free Concurrency via DashMap, ADR-005: Camunda-Compatible Service Task API, ADR-006: SSE Push Events for UI, ADR-007: Two-Stage Persistence Retry, ADR-008: Rhai Script Engine (not JavaScript) (+3 more)

### Community 120 - "X2 / X9b — Conditional events"
Cohesion: 0.20
Nodes (10): Acceptance and delivery, Expression contract, First increment: shared evaluation and X9b, Implementation and acceptance checkpoint (2026-10-15), Later X2 increments, Persistence, suspension, and failure, References and limits, Trigger and ordering contract (+2 more)

### Community 121 - "tests/task_handler.rs"
Cohesion: 0.10
Nodes (23): FALLBACK_TICK, IN_PROCESS_WORKER_ID, LOCK_DURATION_SECS, MAX_CONCURRENT_HANDLERS, TaskHandler, TaskOutcome, BpmnError, Complete (+15 more)

### Community 122 - ".handle_next_action"
Cohesion: 0.18
Nodes (5): WorkflowEngine, candidate_ids(), insert(), remove(), WorkflowEngine

### Community 123 - "MonitoringPage.tsx"
Cohesion: 0.10
Nodes (41): Badge(), BadgeProps, badgeVariants, Table, TableBody, TableCell, TableHead, TableHeader (+33 more)

### Community 124 - "api-spec/index.md"
Cohesion: 0.20
Nodes (7): api-spec, api-spec — Interfaces, Deployed Artifacts, TypeSpec Source, api-spec — Responsibilities, Regeneration, What It Owns

### Community 125 - "BPMN 2.0.2 Element Matrix"
Cohesion: 0.22
Nodes (9): Artifacts, BPMN 2.0.2 Element Matrix, Connecting Objects, Data, Flow Objects — Activities, Flow Objects — Events, Flow Objects — Gateways, Maintenance (+1 more)

### Community 126 - "tests/event_subprocess.rs"
Cohesion: 0.08
Nodes (52): activity_error_boundary_precedes_process_handler(), cancellation_cascades_to_call_activity_child(), concurrent_interrupting_signals_claim_one_handler(), conditional_administrative_relocation_is_rejected_without_changing_work(), conditional_call_outputs_interrupt_parent_before_continuation(), conditional_claim_prevents_a_stale_completion_decision(), conditional_competing_interruptions_use_stable_handler_order(), conditional_concurrent_updates_claim_once_and_deletion_rearms() (+44 more)

### Community 127 - "commands/tasks.rs"
Cohesion: 0.42
Nodes (9): complete_service_task(), complete_task(), fetch_and_lock_service_tasks(), get_pending_message_catches(), get_pending_service_tasks(), get_pending_tasks(), get_pending_timers(), resolve_incident() (+1 more)

### Community 128 - "DataMappingsPropertiesProvider.ts"
Cohesion: 0.17
Nodes (16): ActivityInputEditor(), ActivityOutputEditor(), ChildInputEditor(), ChildOutputEditor(), DataMappingsPropertiesProvider, DefaultsEditor(), MappingEditor(), MappingValue (+8 more)

### Community 129 - "e2e_suspend_resume.rs"
Cohesion: 0.42
Nodes (8): deploy_and_start(), move_token_to_existing_node_returns_204(), move_token_to_nonexistent_node_returns_404(), resume_suspended_instance_returns_204(), start_server(), suspend_instance_returns_204(), suspend_reflects_suspended_state(), TWO_TASK_BPMN

### Community 130 - "e2e_gateways.rs"
Cohesion: 0.36
Nodes (7): deploy_and_execute_parallel_gateway(), link_events_route_to_matching_catch(), LINK_XML, multi_instance_and_standard_loop_over_rest(), MULTI_INSTANCE_XML, PARALLEL_XML, start_server()

### Community 131 - "Test Metrics"
Cohesion: 0.20
Nodes (10): Benchmarks, bpmn-parser Tests (46 tests), Code Statistics, Continuous Fuzzing, desktop-tauri E2E Tests (52 tests, 1 file), engine-core Breakdown (264 tests), engine-server E2E Tests (74 tests, 19 files), Mutation Score (+2 more)

### Community 132 - "engine_bench.rs"
Cohesion: 0.29
Nodes (8): bench_condition(), bench_execution_throughput(), bench_gateways_and_scripts(), drain_service_tasks(), linear_definition(), parallel_definition(), script_chain_definition(), xor_definition()

### Community 133 - "commands/deploy.rs"
Cohesion: 0.53
Nodes (6): delete_all_definitions(), delete_definition(), deploy_definition(), deploy_simple_process(), get_definition_xml(), list_definitions()

### Community 134 - "Data Flows"
Cohesion: 0.22
Nodes (9): 1. Deployment Flow, 2. Instance Execution Flow, 3. Service Task (Fetch-and-Lock), 4. Timer Processing, 5. SSE Event Push, 6. Startup Restore Flow, 7. Health vs Readiness, 8. Persistence Retry (Fault-Tolerant) (+1 more)

### Community 135 - "Public API Surface"
Cohesion: 0.17
Nodes (12): Core Structs (re-exported via `engine-core::*`), EngineEvent (broadcast channel), Error Types, History Types, In-Process-Task-Handler (`engine/task_handler.rs`), Loops / Multi-Instance (`engine/loops.rs`, X4/X5), Persistence Trait, Public API Surface (+4 more)

### Community 136 - "Overview"
Cohesion: 0.22
Nodes (9): Architecture Summary, Boundaries & Invariants, Key Design Decisions, Overview, Production Hardening (env), Project Mission, Tech Stack, Test Coverage (+1 more)

### Community 137 - "InstanceState"
Cohesion: 0.15
Nodes (13): InstanceState, Completed, CompletedWithError, ParallelExecution, Running, Suspended, WaitingOnCallActivity, WaitingOnConditional (+5 more)

### Community 138 - "e2e_file_variables.rs"
Cohesion: 0.50
Nodes (7): BPMN_USER_TASK, deploy_and_start(), file_variable_survives_task_completion(), file_variable_upload_creates_reference(), multiple_file_variables_and_delete_one(), start_server(), upload_file()

### Community 139 - "scripts"
Cohesion: 0.22
Nodes (9): scripts, build, dev, lint, preview, tauri, test:deps, test:e2e (+1 more)

### Community 140 - "app.spec.ts"
Cohesion: 0.22
Nodes (5): DEFAULT_MOCK_STATE, injectTauriMock(), loadXmlAndClickElement(), MockState, @playwright/test

### Community 141 - "bpmn-js-properties-panel"
Cohesion: 0.20
Nodes (8): AssigneePropertiesProvider, AssigneeProps(), CustomAssigneeGroup(), CustomTopicGroup(), TopicPropertiesProvider, TopicProps(), ⚠️ bpmn-js requires Camunda moddle, bpmn-js-properties-panel

### Community 143 - "UI/Desktop Agent (Frontend)"
Cohesion: 0.29
Nodes (6): CSS & Theming Conventions, Key Files, Rules, Tech Stack (NEVER deviate), UI/Desktop Agent (Frontend), UI Patterns

### Community 144 - "Dependencies"
Cohesion: 0.25
Nodes (8): bpmn-parser, Crate-Level Dependency Graph, Dependencies, Detailed Dependency Table, engine-core, engine-server, Inbound Dependencies (Who Calls Me), Key External Dependencies

### Community 145 - "variables.rs"
Cohesion: 0.33
Nodes (7): decode_variables(), encode_variable(), inferred(), matches(), stale_hints_fall_back_and_double_values_use_floating_json(), TypedVariable, TYPES

### Community 146 - "definitions"
Cohesion: 0.25
Nodes (8): 1. Technology Stack (fixed), 2. NATS Feature Split (NEVER change this!), 3. Important Principles, 4. Forbidden, NATS Message Broker Rules (BPMNinja Project), definitions(), ⚠️ BPMN XML is stored separately from definitions, ⚠️ KV bucket names cannot change after creation

### Community 147 - "X9 — Event subprocesses"
Cohesion: 0.14
Nodes (14): Conditional-start checkpoint (2026-10-15), Definition and parser, Implementation checkpoint (2026-10-15), Later increments and dependencies, Lifecycle and interruption recovery checkpoint (2026-10-15), Modeler and integration checkpoint (2026-10-15), Modeler, contract, and acceptance, Proposed first implementation: X9a (+6 more)

### Community 148 - "migration.rs"
Cohesion: 0.28
Nodes (4): setup_migration_engine_same_ids(), migrate_instance_same_node_ids_succeeds(), migrate_instance_suspended_rejected(), migrate_instance_unknown_target_definition_fails()

### Community 149 - "json"
Cohesion: 0.28
Nodes (8): find_incoming(), load_json(), main(), merge_metrics(), mutation_has_signal(), patch_readme(), render_readme_block(), write_badges()

### Community 152 - "BPMNinja Desktop (Tauri + React + bpmn-js)"
Cohesion: 0.29
Nodes (6): BPMNinja Desktop (Tauri + React + bpmn-js), Entwicklung, Flow Conditions (Camunda-7-kompatibel), Qualitätssicherung, Voraussetzungen, Zentrale Features

### Community 155 - "Maintenance"
Cohesion: 0.29
Nodes (6): Content Quality Rules, File Structure Convention, How to Update, Last bulk sync, Maintenance, When to Update This Knowledge Base

### Community 156 - "engine-core/index.md"
Cohesion: 0.25
Nodes (3): engine-core — Gotchas, engine-core, engine-core — Interfaces

### Community 157 - "@bpmn-io/properties-panel"
Cohesion: 0.38
Nodes (4): CalledElementGroup(), CalledElementPropertiesProvider, CalledElementProps(), @bpmn-io/properties-panel

### Community 158 - "BPMNinja"
Cohesion: 0.05
Nodes (43): 1. Run the backend prerequisites, 2. Install the desktop app binary release, 3. Running from source (for developers), Activities & Sub-Processes, Additional Concepts, Architecture, Basic Elements, BPMNinja (+35 more)

### Community 159 - "e2e_history.rs"
Cohesion: 0.48
Nodes (6): MINIMAL_BPMN_XML, start_server_with_nats(), test_completed_instance_appears_in_history_instances(), test_history_instances_filter_by_business_key(), test_history_instances_pagination(), verify_instance_history_is_generated_and_retrieved()

### Community 160 - "e2e_instances_page.rs"
Cohesion: 0.48
Nodes (5): deploy_and_start_n(), list_instances_page_sets_x_total_count(), list_instances_without_query_is_full_array_without_total_header(), start_server(), USER_TASK_BPMN

### Community 161 - "e2e_monitoring.rs"
Cohesion: 0.48
Nodes (5): health_endpoint_returns_200(), info_endpoint_returns_backend_type(), monitoring_endpoint_returns_stats(), ready_endpoint_returns_200_without_nats(), start_server()

### Community 162 - "e2e_start_errors.rs"
Cohesion: 0.48
Nodes (6): start_instance_with_timer_start_returns_400(), start_latest_with_unknown_bpmn_id_returns_400(), start_server(), start_with_invalid_uuid_format_returns_400(), start_with_unknown_definition_uuid_returns_404(), TIMER_START_BPMN_XML

### Community 164 - "Task Memory Protocol & Context Resets"
Cohesion: 0.33
Nodes (5): Abschluss, Der `/compact` Re-Entry Workflow, Task Memory Protocol & Context Resets, Wann wird `TASK_MEMORY.md` im Projekt-Root angelegt?, Zwingender Inhalt

### Community 165 - "Dependency Management"
Cohesion: 0.33
Nodes (5): Adding New Dependencies, Dependency Management, Feature Flags, Forbidden, Workspace Dependencies (mandatory)

### Community 166 - "Server Agent"
Cohesion: 0.33
Nodes (5): API Conventions, Architecture, Middleware, Rules, Server Agent

### Community 167 - "scripting.rs"
Cohesion: 0.18
Nodes (3): build_script_test_definition(), script_mutates_state_and_executes_logic(), test_script_start_vs_end_listener_distinction()

### Community 168 - "UI/Desktop Agent (Tauri Rust Backend)"
Cohesion: 0.33
Nodes (5): AppState, Key File: `src-tauri/src/main.rs`, Rules, Tauri Commands (all async, all take `State<AppState>`), UI/Desktop Agent (Tauri Rust Backend)

### Community 170 - "data_tests.rs"
Cohesion: 0.31
Nodes (6): x11_call_in_out_are_explicit_transfers(), x11_default_extension_resolves_namespace_aliases(), x11_defaults_declarations_and_nested_camunda_parameters(), x11_standard_io_associations_resolve_ids_to_variable_names(), x11_task_property_is_an_association_endpoint(), xml()

### Community 171 - "commands/files.rs"
Cohesion: 0.67
Nodes (3): delete_instance_file(), download_instance_file(), upload_instance_file()

### Community 172 - "desktop-tauri/tsconfig.json"
Cohesion: 0.33
Nodes (5): compilerOptions, baseUrl, paths, files, references

### Community 173 - "LoadTest"
Cohesion: 0.22
Nodes (4): LoadTest, do_GET(), do_POST(), respond()

### Community 174 - "engine-server — Interfaces"
Cohesion: 0.10
Nodes (16): engine-server, AppError → HTTP Status Mapping, AppState (shared state for all route handlers), Auth & CORS env, Builder Functions, engine-server — Interfaces, Health vs Readiness, HTTP lock duration (+8 more)

### Community 175 - "server/messages.rs"
Cohesion: 0.44
Nodes (6): broadcast_signal(), BroadcastSignalRequest, correlate_message(), CorrelateMessageRequest, CorrelateMessageResponse, get_pending_messages()

### Community 176 - "audit.rs"
Cohesion: 0.31
Nodes (4): AUDIT_ACCESS, AuditPruneReport, failure(), NatsPersistence

### Community 177 - "engine-core"
Cohesion: 0.60
Nodes (6): bpmn-parser, bpmninja-fuzz, engine-core, engine-server, persistence-memory, persistence-nats

### Community 178 - "Graph Maintenance & Metriken"
Cohesion: 0.40
Nodes (4): Graph Maintenance & Metriken, Manueller Full Re-Build, Nach jedem Update – Metriken in `GRAPH_REPORT.md` prüfen:, Wann `graphify update .` ausgeführt werden MUSS:

### Community 179 - "Parser Agent"
Cohesion: 0.40
Nodes (4): Error Handling, Parser Agent, Parsing Rules, Testing

### Community 180 - "RTK - Rust Token Killer (Google Antigravity)"
Cohesion: 0.40
Nodes (4): Meta Commands, RTK - Rust Token Killer (Google Antigravity), Rule, Why

### Community 181 - "EventSubprocessPropertiesProvider.ts"
Cohesion: 0.29
Nodes (11): ConditionalValue(), eventDefinition(), EventSubprocessPropertiesProvider, Interrupting(), ReferenceId(), references, ReferenceValue(), TimerKind() (+3 more)

### Community 182 - "event_subprocess_tests.rs"
Cohesion: 0.39
Nodes (7): conditional_event_subprocess_validates_inline_expression_contract(), x9a_container_listener_validation_rejects_unsupported_forms_and_colliding_ids(), x9a_container_listeners_keep_the_container_id(), x9a_handler_tasks_retain_x11_mappings(), x9a_parses_all_five_starts_without_creating_another_process_start(), x9a_rejects_illegal_starts_and_scopes(), xml()

### Community 183 - "BPMN Model Checking and State-Space Reduction"
Cohesion: 0.22
Nodes (9): BPMN Model Checking and State-Space Reduction, BPMN verification and reduction, BPMN Verification Pipeline, BPMNinja Implementation, Other State-Space Techniques, Partial order reduction, Partial Order Reduction (POR), Purpose (+1 more)

### Community 185 - "package.json"
Cohesion: 0.50
Nodes (3): dependencies, @typespec/compiler, @typespec/compiler

### Community 186 - "fuzz/index.md"
Cohesion: 0.40
Nodes (3): fuzz, fuzz — Responsibilities, What It Owns

### Community 187 - "boundary.rs"
Cohesion: 0.43
Nodes (4): setup_boundary_events(), test_setup_boundary_message_event(), test_setup_boundary_no_events(), test_setup_boundary_timer_event()

### Community 188 - "X2a — Intermediate conditional catches"
Cohesion: 0.22
Nodes (9): Acceptance and delivery, Current code and required integration, Event-based gateway competition, Implementation checkpoint (2026-10-15), Persisted state and atomic claims, Recovery, suspension, and administrative operations, Scope and observable behavior, Semantic reference (+1 more)

### Community 189 - "persistence — Responsibilities"
Cohesion: 0.33
Nodes (5): Key Entry Points, persistence-memory (Testing/Development), persistence-nats (Production), persistence — Responsibilities, Summary

### Community 192 - "Global Agent Directives & Workflow"
Cohesion: 0.50
Nodes (3): Global Agent Directives & Workflow, Priorität bei Regelkonflikten, Workflow

### Community 193 - "Rust Best Practices for BPMNinja"
Cohesion: 0.50
Nodes (3): Anti-Patterns (NEVER do), Must-Follow Rust Rules, Rust Best Practices for BPMNinja

### Community 194 - "Versioning Convention"
Cohesion: 0.50
Nodes (3): Files to Update, Rules, Versioning Convention

### Community 196 - "Server allocator and release optimization experiments"
Cohesion: 0.33
Nodes (6): Compare allocators and LTO, Evaluate profile-guided optimization on Linux, HTTP-/NATS-Vergleich mit RSS und Tail-Latenz, Initial local allocator result (2026-10-13), Lokaler HTTP-/NATS-Vergleich (2026-10-13), Server allocator and release optimization experiments

### Community 199 - "e2e_lifecycle.rs"
Cohesion: 0.18
Nodes (16): correlate_message_with_no_match_returns_empty(), delete_definition_cascade_removes_instances(), delete_definition_without_cascade_with_instances_returns_409(), delete_instance_returns_204(), deploy_and_start(), get_unknown_instance_returns_404(), instantiating_receive_task_starts_instance_on_message(), _MINIMAL_BPMN (+8 more)

### Community 202 - "super"
Cohesion: 0.14
Nodes (4): legacy_snapshots_default_type_hints_and_native_updates_clear_them(), retry_deadline_survives_restore_and_blocks_fetch_until_expiry(), service(), suspended_instances_are_not_fetchable_and_millisecond_extension_is_exact()

### Community 203 - "ProcessDefinition"
Cohesion: 0.08
Nodes (12): DataScope, ProcessDefinition, SequenceFlow, enter(), leave(), find_any_boundary_escalation_event(), find_boundary_error_event(), find_boundary_escalation_event() (+4 more)

### Community 230 - "X11 — Data defaults and activity mappings"
Cohesion: 0.29
Nodes (6): Acceptance and verification, Boundaries, Implementation sequence, Scope and behavior, Verification result (2026-10-14), X11 — Data defaults and activity mappings

### Community 231 - "instance_pagination.rs"
Cohesion: 0.38
Nodes (5): deploy_start_end(), dummy_instance(), list_instances_page_offset_limit_clones_slice(), list_instances_page_sorts_started_at_desc_then_id(), list_instances_without_limit_returns_all()

### Community 232 - "X2c — Explicit top-level conditional starts"
Cohesion: 0.33
Nodes (5): Acceptance gates, Durable evaluation and recovery, Implementation outline, Scope and observable behavior, X2c — Explicit top-level conditional starts

### Community 233 - "e2e_deploy.rs"
Cohesion: 0.33
Nodes (7): deploy_and_start_minimal_workflow_completes_immediately(), deploy_invalid_xml_returns_400(), deploy_minimal_workflow_returns_definition_id(), MINIMAL_BPMN_XML, start_server(), swimlanes_and_artifacts_deploy_run_and_export_unchanged(), SWIMLANES_ARTIFACTS_XML

### Community 234 - "ExternalTaskClient"
Cohesion: 0.15
Nodes (9): ExternalTaskClient, BPMNinja Knowledge Base, Cross-Cutting Topics, Directory Map, Key Design Principles, Module Quick Index, Quick Start for Agents, agent-orchestrator (+1 more)

### Community 236 - "persistence — Interfaces"
Cohesion: 0.18
Nodes (10): Monitoring Data Types (from engine-core port), Optional compact instance encoding (P8), Optional delta/checkpoint journal (P8, 2026-10-13), Optional separate audit storage (P8), Per-instance audit cleanup (P8, 2026-10-13), persistence — Interfaces, persistence-memory Public API, persistence-nats Public API (+2 more)

### Community 239 - "retry.ts"
Cohesion: 0.29
Nodes (11): When to Activate, calculateBackoff(), RetryResult, sleep(), withRetry(), external-task-client — Responsibilities, Invariants, Key Entry Points (+3 more)

### Community 240 - "ActorType"
Cohesion: 0.33
Nodes (6): ActorType, Engine, Listener, ServiceWorker, Timer, User

### Community 241 - "truncate_value_for_diff"
Cohesion: 0.33
Nodes (6): test_truncate_value_for_diff_exactly_1024(), test_truncate_value_for_diff_exactly_128_array(), test_truncate_value_for_diff_large_array(), test_truncate_value_for_diff_long_string(), test_truncate_value_for_diff_small_array(), truncate_value_for_diff()

### Community 242 - "persistence — Dependencies"
Cohesion: 0.29
Nodes (5): Inbound, Outbound, persistence — Dependencies, Trait Implementation Diagram, persistence

### Community 243 - "ScriptPropertiesProvider.ts"
Cohesion: 0.43
Nodes (4): findListener(), ScriptEntry(), ScriptGroup(), ScriptPropertiesProvider

### Community 244 - "constants.rs"
Cohesion: 0.33
Nodes (4): MAX_AUDIT_LOG_ENTRIES, MAX_EXECUTION_STEPS, MAX_INSTANCE_PAYLOAD_BYTES, YIELD_EVERY_N_STEPS

### Community 245 - "Data defaults and mappings (X11)"
Cohesion: 0.33
Nodes (6): Call Activities, Camunda activity parameters, Data defaults and mappings (X11), Modeler and compatibility, Process data, Standard associations

### Community 246 - "vite.config.ts"
Cohesion: 0.40
Nodes (3): @tailwindcss/vite, vite, @vitejs/plugin-react

### Community 248 - "history_batch.rs"
Cohesion: 0.47
Nodes (6): batch_history_is_written_in_one_call(), engine_with(), history_is_flushed_when_the_batch_fails(), history_of(), no_history_work_without_persistence(), script_chain()

### Community 249 - "X2b — Conditional boundary catches"
Cohesion: 0.33
Nodes (6): Acceptance gates, Implementation checkpoint (2026-10-16), Implementation outline, Observable contract, Scope, X2b — Conditional boundary catches

### Community 250 - "6. REST API (engine-server)"
Cohesion: 0.40
Nodes (5): 6.1 Route Overview (46 endpoints), 6.2 Server Architecture, 6.3 Background Timer Scheduler, 6.4 Health & Readiness, 6. REST API (engine-server)

### Community 251 - "5.2 Implementations"
Cohesion: 0.50
Nodes (4): 5.1 WorkflowPersistence Trait, 5.2 Implementations, 5.3 Fault-Tolerant Retry Queue (K6), 5. Persistence Architecture

### Community 252 - "Intermediate conditional catches (X2a)"
Cohesion: 0.67
Nodes (3): Event-based gateways, Intermediate conditional catches (X2a), Recovery and visibility

### Community 253 - "e2e_versioning.rs"
Cohesion: 0.43
Nodes (6): BPMN_V1, BPMN_V2, deploy_same_bpmn_id_increments_version(), instances_stay_on_original_version(), start_latest_uses_newest_version(), start_server()

### Community 254 - "engine-core — Dependencies"
Cohesion: 0.40
Nodes (4): engine-core — Dependencies, Key Cargo.toml Excerpt, Outbound (What engine-core Depends On), Trait Relationship Diagram

### Community 256 - "call_activity_parallel.rs"
Cohesion: 0.60
Nodes (3): child_process(), linear_call_activity_child_complete_parent_completes(), parallel_call_activities_both_children_complete_parent()

### Community 257 - "8. Concurrency & Locking (K1)"
Cohesion: 0.67
Nodes (3): 8.1 Lock Hierarchy, 8.2 Deadlock Prevention Pattern, 8. Concurrency & Locking (K1)

### Community 258 - "persist_lock.rs"
Cohesion: 0.48
Nodes (5): concurrent_saves_persist_the_latest_state(), inline_retries_reuse_the_checked_json_snapshot(), instance_stays_readable_while_save_is_in_flight(), oversized_snapshot_never_reaches_the_backend(), setup()

### Community 259 - "Event subprocess runtime (X9a/X9b)"
Cohesion: 0.40
Nodes (5): Conditional starts (X9b), Container execution listeners, Editing a start in the modeler, Event subprocess runtime (X9a/X9b), Lifecycle history and notifications

### Community 260 - "Camunda-7-Worker-Fassade"
Cohesion: 0.50
Nodes (4): Camunda-7-Worker-Fassade, Failure, Lock-Verlängerung und BPMN Error, Fetch und Completion, Grenzen und Authentifizierung

### Community 261 - "Conditional boundary catches (X2b)"
Cohesion: 0.67
Nodes (3): Activation and delivery, Conditional boundary catches (X2b), Persistence and observability

### Community 263 - "SSE-Zustandsupdates"
Cohesion: 0.67
Nodes (3): Desktop-Verhalten, SSE-Zustandsupdates, Verlust und Wiederverbindung

### Community 265 - "server/mod.rs"
Cohesion: 0.20
Nodes (5): build_app(), build_app_with_config(), build_app_with_engine(), build_app_with_options(), build_cors_layer()

### Community 268 - "architecture/index.md"
Cohesion: 0.25
Nodes (4): Components, Logical Component Map, Architecture, Quick Reference

### Community 271 - "Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)"
Cohesion: 0.25
Nodes (8): 4a — Engine-Performance (Hot Path), 4b — Fundament für Verteilung (Voraussetzung für 4c–4e), 4c — Native Messaging: NATS, Redpanda, Kafka, 4d — Microservices & Dapr, 4e — Serverless & Service Mesh, 4f — BPMN-2.0.2-Vollabdeckung (Vorschlag, 2026-10-12), Empfohlene Reihenfolge, Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)

### Community 279 - "4. Execution Architecture"
Cohesion: 0.50
Nodes (4): 4.1 Token Lifecycle (K4), 4.2 Execution Loop (run_instance_batch), 4.3 Gateway Routing, 4. Execution Architecture

## Knowledge Gaps
- **228 isolated node(s):** `agent-orchestrator`, `@typespec/compiler`, `@typespec/http`, `@typespec/rest`, `@typespec/openapi3` (+223 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1719 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **62 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `reject_unsupported_elements()` connect `parser.rs` to `tests/data_mappings.rs`, `X9 — Event subprocesses`, `parse_bpmn_xml`?**
  _High betweenness centrality (0.277) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `cn()` (e.g. with `CSS & Theming Conventions` and `Key Files`) actually correct?**
  _`cn()` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `agent-orchestrator`, `@typespec/compiler`, `@typespec/http` to the rest of the system?**
  _228 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `NatsPersistence` be split into smaller, more focused modules?**
  _Cohesion score 0.14509246088193456 - nodes in this community are weakly interconnected._
- **Why does `Starting point before X9a` connect `X9 — Event subprocesses` to `parser.rs`?**
  _High betweenness centrality (0.276) - this node is a cross-community bridge._
- **Are the 68 inferred relationships involving `parse_bpmn_xml()` (e.g. with `x11_call_in_out_are_explicit_transfers()` and `x11_defaults_declarations_and_nested_camunda_parameters()`) actually correct?**
  _`parse_bpmn_xml()` has 68 INFERRED edges - model-reasoned connections that need verification._
- **Should `InMemoryPersistence` be split into smaller, more focused modules?**
  _Cohesion score 0.07211538461538461 - nodes in this community are weakly interconnected._