import { useState, useEffect, useRef } from 'react';
import { getPendingServiceTasks, type PendingServiceTask } from './lib/tauri';
import { useToast } from './ToastContext';
import { AlertTriangle, RefreshCw, ExternalLink } from 'lucide-react';

export function IncidentsView({ onViewInstance }: { onViewInstance?: (id: string) => void }) {
  const toast = useToast();
  const [incidents, setIncidents] = useState<PendingServiceTask[]>([]);
  const [loading, setLoading] = useState(true);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchIncidents = async () => {
    try {
      const all = await getPendingServiceTasks();
      // Filter: incidents = service tasks with retries <= 0
      setIncidents(all.filter(t => t.retries <= 0));
      setLoading(false);
    } catch (e) { 
      toast.error('Failed to load incidents: ' + e); 
    }
  };

  useEffect(() => {
    fetchIncidents();
    intervalRef.current = setInterval(fetchIncidents, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  return (
    <div>
      <h2>🚨 Incidents</h2>
      <div className="header-actions">
        <button className="button" onClick={fetchIncidents}>
          <RefreshCw size={16} /> Refresh
        </button>
        <span className="auto-refresh-label">Auto-refreshing</span>
      </div>

      {loading && <div style={{ margin: 20 }}>Loading...</div>}
      
      {!loading && incidents.length === 0 && (
        <div className="card" style={{ textAlign: 'center', color: '#16a34a' }}>
          <AlertTriangle size={32} /> No incidents — all systems operational.
        </div>
      )}

      {incidents.map(inc => (
        <div key={inc.id} className="card incident-card">
          <div className="card-title" style={{ color: '#dc2626' }}>
            <AlertTriangle size={18} /> {inc.node_id}
          </div>
          <div className="incident-meta">
            <span>Topic: <strong>{inc.topic}</strong></span>
            <span>Instance: <code>{inc.instance_id.substring(0, 8)}</code></span>
          </div>
          <div className="incident-error">
            <strong>Error:</strong> {inc.error_message || 'No error message'}
          </div>
          {inc.error_details && (
            <pre className="incident-details">{inc.error_details}</pre>
          )}
          <div className="incident-actions">
            {onViewInstance && (
              <button className="button button-secondary" 
                onClick={() => onViewInstance(inc.instance_id)}>
                <ExternalLink size={14} /> View Instance
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
