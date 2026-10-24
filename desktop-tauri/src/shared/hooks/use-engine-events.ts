import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

import { createEngineEventBatcher, matchesEngineEvent, type EngineEventPayload, type EngineEventType, type EngineEventScope } from '../lib/engine-events';
export type { EngineEventPayload, EngineEventType } from '../lib/engine-events';

/**
 * Abonniert Tauri-Engine-Events und ruft bei jedem passenden Event `callback` auf.
 *
 * @param callback Funktion, die bei einem Engine-Event aufgerufen wird.
 * @param filter   Optionale Liste von Event-Typen. Wenn leer/nicht angegeben, werden alle Events verarbeitet.
 * @param enabled  Ob das Abonnement aktiv ist (Standard: true).
 * @param scope    Entity-Filter und Bündelungsfenster; 0 liefert jedes Event sofort.
 */
export function useEngineEvents(
  callback: (event: EngineEventPayload) => void,
  filter?: EngineEventType[],
  enabled: boolean = true,
  scope: EngineEventScope = {},
) {
  const callbackRef = useRef(callback);
  const scopeRef = useRef(scope);
  const filterRef = useRef(filter);
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);
  useEffect(() => { scopeRef.current = scope; }, [scope]);
  useEffect(() => { filterRef.current = filter; }, [filter]);

  useEffect(() => {
    if (!enabled) return;

    let unlisten: (() => void) | undefined;
    let disposed = false;
    const batcher = createEngineEventBatcher(payload => {
      if (!disposed && matchesEngineEvent(payload, filterRef.current, scopeRef.current)) callbackRef.current(payload);
    }, () => scopeRef.current.debounceMs ?? 150);

    listen<EngineEventPayload>('engine-event', (event) => {
      if (disposed || !event.payload || !matchesEngineEvent(event.payload, filterRef.current, scopeRef.current)) return;
      batcher.push(event.payload);
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });

    return () => {
      disposed = true;
      batcher.close();
      unlisten?.();
    };
  }, [enabled]);
}
