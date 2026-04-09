/**
 * @bpmninja/external-task-client
 *
 * Production-ready External Task Worker Client for the BPMNinja BPMN Engine.
 * Analogous to the Camunda External Task Client, but adapted for BPMNinja's
 * REST API endpoints under /api/service-task/*.
 *
 * @example
 * ```ts
 * import { ExternalTaskClient } from "@bpmninja/external-task-client";
 *
 * const client = new ExternalTaskClient({
 *   baseUrl: "http://localhost:8080",
 *   maxRetries: 3,
 * });
 *
 * client.subscribe("send-email", async (task, service) => {
 *   console.log("Processing:", task.variables_snapshot);
 *   await service.complete({ emailSent: true });
 * });
 *
 * client.start();
 * ```
 *
 * @packageDocumentation
 */

export { ExternalTaskClient } from "./ExternalTaskClient.js";
export { TaskService } from "./TaskService.js";
export { withRetry, sleep, calculateBackoff } from "./utils/retry.js";

export type {
  ClientConfig,
  ExternalTask,
  FetchAndLockRequest,
  TopicRequest,
  CompleteRequest,
  FailureRequest,
  ExtendLockRequest,
  BpmnErrorRequest,
  TaskHandler,
  SubscriptionOptions,
  Subscription,
  Logger,
  TaskServiceInterface,
  ResolvedConfig,
} from "./types.js";
