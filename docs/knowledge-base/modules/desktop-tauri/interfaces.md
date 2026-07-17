# desktop-tauri — Interfaces

## Tauri Command Wrappers (lib/tauri.ts)

All API calls go through typed wrappers exported from `lib/tauri.ts`:

```typescript
// Definitions
deployDefinition(xml: string): Promise<{ key: string; version: number }>
listDefinitions(): Promise<Definition[]>
getDefinitionXml(key: string): Promise<string>
deleteDefinition(key: string, cascade: boolean): Promise<void>

// Instances
startInstance(key: string, variables: Record<string, any>, businessKey: string): Promise<Instance>
startLatestInstance(bpmnProcessId: string, variables: Record<string, any>): Promise<Instance>
listInstances(): Promise<Instance[]>
getInstance(id: string): Promise<Instance>
deleteInstance(id: string): Promise<void>
suspendInstance(id: string): Promise<void>
resumeInstance(id: string): Promise<void>
updateVariables(id: string, variables: Record<string, any>): Promise<void>
moveToken(id: string, tokenId: string, targetNode: string): Promise<void>
migrateInstance(id: string, newKey: string, mapping: Record<string, string>): Promise<void>

// Tasks
listTasks(): Promise<UserTask[]>
completeTask(id: string, variables: Record<string, any>): Promise<void>
listServiceTasks(): Promise<ServiceTask[]>
fetchAndLockTasks(topics: string[], workerId: string, maxTasks: number): Promise<ServiceTask[]>

// Files
uploadFile(instanceId: string, varName: string, file: File): Promise<void>
downloadFile(instanceId: string, varName: string): Promise<Blob>

// Monitoring
getInfo(): Promise<BackendInfo>
getMonitoring(): Promise<MonitoringData>
getBucketEntries(bucket: string): Promise<BucketEntry[]>
getLogs(level?: string, search?: string, limit?: number): Promise<LogEntry[]>

// History
getHistory(instanceId: string): Promise<HistoryEntry[]>
listCompletedInstances(filters: CompletedInstanceQuery): Promise<Instance[]>
```

## SSE Event Flow

```
Engine (engine-core) → broadcast channel → engine-server SSE handler → GET /api/events
    → Tauri background task (Rust, src-tauri/) → Tauri event system
    → React useEffect hook → re-fetch via REST
```

Events are coarse-grained: `instance_changed`, `task_changed`, `definition_changed`.

## UI Tab Structure

| Tab | Route | Key Component |
|-----|-------|---------------|
| Modeler | `/modeler` | `ModelerPage.tsx` |
| Instances | `/instances` | `InstancesPage.tsx` + `InstanceDetailDialog.tsx` |
| Process Definitions | `/definitions` | `DeployedProcessesPage.tsx` |
| Pending Tasks | `/tasks` | `PendingTasksPage.tsx` + `IncidentsPage.tsx` |
| Overview | `/overview` | Stats cards (timers, messages, service jobs) |
| History | `/history` | Completed instances search + pagination |
| Monitoring | `/monitoring` | `MonitoringPage.tsx` (stats, buckets, logs) |
| Settings | `/settings` | `SettingsPage.tsx` (API URL + verify) |
