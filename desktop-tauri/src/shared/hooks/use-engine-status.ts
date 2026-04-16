import { useState, useEffect, useCallback, useRef } from 'react'
import { getMonitoringData } from '../lib/tauri'

export type EngineStatus = 'checking' | 'online' | 'offline'
export type StorageMode = 'nats' | 'memory' | null

export function useEngineStatus(intervalMs = 10_000) {
  const [status, setStatus] = useState<EngineStatus>('checking')
  const [storageMode, setStorageMode] = useState<StorageMode>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const check = useCallback(async () => {
    try {
      const data = await getMonitoringData()
      setStatus('online')
      setStorageMode(data.storage_info != null ? 'nats' : 'memory')
    } catch {
      setStatus('offline')
      setStorageMode(null)
    }
  }, [])

  useEffect(() => {
    check()
    intervalRef.current = setInterval(check, intervalMs)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [check, intervalMs])

  return { status, storageMode, refresh: check }
}
