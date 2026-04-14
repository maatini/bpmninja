import { useState, useEffect, useCallback } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { listDefinitions, getDefinitionXml, listInstances, deleteDefinition, deleteAllDefinitions, type DefinitionInfo, type ProcessInstance } from '../../shared/lib/tauri';
import { RefreshCw, Eye, Download, Activity, Clock, Trash, FileCode2, Database, ChevronRight, ChevronDown } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';
import { cn } from '@/lib/utils';

function groupByProcess(defs: DefinitionInfo[]): Map<string, DefinitionInfo[]> {
  const map = new Map<string, DefinitionInfo[]>();
  for (const d of defs) {
    const existing = map.get(d.bpmn_id) || [];
    existing.push(d);
    map.set(d.bpmn_id, existing);
  }
  for (const [, versions] of map) {
    versions.sort((a, b) => b.version - a.version);
  }
  return map;
}

export function DeployedProcessesPage({ onView, onViewInstance, onViewDefinition }: { onView: (xml: string) => void, onViewInstance?: (id: string) => void, onViewDefinition?: (key: string) => void }) {
  const { toast } = useToast();
  const [definitions, setDefinitions] = useState<DefinitionInfo[]>([]);
  const [instances, setInstances] = useState<ProcessInstance[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [viewingId, setViewingId] = useState<string | null>(null);
  const [deleteRequest, setDeleteRequest] = useState<{defId: string, bpmnId?: string, isAll: boolean, cascade: boolean, msg: string} | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const toggleExpanded = (bpmnId: string) => {
    setExpanded(prev => {
      const next = new Set(prev);
      if (next.has(bpmnId)) next.delete(bpmnId);
      else next.add(bpmnId);
      return next;
    });
  };
  
  const fetchDefinitions = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [list, instList] = await Promise.all([listDefinitions(), listInstances()]);
      setDefinitions(list);
      setInstances(instList);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchDefinitions();
  }, [fetchDefinitions]);

  const handleDownload = async (defId: string) => {
    setDownloading(defId);
    try {
      const xml = await getDefinitionXml(defId);
      const filePath = await save({
        defaultPath: `definition-${defId.substring(0, 8)}.bpmn`,
        filters: [{ name: 'BPMN', extensions: ['bpmn', 'xml'] }],
      });
      if (filePath) {
        await writeTextFile(filePath, xml);
      }
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Download failed: ' + e });
    } finally {
      setDownloading(null);
    }
  };

  const handleView = async (defId: string) => {
    setViewingId(defId);
    try {
      const xml = await getDefinitionXml(defId);
      onView(xml);
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Failed to load definition: ' + e });
    } finally {
      setViewingId(null);
    }
  };

  const handleDeleteCheck = (defId: string) => {
    const relatedInstances = instances.filter(i => i.definition_key === defId);
    let cascade = false;
    let msg = "Are you sure you want to delete this process definition version?";

    if (relatedInstances.length > 0) {
      msg = `This version has ${relatedInstances.length} associated instance(s). Deleting it will also permanently delete all associated instances.\n\nAre you sure?`;
      cascade = true;
    }
    setDeleteRequest({ defId, isAll: false, cascade, msg });
  };

  const handleDeleteAllCheck = (bpmnId: string, versions: DefinitionInfo[]) => {
    const versionKeys = versions.map(v => v.key);
    const relatedInstances = instances.filter(i => versionKeys.includes(i.definition_key));
    let cascade = false;
    
    const versionListInfo = versions.length > 1 
      ? `\n\nVersions to be deleted: ${versions.length}` 
      : '';

    let msg = `Are you sure you want to delete ALL versions of process "${bpmnId}"?${versionListInfo}`;

    if (relatedInstances.length > 0) {
      msg = `Process "${bpmnId}" has ${relatedInstances.length} associated instance(s) across all versions.${versionListInfo}\n\nDeleting the entire deployment will also permanently delete all associated instances.\n\nAre you absolutely sure?`;
      cascade = true;
    }
    setDeleteRequest({ defId: '', bpmnId, isAll: true, cascade, msg });
  };

  const confirmDelete = async () => {
    if (!deleteRequest) return;
    try {
      if (deleteRequest.isAll && deleteRequest.bpmnId) {
        await deleteAllDefinitions(deleteRequest.bpmnId, deleteRequest.cascade);
      } else {
        await deleteDefinition(deleteRequest.defId, deleteRequest.cascade);
      }
      fetchDefinitions();
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Delete failed: ' + e });
    } finally {
      setDeleteRequest(null);
    }
  };

  const grouped = groupByProcess(definitions);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
        <h2 className="text-2xl font-bold tracking-tight">Deployed Processes</h2>
        <Button onClick={fetchDefinitions} variant="outline" size="sm" className="gap-2">
          <RefreshCw className="h-4 w-4" /> Refresh
        </Button>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-6">
          {loading && (
            <div className="space-y-3">
              {[1, 2, 3].map(i => (
                <Card key={i} className="p-4">
                  <div className="flex items-center gap-4">
                    <Skeleton className="h-6 w-6 rounded" />
                    <Skeleton className="h-5 w-[200px]" />
                    <Skeleton className="h-5 w-[100px]" />
                  </div>
                </Card>
              ))}
            </div>
          )}

          {error && <div className="text-destructive font-medium">Error: {error}</div>}

          {!loading && !error && grouped.size === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-center">
              <Database className="h-16 w-16 text-muted-foreground/30 mb-4" />
              <h3 className="text-lg font-semibold text-muted-foreground">No Deployed Processes</h3>
              <p className="text-sm text-muted-foreground/70 mt-1 max-w-sm">
                Deploy a BPMN process from the Modeler to see it listed here.
              </p>
            </div>
          )}

          {!loading && !error && grouped.size > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[32px]" />
                  <TableHead>Process</TableHead>
                  <TableHead className="w-[110px]">Latest</TableHead>
                  <TableHead className="w-[90px]">Versions</TableHead>
                  <TableHead className="w-[120px]">Active</TableHead>
                  <TableHead className="w-[90px]">Nodes</TableHead>
                  <TableHead className="w-[260px] text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              {[...grouped.entries()].map(([bpmnId, versions]) => {
                const latest = versions[0];
                const olderVersions = versions.slice(1);
                const instancesForProcess = instances.filter(i => versions.some(v => v.key === i.definition_key) && i.state !== 'Completed');
                const isOpen = expanded.has(bpmnId);
                const hasDetails = olderVersions.length > 0 || instancesForProcess.length > 0;

                return (
                  <TableBody key={bpmnId} className="process-group-card">
                    <TableRow className="hover:bg-muted/40">
                        <TableCell>
                          {hasDetails ? (
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-6 w-6"
                              onClick={() => toggleExpanded(bpmnId)}
                              title={isOpen ? 'Collapse' : 'Expand'}
                            >
                              {isOpen ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                            </Button>
                          ) : null}
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <FileCode2 className="h-4 w-4 text-primary flex-shrink-0" />
                            <span className="font-semibold">{bpmnId}</span>
                            <span className="text-xs text-muted-foreground font-mono ml-1">{latest.key.substring(0, 8)}…</span>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge className="bg-blue-500/10 text-blue-700 border-blue-500/30 dark:text-blue-400 hover:bg-blue-500/20 font-mono text-xs">
                            v{latest.version}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" className="font-mono text-xs">{versions.length}</Badge>
                        </TableCell>
                        <TableCell>
                          {instancesForProcess.length > 0 ? (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-7 px-2 gap-1.5 text-amber-700 dark:text-amber-400 hover:bg-amber-500/10"
                              onClick={() => toggleExpanded(bpmnId)}
                            >
                              <Activity className="h-3.5 w-3.5" /> {instancesForProcess.length} active
                            </Button>
                          ) : (
                            <span className="text-xs text-muted-foreground">—</span>
                          )}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">{latest.node_count}</TableCell>
                        <TableCell>
                          <div className="flex gap-1 justify-end">
                            {onViewDefinition && (
                              <Button onClick={() => onViewDefinition(latest.key)} size="sm" className="gap-1.5 h-8" title="Live View">
                                <Activity className="h-3.5 w-3.5" /> Live
                              </Button>
                            )}
                            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => handleView(latest.key)} disabled={viewingId === latest.key} title="View BPMN">
                              <Eye className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => handleDownload(latest.key)} disabled={downloading === latest.key} title="Download BPMN">
                              <Download className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="icon" className="text-destructive hover:bg-destructive/10 hover:text-destructive h-8 w-8" onClick={() => handleDeleteCheck(latest.key)} title="Delete latest version">
                              <Trash className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="icon" className="text-destructive hover:bg-destructive/10 hover:text-destructive h-8 w-8" onClick={() => handleDeleteAllCheck(bpmnId, versions)} title="Delete all versions">
                              <Trash className="h-4 w-4" strokeWidth={2.5} />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>

                    {isOpen && hasDetails && (
                      <TableRow key={bpmnId + '-detail'} className="bg-muted/20 hover:bg-muted/20">
                          <TableCell />
                          <TableCell colSpan={6} className="py-4">
                            <div className="space-y-4">
                              {olderVersions.length > 0 && (
                                <div>
                                  <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                                    Older Versions
                                  </div>
                                  <div className="divide-y border rounded-md bg-background">
                                    {olderVersions.map(ver => {
                                      const verInstances = instances.filter(i => i.definition_key === ver.key && i.state !== 'Completed');
                                      return (
                                        <div key={ver.key} className="flex items-center justify-between py-2 px-3 text-sm">
                                          <div className="flex items-center gap-3">
                                            <Badge variant="secondary" className="font-mono text-xs">v{ver.version}</Badge>
                                            <span className="text-muted-foreground font-mono text-xs">{ver.key.substring(0, 8)}…</span>
                                            <span className="text-muted-foreground text-xs">{ver.node_count} nodes</span>
                                            {verInstances.length > 0 && (
                                              <Badge variant="outline" className="bg-amber-500/10 text-amber-600 border-amber-500/20 text-xs">{verInstances.length} active</Badge>
                                            )}
                                          </div>
                                          <div className="flex gap-1">
                                            {onViewDefinition && (
                                              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => onViewDefinition(ver.key)} title="Live View">
                                                <Activity className="h-3.5 w-3.5" />
                                              </Button>
                                            )}
                                            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => handleView(ver.key)} title="View BPMN">
                                              <Eye className="h-3.5 w-3.5" />
                                            </Button>
                                            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => handleDownload(ver.key)} title="Download BPMN">
                                              <Download className="h-3.5 w-3.5" />
                                            </Button>
                                            <Button variant="ghost" size="icon" className="text-destructive hover:bg-destructive/10 hover:text-destructive h-7 w-7" onClick={() => handleDeleteCheck(ver.key)} title="Delete version">
                                              <Trash className="h-3.5 w-3.5" />
                                            </Button>
                                          </div>
                                        </div>
                                      );
                                    })}
                                  </div>
                                </div>
                              )}

                              {instancesForProcess.length > 0 && (
                                <div>
                                  <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 flex items-center gap-1.5">
                                    <Activity className="h-3.5 w-3.5 text-green-500" /> Active Instances ({instancesForProcess.length})
                                  </div>
                                  <div className="divide-y border rounded-md bg-background">
                                    {instancesForProcess.map(inst => {
                                      const instDef = versions.find(v => v.key === inst.definition_key);
                                      return (
                                        <div
                                          key={inst.id}
                                          className="instance-list-item flex items-center justify-between py-2 px-3 hover:bg-accent/50 cursor-pointer transition-colors text-sm"
                                          onClick={() => onViewInstance?.(inst.id)}
                                        >
                                          <div className="flex items-center gap-2">
                                            {inst.state === 'Running' ? <Activity className="h-3.5 w-3.5 text-green-500" /> : <Clock className="h-3.5 w-3.5 text-amber-500" />}
                                            <span className="font-medium">{inst.business_key || inst.id.substring(0, 8)}</span>
                                          </div>
                                          <div className="flex gap-2 items-center">
                                            {instDef && (
                                              <Badge variant={instDef.is_latest ? 'default' : 'secondary'} className={cn('font-mono text-xs', instDef.is_latest ? 'bg-blue-500/10 text-blue-700 border-blue-500/30 dark:text-blue-400' : '')}>
                                                v{instDef.version}
                                              </Badge>
                                            )}
                                            <Badge variant="outline" className="text-xs">Current: {inst.current_node}</Badge>
                                          </div>
                                        </div>
                                      );
                                    })}
                                  </div>
                                </div>
                              )}
                            </div>
                          </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                );
              })}
            </Table>
          )}
        </div>
      </ScrollArea>
      
      <AlertDialog open={!!deleteRequest} onOpenChange={open => !open && setDeleteRequest(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Confirm Deletion</AlertDialogTitle>
            <AlertDialogDescription className="whitespace-pre-wrap">
              {deleteRequest?.msg}
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
