/**
 * @module utils/retry
 *
 * Exponential-backoff retry wrapper used by the ExternalTaskClient
 * to automatically retry failing task handlers.
 */

import type { Logger } from "../types.js";

/**
 * Sleeps for the given number of milliseconds.
 * Returns a promise that resolves after the delay.
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Calculates the exponential backoff delay for the given attempt.
 *
 * Formula: baseDelay * 2^(attempt - 1), capped at 30 seconds.
 *
 * @param attempt   — Current attempt number (1-based).
 * @param baseDelay — Base delay in milliseconds.
 * @returns Delay in milliseconds to wait before retrying.
 */
export function calculateBackoff(attempt: number, baseDelay: number): number {
  const delay = baseDelay * Math.pow(2, attempt - 1);
  return Math.min(delay, 30_000); // Cap at 30s
}

/**
 * Options for the withRetry wrapper.
 */
export interface RetryOptions {
  /** Maximum number of retry attempts (excluding the initial try). */
  maxRetries: number;

  /** Base delay in ms for exponential backoff. */
  baseDelay: number;

  /** Logger instance for retry attempt logging. */
  logger: Logger;

  /** Human-readable label for log messages (e.g. "task abc123"). */
  label: string;
}

/**
 * Result of a retry-wrapped execution.
 */
export type RetryResult =
  | { success: true }
  | { success: false; error: Error; attempts: number };

/**
 * Wraps an async function with exponential-backoff retry logic.
 *
 * On each failure:
 * 1. Logs the error with stacktrace and attempt count.
 * 2. Waits for exponentially increasing delay.
 * 3. Retries the function.
 *
 * After all retries are exhausted, returns the last error
 * so the caller can report it to the engine as an incident.
 *
 * @param fn      — The async function to execute with retries.
 * @param options — Retry configuration.
 * @returns A RetryResult indicating success or the final error.
 */
export async function withRetry(
  fn: () => Promise<void>,
  options: RetryOptions,
): Promise<RetryResult> {
  const { maxRetries, baseDelay, logger, label } = options;

  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    try {
      await fn();
      return { success: true };
    } catch (err) {
      const error =
        err instanceof Error ? err : new Error(String(err));
      const remaining = maxRetries + 1 - attempt;

      if (remaining > 0) {
        const delay = calculateBackoff(attempt, baseDelay);
        logger.warn(
          `[${label}] Handler failed (attempt ${attempt}/${maxRetries + 1}), ` +
            `${remaining} retries remaining. Retrying in ${delay}ms...\n` +
            `  Error: ${error.message}\n` +
            `  Stack: ${error.stack ?? "N/A"}`,
        );
        await sleep(delay);
      } else {
        logger.error(
          `[${label}] Handler failed after ${attempt} attempts. ` +
            `No retries remaining — reporting incident to engine.\n` +
            `  Error: ${error.message}\n` +
            `  Stack: ${error.stack ?? "N/A"}`,
        );
        return { success: false, error, attempts: attempt };
      }
    }
  }

  // This should never be reached, but TypeScript needs it
  return { success: false, error: new Error("Unexpected retry exit"), attempts: maxRetries + 1 };
}
