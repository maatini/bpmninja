/**
 * @module ExternalTaskClient
 *
 * Main client class for the BPMNinja External Task Worker.
 *
 * Usage pattern (analogous to Camunda External Task Client):
 * ```ts
 * const client = new ExternalTaskClient({ baseUrl: "http://localhost:8080" });
 * client.subscribe("my-topic", async (task, service) => {
 *   const result = await doWork(task.variables_snapshot);
 *   await service.complete({ result });
 * });
 * client.start();
 * ```
 */

import pino from "pino";
import { TaskService } from "./TaskService.js";
import { withRetry } from "./utils/retry.js";
import { sleep } from "./utils/retry.js";
import type {
  ClientConfig,
  ExternalTask,
  FetchAndLockRequest,
  Logger,
  ResolvedConfig,
  Subscription,
  SubscriptionOptions,
  TaskHandler,
} from "./types.js";

// ---------------------------------------------------------------------------
// Default configuration values
// ---------------------------------------------------------------------------

const DEFAULTS = {
  baseUrl: "http://localhost:8080",
  lockDuration: 30_000,
  maxTasks: 10,
  asyncResponseTimeout: 10_000,
  pollingInterval: 300,
  maxRetries: 3,
  baseRetryDelay: 1_000,
  autoExtendLock: false,
  autoExtendLockInterval: 10_000,
} as const;

/**
 * ExternalTaskClient — the main entry point for building BPMNinja workers.
 *
 * Features:
 * - Multi-topic subscription (each topic gets its own handler).
 * - Long-polling with configurable timeout.
 * - Global retry with exponential backoff for handler failures.
 * - Automatic lock extension for long-running tasks.
 * - Graceful shutdown (waits for in-flight handlers to finish).
 * - Resilient connection handling with automatic reconnection.
 */
export class ExternalTaskClient {
  private readonly config: ResolvedConfig;
  private readonly subscriptions: Map<string, Subscription> = new Map();
  private running = false;
  private pollAbortController: AbortController | null = null;
  private activeHandlers: Set<Promise<void>> = new Set();
  private pollLoopPromise: Promise<void> | null = null;

  /**
   * Creates a new ExternalTaskClient.
   *
   * @param userConfig — Partial configuration; defaults are applied for missing fields.
   */
  constructor(userConfig: ClientConfig = {}) {
    const logger = this.resolveLogger(userConfig.logger);

    this.config = {
      baseUrl: (userConfig.baseUrl ?? DEFAULTS.baseUrl).replace(/\/+$/, ""),
      workerId: userConfig.workerId ?? `worker-${randomId()}`,
      lockDuration: userConfig.lockDuration ?? DEFAULTS.lockDuration,
      maxTasks: userConfig.maxTasks ?? DEFAULTS.maxTasks,
      asyncResponseTimeout:
        userConfig.asyncResponseTimeout ?? DEFAULTS.asyncResponseTimeout,
      pollingInterval: userConfig.pollingInterval ?? DEFAULTS.pollingInterval,
      maxRetries: userConfig.maxRetries ?? DEFAULTS.maxRetries,
      baseRetryDelay: userConfig.baseRetryDelay ?? DEFAULTS.baseRetryDelay,
      autoExtendLock: userConfig.autoExtendLock ?? DEFAULTS.autoExtendLock,
      autoExtendLockInterval:
        userConfig.autoExtendLockInterval ?? DEFAULTS.autoExtendLockInterval,
      logger,
    };

    this.config.logger.info(
      `ExternalTaskClient initialized (worker: ${this.config.workerId}, ` +
        `engine: ${this.config.baseUrl})`,
    );
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Registers a handler for a specific topic.
   *
   * The handler is called for each fetched task whose topic matches.
   * Multiple topics can be subscribed simultaneously; each topic
   * must have exactly one handler.
   *
   * @param topic   — The service task topic name (as defined in the BPMN model).
   * @param handler — Async function to process the task.
   * @param options — Optional overrides for lock duration and retries.
   * @returns `this` for chaining.
   */
  subscribe(
    topic: string,
    handler: TaskHandler,
    options?: SubscriptionOptions,
  ): this {
    if (this.subscriptions.has(topic)) {
      throw new Error(`Topic "${topic}" is already subscribed`);
    }

    this.subscriptions.set(topic, {
      topic,
      handler,
      options: {
        lockDuration: options?.lockDuration ?? this.config.lockDuration,
        maxRetries: options?.maxRetries ?? this.config.maxRetries,
      },
    });

    this.config.logger.info(`Subscribed to topic "${topic}"`);
    return this;
  }

  /**
   * Removes a topic subscription.
   *
   * In-flight handlers for that topic will finish, but no new tasks
   * will be fetched for it.
   *
   * @param topic — The topic to unsubscribe.
   * @returns `true` if the subscription existed, `false` otherwise.
   */
  unsubscribe(topic: string): boolean {
    const removed = this.subscriptions.delete(topic);
    if (removed) {
      this.config.logger.info(`Unsubscribed from topic "${topic}"`);
    }
    return removed;
  }

  /**
   * Starts the polling loop.
   *
   * Continuously fetches tasks from the engine and dispatches them
   * to registered handlers. The loop runs until `stop()` is called.
   */
  start(): void {
    if (this.running) {
      this.config.logger.warn("Client is already running");
      return;
    }

    if (this.subscriptions.size === 0) {
      throw new Error("Cannot start: no topic subscriptions registered");
    }

    this.running = true;
    this.pollAbortController = new AbortController();

    this.config.logger.info(
      `Polling started (${this.subscriptions.size} topic(s), ` +
        `interval: ${this.config.pollingInterval}ms, ` +
        `long-poll timeout: ${this.config.asyncResponseTimeout}ms)`,
    );

    this.pollLoopPromise = this.pollLoop();
  }

  /**
   * Gracefully stops the client.
   *
   * 1. Signals the poll loop to stop.
   * 2. Waits for all in-flight task handlers to finish.
   * 3. Returns once everything is cleaned up.
   */
  async stop(): Promise<void> {
    if (!this.running) {
      return;
    }

    this.config.logger.info("Stopping client...");
    this.running = false;

    // Abort any pending fetch request
    this.pollAbortController?.abort();

    // Wait for the poll loop to exit
    if (this.pollLoopPromise) {
      await this.pollLoopPromise;
    }

    // Wait for all in-flight handlers
    if (this.activeHandlers.size > 0) {
      this.config.logger.info(
        `Waiting for ${this.activeHandlers.size} in-flight handler(s) to finish...`,
      );
      await Promise.allSettled([...this.activeHandlers]);
    }

    this.config.logger.info("Client stopped gracefully");
  }

  /**
   * Returns whether the client is currently running.
   */
  get isRunning(): boolean {
    return this.running;
  }

  // -------------------------------------------------------------------------
  // Polling Loop
  // -------------------------------------------------------------------------

  /**
   * Main poll loop — runs until `this.running` is set to false.
   *
   * Each iteration:
   * 1. Builds a fetchAndLock request for all subscribed topics.
   * 2. Sends the long-poll HTTP request to the engine.
   * 3. Dispatches each returned task to its handler (fire-and-forget).
   * 4. Waits for `pollingInterval` before the next iteration.
   */
  private async pollLoop(): Promise<void> {
    while (this.running) {
      try {
        const tasks = await this.fetchAndLock();

        for (const task of tasks) {
          const sub = this.subscriptions.get(task.topic);
          if (!sub) {
            this.config.logger.warn(
              `Received task for unsubscribed topic "${task.topic}", skipping`,
            );
            continue;
          }

          // Fire-and-forget: dispatch handler and track the promise
          const handlerPromise = this.executeHandler(task, sub);
          this.activeHandlers.add(handlerPromise);
          handlerPromise.finally(() => {
            this.activeHandlers.delete(handlerPromise);
          });
        }
      } catch (err) {
        if (!this.running) {
          // Expected during shutdown — abort signal throws
          break;
        }

        const error = err instanceof Error ? err : new Error(String(err));
        this.config.logger.error(
          `Poll cycle failed: ${error.message}. Retrying in ${this.config.pollingInterval}ms...`,
        );
      }

      // Wait before next poll (interruptible via abort)
      if (this.running) {
        await sleep(this.config.pollingInterval);
      }
    }
  }

  /**
   * Sends a fetchAndLock request to the BPMNinja engine.
   *
   * Uses native `fetch()` with an AbortSignal for clean cancellation.
   * Enables long-polling via `asyncResponseTimeout`.
   *
   * @returns Array of fetched external tasks (may be empty).
   */
  private async fetchAndLock(): Promise<ExternalTask[]> {
    const url = `${this.config.baseUrl}/api/service-task/fetchAndLock`;

    const topics = [...this.subscriptions.values()].map((sub) => ({
      topicName: sub.topic,
      lockDuration: Math.ceil(sub.options.lockDuration / 1000), // Engine expects seconds
    }));

    const body: FetchAndLockRequest = {
      workerId: this.config.workerId,
      maxTasks: this.config.maxTasks,
      topics,
      asyncResponseTimeout: this.config.asyncResponseTimeout,
    };

    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      signal: this.pollAbortController?.signal,
    });

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`fetchAndLock failed: ${res.status} ${text}`);
    }

    const tasks: ExternalTask[] = await res.json() as ExternalTask[];

    if (tasks.length > 0) {
      this.config.logger.debug(`Fetched ${tasks.length} task(s)`);
    }

    return tasks;
  }

  // -------------------------------------------------------------------------
  // Handler Execution (with retry + lock extension)
  // -------------------------------------------------------------------------

  /**
   * Executes a task handler with:
   * 1. Automatic retry with exponential backoff.
   * 2. Optional automatic lock extension.
   * 3. Incident creation after all retries are exhausted.
   *
   * @param task — The external task to process.
   * @param sub  — The subscription (handler + options) for this task.
   */
  private async executeHandler(
    task: ExternalTask,
    sub: Subscription,
  ): Promise<void> {
    const taskService = new TaskService(
      this.config.baseUrl,
      task,
      this.config.workerId,
      this.config.logger.child({ taskId: task.id, topic: task.topic }),
    );

    // Optional: automatic lock extension timer
    let lockExtensionTimer: ReturnType<typeof setInterval> | null = null;
    if (this.config.autoExtendLock) {
      lockExtensionTimer = setInterval(async () => {
        try {
          await taskService.extendLock(this.config.lockDuration);
        } catch (err) {
          this.config.logger.warn(
            `Failed to extend lock for task ${task.id}: ${err instanceof Error ? err.message : String(err)}`,
          );
        }
      }, this.config.autoExtendLockInterval);
    }

    try {
      const result = await withRetry(
        () => sub.handler(task, taskService),
        {
          maxRetries: sub.options.maxRetries,
          baseDelay: this.config.baseRetryDelay,
          logger: this.config.logger,
          label: `task ${task.id} (topic: ${task.topic})`,
        },
      );

      if (!result.success) {
        // All retries exhausted — report incident to the engine
        try {
          await taskService.failure(
            result.error.message,
            result.error.stack ?? "No stack trace available",
            0, // retries = 0 → engine creates incident
          );
        } catch (failErr) {
          this.config.logger.error(
            `Failed to report incident for task ${task.id}: ` +
              `${failErr instanceof Error ? failErr.message : String(failErr)}`,
          );
        }
      }
    } finally {
      // Always clean up the lock extension timer
      if (lockExtensionTimer) {
        clearInterval(lockExtensionTimer);
      }
    }
  }

  // -------------------------------------------------------------------------
  // Logger resolution
  // -------------------------------------------------------------------------

  /**
   * Resolves the logger to use.
   * - If explicit Logger instance provided → use it.
   * - If `false` → create a silent noop logger.
   * - If undefined → create a default pino logger.
   */
  private resolveLogger(input: ClientConfig["logger"]): Logger {
    if (input === false) {
      return noopLogger();
    }

    if (input) {
      return input;
    }

    return pino({
      level: "info",
      transport: {
        target: "pino-pretty",
        options: {
          colorize: true,
          translateTime: "HH:MM:ss.l",
          ignore: "pid,hostname",
        },
      },
    });
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Generates a short random ID for default worker names. */
function randomId(): string {
  return Math.random().toString(36).substring(2, 10);
}

/** Creates a silent logger that discards all output. */
function noopLogger(): Logger {
  const noop = () => {};
  const logger: Logger = {
    info: noop,
    warn: noop,
    error: noop,
    debug: noop,
    child: () => logger,
  };
  return logger;
}
