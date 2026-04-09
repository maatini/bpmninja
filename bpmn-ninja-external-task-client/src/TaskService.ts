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
  TaskServiceInterface,
} from "./types.js";

/**
 * TaskService — scoped to one ExternalTask.
 *
 * Created by the ExternalTaskClient for each fetched task and passed
 * into the user's handler alongside the task itself.
 *
 * Uses native `fetch()` (Node 18+) — no external HTTP library needed.
 */
export class TaskService implements TaskServiceInterface {
  private readonly baseUrl: string;
  private readonly taskId: string;
  private readonly workerId: string;
  private readonly logger: Logger;

  constructor(
    baseUrl: string,
    task: ExternalTask,
    workerId: string,
    logger: Logger,
  ) {
    this.baseUrl = baseUrl;
    this.taskId = task.id;
    this.workerId = workerId;
    this.logger = logger;
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

    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(
        `Failed to complete task ${this.taskId}: ${res.status} ${text}`,
      );
    }

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

    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(
        `Failed to report failure for task ${this.taskId}: ${res.status} ${text}`,
      );
    }

    this.logger.warn(
      `Task ${this.taskId} reported as failed (retries=${retries}): ${errorMessage}`,
    );
  }

  /**
   * Extends the lock on this task to prevent it from expiring
   * while processing is still in progress.
   *
   * @param additionalDurationMs — Additional time in milliseconds.
   */
  async extendLock(additionalDurationMs: number): Promise<void> {
    const url = `${this.baseUrl}/api/service-task/${this.taskId}/extendLock`;
    // Engine expects seconds
    const newDuration = Math.ceil(additionalDurationMs / 1000);
    const body: ExtendLockRequest = {
      workerId: this.workerId,
      newDuration,
    };

    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(
        `Failed to extend lock for task ${this.taskId}: ${res.status} ${text}`,
      );
    }

    this.logger.debug(
      `Task ${this.taskId}: lock extended by ${newDuration}s`,
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

    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(
        `Failed to throw BPMN error for task ${this.taskId}: ${res.status} ${text}`,
      );
    }

    this.logger.info(
      `Task ${this.taskId}: BPMN error '${errorCode}' thrown`,
    );
  }
}
