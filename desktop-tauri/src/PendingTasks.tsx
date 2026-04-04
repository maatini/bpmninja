import { useState, useEffect, useRef } from 'react';
import { getPendingTasks, getPendingServiceTasks, completeTask, 
         fetchAndLockServiceTasks, completeServiceTask, 
         type PendingUserTask, type PendingServiceTask } from './lib/tauri';
import { useToast } from './ToastContext';
import { VariableEditor, type VariableRow, serializeVariables } from './VariableEditor';
import { RefreshCw } from 'lucide-react';

export function PendingTasks() {
  const toast = useToast();
  const [tasks, setTasks] = useState<PendingUserTask[]>([]);
  const [serviceTasks, setServiceTasks] = useState<PendingServiceTask[]>([]);
  
  // Complete dialog state
  const [completingTask, setCompletingTask] = useState<PendingUserTask | null>(null);
  const [completeVars, setCompleteVars] = useState<VariableRow[]>([]);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchTasks = async () => {
    try {
      const [pending, pendingServices] = await Promise.all([
        getPendingTasks(),
        getPendingServiceTasks()
      ]);
      setTasks(pending);
      setServiceTasks(pendingServices);
    } catch (e) {
      console.error("Failed to fetch tasks", e);
    }
  };

  useEffect(() => {
    fetchTasks();
    intervalRef.current = setInterval(fetchTasks, 3000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  const handleCompleteClick = (task: PendingUserTask) => {
    setCompletingTask(task);
    setCompleteVars([]);  // Empty — user adds vars as needed
  };

  const handleCompleteConfirm = async () => {
    if (!completingTask) return;
    const vars = serializeVariables(completeVars);
    if (vars === null) {
      toast.error('Invalid variables format. Please check JSON or Numbers.');
      return; // Validation failed
    }

    try {
      await completeTask(completingTask.task_id, vars);
      toast.success('Task completed!');
      setCompletingTask(null);
      fetchTasks();
    } catch (e) {
      toast.error('Error: ' + e);
    }
  };

  const handleCompleteServiceTask = async (task: PendingServiceTask) => {
    try {
      if (!task.worker_id) {
        // Automatically fetch and lock the specific task's topic first
        const lockedTasks = await fetchAndLockServiceTasks("tauri-ui", 10, task.topic, 5000)
        if (!lockedTasks.some(t => t.id === task.id)) {
          toast.error("Could not lock task! It might have been acquired by another worker.")
          fetchTasks()
          return
        }
      } else if (task.worker_id !== "tauri-ui") {
        toast.error("Task is currently locked by another worker: " + task.worker_id)
        return
      }

      await completeServiceTask(task.id, "tauri-ui")
      fetchTasks()
      toast.success("Service Task completed!")
    } catch (e) {
      toast.error("Error completing service task: " + e)
    }
  }

  return (
    <div>
      <h2>Pending Tasks</h2>
      <div className="header-actions">
        <button className="button" onClick={fetchTasks} style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <RefreshCw size={16} /> Refresh
        </button>
        <span style={{ fontSize: '0.75rem', color: '#94a3b8', display: 'flex', alignItems: 'center', marginLeft: 'auto' }}>
          Auto-refreshing
        </span>
      </div>

      <h3 style={{ marginTop: 24, marginBottom: 16 }}>User Tasks</h3>
      {tasks.length === 0 && <div style={{marginLeft: 20}}>No pending user tasks.</div>}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '16px' }}>
        {tasks.map(task => (
          <div key={task.task_id} className="card">
            <div className="card-title">Task: {task.node_id}</div>
            <div>Assignee: {task.assignee}</div>
            <div>Instance: {task.instance_id}</div>
            <div style={{marginTop: 10}}>
              <button className="button" onClick={() => handleCompleteClick(task)}>Complete Task</button>
            </div>
          </div>
        ))}
      </div>

      <h3 style={{ marginTop: 32, marginBottom: 16 }}>Service Tasks (External)</h3>
      {serviceTasks.length === 0 && <div style={{marginLeft: 20}}>No pending service tasks.</div>}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '16px' }}>
        {serviceTasks.map(task => (
          <div key={task.id} className="card" style={{ borderLeft: '4px solid #8b5cf6' }}>
            <div className="card-title" style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span>Task: {task.node_id}</span>
              <span style={{ fontSize: '0.8rem', padding: '2px 8px', background: '#eedeff', color: '#6d28d9', borderRadius: '12px', fontWeight: 600 }}>
                Topic: {task.topic}
              </span>
            </div>
            <div>Worker: {task.worker_id || <span style={{ color: '#94a3b8', fontStyle: 'italic' }}>Unlocked</span>}</div>
            <div>Instance: {task.instance_id}</div>
            <div>Retries left: {task.retries}</div>
            {task.error_message && (
              <div style={{ color: '#ef4444', fontSize: '0.9rem', marginTop: 4 }}>Error: {task.error_message}</div>
            )}
            <div style={{marginTop: 10}}>
              <button className="button" style={{ background: '#8b5cf6' }} onClick={() => handleCompleteServiceTask(task)}>
                Complete as 'tauri-ui'
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* Complete Task Dialog */}
      {completingTask && (
        <div className="vars-dialog-overlay" onClick={() => setCompletingTask(null)}>
          <div className="vars-dialog" onClick={e => e.stopPropagation()}>
            <h3 style={{ margin: '0 0 4px' }}>Complete Task: {completingTask.node_id}</h3>
            <p style={{ color: '#64748b', fontSize: '0.9rem' }}>
              Add output variables (optional):
            </p>
            <VariableEditor
              variables={completeVars}
              onChange={setCompleteVars}
            />
            <div className="vars-dialog-actions" style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px', marginTop: '12px' }}>
              <button className="button" onClick={() => setCompletingTask(null)} 
                style={{ backgroundColor: '#6b7280' }}>Cancel</button>
              <button className="button save-vars-btn" onClick={handleCompleteConfirm} style={{ backgroundColor: '#10b981' }}>
                Complete Task
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
