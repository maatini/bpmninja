import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

import { matchesEngineEvent, type EngineEventPayload, type EngineEventType, type EngineEventScope } from '../lib/engine-events';
export type { EngineEventPayload, EngineEventType } from '../lib/engine-events';

/**
 * Abonniert Tauri-Engine-Events und ruft bei jedem passenden Event `callback` auf.
 *
 * @param callback Funktion, die bei einem Engine-Event aufgerufen wird.
 * @param filter   Optionale Liste von Event-Typen. Wenn leer/nicht angegeben, werden alle Events verarbeitet.
 * @param enabled  Ob das Abonnement aktiv ist (Standard: true).
 */
export function useEngineEvents(
  callback: (event: EngineEventPayload) => void,
  filter?: EngineEventType[],
  enabled: boolean = true,
  scope: EngineEventScope = {},
) {
  const callbackRef = useRef(callback);
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    if (!enabled) return;

    let unlisten: (() => void) | undefined;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let pending: EngineEventPayload | undefined;

    listen<EngineEventPayload>('engine-event', (event) => {
      if (disposed || !event.payload || !matchesEngineEvent(event.payload, filter, scope)) return;
      if (scope.debounceMs === 0) {
        callbackRef.current(event.payload);
        return;
      }
      // Festes Fenster statt endlosem Debounce bei kontinuierlicher Last.
      if (pending?.type !== 'resync_required') pending = event.payload;
      timer ??= setTimeout(() => {
        timer = undefined;
        const payload = pending;
        pending = undefined;
        if (!disposed && payload) callbackRef.current(payload);
      }, scope.debounceMs ?? 150);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });

    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      unlisten?.();
    };
  }, [enabled, filter?.join(','), scope.instanceId, scope.definitionKey, scope.debounceMs]);
}
