import type { ProcessInstance } from '../types/engine';

export type EngineEventType = 'instance_changed' | 'task_changed' | 'definition_changed' | 'resync_required';

export interface EngineEventPayload {
  type: EngineEventType;
  instance_id?: string;
  task_id?: string;
  definition_key?: string | null;
  removed?: boolean;
  delta?: Pick<ProcessInstance, 'state' | 'current_node' | 'business_key' | 'started_at' | 'completed_at'>;
}

export interface EngineEventScope {
  instanceId?: string;
  definitionKey?: string;
  debounceMs?: number;
}

/** Fehlende IDs alter Server invalidieren breit; Resync umgeht jeden Filter. */
export function matchesEngineEvent(event: EngineEventPayload, filter?: EngineEventType[], scope: EngineEventScope = {}) {
  if (event.type === 'resync_required') return true;
  if (filter?.length && !filter.includes(event.type)) return false;
  if (scope.instanceId && event.instance_id && scope.instanceId !== event.instance_id) return false;
  if (scope.definitionKey && event.definition_key && scope.definitionKey !== event.definition_key) return false;
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
