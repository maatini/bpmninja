import "@typespec/http";
import "@typespec/rest";
import "@typespec/openapi3";

using TypeSpec.Http;
using TypeSpec.Rest;
using TypeSpec.OpenAPI;

@service(#{
  title: "bpmninja Engine Server API"
})
@server("http://localhost:8081", "Local Engine Server")
@doc("""
Optional API-key auth when env BPMNINJA_API_KEY is set.
Send `Authorization: Bearer <key>` or `X-API-Key: <key>`.
GET /api/health and GET /api/ready remain public. lockDuration/newDuration are milliseconds.
""")
@useAuth(BearerAuth | ApiKeyAuth<ApiKeyLocation.header, "X-API-Key"> | NoAuth)
namespace MiniBpm;

// --------------------------------------------------------------------------
// Models
// --------------------------------------------------------------------------

@error
model AppError {
  error: string;
}

model VariableValue {
  type: string;
  value?: unknown;
  filename?: string;
  size_bytes?: int64;
  content_type?: string;
}

@doc("Lifecycle state. Terminal states are `Completed`, `CompletedWithError`, and `Terminated`. Waiting variants carry additional payloads in the JSON object form.")
model ProcessInstance {
  id: string;
  definition_key: string;
  business_key?: string;
  parent_instance_id?: string;
  state: unknown;
  current_node?: string;
  variables: Record<unknown>;
  @doc("Optional transport type hints; absent from legacy/native-only snapshots.")
  variable_types?: Record<string>;
  started_at?: string;
  completed_at?: string;
}

model DeployRequest {
  @doc("BPMN 2.0 XML")
  xml: string;
  name: string;
}

model ModelCheckViolation {
  property: "safeness" | "option_to_complete" | "proper_completion" | "no_dead_activities";
  elements?: string[];
}

model ModelCheckReport {
  status: "passed" | "violations" | "skipped" | "inconclusive";
  violations?: ModelCheckViolation[];
  reason?: string;
}

model DeployResponse {
  definition_key: string;
  version: int32;
  model_check?: ModelCheckReport;
}

model StartRequest {
  definition_key: string;
  business_key?: string;
  variables?: Record<unknown>;
}

model StartLatestRequest {
  bpmn_id: string;
  business_key?: string;
  variables?: Record<unknown>;
}

model StartResponse {
  instance_id: string;
}

model CompleteRequest {
  variables?: Record<unknown>;
}

model MoveTokenRequest {
  target_node_id: string;
}

model MigrateInstanceRequest {
  /** UUID of the target ProcessDefinition. */
  target_definition_key: string;
  /** Optional mapping of old BPMN node IDs to new BPMN node IDs. Required for nodes that were renamed between versions. */
  node_mapping?: Record<string>;
}

model PendingUserTask {
  id: string;
  instance_id: string;
  node_id: string;
  state: "Pending" | "Completed";
  created_at: string;
}

model PendingServiceTask {
  id: string;
  instance_id: string;
  definition_key?: string;
  node_id: string;
  topic: string;
  state: string;
  locked_by?: string;
  lock_expires_at?: string;
  variables: Record<VariableValue>;
  retries?: int32;
  error_message?: string;
}

model TopicRequest {
  topicName: string;
  /** Lock duration in milliseconds (Camunda-compatible). Server converts to seconds for the engine. */
  lockDuration: int64;
}

model FetchAndLockRequest {
  workerId: string;
  maxTasks: int32;
  topics: TopicRequest[];
  asyncResponseTimeout?: int64;
}

model CompleteServiceTaskRequest {
  workerId: string;
  variables?: Record<unknown>;
}

model FailServiceTaskRequest {
  workerId: string;
  retries?: int32;
  errorMessage?: string;
  errorDetails?: string;
}

model ExtendLockRequest {
  workerId: string;
  /** New lock duration in milliseconds. Server converts to seconds for the engine. */
  newDuration: int64;
}

model BpmnErrorRequest {
  workerId: string;
  errorCode: string;
}

model DefinitionInfo {
  key: string;
  bpmn_id: string;
  version: int32;
  node_count: int32;
}

model CorrelateMessageRequest {
  messageName: string;
  businessKey?: string;
  variables?: Record<unknown>;
}

model CorrelateMessageResponse {
  affectedInstances: string[];
}

model BroadcastSignalRequest {
  signalName: string;
  variables?: Record<unknown>;
}

model BroadcastSignalResponse {
  @doc("Instances resumed by the signal plus instances started by signal start events.")
  affectedInstances: string[];
}

model ProcessTimersResponse {
  triggered: int32;
}

model PendingTimer {
  instance_id: string;
  node_id: string;
  fire_at: string;
}

model PendingMessage {
  instance_id: string;
  node_id: string;
  @doc("Message or signal name, depending on `kind`.")
  message_name: string;
  kind?: "Message" | "Signal";
}

model PendingConditionalCatch {
  kind?: "intermediate" | "boundary";
  activity_activation_id?: string;
  attached_to?: string;
  interrupting?: boolean;
  id: string;
  instance_id: string;
  node_id: string;
  token_id: string;
  generation: uint64;
  claimed: boolean;
}

model BackendInfo {
  backend_type: string;
  nats_url?: string;
  connected: boolean;
}

model StorageInfo {
  host: string;
  port: int32;
  messages: int32;
  bytes: int64;
}

model MonitoringData {
  definitions_count: int32;
  instances_total: int32;
  instances_running: int32;
  instances_completed: int32;
  pending_user_tasks: int32;
  pending_service_tasks: int32;
  storage_info?: StorageInfo;
}

model BucketEntry {
  key: string;
  size_bytes?: int64;
  revision?: int64;
  updated_at?: string;
}

model BucketEntryDetail {
  key: string;
  value: unknown;
}

model HistoryEntry {
  id: string;
  instance_id: string;
  node_id?: string;
  event_type: string;
  actor_type: string;
  timestamp: string;
  details: unknown;
}

model UpdateVariablesRequest {
  variables: Record<unknown>;
}

model UploadFileRequest {
  file: HttpPart<bytes>;
}

model LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

// --------------------------------------------------------------------------
// Endpoints
// --------------------------------------------------------------------------

@route("/api")
namespace Api {

  // ---- Deployment & lifecycle ----

  @post @route("/deploy")
  op deploy(@body request: DeployRequest): DeployResponse | AppError;

  @post @route("/start")
  op start(@body request: StartRequest): StartResponse | AppError;

  @post @route("/start/latest")
  @doc("Start an instance of the latest deployed version of a BPMN definition (by bpmn_id).")
  op startLatest(@body request: StartLatestRequest): StartResponse | AppError;

  @post @route("/start/timer")
  @doc("Start an instance whose start event carries a timerEventDefinition.")
  op startTimer(@body request: StartRequest): StartResponse | AppError;

  // ---- User tasks ----

  @get @route("/tasks")
  op getTasks(): PendingUserTask[] | AppError;

  @post @route("/complete/{id}")
  op completeTask(@path id: string, @body request: CompleteRequest): { @statusCode statusCode: 204 } | AppError;

  // ---- Active instances ----

  @get @route("/instances")
  @doc("List process instances as a JSON array. Paginate with `limit`/`offset`; total count is in the `X-Total-Count` header.")
  op listInstances(
    @query limit?: int32,
    @query offset?: int32
  ): {
    @header("X-Total-Count") xTotalCount: int32;
    @body body: ProcessInstance[];
  } | AppError;

  @get @route("/instances/{id}")
  @doc("Returns an instance by id. Falls back to the history archive if the instance is no longer active.")
  op getInstance(@path id: string): ProcessInstance | AppError;

  @delete @route("/instances/{id}")
  op deleteInstance(@path id: string): { @statusCode statusCode: 204 } | AppError;

  @put @route("/instances/{id}/variables")
  op updateInstanceVariables(@path id: string, @body request: UpdateVariablesRequest): { @statusCode statusCode: 204 } | AppError;

  @post @route("/instances/{id}/suspend")
  op suspendInstance(@path id: string): { @statusCode statusCode: 204 } | AppError;

  @post @route("/instances/{id}/resume")
  op resumeInstance(@path id: string): { @statusCode statusCode: 204 } | AppError;

  @post @route("/instances/{id}/move-token")
  op moveToken(@path id: string, @body request: MoveTokenRequest): { @statusCode statusCode: 204 } | AppError;

  /** Migrate a running instance to a different process definition version. */
  @post @route("/instances/{id}/migrate")
  op migrateInstance(@path id: string, @body request: MigrateInstanceRequest): { @statusCode statusCode: 204 } | AppError;

  // ---- Files (variable-attached) ----

  @post @route("/instances/{id}/files/{var_name}")
  op uploadInstanceFile(
    @path id: string,
    @path var_name: string,
    @header contentType: "multipart/form-data",
    @multipartBody body: UploadFileRequest
  ): { @statusCode statusCode: 201 } | AppError;

  @get @route("/instances/{id}/files/{var_name}")
  op getInstanceFile(@path id: string, @path var_name: string): bytes | AppError;

  @delete @route("/instances/{id}/files/{var_name}")
  op deleteInstanceFile(@path id: string, @path var_name: string): { @statusCode statusCode: 204 } | AppError;

  // ---- Per-instance history ----

  @get @route("/instances/{id}/history")
  op getInstanceHistory(
    @path id: string,
    @query event_types?: string,
    @query node_id?: string,
    @query actor_type?: string,
    @query from?: string,
    @query to?: string,
    @query limit?: int32,
    @query offset?: int32
  ): HistoryEntry[] | AppError;

  @get @route("/instances/{id}/history/{event_id}")
  op getInstanceHistoryEntry(@path id: string, @path event_id: string): HistoryEntry | AppError;

  // ---- Archived (completed) instances ----

  @get @route("/history/instances")
  @doc("Query archived (completed) process instances.")
  op listCompletedInstances(
    @query definition_key?: string,
    @query business_key?: string,
    @query from?: string,
    @query to?: string,
    @doc("Terminal-state filter: `completed` or `error`.")
    @query state?: "completed" | "error",
    @query limit?: int32,
    @query offset?: int32
  ): ProcessInstance[] | AppError;

  @get @route("/history/instances/{id}")
  op getCompletedInstance(@path id: string): ProcessInstance | AppError;

  // ---- Definitions ----

  @get @route("/definitions")
  op listDefinitions(): DefinitionInfo[] | AppError;

  @get @route("/definitions/{id}/xml")
  op getDefinitionXml(@path id: string): string | AppError;

  @delete @route("/definitions/{id}")
  op deleteDefinition(@path id: string, @query cascade?: boolean): { @statusCode statusCode: 204 } | AppError;

  @delete @route("/definitions/bpmn/{bpmn_id}")
  @doc("Delete all versions of a definition by its bpmn_id.")
  op deleteAllDefinitions(@path bpmn_id: string, @query cascade?: boolean): { @statusCode statusCode: 204 } | AppError;

  // ---- Messages & timers ----

  @post @route("/message")
  op correlateMessage(@body request: CorrelateMessageRequest): CorrelateMessageResponse | AppError;

  @post @route("/signal")
  @doc("Broadcast a signal to all waiting signal catch/boundary events and start processes with a matching signal start event.")
  op broadcastSignal(@body request: BroadcastSignalRequest): BroadcastSignalResponse | AppError;

  @get @route("/timers")
  op getPendingTimers(): PendingTimer[] | AppError;

  @post @route("/timers/process")
  op processTimers(): ProcessTimersResponse | AppError;

  @get @route("/messages")
  op getPendingMessages(): PendingMessage[] | AppError;

  @doc("Live conditional catch occurrences, without process variables.")
  @get @route("/conditional-catches")
  op getPendingConditionalCatches(): PendingConditionalCatch[] | AppError;

  // ---- Service tasks (External-Task-Worker-kompatibel) ----

  @get @route("/service-tasks")
  op getServiceTasks(): PendingServiceTask[] | AppError;

  @post @route("/service-task/fetchAndLock")
  op fetchAndLockServiceTasks(@body request: FetchAndLockRequest): PendingServiceTask[] | AppError;

  @post @route("/service-task/{id}/complete")
  op completeServiceTask(@path id: string, @body request: CompleteServiceTaskRequest): { @statusCode statusCode: 204 } | AppError;

  @post @route("/service-task/{id}/failure")
  op failServiceTask(@path id: string, @body request: FailServiceTaskRequest): { @statusCode statusCode: 204 } | AppError;

  @post @route("/service-task/{id}/retry")
  op retryIncident(@path id: string): { @statusCode statusCode: 204 } | AppError;

  @post @route("/service-task/{id}/resolve")
  op resolveIncident(@path id: string): { @statusCode statusCode: 204 } | AppError;

  @post @route("/service-task/{id}/extendLock")
  op extendLock(@path id: string, @body request: ExtendLockRequest): { @statusCode statusCode: 204 } | AppError;

  @post @route("/service-task/{id}/bpmnError")
  op bpmnError(@path id: string, @body request: BpmnErrorRequest): { @statusCode statusCode: 204 } | AppError;

  // ---- Monitoring & health ----

  @get @route("/info")
  op getInfo(): BackendInfo | AppError;

  @get @route("/monitoring")
  op getMonitoring(): MonitoringData | AppError;

  @get @route("/monitoring/buckets/{bucket}/entries")
  op getBucketEntries(@path bucket: string): BucketEntry[] | AppError;

  @get @route("/monitoring/buckets/{bucket}/entries/{key}")
  op getBucketEntryDetail(@path bucket: string, @path key: string): BucketEntryDetail | AppError;

  @useAuth(NoAuth)
  @get @route("/health")
  op health(): { @statusCode statusCode: 200 } | AppError;

  @useAuth(NoAuth)
  @get @route("/ready")
  op ready(): { @statusCode statusCode: 200 } | AppError;

  @get @route("/logs")
  @doc("Filtered snapshot of the in-memory log buffer (newest first, max 5000).")
  op getLogs(
    @query level?: string,
    @query search?: string,
    @query limit?: int32
  ): LogEntry[] | AppError;

  @get @route("/events")
  @doc("SSE stream of engine state changes (`instance_changed`, `task_changed`, `definition_changed`). Broadcast capacity is 256; slow clients may miss events.")
  op engineEvents(): {
    @header contentType: "text/event-stream";
    @body body: string;
  };
}

@get
@route("/metrics")
@doc("Prometheus text format. Mounted only when the Prometheus recorder is installed at process start.")
op metrics(): {
  @header contentType: "text/plain";
  @body body: string;
};

@route("/engine-rest/external-task")
@doc("Camunda 7 worker facade. Primitive/Date/Json variables only; Java objects, local scopes, tenant/definition filters and extension properties are rejected.")
namespace CamundaExternalTasks {
  model TypedVariable {
    type?: "Null" | "Boolean" | "Short" | "Integer" | "Long" | "Double" | "String" | "Date" | "Json";
    value?: unknown;
    @doc("Null/empty metadata, transient=false, and Json serializationDataFormat=application/json are supported.")
    valueInfo?: Record<unknown> | null;
  }
  model Topic {
    topicName: string;
    @minValue(1) lockDuration: int64;
    variables?: string[] | null;
    deserializeValues?: boolean;
    includeExtensionProperties?: false;
    localVariables?: false;
    withoutTenantId?: boolean;
  }
  model FetchRequest {
    workerId: string;
    @minValue(1) maxTasks: int32;
    topics?: Topic[];
    @maxValue(1800000) asyncResponseTimeout?: uint64 | null;
    @doc("Accepted; all service-task priorities are zero.")
    usePriority?: boolean;
  }
  model FetchedTask {
    id: string;
    activityId: string;
    activityInstanceId: string;
    processInstanceId: string;
    processDefinitionId: string;
    processDefinitionKey: string | null;
    processDefinitionVersionTag: null;
    topicName: string;
    workerId: string;
    lockExpirationTime: string;
    retries: int32;
    errorMessage: string | null;
    errorDetails: string | null;
    businessKey: string;
    tenantId: null;
    priority: 0;
    suspended: false;
    variables: Record<TypedVariable>;
  }
  model CompleteRequest {
    workerId: string;
    variables?: Record<TypedVariable> | null;
    @doc("Only null or an empty object is supported; there are no local scopes.")
    localVariables?: Record<TypedVariable> | null;
  }
  model FailureRequest {
    workerId: string;
    @minValue(0) retries?: int32 | null;
    @minValue(0) retryTimeout?: int64 | null;
    errorMessage?: string | null;
    errorDetails?: string | null;
  }
  model ExtendRequest { workerId: string; @minValue(1) newDuration: int64; }
  model BpmnErrorRequest {
    workerId: string;
    errorCode: string;
    errorMessage?: string | null;
    variables?: Record<TypedVariable> | null;
  }
  model ErrorBody { type: string; message: string; code: int32; }
  @error model ErrorResponse {
    @statusCode statusCode: 400 | 401 | 404 | 413 | 500;
    @body body: ErrorBody;
  }
  @post @route("/fetchAndLock")
  op fetchAndLock(@body request: FetchRequest): FetchedTask[] | ErrorResponse;
  @post @route("/{id}/complete")
  op complete(@path id: string, @body request: CompleteRequest): { @statusCode statusCode: 204 } | ErrorResponse;
  @post @route("/{id}/failure")
  op failure(@path id: string, @body request: FailureRequest): { @statusCode statusCode: 204 } | ErrorResponse;
  @post @route("/{id}/extendLock")
  op extendLock(@path id: string, @body request: ExtendRequest): { @statusCode statusCode: 204 } | ErrorResponse;
  @post @route("/{id}/bpmnError")
  op bpmnError(@path id: string, @body request: BpmnErrorRequest): { @statusCode statusCode: 204 } | ErrorResponse;
}
