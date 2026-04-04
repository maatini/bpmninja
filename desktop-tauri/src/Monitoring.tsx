import { useState, useEffect, useRef } from 'react'
import { getMonitoringData, type MonitoringData } from './lib/tauri'
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Table, TableHeader, TableRow, TableHead, TableBody, TableCell } from '@/components/ui/table'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Server, Settings2, Database } from 'lucide-react'
import { Skeleton } from '@/components/ui/skeleton'

/**
 * Formats bytes into a human-readable string (B, KB, MB, GB).
 */
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const value = bytes / Math.pow(1024, i)
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}

/** Maps bucket_type to a human-readable label. */
function bucketTypeLabel(t: string): string {
  if (t === 'kv') return 'KV Store'
  if (t === 'object_store') return 'Object Store'
  if (t === 'stream') return 'Stream'
  return t
}

export function Monitoring() {
  const [data, setData] = useState<MonitoringData | null>(null)
  const [error, setError] = useState<string | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const refresh = async () => {
    try {
      const result = await getMonitoringData()
      setData(result)
      setError(null)
    } catch (e: any) {
      setError(String(e))
    }
  }

  useEffect(() => {
    refresh()
    intervalRef.current = setInterval(refresh, 5000)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [])

  return (
    <div className="flex flex-col h-full bg-background">
      <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
        <h2 className="text-2xl font-bold tracking-tight">Monitoring</h2>
        <span className="text-xs text-muted-foreground bg-muted px-2 py-1 rounded-md">Auto-refreshing every 5s</span>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-6 space-y-6">
          {error && (
            <Card className="border-destructive/50 bg-destructive/10">
              <CardContent className="p-4 text-destructive font-medium">
                Error loading monitoring data: {error}
              </CardContent>
            </Card>
          )}

          {/* Engine Metrics */}
          <Card>
            <CardHeader className="pb-3 border-b bg-muted/20">
              <CardTitle className="text-lg flex items-center gap-2">
                <Settings2 className="h-5 w-5 text-primary" /> Engine Metrics
              </CardTitle>
            </CardHeader>
            <CardContent className="pt-6">
              {!data && !error && (
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  {[1,2,3,4,5,6,7,8].map(i => (
                    <Card key={i} className="bg-muted/30 border-muted-foreground/20">
                      <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                        <Skeleton className="h-8 w-12 mb-2" />
                        <Skeleton className="h-3 w-[100px]" />
                      </CardContent>
                    </Card>
                  ))}
                </div>
              )}
              {data && (
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <Card className="bg-muted/30 border-muted-foreground/20">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-3xl font-bold tracking-tight">{data?.definitions_count ?? '–'}</span>
                    <span className="text-xs text-muted-foreground mt-1 uppercase tracking-wider font-semibold">Deployed Definitions</span>
                  </CardContent>
                </Card>
                <Card className="bg-muted/30 border-muted-foreground/20">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-3xl font-bold tracking-tight">{data?.instances_total ?? '–'}</span>
                    <span className="text-xs text-muted-foreground mt-1 uppercase tracking-wider font-semibold">Total Instances</span>
                  </CardContent>
                </Card>
                <Card className="border-blue-500/30 bg-blue-500/5">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-3xl font-bold tracking-tight text-blue-600 dark:text-blue-400">{data?.instances_running ?? '–'}</span>
                    <span className="text-xs text-blue-600/80 dark:text-blue-400/80 mt-1 uppercase tracking-wider font-semibold">Running</span>
                  </CardContent>
                </Card>
                <Card className="border-green-500/30 bg-green-500/5">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-3xl font-bold tracking-tight text-green-600 dark:text-green-400">{data?.instances_completed ?? '–'}</span>
                    <span className="text-xs text-green-600/80 dark:text-green-400/80 mt-1 uppercase tracking-wider font-semibold">Completed</span>
                  </CardContent>
                </Card>
                <Card className="bg-muted/30 border-muted-foreground/20">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-2xl font-bold">{data?.pending_user_tasks ?? '–'}</span>
                    <span className="text-xs text-muted-foreground mt-1 text-center">Pending User Tasks</span>
                  </CardContent>
                </Card>
                <Card className="bg-muted/30 border-muted-foreground/20">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-2xl font-bold">{data?.pending_service_tasks ?? '–'}</span>
                    <span className="text-xs text-muted-foreground mt-1 text-center">Pending External Tasks</span>
                  </CardContent>
                </Card>
                <Card className="bg-muted/30 border-muted-foreground/20">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-2xl font-bold">{data?.pending_timers ?? '–'}</span>
                    <span className="text-xs text-muted-foreground mt-1 text-center">Pending Timers</span>
                  </CardContent>
                </Card>
                <Card className="bg-muted/30 border-muted-foreground/20">
                  <CardContent className="p-4 flex flex-col items-center justify-center text-center h-full">
                    <span className="text-2xl font-bold">{data?.pending_message_catches ?? '–'}</span>
                    <span className="text-xs text-muted-foreground mt-1 text-center">Pending Messages</span>
                  </CardContent>
                </Card>
                </div>
              )}
            </CardContent>
          </Card>

          {/* Storage Backend Info */}
          <Card>
            <CardHeader className="pb-3 border-b bg-muted/20">
              <CardTitle className="text-lg flex items-center gap-2">
                <Server className="h-5 w-5 text-emerald-500" /> Storage Backend
              </CardTitle>
            </CardHeader>
            <CardContent className="pt-6">
              {data?.storage_info ? (
                <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
                  <div className="bg-muted/30 border rounded-lg p-3 text-center min-w-0">
                    <div className="text-lg font-bold truncate px-1" title={data.storage_info.backend_name}>{data.storage_info.backend_name}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Backend</div>
                  </div>
                  <div className="bg-muted/30 border rounded-lg p-3 text-center">
                    <div className="text-lg font-bold">v{data.storage_info.version}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Version</div>
                  </div>
                  <div className="bg-muted/30 border rounded-lg p-3 text-center min-w-0">
                    <div className="text-sm font-semibold truncate mt-1 px-1" title={`${data.storage_info.host}:${data.storage_info.port}`}>{data.storage_info.host}:{data.storage_info.port}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Endpoint</div>
                  </div>
                  <div className="bg-muted/30 border rounded-lg p-3 text-center">
                    <div className="text-lg font-bold">{data.storage_info.streams}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Streams</div>
                  </div>
                  <div className="bg-muted/30 border rounded-lg p-3 text-center">
                    <div className="text-lg font-bold">{data.storage_info.consumers}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Consumers</div>
                  </div>
                  <div className="bg-muted/30 border rounded-lg p-3 text-center">
                    <div className="text-lg font-bold">{formatBytes(data.storage_info.memory_bytes)}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Memory Usage</div>
                  </div>
                  <div className="bg-muted/30 border rounded-lg p-3 text-center">
                    <div className="text-lg font-bold">{formatBytes(data.storage_info.storage_bytes)}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">Storage Usage</div>
                  </div>
                </div>
              ) : (
                <div className="text-muted-foreground italic py-4 text-center border rounded-lg bg-muted/10">
                  No storage backend connected — running in-memory only.
                </div>
              )}
            </CardContent>
          </Card>

          {/* Data Storage Details */}
          {data?.storage_info && data.storage_info.buckets.length > 0 && (
            <Card>
              <CardHeader className="pb-3 border-b bg-muted/20">
                <CardTitle className="text-lg flex items-center gap-2">
                  <Database className="h-5 w-5 text-amber-500" /> Data Storage Details
                </CardTitle>
              </CardHeader>
              <CardContent className="p-0">
                <Table>
                  <TableHeader className="bg-muted/30">
                    <TableRow>
                      <TableHead>Bucket</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead className="text-right">Entries</TableHead>
                      <TableHead className="text-right">Size</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.storage_info.buckets.map((b) => (
                      <TableRow key={b.name} className="hover:bg-muted/50">
                        <TableCell className="font-medium text-foreground">{b.name}</TableCell>
                        <TableCell>
                          <Badge 
                            variant="secondary" 
                            className={
                              b.bucket_type === 'kv' ? 'bg-blue-100 text-blue-700 hover:bg-blue-100/80 dark:bg-blue-900/40 dark:text-blue-400' :
                              b.bucket_type === 'object_store' ? 'bg-amber-100 text-amber-700 hover:bg-amber-100/80 dark:bg-amber-900/40 dark:text-amber-400' : 
                              'bg-green-100 text-green-700 hover:bg-green-100/80 dark:bg-green-900/40 dark:text-green-400'
                            }
                          >
                            {bucketTypeLabel(b.bucket_type)}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-right tabular-nums">{b.entries.toLocaleString()}</TableCell>
                        <TableCell className="text-right tabular-nums text-muted-foreground">{formatBytes(b.size_bytes)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          )}

        </div>
      </ScrollArea>
    </div>
  )
}
