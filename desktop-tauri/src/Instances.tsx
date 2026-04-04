import { useState, useEffect, useCallback } from 'react';
import { listInstances, getInstanceDetails, getPendingTasks, getPendingServiceTasks, updateInstanceVariables, getDefinitionXml, deleteInstance, listDefinitions, type ProcessInstance, type PendingUserTask, type PendingServiceTask, type DefinitionInfo } from './lib/tauri';
import { InstanceViewer } from './InstanceViewer';
import { RefreshCw, Activity, CheckCircle, Clock, Trash, FileCode2, Network, ScrollText, Layers } from 'lucide-react';
import { VariableEditor, type VariableRow, parseVariables, serializeVariables } from './VariableEditor';
import { HistoryTimeline } from './HistoryTimeline';
import { ErrorBoundary } from './ErrorBoundary';
import { useToast } from '@/hooks/use-toast';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Skeleton } from '@/components/ui/skeleton';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';
import { cn } from '@/lib/utils';
import { ScrollArea } from '@/components/ui/scroll-area';

function stateLabel(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'Running';
  if (state === 'Completed') return 'Completed';
  if ((state as any) === 'Errored') return 'Errored';
  if ((state as any) === 'Cancelled') return 'Cancelled';
  if (typeof state === 'object') {
    if ('WaitingOnUserTask' in state) return 'Wait: User Task';
    if ('WaitingOnServiceTask' in state) return 'Wait: Service Task';
    if ('WaitingOnTimer' in state) return 'Wait: Timer';
    if ('WaitingOnMessage' in state) return 'Wait: Message';
    return Object.keys(state)[0]?.replace(/([A-Z])/g, ' $1').trim() || 'Unknown';
  }
  return String(state);
}

function stateBadgeClass(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'bg-blue-600 hover:bg-blue-700 text-white';
  if (state === 'Completed') return 'bg-green-600 hover:bg-green-700 text-white border-none';
  if ((state as any) === 'Errored') return 'bg-destructive/10 text-destructive hover:bg-destructive/20 border-destructive/20 outline outline-1 outline-destructive';
  if ((state as any) === 'Cancelled') return 'bg-muted text-muted-foreground hover:bg-muted/80 outline outline-1 outline-muted';
  if (typeof state === 'object') {
    if ('WaitingOnUserTask' in state) return 'bg-amber-500/20 text-amber-700 hover:bg-amber-500/30 border-amber-500/30 dark:text-amber-400';
    if ('WaitingOnServiceTask' in state) return 'bg-purple-500/20 text-purple-700 hover:bg-purple-500/30 border-purple-500/30 dark:text-purple-400';
    if ('WaitingOnTimer' in state) return 'bg-cyan-500/20 text-cyan-700 hover:bg-cyan-500/30 border-cyan-500/30 dark:text-cyan-400';
    if ('WaitingOnMessage' in state) return 'bg-indigo-500/20 text-indigo-700 hover:bg-indigo-500/30 border-indigo-500/30 dark:text-indigo-400';
  }
  return 'bg-secondary text-secondary-foreground hover:bg-secondary/80';
}

function groupInstances(instances: ProcessInstance[], definitions: DefinitionInfo[]) {
  const defMap = new Map<string, DefinitionInfo>();
  for (const d of definitions) defMap.set(d.key, d);

  const groups = new Map<string, ProcessInstance[]>();
  const unknownGroup: ProcessInstance[] = [];

  for (const inst of instances) {
    const def = defMap.get(inst.definition_key);
    if (def) {
      const arr = groups.get(def.bpmn_id) || [];
      arr.push(inst);
      groups.set(def.bpmn_id, arr);
    } else {
      unknownGroup.push(inst);
    }
  }

  for (const [, insts] of groups) {
    insts.sort((a, b) => {
      if (a.state === 'Completed' && b.state !== 'Completed') return 1;
      if (a.state !== 'Completed' && b.state === 'Completed') return -1;
      return a.id.localeCompare(b.id);
    });
  }

  return { groups, unknownGroup, defMap };
}

export function Instances({ selectedInstanceId, onClearSelection }: { selectedInstanceId?: string | null, onClearSelection?: () => void }) {
  const { toast } = useToast();
  const [instances, setInstances] = useState<ProcessInstance[]>([]);
  const [definitions, setDefinitions] = useState<DefinitionInfo[]>([]);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<ProcessInstance | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [pendingTasks, setPendingTasks] = useState<PendingUserTask[]>([]);
  const [pendingServiceTasks, setPendingServiceTasks] = useState<PendingServiceTask[]>([]);
  const [variables, setVariables] = useState<VariableRow[]>([]);
  const [deletedKeys, setDeletedKeys] = useState<Set<string>>(new Set());
  const [historyRefreshTrigger, setHistoryRefreshTrigger] = useState(0);

  const [definitionXml, setDefinitionXml] = useState<string | null>(null);
  const [showNodeDetails, setShowNodeDetails] = useState(true);
  const [instanceToDelete, setInstanceToDelete] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [instList, defList] = await Promise.all([listInstances(), listDefinitions()]);
      setInstances(instList);
      setDefinitions(defList);
      setHistoryRefreshTrigger(prev => prev + 1);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    if (selected) return;
    const id = setInterval(fetchData, 3000);
    return () => clearInterval(id);
  }, [fetchData, selected]);

  const handleSelect = useCallback(async (inst: ProcessInstance) => {
    setDetailLoading(true);
    setDefinitionXml(null);
    setShowNodeDetails(true);
    setHistoryRefreshTrigger(prev => prev + 1);
    try {
      const details = await getInstanceDetails(inst.id);
      setSelected(details);
      
      try {
        const xml = await getDefinitionXml(details.definition_key);
        setDefinitionXml(xml);
      } catch (xmlError) {
        console.error("Failed to fetch layout XML:", xmlError);
      }

      setVariables(parseVariables(details.variables));
      if (typeof details.state === 'object') {
        if ('WaitingOnUserTask' in details.state) {
          const tasks = await getPendingTasks();
          setPendingTasks(tasks.filter(t => t.instance_id === details.id));
          setPendingServiceTasks([]);
        } else if ('WaitingOnServiceTask' in details.state) {
          const sTasks = await getPendingServiceTasks();
          setPendingServiceTasks(sTasks.filter(t => t.instance_id === details.id));
          setPendingTasks([]);
        } else {
          setPendingTasks([]);
          setPendingServiceTasks([]);
        }
      } else {
        setPendingTasks([]);
        setPendingServiceTasks([]);
      }
    } catch {
      setSelected(inst);
      setVariables(parseVariables(inst.variables));
      setPendingTasks([]);
      setPendingServiceTasks([]);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const handleSaveVariables = async () => {
    if (!selected) return;
    const varsToSave = serializeVariables(variables, deletedKeys);
    if (varsToSave === null) {
      toast({ variant: 'destructive', description: 'Invalid variables format (check JSON or Numbers)' });
      return;
    }

    try {
      await updateInstanceVariables(selected.id, varsToSave);
      toast({ description: 'Variables saved successfully.' });
      const updated = await getInstanceDetails(selected.id);
      setSelected(updated);
      setVariables(parseVariables(updated.variables));
      setDeletedKeys(new Set());
      setHistoryRefreshTrigger(prev => prev + 1);
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Error saving variables: ' + e });
    }
  };

  const handleClose = () => {
    setSelected(null);
    setPendingTasks([]);
    setPendingServiceTasks([]);
    setDefinitionXml(null);
    setShowNodeDetails(true);
  };

  const handleDeleteRequest = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setInstanceToDelete(id);
  };

  const confirmDelete = async () => {
    if (!instanceToDelete) return;
    try {
      await deleteInstance(instanceToDelete);
      if (selected?.id === instanceToDelete) {
        handleClose();
      }
      fetchData();
    } catch (err) {
      toast({ variant: 'destructive', description: "Failed to delete instance: " + err });
    } finally {
      setInstanceToDelete(null);
    }
  };

  useEffect(() => {
    if (selectedInstanceId && instances.length > 0 && (!selected || selected.id !== selectedInstanceId)) {
      const inst = instances.find(i => i.id === selectedInstanceId);
      if (inst) {
        handleSelect(inst);
        if (onClearSelection) onClearSelection();
      }
    }
  }, [selectedInstanceId, instances, selected, handleSelect, onClearSelection]);

  const { groups, unknownGroup, defMap } = groupInstances(instances, definitions);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
        <h2 className="text-2xl font-bold tracking-tight">Instances</h2>
        <div className="flex items-center gap-4">
          <span className="text-xs text-muted-foreground">Auto-refreshing</span>
          <Button onClick={fetchData} variant="outline" size="sm" className="gap-2">
            <RefreshCw className="h-4 w-4" /> Refresh
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-6 space-y-6">
          {loading && (
            <div className="space-y-4">
              {[1,2,3].map(i => (
                <Card key={i} className="p-4">
                  <div className="flex items-center gap-4">
                    <Skeleton className="h-6 w-[120px] rounded-full" />
                    <Skeleton className="h-5 w-[200px]" />
                    <Skeleton className="h-5 w-[80px] ml-auto" />
                  </div>
                </Card>
              ))}
            </div>
          )}
          {error && <div className="text-destructive font-medium">Error: {error}</div>}
          {!loading && !error && instances.length === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-center">
              <Layers className="h-16 w-16 text-muted-foreground/30 mb-4" />
              <h3 className="text-lg font-semibold text-muted-foreground">No Instances Yet</h3>
              <p className="text-sm text-muted-foreground/70 mt-1 max-w-sm">
                Deploy a BPMN process and start your first instance from the Modeler.
              </p>
            </div>
          )}

          {[...groups.entries()].map(([bpmnId, groupInstances]) => {
            const activeCount = groupInstances.filter(i => i.state !== 'Completed').length;
            
            return (
              <Card key={bpmnId} className="overflow-hidden">
                <CardHeader className="bg-muted/40 py-4 flex flex-row items-center justify-between border-b">
                  <div className="flex items-center gap-2">
                    <FileCode2 className="h-5 w-5 text-primary" />
                    <CardTitle className="text-xl">{bpmnId}</CardTitle>
                  </div>
                  <div className="flex gap-2">
                    <Badge variant="secondary">{groupInstances.length} total</Badge>
                    {activeCount > 0 && <Badge variant="default" className="bg-yellow-500/20 text-yellow-700 hover:bg-yellow-500/30 border-yellow-500/50 dark:text-yellow-400">{activeCount} active</Badge>}
                  </div>
                </CardHeader>
                <CardContent className="p-0">
                  <div className="divide-y text-sm">
                    {groupInstances.map(inst => {
                      const def = defMap.get(inst.definition_key);
                      const varCount = Object.keys(inst.variables || {}).length;
                      const logCount = inst.audit_log?.length || 0;
                      
                      return (
                        <div
                          key={inst.id}
                          className="flex items-center justify-between p-4 hover:bg-accent/50 cursor-pointer transition-colors"
                          onClick={() => handleSelect(inst)}
                        >
                          <div className="flex gap-6 items-center flex-1">
                            <div className="w-[140px]">
                              <Badge className={cn(
                                "flex items-center justify-center gap-1.5 w-full",
                                stateBadgeClass(inst.state)
                              )}>
                                {inst.state === 'Running' && <Activity className="h-3 w-3" />}
                                {inst.state === 'Completed' && <CheckCircle className="h-3 w-3" />}
                                {typeof inst.state === 'object' && <Clock className="h-3 w-3" />}
                                {stateLabel(inst.state)}
                              </Badge>
                            </div>
                            <div className="flex flex-col gap-1">
                              <span className="font-semibold">
                                {inst.business_key || inst.id.substring(0, 8)} 
                                <span className="font-normal text-muted-foreground ml-2">(#{inst.id.substring(0, 8)})</span>
                              </span>
                              <span className="text-xs text-muted-foreground flex items-center gap-1.5">
                                <Network className="h-3 w-3" /> 
                                {inst.state === 'Completed' 
                                  ? <span className="italic">Process ended</span> 
                                  : inst.current_node}
                              </span>
                            </div>
                          </div>

                          <div className="flex items-center gap-3">
                            {def && (
                              <Badge variant="outline" className={cn("font-mono text-xs", def.is_latest ? "bg-blue-500/10 text-blue-700 border-blue-500/30 dark:text-blue-400" : "")} title={`ByKey: ${def.key}`}>
                                v{def.version}
                              </Badge>
                            )}
                            <Badge variant="secondary" className="flex items-center gap-1"><ScrollText className="h-3 w-3"/>{varCount}</Badge>
                            <Badge variant="secondary" className="flex items-center gap-1"><Activity className="h-3 w-3"/>{logCount}</Badge>
                            
                            <Button
                              variant="ghost"
                              size="icon"
                              className="text-destructive hover:bg-destructive/10 hover:text-destructive h-8 w-8 ml-2"
                              onClick={(e) => handleDeleteRequest(e, inst.id)}
                            >
                              <Trash className="h-4 w-4" />
                            </Button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </CardContent>
              </Card>
            );
          })}

          {unknownGroup.length > 0 && (
            <Card className="opacity-80">
               <CardHeader className="bg-muted py-3">
                  <CardTitle className="text-lg text-muted-foreground">Unknown Definitions</CardTitle>
                </CardHeader>
                <CardContent className="p-0">
                  <div className="divide-y text-sm">
                    {unknownGroup.map(inst => (
                       <div key={inst.id} className="flex items-center gap-6 p-4 hover:bg-accent/50 cursor-pointer" onClick={() => handleSelect(inst)}>
                         <Badge className={stateBadgeClass(inst.state)}>{stateLabel(inst.state)}</Badge>
                         <span className="font-medium">{inst.business_key || inst.id.substring(0, 8)}</span>
                         <span className="text-muted-foreground">{inst.definition_key.substring(0, 8)}…</span>
                       </div>
                    ))}
                  </div>
                </CardContent>
            </Card>
          )}
        </div>
      </ScrollArea>

      {/* Detail view overlay via full-screen dialog */}
      <Dialog open={!!selected} onOpenChange={(open) => !open && handleClose()}>
        <DialogContent className="max-w-[70vw] w-full max-h-[90vh] flex flex-col p-0 overflow-hidden bg-background">
          <DialogHeader className="px-6 py-4 border-b flex flex-row items-center justify-between sticky top-0 bg-background/95 backdrop-blur z-10 shrink-0">
            <DialogTitle className="text-xl">Instance Details: {selected?.id.substring(0, 8)}…</DialogTitle>
            <div className="flex gap-2 items-center !m-0">
              <Button variant="destructive" size="sm" className="gap-2" onClick={(e) => selected && handleDeleteRequest(e as any, selected.id)}>
                <Trash className="h-4 w-4" /> Delete
              </Button>
            </div>
          </DialogHeader>

          <div className="flex-1 p-6 overflow-y-auto min-h-0 relative">
            {detailLoading || !selected ? (
              <div className="text-center text-muted-foreground py-8">Loading instance context...</div>
            ) : (
              <div className="space-y-8">
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                  <Card className="p-4 flex flex-col gap-1.5 shadow-sm">
                    <span className="text-xs uppercase font-semibold text-muted-foreground">State</span>
                    <Badge className={cn("w-fit border-none", stateBadgeClass(selected.state))}>
                      {stateLabel(selected.state)}
                    </Badge>
                  </Card>
                  <Card className="p-4 flex flex-col gap-1.5 shadow-sm">
                    <span className="text-xs uppercase font-semibold text-muted-foreground">Business Key</span>
                    <span className="font-semibold text-base">{selected.business_key || 'None'}</span>
                  </Card>
                  <Card className="p-4 flex flex-col gap-1.5 shadow-sm">
                    <span className="text-xs uppercase font-semibold text-muted-foreground">Process ID</span>
                    <div className="flex items-center gap-2">
                       <span className="font-mono text-base font-semibold">{defMap.get(selected.definition_key)?.bpmn_id || selected.definition_key.substring(0, 8)}</span>
                       {defMap.get(selected.definition_key) && (
                         <Badge variant="outline">v{defMap.get(selected.definition_key)?.version}</Badge>
                       )}
                    </div>
                  </Card>
                </div>

                {definitionXml && (
                  <div className="space-y-4">
                    <h3 className="text-lg font-semibold border-b pb-2">Process Workflow</h3>
                    <ErrorBoundary>
                      <div className="border rounded-md bg-card overflow-hidden h-[400px]">
                        <InstanceViewer 
                          xml={definitionXml} 
                          activeNodeId={selected.current_node} 
                          onNodeClick={() => setShowNodeDetails((prev) => !prev)} 
                        />
                      </div>
                    </ErrorBoundary>
                    {!showNodeDetails && (
                      <p className="text-sm text-muted-foreground">
                        Click on the highlighted active node ({selected.current_node}) to view variables and state details.
                      </p>
                    )}
                  </div>
                )}

                {(!definitionXml || showNodeDetails) && (
                  <div className="space-y-6">
                    <ErrorBoundary>
                      <div className="bg-muted/30 border rounded-lg p-5">
                        <h3 className="text-lg font-semibold flex items-center gap-2 border-b pb-3 mb-4">
                          Node Context: <code className="text-primary bg-primary/10 px-1.5 py-0.5 rounded">{selected.current_node || 'Unknown'}</code>
                        </h3>

                        <div className="space-y-6">
                          {/* Pending user task info */}
                          {pendingTasks?.length > 0 && (
                            <div className="bg-background border rounded-md p-4">
                              <h4 className="font-semibold text-foreground mb-3">Assigned User Tasks:</h4>
                              <div className="space-y-3">
                                {pendingTasks.map(task => (
                                  <div key={task.task_id} className="bg-muted/50 p-3 rounded-md border text-sm">
                                    <div className="font-medium">Node: {task.node_id}</div>
                                    <div className="text-muted-foreground mt-1">Assignee: <span className="font-medium text-foreground">{task.assignee || 'Unassigned'}</span></div>
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}

                          {/* Pending service task info */}
                          {pendingServiceTasks?.length > 0 && (
                            <div className="bg-background border rounded-md p-4">
                              <h4 className="font-semibold text-foreground mb-3">Pending Service Tasks (Workers):</h4>
                              <div className="space-y-3">
                                {pendingServiceTasks.map((task, index) => (
                                  <div key={task?.id || `fallback-${index}`} className="bg-muted/50 p-3 rounded-md border text-sm">
                                    <div className="font-medium">Node: {task?.node_id}</div>
                                    <div className="mt-1">Topic: <Badge variant="secondary" className="font-mono">{task?.topic}</Badge></div>
                                    <div className="text-muted-foreground mt-2">
                                      Worker: {task?.worker_id || 'Unlocked'} &middot; Retries: {task?.retries}
                                    </div>
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}

                          {/* Execution History Timeline */}
                          <div>
                            <h4 className="font-semibold text-foreground mb-3">Execution History:</h4>
                            <div className="bg-background border rounded-md p-4">
                              <HistoryTimeline instanceId={selected.id} refreshTrigger={historyRefreshTrigger} />
                            </div>
                          </div>

                          {/* Editable variables */}
                          <div>
                            <h4 className="font-semibold text-foreground mb-3">Variables:</h4>
                            <div className="bg-background border rounded-md p-4">
                              <VariableEditor
                                variables={variables}
                                onChange={setVariables}
                                readOnlyNames={true}
                                deletedKeys={deletedKeys}
                                onDeletedKeysChange={setDeletedKeys}
                                instanceId={selected.id}
                                onVariablesRefreshRequest={() => handleSelect(selected)}
                              />
                              <div className="mt-4 pt-4 border-t flex justify-end">
                                <Button onClick={handleSaveVariables}>Save Variables</Button>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </ErrorBoundary>
                  </div>
                )}
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
      <AlertDialog open={!!instanceToDelete} onOpenChange={open => !open && setInstanceToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Instance</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this process instance? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete} className="bg-destructive hover:bg-destructive/90 text-destructive-foreground">Delete</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
