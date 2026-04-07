import { useEffect, useRef } from 'react';

/**
 * Custom hook to execute a polling function at a specified interval.
 * 
 * @param fetchFn The async function to execute.
 * @param intervalMs The polling interval in milliseconds.
 * @param enabled Whether polling is currently enabled (defaults to true).
 */
export function usePolling(fetchFn: () => Promise<void> | void, intervalMs: number, enabled: boolean = true) {
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    // Initial fetch
    if (enabled) {
      fetchFn();
    }

    if (enabled && intervalMs > 0) {
      intervalRef.current = setInterval(fetchFn, intervalMs);
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [fetchFn, intervalMs, enabled]);
}
