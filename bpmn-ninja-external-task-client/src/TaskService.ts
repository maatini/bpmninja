/**
 * @module TaskService
 *
 * Provides typed helper methods for interacting with the BPMNinja engine
 * within a task handler. Each TaskService instance is scoped to a single
 * fetched ExternalTask and carries the task ID, worker ID, and logger.
 */

import type {
  CompleteRequest,
  ExternalTask,
  ExtendLockRequest,
  FailureRequest,
  BpmnErrorRequest,
  Logger,
  TaskRequestOptions,
  TaskServiceInterface,
} from "./types.js";
import { calculateBackoff, sleep } from "./utils/retry.js";

const DEFAULT_REQUEST_OPTIONS: TaskRequestOptions = {
  timeoutMs: 10_000,
  // The client opts into retries; direct TaskService users keep single-shot calls.
  maxRetries: 0,
  retryDelayMs: 250,
};

/** 408/429 and 5xx are transient; everything else is a deterministic client error. */
function isRetryableStatus(status: number): boolean {
  return status === 408 || status === 429 || status >= 500;
}

function toError(value: unknown): Error {
  return value instanceof Error ? value : new Error(String(value));
}

/**
 * TaskService — scoped to one ExternalTask.
 *
 * Created by the ExternalTaskClient for each fetched task and passed
 * into the user's handler alongside the task itself.
 *
 * Uses native `fetch()` (Node 18+) — no external HTTP library needed.
 * Task-state calls use a per-request timeout and retry transient failures.
 */
export class TaskService implements TaskServiceInterface {
  private readonly baseUrl: string;
  private readonly taskId: string;
  private readonly workerId: string;
  private readonly logger: Logger;
  private readonly apiKey?: string;
  private readonly requestOptions: TaskRequestOptions;

  constructor(
    baseUrl: string,
    task: ExternalTask,
    workerId: string,
    logger: Logger,
    apiKey?: string,
    requestOptions?: Partial<TaskRequestOptions>,
  ) {
    this.baseUrl = baseUrl;
    this.taskId = task.id;
    this.workerId = workerId;
    this.logger = logger;
    this.apiKey = apiKey;
    this.requestOptions = { ...DEFAULT_REQUEST_OPTIONS, ...requestOptions };
  }

  /** JSON headers plus optional `Authorization: Bearer <apiKey>`. */
  private jsonHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.apiKey) {
      headers.Authorization = `Bearer ${this.apiKey}`;
    }
    return headers;
  }

  /**
   * Sends one POST with an abort-based timeout.
   *
   * A timer clears on every outcome so a resolved request cannot keep the
   * process alive or abort a later call.
   */
  private async fetchWithTimeout(url: string, body: unknown): Promise<Response> {
    const controller = new AbortController();
    const timer = setTimeout(
      () => controller.abort(),
      this.requestOptions.timeoutMs,
    );

    try {
      return await fetch(url, {
        method: "POST",
        headers: this.jsonHeaders(),
        body: JSON.stringify(body),
        signal: controller.signal,
      });
    } finally {
      clearTimeout(timer);
    }
  }

  /**
   * Executes a task-state call with timeout and bounded retries.
   *
   * Retries network errors, timeouts, and 408/429/5xx responses. Other 4xx
   * responses (e.g. a completed task or a lock owned by another worker) are
   * returned immediately because repeating them cannot succeed.
   */
  private async requestWithRetry(
    url: string,
    body: unknown,
    failurePrefix: string,
  ): Promise<void> {
    const { maxRetries, retryDelayMs, timeoutMs } = this.requestOptions;
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      if (attempt > 0) {
        await sleep(calculateBackoff(attempt, retryDelayMs));
      }

      let response: Response;
      try {
        response = await this.fetchWithTimeout(url, body);
      } catch (err) {
        const error = toError(err);
        lastError =
          error.name === "AbortError" || error.name === "TimeoutError"
            ? new Error(`${failurePrefix}: request timed out after ${timeoutMs}ms`)
            : new Error(`${failurePrefix}: ${error.message}`);
        if (attempt === maxRetries) {
          break;
        }
        this.logger.warn(
          `${lastError.message} — retrying ` +
            `(${attempt + 1}/${maxRetries})`,
        );
        continue;
      }

      if (response.ok) {
        return;
      }

      const text = await response.text();
      const error = new Error(`${failurePrefix}: ${response.status} ${text}`);
      if (!isRetryableStatus(response.status) || attempt === maxRetries) {
        throw error;
      }
      lastError = error;
      this.logger.warn(
        `${failurePrefix}: HTTP ${response.status} — retrying ` +
          `(${attempt + 1}/${maxRetries})`,
      );
    }

    throw lastError ?? new Error(`${failurePrefix}: request failed`);
  }

  /**
   * Completes the task successfully, optionally merging new variables
   * into the process instance.
   *
   * @param variables — Optional key/value pairs to merge into the process.
   */
  async complete(variables?: Record<string, unknown>): Promise<void> {
    const url = `${this.baseUrl}/api/service-task/${this.taskId}/complete`;
    const body: CompleteRequest = {
      workerId: this.workerId,
      variables,
    };

    await this.requestWithRetry(
      url,
      body,
      `Failed to complete task ${this.taskId}`,
    );

    this.logger.info(`Task ${this.taskId} completed successfully`);
  }

  /**
   * Reports a task failure to the engine.
   *
   * When retries reach 0, the engine creates an incident on the process instance.
   *
   * @param errorMessage  — Human-readable error description.
   * @param errorDetails  — Full stack trace or diagnostic details.
   * @param retries       — Remaining retries (default: 0 = create incident).
   */
  async failure(
    errorMessage: string,
    errorDetails?: string,
    retries: number = 0,
  ): Promise<void> {
    const url = `${this.baseUrl}/api/service-task/${this.taskId}/failure`;
    const body: FailureRequest = {
      workerId: this.workerId,
      retries,
      errorMessage,
      errorDetails,
    };

    await this.requestWithRetry(
      url,
      body,
      `Failed to report failure for task ${this.taskId}`,
    );

    this.logger.warn(
      `Task ${this.taskId} reported as failed (retries=${retries}): ${errorMessage}`,
    );
  }

  /**
   * Extends the lock on this task to prevent it from expiring
   * while processing is still in progress.
   *
   * @param additionalDurationMs — New lock duration in milliseconds.
   */
  async extendLock(additionalDurationMs: number): Promise<void> {
    const url = `${this.baseUrl}/api/service-task/${this.taskId}/extendLock`;
    // Engine HTTP expects milliseconds
    const body: ExtendLockRequest = {
      workerId: this.workerId,
      newDuration: additionalDurationMs,
    };

    await this.requestWithRetry(
      url,
      body,
      `Failed to extend lock for task ${this.taskId}`,
    );

    this.logger.debug(
      `Task ${this.taskId}: lock extended by ${additionalDurationMs}ms`,
    );
  }

  /**
   * Throws a BPMN error that can be caught by a boundary error event
   * on the service task in the BPMN process.
   *
   * @param errorCode — The BPMN error code (matched against boundary events).
   */
  async bpmnError(errorCode: string): Promise<void> {
    const url = `${this.baseUrl}/api/service-task/${this.taskId}/bpmnError`;
    const body: BpmnErrorRequest = {
      workerId: this.workerId,
      errorCode,
    };

    await this.requestWithRetry(
      url,
      body,
      `Failed to throw BPMN error for task ${this.taskId}`,
    );

    this.logger.info(
      `Task ${this.taskId}: BPMN error '${errorCode}' thrown`,
    );
  }
}
