# Graph Report - bpmninja  (2026-10-13)

## Corpus Check
- 377 files · ~487,742 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 18 file(s) not represented in the graph (top: (none) 8, .toml 2, .xml 2)

## Summary
- 4062 nodes · 8749 edges · 260 communities (196 shown, 64 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 248 edges (avg confidence: 0.89)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `86d98855`
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
- EngineError
- build_mutation_metrics.py
- stress_tests.rs
- desktop-tauri/package.json
- server/instances.rs
- Token
- tests/gateways.rs
- bpmn-parser/src/models.rs
- persistence — Gotchas
- dependencies
- .persist_instance
- bpmn-ninja-external-task-client/package.json
- log_buffer.rs
- condition.rs
- agents
- bpmn-parser — Gotchas
- react
- BpmnElement
- sse_consumer.rs
- MonitoringPage.tsx
- use-toast.ts
- server/state.rs
- engine-server — Gotchas
- instance.rs
- server/tasks.rs
- index.ts
- timer.rs
- Uuid
- cn
- .handle_bpmn_error_with_variables
- BPMNinja External Task Client Skill
- runner.rs
- auth.rs
- scripts
- parser.rs
- TimerDueIndex
- compilerOptions
- Critical Rules
- Value
- WorkflowEngine
- compilerOptions
- components.json
- HistoryEventType
- serialize
- server/mod.rs
- FuzzValue
- TaskService
- devDependencies
- compilerOptions
- ProcessDefinitionBuilder
- server/files.rs
- BPMNinja Quality Skill
- api-spec/package.json
- external-task-client — Dependencies
- useEngineEvents
- call_errors.rs
- pending.rs
- history/mod.rs
- BPMNinja Desktop Skill
- BPMNinja Server Skill
- client.rs
- http_load.py
- desktop-tauri — Gotchas
- tauri.conf.json
- .record_history_event
- ScriptPropertiesProvider.ts
- nats_bench.rs
- PendingTasksPage.tsx
- api_helpers.rs
- history.rs
- commands/instances.rs
- boundary.rs
- server/deploy.rs
- server/events.rs
- CLAUDE.md — BPMNinja Project Guide
- bpmninja — Architecture Documentation
- super
- scripting.rs
- BPMNinja Parser Skill
- fuzz_history_diff.rs
- Component Responsibilities
- desktop-tauri/index.md
- persistence_ops.rs
- e2e_call_activity.rs
- value
- BPMNinja — Priorisierte Verbesserungen
- commands/monitoring.rs
- Shared Patterns
- e2e_lifecycle.rs
- e2e_migration.rs
- e2e_service_tasks.rs
- BPMNinja Engine Skill
- engine_bench.rs
- `ExternalTaskClient`
- @bpmninja/external-task-client
- ExternalTaskClient.test.ts
- ConditionPropertiesProvider.ts
- timer_due_index.rs
- StartupCoordinator
- codec.rs
- parallel.rs
- tests/topic_index.rs
- assert-typescript-peer.mjs
- Key Architectural Decisions
- WorkflowEngine
- commands/tasks.rs
- engine/topic_index.rs
- instance_pagination.rs
- api-spec/index.md
- BPMN 2.0.2 Element Matrix
- variables.rs
- InstanceDetailDialog.tsx
- bpmn_compliance.rs
- e2e_suspend_resume.rs
- camunda/tests.rs
- Test Metrics
- observability.rs
- commands/deploy.rs
- Data Flows
- Public API Surface
- Overview
- history_batch.rs
- e2e_file_variables.rs
- scripts
- app.spec.ts
- AssigneePropertiesProvider.ts
- CalledElementPropertiesProvider.ts
- engine-server/src/main.rs
- Dependencies
- tests/task_handler.rs
- InstanceState
- HistoryEntry
- timers.rs
- json
- persistence-nats/src/tests.rs
- server/monitoring.rs
- BPMNinja Desktop (Tauri + React + bpmn-js)
- README.md
- test_mutation_metrics.py
- Maintenance
- engine-core/index.md
- REST API
- BPMNinja
- e2e_history.rs
- e2e_instances_page.rs
- e2e_monitoring.rs
- e2e_start_errors.rs
- e2e_versioning.rs
- Task Memory Protocol & Context Resets
- Dependency Management
- Server Agent
- NATS Message Broker Rules (BPMNinja Project)
- UI/Desktop Agent (Tauri Rust Backend)
- knowledge-base/index.md
- log_nats.rs
- commands/files.rs
- desktop-tauri/tsconfig.json
- LoadTest
- engine-server — Interfaces
- server/messages.rs
- .execute_task_handler
- engine-core
- Graph Maintenance & Metriken
- Parser Agent
- RTK - Rust Token Killer (Google Antigravity)
- TopicPropertiesProvider.ts
- BPMN Model Checking and State-Space Reduction
- LogBuffer
- package.json
- fuzz/index.md
- Supported BPMN Elements
- 4. Execution Architecture
- InstanceStatus
- ALLOCATOR
- Global Agent Directives & Workflow
- Rust Best Practices for BPMNinja
- Versioning Convention
- vite.config.ts
- Server allocator and release optimization experiments
- AGENTS.md
- copilot-instructions.md
- desktop-tauri — Dependencies
- Execution Engine Agent
- Workflow: graphify
- MessageVisitor
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
- handlers/tasks.rs
- calculate_diff
- e2e_stress.rs
- e2e_deploy.rs
- ExternalTaskClient
- completion.rs
- .handle_next_action
- constants.rs
- logs.rs
- retry.ts
- Empfohlener Aktionsplan (phasiert)
- Desktop Application (UI)
- External Task Client (TypeScript)
- camunda_compat.rs
- AppError
- 2. engine-core — Core Architecture
- 3. Data Model
- Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)
- in_memory.rs
- inclusive_join.rs
- BPMNinja Knowledge Base
- engine-core — Dependencies
- correlate_message
- 8. Concurrency & Locking (K1)
- InnerBuffer
- ActorType
- EngineEvent

## God Nodes (most connected - your core abstractions)
1. `Value` - 130 edges
2. `ProcessDefinition` - 112 edges
3. `cn()` - 93 edges
4. `Token` - 79 edges
5. `WorkflowEngine` - 71 edges
6. `parse_bpmn_xml()` - 62 edges
7. `WorkflowPersistence` - 58 edges
8. `NextAction` - 55 edges
9. `InMemoryPersistence` - 54 edges
10. `InstrumentedPersistence` - 49 edges

## Surprising Connections (you probably didn't know these)
- `Invariants` --references--> `TaskService`  [INFERRED]
  docs/knowledge-base/modules/external-task-client/responsibility.md → bpmn-ninja-external-task-client/src/TaskService.ts
- `In-Process-Task-Handler (`engine/task_handler.rs`)` --references--> `TaskHandler`  [INFERRED]
  docs/knowledge-base/modules/engine-core/interfaces.md → bpmn-ninja-external-task-client/src/types.ts
- `⚠️ Playwright tests use specific selectors` --references--> `InstancesPage()`  [INFERRED]
  docs/knowledge-base/modules/desktop-tauri/gotchas.md → desktop-tauri/src/features/instances/InstancesPage.tsx
- `Grenzen und Authentifizierung` --references--> `ProcessInstance`  [INFERRED]
  docs/camunda7-workers.md → desktop-tauri/src/shared/types/engine.ts
- `ADR-002: Token-Based Execution Model` --references--> `ProcessInstance`  [INFERRED]
  docs/knowledge-base/architecture/decisions.md → desktop-tauri/src/shared/types/engine.ts

## Import Cycles
- 2-file cycle: `engine-server/src/server/camunda.rs -> engine-server/src/server/camunda/variables.rs -> engine-server/src/server/camunda.rs`

## Communities (260 total, 64 thin omitted)

### Community 2 - "hashmap"
Cohesion: 0.07
Nodes (4): FALLBACK_TICK, IN_PROCESS_WORKER_ID, LOCK_DURATION_SECS, MAX_CONCURRENT_HANDLERS

### Community 3 - "tauri.ts"
Cohesion: 0.08
Nodes (46): 1. Model Layer (`model.rs`), 2. Engine Core (`engine/` submodule), 3. External Tasks (`service_task.rs`), 4. Condition Evaluator (`condition.rs`), 5. Script Runner (`script_runner.rs`), 6. Persistence (`persistence.rs`), 7. Error Handling (`error.rs`), 8. History Module (`history.rs`) (+38 more)

### Community 4 - "FailSaveXmlPersistence"
Cohesion: 0.07
Nodes (10): deploy(), deploy_save_bpmn_xml_failure_is_5xx_and_not_listed(), deploy_without_persistence_is_200_and_listed(), FailSaveXmlPersistence, isolated_config(), list_definitions(), MINIMAL_BPMN_XML, serve() (+2 more)

### Community 5 - "parse_bpmn_xml"
Cohesion: 0.07
Nodes (55): parse_bpmn_xml(), embedded_subprocess_enters_at_start_successor_without_noop_task(), flatten_subprocess_keeps_send_inclusive_event_and_catch(), parse_boundary_error_event(), parse_boundary_timer_event(), parse_business_rule_task_with_topic(), parse_camunda_assignee_and_topic(), parse_compensation_events() (+47 more)

### Community 6 - "camunda.rs"
Cohesion: 0.19
Nodes (20): body(), bpmn_error(), CamundaError, complete(), CompleteRequest, duration(), ErrorRequest, extend() (+12 more)

### Community 7 - "WorkflowPersistence"
Cohesion: 0.08
Nodes (7): BucketEntry, BucketEntryDetail, BucketInfo, CompletedInstanceQuery, HistoryQuery, StorageInfo, WorkflowPersistence

### Community 8 - "retry_queue.rs"
Cohesion: 0.06
Nodes (28): InstanceStore, create_retry_queue(), create_retry_queue_with_capacity(), DEFAULT_RETRY_QUEUE_CAPACITY, execute_job(), INITIAL_BACKOFF_MS, INLINE_BACKOFF_MS, INLINE_RETRIES (+20 more)

### Community 9 - "Button"
Cohesion: 0.13
Nodes (45): Button, DialogContent, DialogDescription, DialogFooter(), DialogHeader(), DialogOverlay, DialogTitle, Input (+37 more)

### Community 10 - "model_check.rs"
Cohesion: 0.07
Nodes (40): AppBuildConfig, analyze(), DEFAULT_MODEL_CHECK_TIMEOUT_MS, expand_empty_elements(), expands_self_closing_elements(), ModelChecker, ModelCheckMode, Enforce (+32 more)

### Community 12 - "EngineError"
Cohesion: 0.06
Nodes (43): EngineError, AlreadyCompleted, DefinitionHasInstances, ExecutionLimitExceeded, InstanceSuspended, InvalidDefinition, NoMatchingCondition, NoSuchDefinition (+35 more)

### Community 13 - "build_mutation_metrics.py"
Cohesion: 0.34
Nodes (10): build_metrics(), count_from_outcomes_list(), count_from_summary_dict(), count_txt_lines(), counts_from_txt_dir(), find_outcomes_path(), _label(), main() (+2 more)

### Community 14 - "stress_tests.rs"
Cohesion: 0.10
Nodes (10): correctness_inclusive_gateway_partial_match(), correctness_mixed_xor_and_parallel(), drain_service_tasks(), history_query_limit_and_offset(), linear_definition(), parallel_definition(), race_concurrent_fetch_same_task(), stress_concurrent_fetch_and_lock() (+2 more)

### Community 15 - "desktop-tauri/package.json"
Cohesion: 0.05
Nodes (40): @types/node, typescript, license, name, private, type, version, autoprefixer (+32 more)

### Community 16 - "server/instances.rs"
Cohesion: 0.21
Nodes (20): delete_instance(), get_instance(), list_instances(), ListInstancesQuery, migrate_instance(), MigrateInstanceRequest, move_token(), MoveTokenRequest (+12 more)

### Community 17 - "Token"
Cohesion: 0.13
Nodes (24): Token, execute_complex_gateway(), execute_exclusive_gateway(), execute_inclusive_gateway(), execute_parallel_gateway(), WorkflowEngine, WorkflowEngine, NextAction (+16 more)

### Community 18 - "tests/gateways.rs"
Cohesion: 0.08
Nodes (18): exclusive_gateway_takes_matching_path(), exclusive_gateway_uses_default_when_no_match(), inclusive_gateway_forks_multiple_paths(), inclusive_gateway_single_match_no_fork(), test_same_gateway_type_detects_mismatch(), xor_gateway_negative_x_routes_to_user_task_2(), xor_gateway_positive_x_routes_to_user_task_1(), xor_gateway_user_task_merges_variables() (+10 more)

### Community 19 - "bpmn-parser/src/models.rs"
Cohesion: 0.19
Nodes (35): BpmnBoundaryEvent, BpmnCallActivity, BpmnCompensateEventDefinition, BpmnComplexGateway, BpmnConditionExpression, BpmnDefinitions, BpmnEndEvent, BpmnErrorDef (+27 more)

### Community 20 - "persistence — Gotchas"
Cohesion: 0.05
Nodes (35): Inbound, Outbound, persistence — Dependencies, Trait Implementation Diagram, ⚠️ Adding a new entity type, ⚠️ BPMN XML is stored separately from definitions, ⚠️ Bucket handles are cached, ⚠️ Completed instances are archived, not deleted (+27 more)

### Community 21 - "dependencies"
Cohesion: 0.06
Nodes (35): dependencies, @base-ui/react, @bpmn-io/properties-panel, bpmn-js, bpmn-js-properties-panel, camunda-bpmn-moddle, class-variance-authority, clsx (+27 more)

### Community 23 - "bpmn-ninja-external-task-client/package.json"
Cohesion: 0.06
Nodes (33): author, dependencies, pino, pino-pretty, description, devDependencies, tsx, @types/node (+25 more)

### Community 24 - "log_buffer.rs"
Cohesion: 0.16
Nodes (7): append_entry_to_file(), COMPACT_AFTER, compact_file(), load_entries_from_file(), LogEntry, MAX_ENTRIES, parse_level()

### Community 25 - "condition.rs"
Cohesion: 0.11
Nodes (16): CompiledCondition, Comparison, Empty, Truthy, evaluate_condition(), Operator, Eq, Ge (+8 more)

### Community 27 - "agents"
Cohesion: 0.06
Nodes (31): agents, engine, orchestrator, parser, persistence, quality, server, ui (+23 more)

### Community 28 - "bpmn-parser — Gotchas"
Cohesion: 0.06
Nodes (27): bpmn-parser — Dependencies, Dependency Graph, Inbound, Outbound, ⚠️ Adding a new BpmnElement variant, bpmn-parser — Gotchas, ⚠️ Camunda namespace handling, ⚠️ InclusiveGateway has no join metadata in the model (+19 more)

### Community 29 - "react"
Cohesion: 0.14
Nodes (18): ButtonProps, formatDateTime(), isActive(), ProcessDefinitionPage(), Props, formatDateTime(), formatDuration(), HistoryPage() (+10 more)

### Community 30 - "BpmnElement"
Cohesion: 0.06
Nodes (30): BpmnElement, BoundaryCompensationEvent, BoundaryErrorEvent, BoundaryEscalationEvent, BoundaryMessageEvent, BoundaryTimerEvent, CallActivity, CompensationEndEvent (+22 more)

### Community 31 - "sse_consumer.rs"
Cohesion: 0.10
Nodes (19): connect_and_consume(), ConsumeOutcome, StreamEnded, UrlChanged, forwards_multiline_payload_only_at_message_boundary(), run_consumer(), sleep_or_url_change(), spawn() (+11 more)

### Community 32 - "MonitoringPage.tsx"
Cohesion: 0.08
Nodes (43): Badge(), BadgeProps, badgeVariants, Table, TableBody, TableCaption, TableCell, TableFooter (+35 more)

### Community 33 - "use-toast.ts"
Cohesion: 0.10
Nodes (25): Toast, ToastAction, ToastActionElement, ToastClose, ToastDescription, ToastProps, ToastTitle, toastVariants (+17 more)

### Community 34 - "server/state.rs"
Cohesion: 0.08
Nodes (14): api_key_from_env(), AppState, cors_origins_from_env(), cors_unset_is_allowlist_not_any(), DEFAULT_CORS_ORIGINS, DEFAULT_MAX_UPLOAD_BYTES, max_upload_bytes_from_env(), MAX_XML_BYTES (+6 more)

### Community 35 - "engine-server — Gotchas"
Cohesion: 0.08
Nodes (21): ⚠️ Adding a new route, ⚠️ Body size limit is 10 MiB for XML/JSON, ⚠️ CORS is an allowlist (not `Any` by default), ⚠️ Deploy XML save failure is HTTP 500, engine-server — Gotchas, ⚠️ fetchAndLock long polling is notification-driven, ⚠️ In-memory fallback is opt-in (dev), ⚠️ `lockDuration` / `newDuration` are milliseconds at the HTTP boundary (+13 more)

### Community 36 - "instance.rs"
Cohesion: 0.13
Nodes (18): ActiveToken, append_audit_log_enforces_limit(), append_audit_log_exactly_at_limit_does_not_trim(), append_audit_log_under_limit(), CompensationRecord, file_variable_names_returns_correct_list(), file_variable_names_returns_empty_when_no_files(), get_file_reference_returns_none_for_non_file() (+10 more)

### Community 37 - "server/tasks.rs"
Cohesion: 0.19
Nodes (21): lock_duration_ms_to_secs(), bpmn_error(), BpmnErrorRequest, complete_service_task(), complete_task(), CompleteRequest, CompleteServiceTaskRequest, extend_lock() (+13 more)

### Community 38 - "index.ts"
Cohesion: 0.17
Nodes (16): DEFAULTS, noopLogger(), randomId(), BpmnErrorRequest, ClientConfig, CompleteRequest, ExtendLockRequest, ExternalTask (+8 more)

### Community 39 - "timer.rs"
Cohesion: 0.11
Nodes (12): initial_remaining_absolute_date_none(), is_recurring_absolute_false(), next_expiry_absolute_date_exact_now_returns_none(), next_expiry_absolute_date_future_returns_some(), next_expiry_absolute_date_past_returns_none(), next_expiry_duration_adds_to_now(), next_expiry_repeating_interval(), TimerDefinition (+4 more)

### Community 40 - "Uuid"
Cohesion: 0.21
Nodes (4): WorkflowEngine, inline_retry!, WorkflowEngine, WorkflowEngine

### Community 41 - "cn"
Cohesion: 0.11
Nodes (41): CSS & Theming Conventions, Key Files, Rules, Tech Stack (NEVER deviate), UI/Desktop Agent (Frontend), UI Patterns, AccordionContent, AccordionItem (+33 more)

### Community 42 - ".handle_bpmn_error_with_variables"
Cohesion: 0.18
Nodes (4): WorkflowEngine, verify_lock_ownership(), WorkflowEngine, WorkflowEngine

### Community 43 - "BPMNinja External Task Client Skill"
Cohesion: 0.12
Nodes (16): API Contract (BPMNinja Engine, Rust/Axum), Architecture, BPMNinja External Task Client Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Configuration & Example, Domain Rules & Patterns, Evolution Log, Scope & File Map (+8 more)

### Community 44 - "runner.rs"
Cohesion: 0.16
Nodes (6): execute_script_safe(), MAX_CACHED_SCRIPTS, run_end_scripts(), run_node_scripts(), ScriptConfig, ScriptRunner

### Community 45 - "auth.rs"
Cohesion: 0.18
Nodes (6): auth_middleware(), extract_api_key(), HEALTH_PATH, READY_PATH, strip_bearer(), api_keys_equal()

### Community 46 - "scripts"
Cohesion: 0.08
Nodes (23): env, ENGINE_API_URL, NATS_URL, packages, $schema, build, dev, engine:docker (+15 more)

### Community 47 - "parser.rs"
Cohesion: 0.18
Nodes (18): BpmnTextValue, add_listeners(), EVENT_KINDS, first_attr(), flatten_subprocess(), nonempty_attr(), parse_iso8601_duration(), parse_multi_instance() (+10 more)

### Community 49 - "compilerOptions"
Cohesion: 0.09
Nodes (22): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, jsx, lib, module, moduleDetection, moduleResolution (+14 more)

### Community 50 - "Critical Rules"
Cohesion: 0.09
Nodes (23): ⚠️ Backward-compatible re-exports, ⚠️ `BpmnElement` is a closed enum — must be exhaustive, ⚠️ Call-Activity tracking is `outstanding_calls`, not only `WaitingOnCallActivity`, Critical Rules, ⚠️ Event channel capacity, ⚠️ Exclusive gateway takes unconditional non-default flows, ⚠️ Executor history is buffered per batch, ⚠️ History snapshots (+15 more)

### Community 51 - "Value"
Cohesion: 0.12
Nodes (20): Value, BpmnErrorRequest, BucketEntriesQuery, CompletedInstancesQuery, CompleteRequest, CompleteServiceTaskRequest, CorrelateMessageRequest, DeleteDefinitionQuery (+12 more)

### Community 53 - "compilerOptions"
Cohesion: 0.09
Nodes (21): compilerOptions, declaration, declarationMap, esModuleInterop, forceConsistentCasingInFileNames, lib, module, moduleResolution (+13 more)

### Community 54 - "components.json"
Cohesion: 0.09
Nodes (21): aliases, components, hooks, lib, ui, utils, iconLibrary, menuAccent (+13 more)

### Community 57 - "HistoryEventType"
Cohesion: 0.09
Nodes (21): HistoryEventType, BranchCompleted, CallActivityCompleted, CallActivityStarted, CompensationTriggered, Error, EscalationThrown, GatewayTaken (+13 more)

### Community 58 - "serialize"
Cohesion: 0.13
Nodes (7): FileReference, ExecutionListener, ListenerEvent, End, Start, MultiInstanceDef, NatsInfo

### Community 59 - "server/mod.rs"
Cohesion: 0.17
Nodes (5): build_app(), build_app_with_config(), build_app_with_engine(), build_app_with_options(), build_cors_layer()

### Community 60 - "FuzzValue"
Cohesion: 0.12
Nodes (7): ConditionInput, FuzzValue, Bool, Float, Int, Null, Str

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
Cohesion: 0.11
Nodes (7): conditions_are_compiled_on_deploy_and_restore(), ProcessDefinitionBuilder, SequenceFlow, ScopeEventListener, Error, Message, Timer

### Community 65 - "server/files.rs"
Cohesion: 0.19
Nodes (8): content_disposition_attachment(), content_disposition_encodes_quotes_and_utf8(), content_disposition_strips_header_injection_and_paths(), content_disposition_uses_quoted_ascii_and_rfc5987(), delete_instance_file(), get_instance_file(), rfc5987_encode(), upload_instance_file()

### Community 66 - "BPMNinja Quality Skill"
Cohesion: 0.11
Nodes (18): BPMNinja Quality Skill, CI/CD, CI/CD Workflows, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Configuration, Documentation, Documentation (+10 more)

### Community 67 - "api-spec/package.json"
Cohesion: 0.11
Nodes (18): dependencies, @typespec/compiler, @typespec/http, @typespec/openapi, @typespec/openapi3, @typespec/rest, description, @typespec/compiler (+10 more)

### Community 68 - "external-task-client — Dependencies"
Cohesion: 0.11
Nodes (14): Architecture, Dev Dependencies, external-task-client — Dependencies, Inbound, Outbound, external-task-client, external-task-client — Interfaces, HTTP Endpoints Called (+6 more)

### Community 69 - "useEngineEvents"
Cohesion: 0.21
Nodes (11): useEngineEvents(), applyInstanceEvent(), createEngineEventBatcher(), EngineEventPayload, EngineEventScope, EngineEventType, matchesEngineEvent(), reconcileInstanceSnapshot() (+3 more)

### Community 70 - "call_errors.rs"
Cohesion: 0.12
Nodes (4): call_activity_propagates_error_to_matching_boundary_event(), call_activity_propagates_error_to_wildcard_boundary_event(), call_activity_unhandled_error_becomes_incident(), build_child_error_process()

### Community 71 - "pending.rs"
Cohesion: 0.49
Nodes (5): PendingMessageCatch, PendingServiceTask, PendingTimer, PendingUserTask, ServiceTaskLease

### Community 72 - "history/mod.rs"
Cohesion: 0.15
Nodes (10): format_file_human_text(), test_format_file_human_text_exactly_1mb(), test_format_file_human_text_kb(), test_format_file_human_text_mb(), test_truncate_value_for_diff_exactly_1024(), test_truncate_value_for_diff_exactly_128_array(), test_truncate_value_for_diff_large_array(), test_truncate_value_for_diff_long_string() (+2 more)

### Community 73 - "BPMNinja Desktop Skill"
Cohesion: 0.11
Nodes (17): BPMNinja Desktop Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Cross-Stack, Domain Rules & Patterns, Evolution Log, Key Configuration Files, React Frontend (+9 more)

### Community 74 - "BPMNinja Server Skill"
Cohesion: 0.11
Nodes (17): agent-orchestrator, agent-orchestrator, BPMNinja Server Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Domain Rules & Patterns, engine-server, engine-server (+9 more)

### Community 75 - "client.rs"
Cohesion: 0.13
Nodes (4): HandleCache, KV_BUCKETS, NatsPersistence, OBJECT_STORES

### Community 76 - "http_load.py"
Cohesion: 0.13
Nodes (7): Client, main(), percentile(), rss_reader(), run_phase(), worker(), workflow()

### Community 77 - "desktop-tauri — Gotchas"
Cohesion: 0.17
Nodes (12): ⚠️ API-Key headers (`BPMNINJA_API_KEY`), ⚠️ Custom properties (ConditionPropertiesProvider), desktop-tauri — Gotchas, ⚠️ Dialog accessibility (Radix UI), ⚠️ Identifier, CSP, no dummy updater, ⚠️ Playwright tests use specific selectors, ⚠️ Rerunning the desktop app in dev mode, ⚠️ SSE connection lifecycle (+4 more)

### Community 78 - "tauri.conf.json"
Cohesion: 0.11
Nodes (17): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+9 more)

### Community 80 - "ScriptPropertiesProvider.ts"
Cohesion: 0.43
Nodes (4): findListener(), ScriptEntry(), ScriptGroup(), ScriptPropertiesProvider

### Community 81 - "nats_bench.rs"
Cohesion: 0.18
Nodes (4): bench_nats(), nats_url(), sample_instance(), script_chain()

### Community 82 - "PendingTasksPage.tsx"
Cohesion: 0.11
Nodes (38): App(), Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle, Separator (+30 more)

### Community 83 - "api_helpers.rs"
Cohesion: 0.30
Nodes (11): api_delete(), api_get(), api_post(), api_post_no_body(), api_put(), auth_headers(), blank_key_sends_no_headers(), empty_key_sends_no_headers() (+3 more)

### Community 84 - "history.rs"
Cohesion: 0.28
Nodes (6): CompletedInstancesQuery, get_completed_instance(), get_instance_history(), get_instance_history_entry(), list_completed_instances(), ServerHistoryQuery

### Community 85 - "commands/instances.rs"
Cohesion: 0.33
Nodes (13): delete_instance(), get_completed_instance(), get_instance_details(), get_instance_history(), list_instances(), migrate_instance(), move_token(), query_completed_instances() (+5 more)

### Community 86 - "boundary.rs"
Cohesion: 0.25
Nodes (5): setup_boundary_events(), test_setup_boundary_message_event(), test_setup_boundary_no_events(), test_setup_boundary_timer_event(), WorkflowEngine

### Community 87 - "server/deploy.rs"
Cohesion: 0.29
Nodes (9): DefinitionInfo, delete_all_definitions(), delete_definition(), DeleteDefinitionQuery, deploy_definition(), DeployRequest, DeployResponse, get_definition_xml() (+1 more)

### Community 88 - "server/events.rs"
Cohesion: 0.25
Nodes (5): engine_events(), event_stream(), lag_resync_discards_stale_backlog(), resync_event(), sse_transmits_entity_payload()

### Community 89 - "CLAUDE.md — BPMNinja Project Guide"
Cohesion: 0.12
Nodes (16): Agenten-System, Architektur-Regeln, CLAUDE.md — BPMNinja Project Guide, Claude Workflow (unbedingt einhalten), Coding-Konventionen, Git & Release Workflow, graphify, KI-Verhaltensrichtlinien (Behavioral Guidelines) (+8 more)

### Community 90 - "bpmninja — Architecture Documentation"
Cohesion: 0.12
Nodes (17): 10. Code Statistics, 1. Workspace Overview, 5.1 WorkflowPersistence Trait, 5.3 Fault-Tolerant Retry Queue (K6), 5. Persistence Architecture, 6.1 Route Overview (46 endpoints), 6.2 Server Architecture, 6.3 Background Timer Scheduler (+9 more)

### Community 91 - "super"
Cohesion: 0.09
Nodes (10): boundary_timer_persists_and_restores(), user_task_with_boundary_timer(), child_process(), linear_call_activity_child_complete_parent_completes(), parallel_call_activities_both_children_complete_parent(), concurrent_saves_persist_the_latest_state(), inline_retries_reuse_the_checked_json_snapshot(), instance_stays_readable_while_save_is_in_flight() (+2 more)

### Community 92 - "scripting.rs"
Cohesion: 0.12
Nodes (7): build_script_test_definition(), setup_migration_engine_same_ids(), migrate_instance_same_node_ids_succeeds(), migrate_instance_suspended_rejected(), migrate_instance_unknown_target_definition_fails(), script_mutates_state_and_executes_logic(), test_script_start_vs_end_listener_distinction()

### Community 94 - "BPMNinja Parser Skill"
Cohesion: 0.14
Nodes (13): BPMNinja Parser Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Domain Rules & Patterns, Evolution Log, Key Types, Relationship to engine-core, Scope & File Map (+5 more)

### Community 95 - "fuzz_history_diff.rs"
Cohesion: 0.48
Nodes (3): arb_instance_state(), arb_value(), arb_variables()

### Community 96 - "Component Responsibilities"
Cohesion: 0.12
Nodes (16): bpmn-parser, Component Responsibilities, Components, desktop-tauri, Element Handlers (engine-core/src/engine/handlers/), engine-server, Executor (engine-core/src/engine/executor/), external-task-client (+8 more)

### Community 97 - "desktop-tauri/index.md"
Cohesion: 0.14
Nodes (9): desktop-tauri, desktop-tauri — Interfaces, SSE Event Flow, Tauri Command Wrappers (lib/tauri.ts), UI Tab Structure, desktop-tauri — Responsibilities, Invariants, Key Frontend Components (+1 more)

### Community 98 - "persistence_ops.rs"
Cohesion: 0.14
Nodes (5): InstancePayload, payload_accepts_exact_limit_and_rejects_the_next_byte(), PERSIST_LOCK_STRIPES, PersistLocks, serialization_bounds_escaped_json_bytes()

### Community 99 - "e2e_call_activity.rs"
Cohesion: 0.32
Nodes (12): call_activity_eltern_abschluss_nach_kind(), call_activity_eltern_wartet_auf_kind(), call_activity_kind_ist_unterinstanz(), call_activity_mit_kind_service_task(), call_activity_variablen_propagation(), CHILD_BPMN, CHILD_SERVICE_BPMN, deploy() (+4 more)

### Community 100 - "value"
Cohesion: 0.18
Nodes (6): deploy_and_execute_parallel_gateway(), PARALLEL_XML, start_server(), start_server(), test_update_variables_mid_execution_persists(), USER_TASK_BPMN_XML

### Community 101 - "BPMNinja — Priorisierte Verbesserungen"
Cohesion: 0.13
Nodes (15): Appendix — Größen-Snapshot, BPMNinja — Priorisierte Verbesserungen, Details, Executive Summary, Kritische Issues (Kurzreferenz), Medium-term Improvements, Performance-Hotspots, Quick Wins (+7 more)

### Community 102 - "commands/monitoring.rs"
Cohesion: 0.36
Nodes (8): get_api_key(), get_api_url(), get_bucket_entries(), get_bucket_entry_detail(), get_log_entries(), get_monitoring_data(), set_api_key(), set_api_url()

### Community 103 - "Shared Patterns"
Cohesion: 0.20
Nodes (10): Broadcast Events (SSE Push), Camunda-Compatible Service Task API, Fail-Closed Durability (Server), Fault-Tolerant Persistence Retry, Fine-Grained Per-Instance Locking, Hexagonal Architecture (Ports & Adapters), Lock-Free Wait-State Queues, Shared Patterns (+2 more)

### Community 104 - "e2e_lifecycle.rs"
Cohesion: 0.27
Nodes (11): correlate_message_with_no_match_returns_empty(), delete_definition_cascade_removes_instances(), delete_definition_without_cascade_with_instances_returns_409(), delete_instance_returns_204(), deploy_and_start(), get_unknown_instance_returns_404(), _MINIMAL_BPMN, _MSG_CATCH_BPMN (+3 more)

### Community 105 - "e2e_migration.rs"
Cohesion: 0.36
Nodes (11): BPMN_V1, BPMN_V2_COMPAT, BPMN_V2_RENAMED, deploy(), deploy_and_start(), migrate_instance_same_node_ids_returns_204(), migrate_instance_unknown_target_returns_404(), migrate_instance_updates_definition_key() (+3 more)

### Community 106 - "e2e_service_tasks.rs"
Cohesion: 0.26
Nodes (11): bpmn_error_routes_to_boundary_event(), complete_nonexistent_task_returns_404(), complete_wrong_worker_returns_conflict(), ERROR_BOUNDARY_BPMN, extend_lock_succeeds(), extend_lock_wrong_worker_returns_conflict(), fail_service_task_decrements_retries(), get_service_tasks_returns_list() (+3 more)

### Community 107 - "BPMNinja Engine Skill"
Cohesion: 0.17
Nodes (11): BPMNinja Engine Skill, Co-Evolutionary Verification (EvoSkills, arXiv 2604.01687), Common Pitfalls, Domain Rules & Patterns, Evolution Log, Scope & File Map, Step 1 – Generate, Step 2 – Surrogate Verification (Self-Critique) (+3 more)

### Community 108 - "engine_bench.rs"
Cohesion: 0.33
Nodes (8): bench_condition(), bench_execution_throughput(), bench_gateways_and_scripts(), drain_service_tasks(), linear_definition(), parallel_definition(), script_chain_definition(), xor_definition()

### Community 109 - "`ExternalTaskClient`"
Cohesion: 0.17
Nodes (12): API Reference, `bpmnError(errorCode: string): Promise<void>`, `complete(variables?: Record<string, unknown>): Promise<void>`, `constructor(config?: ClientConfig)`, `extendLock(additionalDurationMs: number): Promise<void>`, `ExternalTaskClient`, `failure(errorMessage: string, errorDetails?: string, retries?: number): Promise<void>`, `start(): void` (+4 more)

### Community 110 - "@bpmninja/external-task-client"
Cohesion: 0.17
Nodes (12): @bpmninja/external-task-client, Configuration, Example, Features, Graceful Shutdown, Installation, License, Per-topic override: (+4 more)

### Community 111 - "ExternalTaskClient.test.ts"
Cohesion: 0.18
Nodes (11): Common Pitfalls, Critical Implementation Details, client, shutdown(), stopClientAndFlush(), createMockTask(), createMockFetch(), mockFetchResponse (+3 more)

### Community 112 - "ConditionPropertiesProvider.ts"
Cohesion: 0.35
Nodes (9): ConditionTypeEntry(), CustomConditionGroup(), CustomPropertiesProvider, ExpressionEntry(), getConditionType(), isDefaultFlow(), SCRIPT_LANGUAGES, ScriptBodyEntry() (+1 more)

### Community 115 - "codec.rs"
Cohesion: 0.14
Nodes (22): audit_key(), CompactBuffer, decode(), decode_instance_record(), detached(), detached_records_keep_audit_bytes_out_of_the_document(), DetachedInstance, encode_compact() (+14 more)

### Community 117 - "tests/topic_index.rs"
Cohesion: 0.18
Nodes (9): complete_removes_task_from_topic_index(), deploy_single_task(), deploy_two_topic_instance(), fetch_only_requested_topic_leaves_other_index_untouched(), long_poll_wakes_when_failed_task_is_unlocked(), long_poll_wakes_when_task_is_created(), NO_FALLBACK, spawn_long_poll() (+1 more)

### Community 118 - "assert-typescript-peer.mjs"
Cohesion: 0.22
Nodes (7): cmp(), declaredNumeric, lock, parseVer(), pkg, root, satisfies()

### Community 119 - "Key Architectural Decisions"
Cohesion: 0.18
Nodes (11): ADR-001: Hexagonal Architecture (Ports & Adapters), ADR-002: Token-Based Execution Model, ADR-003: Sub-Process Flattening at Parse Time, ADR-004: Lock-Free Concurrency via DashMap, ADR-005: Camunda-Compatible Service Task API, ADR-006: SSE Push Events for UI, ADR-007: Two-Stage Persistence Retry, ADR-008: Rhai Script Engine (not JavaScript) (+3 more)

### Community 121 - "commands/tasks.rs"
Cohesion: 0.42
Nodes (9): complete_service_task(), complete_task(), fetch_and_lock_service_tasks(), get_pending_message_catches(), get_pending_service_tasks(), get_pending_tasks(), get_pending_timers(), resolve_incident() (+1 more)

### Community 122 - "engine/topic_index.rs"
Cohesion: 0.31
Nodes (4): candidate_ids(), insert(), remove(), WorkflowEngine

### Community 123 - "instance_pagination.rs"
Cohesion: 0.38
Nodes (5): deploy_start_end(), dummy_instance(), list_instances_page_offset_limit_clones_slice(), list_instances_page_sorts_started_at_desc_then_id(), list_instances_without_limit_returns_all()

### Community 124 - "api-spec/index.md"
Cohesion: 0.20
Nodes (7): api-spec, api-spec — Interfaces, Deployed Artifacts, TypeSpec Source, api-spec — Responsibilities, Regeneration, What It Owns

### Community 125 - "BPMN 2.0.2 Element Matrix"
Cohesion: 0.20
Nodes (9): Artifacts, BPMN 2.0.2 Element Matrix, Connecting Objects, Data, Flow Objects — Activities, Flow Objects — Events, Flow Objects — Gateways, Maintenance (+1 more)

### Community 126 - "variables.rs"
Cohesion: 0.36
Nodes (6): decode_variables(), encode_variable(), inferred(), matches(), stale_hints_fall_back_and_double_values_use_floating_json(), TYPES

### Community 127 - "InstanceDetailDialog.tsx"
Cohesion: 0.17
Nodes (18): TabsContent, TabsList, TabsTrigger, InstanceDetailDialog(), InstanceDetailHeader(), HistoryTab(), HistoryTabProps, extractEmoji() (+10 more)

### Community 128 - "bpmn_compliance.rs"
Cohesion: 0.52
Nodes (5): complete_task_for_node(), compliance_complex_gateway_activation(), compliance_exclusive_gateway_routing(), compliance_parallel_gateway_sync(), create_engine()

### Community 129 - "e2e_suspend_resume.rs"
Cohesion: 0.42
Nodes (8): deploy_and_start(), move_token_to_existing_node_returns_204(), move_token_to_nonexistent_node_returns_404(), resume_suspended_instance_returns_204(), start_server(), suspend_instance_returns_204(), suspend_reflects_suspended_state(), TWO_TASK_BPMN

### Community 130 - "camunda/tests.rs"
Cohesion: 0.28
Nodes (10): bpmn_error_variables_arrive_on_the_boundary_path(), CHAIN, each_topic_uses_its_own_millisecond_lock_duration(), failure_honors_retry_timeout(), fetch(), lifecycle_preserves_typed_variables_and_camel_case_metadata(), malformed_requests_and_auth_use_camunda_error_envelopes(), rejects_unsupported_options_before_locking_or_completing() (+2 more)

### Community 131 - "Test Metrics"
Cohesion: 0.20
Nodes (10): Benchmarks, bpmn-parser Tests (46 tests), Code Statistics, Continuous Fuzzing, desktop-tauri E2E Tests (52 tests, 1 file), engine-core Breakdown (264 tests), engine-server E2E Tests (74 tests, 19 files), Mutation Score (+2 more)

### Community 132 - "observability.rs"
Cohesion: 0.26
Nodes (4): http_metrics_middleware(), install_prometheus_recorder(), metrics_handler(), normalize_path()

### Community 133 - "commands/deploy.rs"
Cohesion: 0.53
Nodes (6): delete_all_definitions(), delete_definition(), deploy_definition(), deploy_simple_process(), get_definition_xml(), list_definitions()

### Community 134 - "Data Flows"
Cohesion: 0.22
Nodes (9): 1. Deployment Flow, 2. Instance Execution Flow, 3. Service Task (Fetch-and-Lock), 4. Timer Processing, 5. SSE Event Push, 6. Startup Restore Flow, 7. Health vs Readiness, 8. Persistence Retry (Fault-Tolerant) (+1 more)

### Community 135 - "Public API Surface"
Cohesion: 0.20
Nodes (10): Core Structs (re-exported via `engine-core::*`), EngineEvent (broadcast channel), Error Types, History Types, In-Process-Task-Handler (`engine/task_handler.rs`), Persistence Trait, Public API Surface, Public Functions (+2 more)

### Community 136 - "Overview"
Cohesion: 0.22
Nodes (9): Architecture Summary, Boundaries & Invariants, Key Design Decisions, Overview, Production Hardening (env), Project Mission, Tech Stack, Test Coverage (+1 more)

### Community 137 - "history_batch.rs"
Cohesion: 0.47
Nodes (6): batch_history_is_written_in_one_call(), engine_with(), history_is_flushed_when_the_batch_fails(), history_of(), no_history_work_without_persistence(), script_chain()

### Community 138 - "e2e_file_variables.rs"
Cohesion: 0.50
Nodes (7): BPMN_USER_TASK, deploy_and_start(), file_variable_survives_task_completion(), file_variable_upload_creates_reference(), multiple_file_variables_and_delete_one(), start_server(), upload_file()

### Community 139 - "scripts"
Cohesion: 0.22
Nodes (9): scripts, build, dev, lint, preview, tauri, test:deps, test:e2e (+1 more)

### Community 140 - "app.spec.ts"
Cohesion: 0.29
Nodes (5): DEFAULT_MOCK_STATE, injectTauriMock(), loadXmlAndClickElement(), MockState, @playwright/test

### Community 141 - "AssigneePropertiesProvider.ts"
Cohesion: 0.32
Nodes (5): AssigneePropertiesProvider, AssigneeProps(), CustomAssigneeGroup(), ⚠️ bpmn-js requires Camunda moddle, @bpmn-io/properties-panel

### Community 142 - "CalledElementPropertiesProvider.ts"
Cohesion: 0.38
Nodes (4): CalledElementGroup(), CalledElementPropertiesProvider, CalledElementProps(), bpmn-js-properties-panel

### Community 144 - "Dependencies"
Cohesion: 0.25
Nodes (8): bpmn-parser, Crate-Level Dependency Graph, Dependencies, Detailed Dependency Table, engine-core, engine-server, Inbound Dependencies (Who Calls Me), Key External Dependencies

### Community 145 - "tests/task_handler.rs"
Cohesion: 0.18
Nodes (17): TaskOutcome, BpmnError, Complete, Fail, FnHandler, FnHandler<F>, handler(), handler_bpmn_error_follows_error_boundary() (+9 more)

### Community 146 - "InstanceState"
Cohesion: 0.17
Nodes (12): InstanceState, Completed, CompletedWithError, ParallelExecution, Running, Suspended, WaitingOnCallActivity, WaitingOnEventBasedGateway (+4 more)

### Community 147 - "HistoryEntry"
Cohesion: 0.18
Nodes (5): HistoryDiff, HistoryEntry, test_with_diff_empty_does_not_set(), test_with_diff_non_empty_sets_description(), VariableDiff

### Community 148 - "timers.rs"
Cohesion: 0.54
Nodes (3): get_pending_timers(), process_timers(), ProcessTimersResponse

### Community 149 - "json"
Cohesion: 0.31
Nodes (8): find_incoming(), load_json(), main(), merge_metrics(), mutation_has_signal(), patch_readme(), render_readme_block(), write_badges()

### Community 150 - "persistence-nats/src/tests.rs"
Cohesion: 0.26
Nodes (11): RestoreStats, connect_nats(), nats_required(), nats_url(), setup_nats_test(), test_definition_instance_restore_roundtrip(), test_history_append_and_load(), test_history_batch_is_ordered_and_deduplicated() (+3 more)

### Community 151 - "server/monitoring.rs"
Cohesion: 0.32
Nodes (8): BackendInfo, BucketEntriesQuery, get_backend_info(), get_bucket_entries(), get_bucket_entry_detail(), get_monitoring_data(), MonitoringData, ready_endpoint()

### Community 152 - "BPMNinja Desktop (Tauri + React + bpmn-js)"
Cohesion: 0.29
Nodes (6): BPMNinja Desktop (Tauri + React + bpmn-js), Entwicklung, Flow Conditions (Camunda-7-kompatibel), Qualitätssicherung, Voraussetzungen, Zentrale Features

### Community 153 - "README.md"
Cohesion: 0.19
Nodes (7): Camunda-7-Worker-Fassade, Failure, Lock-Verlängerung und BPMN Error, Fetch und Completion, Grenzen und Authentifizierung, Desktop-Verhalten, SSE-Zustandsupdates, Verlust und Wiederverbindung

### Community 155 - "Maintenance"
Cohesion: 0.29
Nodes (6): Content Quality Rules, File Structure Convention, How to Update, Last bulk sync, Maintenance, When to Update This Knowledge Base

### Community 156 - "engine-core/index.md"
Cohesion: 0.25
Nodes (3): engine-core — Gotchas, engine-core, engine-core — Interfaces

### Community 157 - "REST API"
Cohesion: 0.18
Nodes (11): Definitions, Error Handling, Events & Messages, Files, History & Archival, Instances, Monitoring & Health, Observability (+3 more)

### Community 158 - "BPMNinja"
Cohesion: 0.14
Nodes (14): Architecture, BPMNinja, Build, Test & Lint, Contributors, Crates (Modules), Docker Compose, License, Overview (+6 more)

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

### Community 163 - "e2e_versioning.rs"
Cohesion: 0.43
Nodes (6): BPMN_V1, BPMN_V2, deploy_same_bpmn_id_increments_version(), instances_stay_on_original_version(), start_latest_uses_newest_version(), start_server()

### Community 164 - "Task Memory Protocol & Context Resets"
Cohesion: 0.33
Nodes (5): Abschluss, Der `/compact` Re-Entry Workflow, Task Memory Protocol & Context Resets, Wann wird `TASK_MEMORY.md` im Projekt-Root angelegt?, Zwingender Inhalt

### Community 165 - "Dependency Management"
Cohesion: 0.33
Nodes (5): Adding New Dependencies, Dependency Management, Feature Flags, Forbidden, Workspace Dependencies (mandatory)

### Community 166 - "Server Agent"
Cohesion: 0.33
Nodes (5): API Conventions, Architecture, Middleware, Rules, Server Agent

### Community 167 - "NATS Message Broker Rules (BPMNinja Project)"
Cohesion: 0.33
Nodes (5): 1. Technology Stack (fixed), 2. NATS Feature Split (NEVER change this!), 3. Important Principles, 4. Forbidden, NATS Message Broker Rules (BPMNinja Project)

### Community 168 - "UI/Desktop Agent (Tauri Rust Backend)"
Cohesion: 0.33
Nodes (5): AppState, Key File: `src-tauri/src/main.rs`, Rules, Tauri Commands (all async, all take `State<AppState>`), UI/Desktop Agent (Tauri Rust Backend)

### Community 169 - "knowledge-base/index.md"
Cohesion: 0.25
Nodes (3): Architecture, Quick Reference, Tag Registry

### Community 170 - "log_nats.rs"
Cohesion: 0.19
Nodes (4): MAX_MESSAGES, NatsLogSink, STREAM_NAME, SUBJECT

### Community 171 - "commands/files.rs"
Cohesion: 0.67
Nodes (3): delete_instance_file(), download_instance_file(), upload_instance_file()

### Community 172 - "desktop-tauri/tsconfig.json"
Cohesion: 0.33
Nodes (5): compilerOptions, baseUrl, paths, files, references

### Community 173 - "LoadTest"
Cohesion: 0.24
Nodes (4): LoadTest, do_GET(), do_POST(), respond()

### Community 174 - "engine-server — Interfaces"
Cohesion: 0.20
Nodes (10): AppError → HTTP Status Mapping, AppState (shared state for all route handlers), Auth & CORS env, Builder Functions, engine-server — Interfaces, Health vs Readiness, HTTP lock duration, Log Buffer Query (+2 more)

### Community 175 - "server/messages.rs"
Cohesion: 0.47
Nodes (4): correlate_message(), CorrelateMessageRequest, CorrelateMessageResponse, get_pending_messages()

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

### Community 181 - "TopicPropertiesProvider.ts"
Cohesion: 0.47
Nodes (3): CustomTopicGroup(), TopicPropertiesProvider, TopicProps()

### Community 183 - "BPMN Model Checking and State-Space Reduction"
Cohesion: 0.17
Nodes (10): Cross-Cutting, BPMN Model Checking and State-Space Reduction, BPMN verification and reduction, BPMN Verification Pipeline, BPMNinja Implementation, Other State-Space Techniques, Partial order reduction, Partial Order Reduction (POR) (+2 more)

### Community 185 - "package.json"
Cohesion: 0.50
Nodes (3): dependencies, @typespec/compiler, @typespec/compiler

### Community 186 - "fuzz/index.md"
Cohesion: 0.40
Nodes (3): fuzz, fuzz — Responsibilities, What It Owns

### Community 187 - "Supported BPMN Elements"
Cohesion: 0.25
Nodes (8): Activities & Sub-Processes, Additional Concepts, Basic Elements, Currently Unsupported BPMN Elements, Deviations from the BPMN 2.0 Standard, Gateways, Intermediate Events, Supported BPMN Elements

### Community 188 - "4. Execution Architecture"
Cohesion: 0.50
Nodes (4): 4.1 Token Lifecycle (K4), 4.2 Execution Loop (run_instance_batch), 4.3 Gateway Routing, 4. Execution Architecture

### Community 189 - "InstanceStatus"
Cohesion: 0.17
Nodes (12): InstanceStatus, Completed, CompletedWithError, ParallelExecution, Running, Suspended, WaitingOnCallActivity, WaitingOnEventBasedGateway (+4 more)

### Community 192 - "Global Agent Directives & Workflow"
Cohesion: 0.50
Nodes (3): Global Agent Directives & Workflow, Priorität bei Regelkonflikten, Workflow

### Community 193 - "Rust Best Practices for BPMNinja"
Cohesion: 0.50
Nodes (3): Anti-Patterns (NEVER do), Must-Follow Rust Rules, Rust Best Practices for BPMNinja

### Community 194 - "Versioning Convention"
Cohesion: 0.50
Nodes (3): Files to Update, Rules, Versioning Convention

### Community 195 - "vite.config.ts"
Cohesion: 0.40
Nodes (3): @tailwindcss/vite, vite, @vitejs/plugin-react

### Community 196 - "Server allocator and release optimization experiments"
Cohesion: 0.33
Nodes (6): Compare allocators and LTO, Evaluate profile-guided optimization on Linux, HTTP-/NATS-Vergleich mit RSS und Tail-Latenz, Initial local allocator result (2026-10-13), Lokaler HTTP-/NATS-Vergleich (2026-10-13), Server allocator and release optimization experiments

### Community 199 - "desktop-tauri — Dependencies"
Cohesion: 0.40
Nodes (5): Architecture Diagram, desktop-tauri — Dependencies, Inbound, Key npm Dependencies, Outbound

### Community 203 - "ProcessDefinition"
Cohesion: 0.12
Nodes (9): ProcessDefinition, find_any_boundary_escalation_event(), find_boundary_error_event(), find_boundary_escalation_event(), find_compensation_handler(), find_downstream_join(), resolve_next_target(), same_gateway_type() (+1 more)

### Community 231 - "calculate_diff"
Cohesion: 0.27
Nodes (9): calculate_diff(), calculate_diff_from_snapshot(), DiffSnapshot, test_calculate_diff(), test_calculate_diff_empty(), test_calculate_diff_file_upload_human_text(), test_calculate_diff_only_removed(), test_calculate_diff_only_variables_changed() (+1 more)

### Community 232 - "e2e_stress.rs"
Cohesion: 0.83
Nodes (3): start_server(), stress_http_concurrent_deployments(), stress_http_concurrent_starts()

### Community 233 - "e2e_deploy.rs"
Cohesion: 0.43
Nodes (5): deploy_and_start_minimal_workflow_completes_immediately(), deploy_invalid_xml_returns_400(), deploy_minimal_workflow_returns_definition_id(), MINIMAL_BPMN_XML, start_server()

### Community 234 - "ExternalTaskClient"
Cohesion: 0.09
Nodes (18): ExternalTaskClient, Module Quick Index, agent-orchestrator, Route Map (46 endpoints), ⚠️ `bpmnError()` is not retried, ⚠️ Default-Port ist 8081, ⚠️ ESM only, ⚠️ Example worker path (+10 more)

### Community 237 - "constants.rs"
Cohesion: 0.20
Nodes (6): MAX_AUDIT_LOG_ENTRIES, MAX_EXECUTION_STEPS, MAX_INSTANCE_PAYLOAD_BYTES, YIELD_EVERY_N_STEPS, EngineStats, InstancePage

### Community 239 - "retry.ts"
Cohesion: 0.53
Nodes (7): When to Activate, calculateBackoff(), RetryResult, sleep(), withRetry(), Key Files, bpmn-ninja-external-task-client Tests (76 tests, 3 files)

### Community 240 - "Empfohlener Aktionsplan (phasiert)"
Cohesion: 0.40
Nodes (5): Empfohlener Aktionsplan (phasiert), Phase 0 — Sofort (1–3 Tage) — ✅ erledigt (2026-07), Phase 1 — Production Minimum Bar (1–2 Wochen) — ✅ erledigt, Phase 2 — Scale & Maintainability (2–6 Wochen), Phase 3 — Platform (Quartal+)

### Community 241 - "Desktop Application (UI)"
Cohesion: 0.40
Nodes (5): 1. Run the backend prerequisites, 2. Install the desktop app binary release, 3. Running from source (for developers), Desktop Application (UI), Generate desktop packages with CI

### Community 242 - "External Task Client (TypeScript)"
Cohesion: 0.40
Nodes (5): Development & Tests, External Task Client (TypeScript), Features, Quick Start, TaskService (per handler)

### Community 243 - "camunda_compat.rs"
Cohesion: 0.53
Nodes (4): legacy_snapshots_default_type_hints_and_native_updates_clear_them(), retry_deadline_survives_restore_and_blocks_fetch_until_expiry(), service(), suspended_instances_are_not_fetchable_and_millisecond_extension_is_exact()

### Community 244 - "AppError"
Cohesion: 0.25
Nodes (5): AppError, BadRequest, Engine, PayloadTooLarge, Unauthorized

### Community 245 - "2. engine-core — Core Architecture"
Cohesion: 0.67
Nodes (3): 2.1 Module Structure, 2.2 WorkflowEngine — Component Breakdown (K2), 2. engine-core — Core Architecture

### Community 246 - "3. Data Model"
Cohesion: 0.67
Nodes (3): 3.1 ProcessInstance (after K4 refactoring), 3.2 BPMN Element Types, 3. Data Model

### Community 247 - "Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)"
Cohesion: 0.25
Nodes (8): 4a — Engine-Performance (Hot Path), 4b — Fundament für Verteilung (Voraussetzung für 4c–4e), 4c — Native Messaging: NATS, Redpanda, Kafka, 4d — Microservices & Dapr, 4e — Serverless & Service Mesh, 4f — BPMN-2.0.2-Vollabdeckung (Vorschlag, 2026-10-12), Empfohlene Reihenfolge, Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)

### Community 250 - "BPMNinja Knowledge Base"
Cohesion: 0.40
Nodes (5): BPMNinja Knowledge Base, Cross-Cutting Topics, Directory Map, Key Design Principles, Quick Start for Agents

### Community 251 - "engine-core — Dependencies"
Cohesion: 0.40
Nodes (4): engine-core — Dependencies, Key Cargo.toml Excerpt, Outbound (What engine-core Depends On), Trait Relationship Diagram

### Community 254 - "8. Concurrency & Locking (K1)"
Cohesion: 0.67
Nodes (3): 8.1 Lock Hierarchy, 8.2 Deadlock Prevention Pattern, 8. Concurrency & Locking (K1)

### Community 279 - "ActorType"
Cohesion: 0.33
Nodes (6): ActorType, Engine, Listener, ServiceWorker, Timer, User

### Community 289 - "EngineEvent"
Cohesion: 0.19
Nodes (5): EngineEvent, DefinitionChanged, InstanceChanged, TaskChanged, InstanceDelta

## Knowledge Gaps
- **191 isolated node(s):** `agent-orchestrator`, `@typespec/compiler`, `@typespec/http`, `@typespec/rest`, `@typespec/openapi3` (+186 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 1581 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **64 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Value` connect `Value` to `bpmn_compliance.rs`, `hashmap`, `camunda/tests.rs`, `FailSaveXmlPersistence`, `commands/deploy.rs`, `camunda.rs`, `model_check.rs`, `InstrumentedPersistence`, `server/instances.rs`, `Token`, `tests/task_handler.rs`, `tests/gateways.rs`, `HistoryEntry`, `.persist_instance`, `condition.rs`, `sse_consumer.rs`, `instance.rs`, `server/tasks.rs`, `Uuid`, `.handle_bpmn_error_with_variables`, `commands/files.rs`, `runner.rs`, `server/messages.rs`, `serialize`, `FuzzValue`, `pending.rs`, `history/mod.rs`, `ProcessDefinition`, `.record_history_event`, `api_helpers.rs`, `commands/instances.rs`, `fuzz_history_diff.rs`, `inclusive_join.rs`, `commands/monitoring.rs`, `calculate_diff`, `commands/tasks.rs`, `correlate_message`, `variables.rs`?**
  _High betweenness centrality (0.076) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `cn()` (e.g. with `CSS & Theming Conventions` and `Key Files`) actually correct?**
  _`cn()` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `agent-orchestrator`, `@typespec/compiler`, `@typespec/http` to the rest of the system?**
  _191 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `NatsPersistence` be split into smaller, more focused modules?**
  _Cohesion score 0.12956810631229235 - nodes in this community are weakly interconnected._
- **Why does `ProcessDefinition` connect `ProcessDefinition` to `NatsPersistence`, `InMemoryPersistence`, `hashmap`, `FailSaveXmlPersistence`, `parse_bpmn_xml`, `WorkflowPersistence`, `history_batch.rs`, `InstrumentedPersistence`, `stress_tests.rs`, `Token`, `tests/gateways.rs`, `tests/task_handler.rs`, `persistence-nats/src/tests.rs`, `condition.rs`, `BpmnElement`, `runner.rs`, `parser.rs`, `Value`, `.list_kv_entries`, `serialize`, `ProcessDefinitionBuilder`, `call_errors.rs`, `.record_history_event`, `nats_bench.rs`, `boundary.rs`, `super`, `WorkflowEngine`, `scripting.rs`, `handlers/tasks.rs`, `completion.rs`, `engine_bench.rs`, `StartupCoordinator`, `in_memory.rs`, `inclusive_join.rs`?**
  _High betweenness centrality (0.035) - this node is a cross-community bridge._
- **Should `InMemoryPersistence` be split into smaller, more focused modules?**
  _Cohesion score 0.13911290322580644 - nodes in this community are weakly interconnected._
- **Why does `ProcessInstance` connect `tauri.ts` to `MonitoringPage.tsx`, `useEngineEvents`, `Shared Patterns`, `cn`, `Button`, `Critical Rules`, `Key Architectural Decisions`, `README.md`, `react`, `InstanceDetailDialog.tsx`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._