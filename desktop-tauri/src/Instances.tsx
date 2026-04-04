import { useState, useEffect, useCallback } from 'react';
import { listInstances, getInstanceDetails, getPendingTasks, getPendingServiceTasks, updateInstanceVariables, getDefinitionXml, deleteInstance, listDefinitions, type ProcessInstance, type PendingUserTask, type PendingServiceTask, type DefinitionInfo } from './lib/tauri';
import { InstanceViewer } from './InstanceViewer';
import { RefreshCw, Activity, CheckCircle, Clock, Trash, FileCode2, Network, ScrollText } from 'lucide-react';
import { VariableEditor, type VariableRow, parseVariables, serializeVariables } from './VariableEditor';
import { HistoryTimeline } from './HistoryTimeline';
import { ErrorBoundary } from './ErrorBoundary';
import { useToast } from './ToastContext';

// Helper to render the instance state as a readable string
function stateLabel(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'Running';
  if (state === 'Completed') return 'Completed';
  if (typeof state === 'object' && 'WaitingOnUserTask' in state) return 'Wait: User Task';
  if (typeof state === 'object' && 'WaitingOnServiceTask' in state) return 'Wait: Service Task';
  if (typeof state === 'object' && 'WaitingOnTimer' in state) return 'Wait: Timer';
  if (typeof state === 'object' && 'WaitingOnMessage' in state) return 'Wait: Message';
  return String(state);
}

// Helper to pick a CSS class for the state badge
function stateBadgeClass(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'state-badge state-running';
  if (state === 'Completed') return 'state-badge state-completed';
  if (typeof state === 'object' && 'WaitingOnServiceTask' in state) return 'state-badge state-waiting state-service-task';
  if (typeof state === 'object' && 'WaitingOnTimer' in state) return 'state-badge state-waiting';
  if (typeof state === 'object' && 'WaitingOnMessage' in state) return 'state-badge state-waiting';
  return 'state-badge state-waiting';
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

  // Sort each group so running processes are at the top, then by instance id roughly
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
  const toast = useToast();
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

  // Variables state is typed using VariableRow from VariableEditor
  const [definitionXml, setDefinitionXml] = useState<string | null>(null);
  const [showNodeDetails, setShowNodeDetails] = useState(true);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [instList, defList] = await Promise.all([listInstances(), listDefinitions()]);
      setInstances(instList);
      setDefinitions(defList);
      setHistoryRefreshTrigger(prev => prev + 1);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    if (selected) return; // Don't poll while detail view is open
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
      // If waiting on user task, fetch pending tasks to show info
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
    } catch (e) {
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
      toast.error('Invalid variables format (check JSON or Numbers)');
      return;
    }

    try {
      await updateInstanceVariables(selected.id, varsToSave);
      toast.success('Variables saved successfully.');
      const updated = await getInstanceDetails(selected.id);
      setSelected(updated);
      setVariables(parseVariables(updated.variables));
      setDeletedKeys(new Set());
      setHistoryRefreshTrigger(prev => prev + 1);
    } catch (e) {
      toast.error('Error saving variables: ' + e);
    }
  };

  const handleClose = () => {
    setSelected(null);
    setPendingTasks([]);
    setPendingServiceTasks([]);
    setDefinitionXml(null);
    setShowNodeDetails(true);
  };

  const handleDelete = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    if (!window.confirm("Are you sure you want to delete this process instance?")) return;
    try {
      await deleteInstance(id);
      if (selected?.id === id) {
        handleClose();
      }
      fetchData();
    } catch (err) {
      toast.error("Failed to delete instance: " + err);
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
    <div>
      <h2>Instances</h2>
      <div className="header-actions">
        <button className="button" onClick={fetchData} style={{ display: 'flex', alignItems: 'center', gap: '6px' }}><RefreshCw size={16} /> Refresh</button>
        <span style={{ fontSize: '0.75rem', color: '#94a3b8', display: 'flex', alignItems: 'center', marginLeft: 'auto' }}>
          Auto-refreshing
        </span>
      </div>

      {loading && <div style={{ margin: 20 }}>Loading instances...</div>}
      {error && <div style={{ margin: 20, color: '#dc2626' }}>Error: {error}</div>}
      {!loading && !error && instances.length === 0 && (
        <div style={{ margin: 20 }}>No instances found.</div>
      )}

      <div style={{ padding: '0 20px' }}>
        {[...groups.entries()].map(([bpmnId, groupInstances]) => {
          const activeCount = groupInstances.filter(i => i.state !== 'Completed').length;
          
          return (
            <div key={bpmnId} className="process-group-card" style={{ margin: '20px 0' }}>
              <div className="process-group-header">
                <div className="process-title" style={{ fontSize: '1.2rem' }}>
                  <FileCode2 size={20} color="#2563eb" /> {bpmnId}
                </div>
                <div className="process-stats">
                  <span className="stat-pill">{groupInstances.length} total</span>
                  {activeCount > 0 && <span className="stat-pill highlight">{activeCount} active</span>}
                </div>
              </div>
              <div className="instance-group-body" style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                {groupInstances.map(inst => {
                  const def = defMap.get(inst.definition_key);
                  const varCount = Object.keys(inst.variables || {}).length;
                  const logCount = inst.audit_log?.length || 0;
                  
                  return (
                    <div
                      key={inst.id}
                      className="instance-list-item"
                      style={{ margin: 0 }}
                      onClick={() => handleSelect(inst)}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '16px', flex: 1 }}>
                        <div style={{ minWidth: '130px' }}>
                          <span className={stateBadgeClass(inst.state)} style={{ width: '100%', textAlign: 'center', boxSizing: 'border-box' }}>
                            {inst.state === 'Running' && <Activity size={10} style={{marginRight: 4}} />}
                            {inst.state === 'Completed' && <CheckCircle size={10} style={{marginRight: 4}} />}
                            {typeof inst.state === 'object' && <Clock size={10} style={{marginRight: 4}} />}
                            {stateLabel(inst.state)}
                          </span>
                        </div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                          <span style={{ fontWeight: 600, color: 'var(--text-color)' }}>
                            {inst.business_key || inst.id.substring(0, 8)} 
                            <span style={{ fontWeight: 'normal', color: 'var(--text-muted)', marginLeft: '8px' }}>(#{inst.id.substring(0, 8)})</span>
                          </span>
                          <span style={{ fontSize: '0.8rem', color: '#64748b' }}>
                            <Network size={12} style={{ display: 'inline', verticalAlign: 'text-bottom' }} /> {inst.current_node}
                          </span>
                        </div>
                      </div>

                      <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                        {def && (
                          <span className={`version-pill ${def.is_latest ? 'latest' : 'older'}`} title={`Definition Key: ${def.key}`}>
                            v{def.version}
                          </span>
                        )}
                        <span className="stat-pill" title={`${varCount} variables active`}><ScrollText size={12} style={{ display: 'inline', verticalAlign: 'text-bottom' }} /> {varCount}</span>
                        <span className="stat-pill" title={`${logCount} audit entries`}><Activity size={12} style={{ display: 'inline', verticalAlign: 'text-bottom' }} /> {logCount}</span>
                        
                        <button
                          className="delete-icon-btn"
                          style={{ background: 'transparent', border: '1px solid transparent', borderRadius: '4px', marginLeft: '8px' }}
                          onClick={(e) => handleDelete(e, inst.id)}
                          title="Delete Instance"
                        >
                          <Trash size={16} />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}

        {unknownGroup.length > 0 && (
          <div className="process-group-card" style={{ margin: '20px 0', opacity: 0.8 }}>
             <div className="process-group-header">
                <div className="process-title" style={{ fontSize: '1.2rem', color: '#64748b' }}>
                  Unknown Definitions
                </div>
              </div>
              <div className="instance-group-body" style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                {unknownGroup.map(inst => (
                   <div key={inst.id} className="instance-list-item" style={{ margin: 0 }} onClick={() => handleSelect(inst)}>
                     <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                       <span className={stateBadgeClass(inst.state)}>{stateLabel(inst.state)}</span>
                       <span style={{ fontWeight: 600 }}>{inst.business_key || inst.id.substring(0, 8)}</span>
                     </div>
                     <div>{inst.definition_key.substring(0, 8)}…</div>
                   </div>
                ))}
              </div>
          </div>
        )}
      </div>

      {/* Detail view overlay */}
      {selected && (
        <div className="detail-overlay" style={{ overflowY: 'auto' }}>
          <div className="instance-detail card" style={{ maxWidth: '900px', width: '100%', margin: '20px auto' }}>
            <div className="detail-header" style={{ marginBottom: '16px' }}>
              <span className="process-title">Instance: {selected.id.substring(0, 8)}…</span>
              <div style={{ display: 'flex', gap: '8px' }}>
                <button className="button" onClick={(e) => handleDelete(e, selected.id)} style={{ background: 'var(--danger-color)', fontSize: '0.85rem', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <Trash size={14} /> Delete
                </button>
                <button className="button button-secondary" onClick={handleClose} style={{ fontSize: '0.85rem', padding: '6px 16px' }}>Close View</button>
              </div>
            </div>

            {detailLoading ? (
              <div style={{ padding: '20px', textAlign: 'center', color: 'var(--text-muted)' }}>Loading instance context...</div>
            ) : (
              <>
                <div className="info-grid">
                  <div className="info-grid-cell">
                    <div className="info-grid-label">State</div>
                    <span className={stateBadgeClass(selected.state)}>{stateLabel(selected.state)}</span>
                  </div>
                  <div className="info-grid-cell">
                    <div className="info-grid-label">Business Key</div>
                    <span style={{ fontWeight: 600 }}>{selected.business_key || 'None'}</span>
                  </div>
                  <div className="info-grid-cell">
                    <div className="info-grid-label">Process ID</div>
                    <span style={{ fontWeight: 600, fontFamily: 'monospace' }}>{defMap.get(selected.definition_key)?.bpmn_id || selected.definition_key.substring(0, 8)}</span>
                    {defMap.get(selected.definition_key) && (
                      <span className="version-pill older" style={{ marginLeft: '8px' }}>v{defMap.get(selected.definition_key)?.version}</span>
                    )}
                  </div>
                </div>

                {definitionXml && (
                  <div style={{ marginBottom: 24, padding: '0 20px' }}>
                    <h3 style={{ fontSize: '1rem', color: 'var(--text-color)', marginBottom: '12px' }}>Process Workflow</h3>
                    <ErrorBoundary>
                      <InstanceViewer 
                        xml={definitionXml} 
                        activeNodeId={selected.current_node} 
                        onNodeClick={() => setShowNodeDetails((prev) => !prev)} 
                      />
                    </ErrorBoundary>
                    {!showNodeDetails && (
                      <div style={{ fontSize: '0.85rem', color: 'var(--text-muted)', marginTop: '8px' }}>
                        Click on the highlighted active node ({selected.current_node}) to view variables and state details.
                      </div>
                    )}
                  </div>
                )}

                {(!definitionXml || showNodeDetails) && (
                  <div style={{ padding: '0 20px 20px 20px' }}>
                  <ErrorBoundary>
                  <div className="node-context-panel">
                    <h3 style={{ fontSize: '1rem', color: 'var(--text-color)', margin: '0 0 16px 0', borderBottom: '1px solid var(--border-color)', paddingBottom: '8px' }}>
                      Node Context: <span style={{ fontFamily: 'monospace', color: 'var(--primary-color)' }}>{selected?.current_node || 'Unknown'}</span>
                    </h3>

                    {/* Pending user task info */}
                    {pendingTasks?.length > 0 && (
                      <div className="context-section">
                        <strong style={{ color: 'var(--text-strong)' }}>Assigned User Tasks:</strong>
                        {pendingTasks.map(task => (
                          <div key={task.task_id} style={{ marginTop: 8, padding: '8px', background: 'var(--bg-subtle)', borderRadius: '4px' }}>
                            <span style={{ fontWeight: 500, color: 'var(--text-color)' }}>Node: {task.node_id}</span>
                            <br/><span style={{ fontSize: '0.9rem', color: 'var(--text-muted)' }}>Assignee: {task.assignee || 'Unassigned'}</span>
                          </div>
                        ))}
                      </div>
                    )}

                    {/* Pending service task info */}
                    {pendingServiceTasks?.length > 0 && (
                      <div className="context-section">
                        <strong style={{ color: 'var(--text-strong)' }}>Pending Service Tasks (Workers):</strong>
                        {pendingServiceTasks.map((task, index) => (
                          <div key={task?.id || `fallback-${index}`} style={{ marginTop: 8, padding: '8px', background: 'var(--bg-subtle)', borderRadius: '4px' }}>
                            <span style={{ fontWeight: 500, color: 'var(--text-color)' }}>Node: {task?.node_id}</span>
                            <br/>Topic: <span style={{ fontWeight: 600, color: 'var(--primary-color)' }}>{task?.topic}</span>
                            <div style={{ fontSize: '0.85em', color: 'var(--text-muted)', marginTop: '4px' }}>
                              Worker ID: {task?.worker_id || 'Unlocked'} · Remaining Retries: {task?.retries}
                            </div>
                          </div>
                        ))}
                      </div>
                    )}

                    {/* Execution History Timeline */}
                    <div style={{ marginBottom: 20 }}>
                      <strong style={{ display: 'block', marginBottom: '12px', color: 'var(--text-strong)' }}>Execution History:</strong>
                      <div className="context-section">
                        <HistoryTimeline instanceId={selected.id} refreshTrigger={historyRefreshTrigger} />
                      </div>
                    </div>

                    {/* Editable variables */}
                    <div style={{ marginTop: 16 }}>
                      <strong style={{ display: 'block', marginBottom: '12px', color: 'var(--text-strong)' }}>Variables:</strong>
                      <div className="context-section">
                        <VariableEditor
                          variables={variables}
                          onChange={setVariables}
                          readOnlyNames={true}
                          deletedKeys={deletedKeys}
                          onDeletedKeysChange={setDeletedKeys}
                          instanceId={selected.id}
                          onVariablesRefreshRequest={() => handleSelect(selected)}
                        />
                        <div style={{ marginTop: 16, display: 'flex', justifyContent: 'flex-end', borderTop: '1px solid var(--border-color)', paddingTop: '16px' }}>
                          <button className="button save-vars-btn" onClick={handleSaveVariables} style={{ padding: '8px 24px', fontSize: '0.95rem' }}>
                            Save Variables
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                  </ErrorBoundary>
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
