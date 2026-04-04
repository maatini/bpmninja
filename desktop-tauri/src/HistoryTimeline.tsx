import { useState, useEffect } from 'react';
import { getInstanceHistory, type HistoryEntry, type HistoryQuery } from './lib/tauri';
import { 
  Play, CheckCircle, Activity, Settings, 
  XCircle, Filter, Camera, ArrowRightCircle, X
} from 'lucide-react';

interface HistoryTimelineProps {
  instanceId: string;
  refreshTrigger?: number;
}

export function HistoryTimeline({ instanceId, refreshTrigger = 0 }: HistoryTimelineProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [eventTypes, setEventTypes] = useState<string>('');
  const [actorTypes, setActorTypes] = useState<string>('');

  // Dialog State
  const [selectedEntry, setSelectedEntry] = useState<HistoryEntry | null>(null);

  const fetchHistory = async () => {
    setLoading(true);
    setError(null);
    try {
      const query: HistoryQuery = {};
      if (eventTypes) query.event_types = eventTypes;
      if (actorTypes) query.actor_types = actorTypes;

      const result = await getInstanceHistory(instanceId, query);
      setEntries(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instanceId, eventTypes, actorTypes, refreshTrigger]);

  const getEventIcon = (type: string) => {
    switch (type) {
      case 'InstanceStarted': return <Play size={16} style={{ color: '#3b82f6' }} />;
      case 'InstanceCompleted': return <CheckCircle size={16} style={{ color: 'var(--success-color)' }} />;
      case 'InstanceFailed': return <XCircle size={16} style={{ color: 'var(--danger-color)' }} />;
      case 'TokenAdvanced': return <ArrowRightCircle size={16} style={{ color: 'var(--text-muted)' }} />;
      case 'VariablesChanged': return <Settings size={16} style={{ color: '#6366f1' }} />;
      default: return <Activity size={16} style={{ color: 'var(--text-muted)' }} />;
    }
  };

  const getActorStyle = (type?: string): React.CSSProperties => {
    if (!type) return { background: 'var(--code-bg)', color: 'var(--text-muted)', borderColor: 'var(--border-color)' };
    switch (type.toLowerCase()) {
      case 'engine': return { background: '#f3e8ff', color: '#7c3aed', borderColor: '#ddd6fe' };
      case 'serviceworker': return { background: '#ffedd5', color: '#c2410c', borderColor: '#fed7aa' };
      case 'user': return { background: '#cffafe', color: '#0e7490', borderColor: '#a5f3fc' };
      case 'timer': return { background: '#fef3c7', color: '#b45309', borderColor: '#fde68a' };
      case 'api': return { background: '#d1fae5', color: '#047857', borderColor: '#a7f3d0' };
      default: return { background: 'var(--code-bg)', color: 'var(--text-muted)', borderColor: 'var(--border-color)' };
    }
  };

  return (
    <div className="history-timeline-container" style={{ marginTop: '16px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
      
      {/* Filters */}
      <div style={{ display: 'flex', gap: '12px', padding: '12px', backgroundColor: 'var(--bg-subtle)', borderRadius: '6px', border: '1px solid var(--border-color)' }}>
         <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
           <Filter size={16} color="var(--text-muted)" />
           <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-color)' }}>Filters</span>
         </div>
         <select 
            value={eventTypes} 
            onChange={e => setEventTypes(e.target.value)}
            style={{ padding: '4px 8px', borderRadius: '4px', border: '1px solid var(--border-color)', fontSize: '0.85rem', background: 'var(--bg-surface)', color: 'var(--text-color)' }}
         >
           <option value="">All Events</option>
           <option value="InstanceStarted,InstanceCompleted,InstanceFailed">Lifecycle Only</option>
           <option value="TokenAdvanced">Token Movements</option>
           <option value="VariablesChanged">Variable Changes</option>
         </select>

         <select 
            value={actorTypes} 
            onChange={e => setActorTypes(e.target.value)}
            style={{ padding: '4px 8px', borderRadius: '4px', border: '1px solid var(--border-color)', fontSize: '0.85rem', background: 'var(--bg-surface)', color: 'var(--text-color)' }}
         >
           <option value="">All Actors</option>
           <option value="engine">Engine</option>
           <option value="serviceworker">Service Worker</option>
           <option value="user">User</option>
           <option value="api">API</option>
         </select>
      </div>

      {loading && <div style={{ fontSize: '0.9rem', color: 'var(--text-muted)' }}>Loading history...</div>}
      {error && <div style={{ fontSize: '0.9rem', color: 'var(--danger-color)' }}>Error: {error}</div>}

      {!loading && entries.length === 0 && (
        <div style={{ fontSize: '0.9rem', color: 'var(--text-muted)' }}>No history entries found for the current filters.</div>
      )}

      {/* Compact List View */}
      <div style={{ display: 'flex', flexDirection: 'column', border: '1px solid var(--border-color)', borderRadius: '8px', overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left' }}>
          <thead style={{ backgroundColor: 'var(--bg-subtle)', borderBottom: '1px solid var(--border-color)' }}>
            <tr>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-muted)', width: '40px' }}></th>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-muted)' }}>Action</th>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-muted)' }}>Who</th>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-muted)' }}>When</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <tr 
                key={entry.id} 
                onClick={() => setSelectedEntry(entry)}
                style={{ borderBottom: '1px solid var(--border-color)', cursor: 'pointer', backgroundColor: 'var(--bg-surface)', transition: 'background 0.2s' }}
                onMouseEnter={e => e.currentTarget.style.backgroundColor = 'var(--hover-bg)'}
                onMouseLeave={e => e.currentTarget.style.backgroundColor = 'var(--bg-surface)'}
              >
                <td style={{ padding: '10px 16px', textAlign: 'center' }}>
                  {getEventIcon(entry.event_type)}
                </td>
                <td style={{ padding: '10px 16px', fontSize: '0.85rem', color: 'var(--text-color)', fontWeight: 500 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }} title="Snapshot details">
                    {(entry.event_type || 'Unknown').replace(/([A-Z])/g, ' $1').trim()}
                    {entry.is_snapshot && <Camera size={12} color="var(--text-muted)" />}
                  </div>
                </td>
                <td style={{ padding: '10px 16px' }}>
                  <span style={{ ...getActorStyle(entry.actor_type), fontSize: '0.75rem', padding: '2px 8px', borderRadius: '12px', border: '1px solid', textTransform: 'capitalize' }}>
                    {(entry.actor_type || 'Unknown').toLowerCase()}{entry.actor_id ? ` (${entry.actor_id})` : ''}
                  </span>
                </td>
                <td style={{ padding: '10px 16px', fontSize: '0.85rem', color: 'var(--text-muted)' }}>
                  {new Date(entry.timestamp).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Detail Dialog */}
      {selectedEntry && (
        <div className="vars-dialog-overlay" style={{ zIndex: 1000 }}>
          <div style={{ backgroundColor: 'var(--bg-surface)', borderRadius: '8px', padding: '24px', width: '90%', maxWidth: '600px', maxHeight: '90vh', overflowY: 'auto', boxShadow: '0 10px 25px rgba(0,0,0,0.4)', position: 'relative' }}>
            <button 
              onClick={() => setSelectedEntry(null)} 
              style={{ position: 'absolute', top: '16px', right: '16px', background: 'none', border: 'none', cursor: 'pointer', color: 'var(--text-muted)' }}
            >
              <X size={20} />
            </button>
            
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: 0, color: 'var(--text-color)', fontSize: '1.25rem' }}>
              {getEventIcon(selectedEntry.event_type)}
              {(selectedEntry.event_type || 'Unknown').replace(/([A-Z])/g, ' $1').trim()}
            </h2>

            <div style={{ display: 'flex', gap: '12px', marginBottom: '16px', flexWrap: 'wrap' }}>
              <span style={{ ...getActorStyle(selectedEntry.actor_type), fontSize: '0.75rem', padding: '4px 10px', borderRadius: '12px', border: '1px solid', textTransform: 'capitalize' }}>
                {selectedEntry.actor_type || 'Unknown'}{selectedEntry.actor_id ? ` (${selectedEntry.actor_id})` : ''}
              </span>
              <span style={{ fontSize: '0.85rem', color: 'var(--text-muted)', display: 'flex', alignItems: 'center' }}>
                {new Date(selectedEntry.timestamp).toLocaleString()}
              </span>
              {selectedEntry.node_id && (
                <span style={{ fontSize: '0.75rem', padding: '4px 10px', backgroundColor: 'var(--bg-subtle)', color: 'var(--text-color)', borderRadius: '12px', border: '1px solid var(--border-color)' }}>
                  Node: {selectedEntry.node_id}
                </span>
              )}
            </div>

            <div style={{ marginBottom: '16px', padding: '12px', backgroundColor: 'var(--bg-subtle)', borderRadius: '6px', border: '1px solid var(--border-color)', fontSize: '0.95rem', color: 'var(--text-color)' }}>
              {selectedEntry.description}
            </div>

            {selectedEntry.diff?.human_readable && (
              <div style={{ marginBottom: '16px' }}>
                <h3 style={{ fontSize: '0.9rem', color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: '8px' }}>Changes</h3>
                <div style={{ padding: '10px 14px', backgroundColor: 'var(--code-bg)', borderLeft: '3px solid var(--border-color)', fontSize: '0.9rem', color: 'var(--text-color)', whiteSpace: 'pre-wrap' }}>
                  {selectedEntry.diff.human_readable}
                </div>
              </div>
            )}

            {selectedEntry.diff?.changes && Object.keys(selectedEntry.diff.changes).length > 0 && (
              <div style={{ marginBottom: '16px' }}>
                <h3 style={{ fontSize: '0.9rem', color: 'var(--text-muted)', textTransform: 'uppercase', marginBottom: '8px' }}>Raw Data Changes</h3>
                <pre style={{ backgroundColor: '#1e293b', color: '#e2e8f0', padding: '12px', borderRadius: '6px', fontSize: '0.85rem', overflowX: 'auto' }}>
                  {JSON.stringify(selectedEntry.diff.changes, null, 2)}
                </pre>
              </div>
            )}
          </div>
        </div>
      )}

    </div>
  );
}
