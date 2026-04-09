/**
 * @module types
 *
 * Core type definitions for the BPMNinja External Task Client.
 * All interfaces are strongly typed to match the BPMNinja REST API contract.
 */

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration for the ExternalTaskClient.
 *
 * Mirrors the Camunda External Task Client config but adjusted for BPMNinja
 * API paths and enhanced with a global retry mechanism.
 */
export interface ClientConfig {
  /** Base URL of the BPMNinja engine (default: "http://localhost:8080") */
  baseUrl?: string;

  /** Unique identifier for this worker instance (default: auto-generated) */
  workerId?: string;

  /** Duration in ms to lock a task after fetching (default: 30000) */
  lockDuration?: number;

  /** Maximum number of tasks to fetch per poll (default: 10) */
  maxTasks?: number;

  /**
   * Long-polling timeout in ms. The server holds the connection open
   * until tasks are available or this timeout is reached (default: 10000).
   * Set to 0 to disable long-polling.
   */
  asyncResponseTimeout?: number;

  /** Interval in ms between poll cycles (default: 300) */
  pollingInterval?: number;

  /**
   * Global maximum retries for handler failures.
   * When a handler throws, the client retries up to this many times
   * with exponential backoff before reporting a failure/incident
   * to the engine (default: 3).
   */
  maxRetries?: number;

  /**
   * Base delay in ms for exponential backoff between retries (default: 1000).
   * Actual delay = baseRetryDelay * 2^(attempt - 1)
   */
  baseRetryDelay?: number;

  /**
   * Whether to automatically extend locks for long-running tasks.
   * When enabled, the client periodically extends the lock while
   * the handler is still executing (default: false).
   */
  autoExtendLock?: boolean;

  /**
   * Interval in ms between automatic lock extensions (default: 10000).
   * Only used when autoExtendLock is true.
   */
  autoExtendLockInterval?: number;

  /** Custom logger instance (must conform to pino interface). Pass `false` to disable logging. */
  logger?: Logger | false;
}

/**
 * Resolved configuration with all defaults applied.
 * Used internally — all fields are guaranteed to be present.
 */
export interface ResolvedConfig {
  baseUrl: string;
  workerId: string;
  lockDuration: number;
  maxTasks: number;
  asyncResponseTimeout: number;
  pollingInterval: number;
  maxRetries: number;
  baseRetryDelay: number;
  autoExtendLock: boolean;
  autoExtendLockInterval: number;
  logger: Logger;
}

// ---------------------------------------------------------------------------
// External Task (API response from BPMNinja)
// ---------------------------------------------------------------------------

/**
 * Represents a service task fetched from the BPMNinja engine.
 * Maps 1:1 to the Rust `PendingServiceTask` struct serialised as JSON
 * (field names use snake_case as sent by the server).
 */
export interface ExternalTask {
  /** Unique task ID (UUID) */
  id: string;

  /** ID of the process instance this task belongs to */
  instance_id: string;

  /** Key of the deployed process definition */
  definition_key: string;

  /** BPMN element ID of the service task node */
  node_id: string;

  /** Topic that this task was registered for */
  topic: string;

  /** Reference to the token in the engine */
  token_id: string;

  /** Snapshot of process variables at task creation */
  variables_snapshot: Record<string, unknown>;

  /** ISO 8601 timestamp when the task was created */
  created_at: string;

  /** Worker ID that currently holds the lock */
  worker_id: string | null;

  /** ISO 8601 timestamp when the lock expires */
  lock_expiration: string | null;

  /** Remaining retries before an incident is created */
  retries: number;

  /** Error message from the last failure */
  error_message: string | null;

  /** Detailed error information from the last failure */
  error_details: string | null;
}

// ---------------------------------------------------------------------------
// API Request / Response payloads
// ---------------------------------------------------------------------------

/** POST body for /api/service-task/fetchAndLock */
export interface FetchAndLockRequest {
  workerId: string;
  maxTasks: number;
  topics: TopicRequest[];
  asyncResponseTimeout?: number;
}

/** Topic subscription within a fetchAndLock request */
export interface TopicRequest {
  topicName: string;
  lockDuration: number;
}

/** POST body for /api/service-task/:id/complete */
export interface CompleteRequest {
  workerId: string;
  variables?: Record<string, unknown>;
}

/** POST body for /api/service-task/:id/failure */
export interface FailureRequest {
  workerId: string;
  retries?: number;
  errorMessage?: string;
  errorDetails?: string;
}

/** POST body for /api/service-task/:id/extendLock */
export interface ExtendLockRequest {
  workerId: string;
  newDuration: number;
}

/** POST body for /api/service-task/:id/bpmnError */
export interface BpmnErrorRequest {
  workerId: string;
  errorCode: string;
}

// ---------------------------------------------------------------------------
// Handler & Subscription
// ---------------------------------------------------------------------------

/**
 * Handler function for processing external tasks.
 *
 * @param task    — The fetched external task with its variables.
 * @param service — Helper service to complete, fail, or extend the task.
 */
export type TaskHandler = (
  task: ExternalTask,
  service: TaskServiceInterface,
) => Promise<void>;

/**
 * Options for subscribing to a specific topic.
 */
export interface SubscriptionOptions {
  /** Override the global lock duration for this topic (ms) */
  lockDuration?: number;

  /** Override the global maxRetries for this topic */
  maxRetries?: number;
}

/**
 * Internal representation of a topic subscription.
 */
export interface Subscription {
  topic: string;
  handler: TaskHandler;
  options: Required<Pick<SubscriptionOptions, "lockDuration" | "maxRetries">>;
}

// ---------------------------------------------------------------------------
// Logger interface (pino-compatible subset)
// ---------------------------------------------------------------------------

export interface Logger {
  info(msg: string, ...args: unknown[]): void;
  warn(msg: string, ...args: unknown[]): void;
  error(msg: string, ...args: unknown[]): void;
  debug(msg: string, ...args: unknown[]): void;
  child(bindings: Record<string, unknown>): Logger;
}

// ---------------------------------------------------------------------------
// TaskService interface (for dependency inversion)
// ---------------------------------------------------------------------------

/**
 * Service object passed to every handler, providing typed methods
 * to interact with the engine regarding the current task.
 */
export interface TaskServiceInterface {
  /** Mark the task as successfully completed, optionally with new variables. */
  complete(variables?: Record<string, unknown>): Promise<void>;

  /** Report a failure (with remaining retries and error info). */
  failure(
    errorMessage: string,
    errorDetails?: string,
    retries?: number,
  ): Promise<void>;

  /** Extend the lock on the current task. */
  extendLock(additionalDurationMs: number): Promise<void>;

  /** Throw a BPMN error that can be caught by a boundary error event. */
  bpmnError(errorCode: string): Promise<void>;
}
