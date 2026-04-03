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
      case 'InstanceStarted': return <Play size={16} className="text-blue-500" />;
      case 'InstanceCompleted': return <CheckCircle size={16} className="text-green-500" />;
      case 'InstanceFailed': return <XCircle size={16} className="text-red-500" />;
      case 'TokenAdvanced': return <ArrowRightCircle size={16} className="text-gray-500" />;
      case 'VariablesChanged': return <Settings size={16} className="text-indigo-500" />;
      default: return <Activity size={16} className="text-gray-400" />;
    }
  };

  const getActorColor = (type?: string) => {
    if (!type) return 'bg-gray-100 text-gray-700 border-gray-200';
    switch (type.toLowerCase()) {
      case 'engine': return 'bg-purple-100 text-purple-700 border-purple-200';
      case 'serviceworker': return 'bg-orange-100 text-orange-700 border-orange-200';
      case 'user': return 'bg-cyan-100 text-cyan-700 border-cyan-200';
      case 'timer': return 'bg-yellow-100 text-yellow-700 border-yellow-200';
      case 'api': return 'bg-emerald-100 text-emerald-700 border-emerald-200';
      default: return 'bg-gray-100 text-gray-700 border-gray-200';
    }
  };

  return (
    <div className="history-timeline-container" style={{ marginTop: '16px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
      
      {/* Filters */}
      <div style={{ display: 'flex', gap: '12px', padding: '12px', backgroundColor: '#f8fafc', borderRadius: '6px', border: '1px solid #e2e8f0' }}>
         <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
           <Filter size={16} color="#64748b" />
           <span style={{ fontSize: '0.85rem', fontWeight: 600, color: '#334155' }}>Filters</span>
         </div>
         <select 
            value={eventTypes} 
            onChange={e => setEventTypes(e.target.value)}
            style={{ padding: '4px 8px', borderRadius: '4px', border: '1px solid #cbd5e1', fontSize: '0.85rem' }}
         >
           <option value="">All Events</option>
           <option value="InstanceStarted,InstanceCompleted,InstanceFailed">Lifecycle Only</option>
           <option value="TokenAdvanced">Token Movements</option>
           <option value="VariablesChanged">Variable Changes</option>
         </select>

         <select 
            value={actorTypes} 
            onChange={e => setActorTypes(e.target.value)}
            style={{ padding: '4px 8px', borderRadius: '4px', border: '1px solid #cbd5e1', fontSize: '0.85rem' }}
         >
           <option value="">All Actors</option>
           <option value="engine">Engine</option>
           <option value="serviceworker">Service Worker</option>
           <option value="user">User</option>
           <option value="api">API</option>
         </select>
      </div>

      {loading && <div style={{ fontSize: '0.9rem', color: '#64748b' }}>Loading history...</div>}
      {error && <div style={{ fontSize: '0.9rem', color: '#ef4444' }}>Error: {error}</div>}

      {!loading && entries.length === 0 && (
        <div style={{ fontSize: '0.9rem', color: '#64748b' }}>No history entries found for the current filters.</div>
      )}

      {/* Compact List View */}
      <div style={{ display: 'flex', flexDirection: 'column', border: '1px solid #e2e8f0', borderRadius: '8px', overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left' }}>
          <thead style={{ backgroundColor: '#f8fafc', borderBottom: '1px solid #e2e8f0' }}>
            <tr>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: '#475569', width: '40px' }}></th>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: '#475569' }}>Action</th>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: '#475569' }}>Who</th>
              <th style={{ padding: '12px 16px', fontSize: '0.85rem', fontWeight: 600, color: '#475569' }}>When</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <tr 
                key={entry.id} 
                onClick={() => setSelectedEntry(entry)}
                style={{ borderBottom: '1px solid #f1f5f9', cursor: 'pointer', backgroundColor: '#fff', transition: 'background 0.2s' }}
                onMouseEnter={e => e.currentTarget.style.backgroundColor = '#f8fafc'}
                onMouseLeave={e => e.currentTarget.style.backgroundColor = '#fff'}
              >
                <td style={{ padding: '10px 16px', textAlign: 'center' }}>
                  {getEventIcon(entry.event_type)}
                </td>
                <td style={{ padding: '10px 16px', fontSize: '0.85rem', color: '#1e293b', fontWeight: 500 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }} title="Snapshot details">
                    {(entry.event_type || 'Unknown').replace(/([A-Z])/g, ' $1').trim()}
                    {entry.is_snapshot && <Camera size={12} color="#64748b" />}
                  </div>
                </td>
                <td style={{ padding: '10px 16px' }}>
                  <span className={getActorColor(entry.actor_type)} style={{ fontSize: '0.75rem', padding: '2px 8px', borderRadius: '12px', border: '1px solid', textTransform: 'capitalize' }}>
                    {(entry.actor_type || 'Unknown').toLowerCase()}{entry.actor_id ? ` (${entry.actor_id})` : ''}
                  </span>
                </td>
                <td style={{ padding: '10px 16px', fontSize: '0.85rem', color: '#64748b' }}>
                  {new Date(entry.timestamp).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Detail Dialog */}
      {selectedEntry && (
        <div style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0,0,0,0.5)', zIndex: 1000, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ backgroundColor: '#fff', borderRadius: '8px', padding: '24px', width: '90%', maxWidth: '600px', maxHeight: '90vh', overflowY: 'auto', boxShadow: '0 10px 25px rgba(0,0,0,0.1)', position: 'relative' }}>
            <button 
              onClick={() => setSelectedEntry(null)} 
              style={{ position: 'absolute', top: '16px', right: '16px', background: 'none', border: 'none', cursor: 'pointer', color: '#64748b' }}
            >
              <X size={20} />
            </button>
            
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: 0, color: '#1e293b', fontSize: '1.25rem' }}>
              {getEventIcon(selectedEntry.event_type)}
              {(selectedEntry.event_type || 'Unknown').replace(/([A-Z])/g, ' $1').trim()}
            </h2>

            <div style={{ display: 'flex', gap: '12px', marginBottom: '16px', flexWrap: 'wrap' }}>
              <span className={getActorColor(selectedEntry.actor_type)} style={{ fontSize: '0.75rem', padding: '4px 10px', borderRadius: '12px', border: '1px solid', textTransform: 'capitalize' }}>
                {selectedEntry.actor_type || 'Unknown'}{selectedEntry.actor_id ? ` (${selectedEntry.actor_id})` : ''}
              </span>
              <span style={{ fontSize: '0.85rem', color: '#64748b', display: 'flex', alignItems: 'center' }}>
                {new Date(selectedEntry.timestamp).toLocaleString()}
              </span>
              {selectedEntry.node_id && (
                <span style={{ fontSize: '0.75rem', padding: '4px 10px', backgroundColor: '#f1f5f9', color: '#475569', borderRadius: '12px', border: '1px solid #cbd5e1' }}>
                  Node: {selectedEntry.node_id}
                </span>
              )}
            </div>

            <div style={{ marginBottom: '16px', padding: '12px', backgroundColor: '#f8fafc', borderRadius: '6px', border: '1px solid #e2e8f0', fontSize: '0.95rem', color: '#334155' }}>
              {selectedEntry.description}
            </div>

            {selectedEntry.diff?.human_readable && (
              <div style={{ marginBottom: '16px' }}>
                <h3 style={{ fontSize: '0.9rem', color: '#64748b', textTransform: 'uppercase', marginBottom: '8px' }}>Changes</h3>
                <div style={{ padding: '10px 14px', backgroundColor: '#fcfcfc', borderLeft: '3px solid #cbd5e1', fontSize: '0.9rem', color: '#475569', whiteSpace: 'pre-wrap' }}>
                  {selectedEntry.diff.human_readable}
                </div>
              </div>
            )}

            {selectedEntry.diff?.changes && Object.keys(selectedEntry.diff.changes).length > 0 && (
              <div style={{ marginBottom: '16px' }}>
                <h3 style={{ fontSize: '0.9rem', color: '#64748b', textTransform: 'uppercase', marginBottom: '8px' }}>Raw Data Changes</h3>
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
