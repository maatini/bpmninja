import type { ProcessInstance } from '../types/engine';

export type EngineEventType = 'instance_changed' | 'task_changed' | 'definition_changed' | 'event_subprocess_changed' | 'resync_required';

export interface EngineEventPayload {
  type: EngineEventType;
  instance_id?: string;
  task_id?: string;
  definition_key?: string | null;
  event_id?: string;
  subprocess_id?: string;
  activation_id?: string;
  phase?: 'activated' | 'scope_interrupted' | 'interrupted' | 'completed';
  reason?: string;
  removed?: boolean;
  delta?: Pick<ProcessInstance, 'state' | 'current_node' | 'business_key' | 'started_at' | 'completed_at'>;
}

export interface EngineEventScope {
  instanceId?: string;
  definitionKey?: string;
  debounceMs?: number;
  accept?: (event: EngineEventPayload) => boolean;
}

/** Fehlende IDs alter Server invalidieren breit; Resync umgeht jeden Filter. */
export function matchesEngineEvent(event: EngineEventPayload, filter?: EngineEventType[], scope: EngineEventScope = {}) {
  if (event.type === 'resync_required') return true;
  if (filter?.length && !filter.includes(event.type)) return false;
  if (scope.instanceId && event.instance_id && scope.instanceId !== event.instance_id) return false;
  if (scope.definitionKey && event.definition_key && scope.definitionKey !== event.definition_key) return false;
  if (scope.accept && !scope.accept(event)) return false;
  return true;
}

/** null bedeutet: kein vollständiges Delta für eine bereits bekannte Listenzeile. */
export function applyInstanceEvent(instances: ProcessInstance[], event: EngineEventPayload): ProcessInstance[] | null {
  if (event.type !== 'instance_changed' || !event.instance_id) return null;
  if (event.removed) return instances.filter(instance => instance.id !== event.instance_id);
  if (!event.delta || !event.definition_key || !instances.some(instance => instance.id === event.instance_id)) return null;
  return instances.map(instance => instance.id === event.instance_id
    ? { ...instance, ...event.delta, definition_key: event.definition_key! }
    : instance);
}

/** Deltas während eines REST-Reloads gewinnen gegen dessen älteren Snapshot. */
export function reconcileInstanceSnapshot(snapshot: ProcessInstance[], current: ProcessInstance[], events: Iterable<EngineEventPayload>): ProcessInstance[] {
  let result = snapshot;
  for (const event of events) {
    const updated = applyInstanceEvent(result, event);
    if (updated) { result = updated; continue; }
    const row = current.find(instance => instance.id === event.instance_id);
    if (row && event.instance_id && !result.some(instance => instance.id === event.instance_id)) {
      result = [...result, row];
      result = applyInstanceEvent(result, event) ?? result;
    }
  }
  return [...result].sort((a, b) => a.id.localeCompare(b.id));
}

/** Festes Zeitfenster: Event-Bursts bündeln, bei Dauerlast regelmäßig liefern. */
export function createEngineEventBatcher(deliver: (event: EngineEventPayload) => void, delay: () => number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pending: EngineEventPayload | undefined;
  let closed = false;
  return {
    push(event: EngineEventPayload) {
      if (closed) return;
      if (delay() === 0) { deliver(event); return; }
      if (pending?.type !== 'resync_required') pending = event;
      timer ??= setTimeout(() => {
        timer = undefined;
        const payload = pending;
        pending = undefined;
        if (!closed && payload) deliver(payload);
      }, delay());
    },
    close() {
      closed = true;
      if (timer) clearTimeout(timer);
      pending = undefined;
    },
  };
}
