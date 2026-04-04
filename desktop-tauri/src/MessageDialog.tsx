import { useState } from 'react';
import { useToast } from './ToastContext';
import { VariableEditor, serializeVariables } from './VariableEditor';
import type { VariableRow } from './VariableEditor';
import { correlateMessage } from './lib/tauri';

interface MessageDialogProps {
  open: boolean;
  onClose: () => void;
}

export function MessageDialog({ open, onClose }: MessageDialogProps) {
  const toast = useToast();
  const [messageName, setMessageName] = useState('');
  const [businessKey, setBusinessKey] = useState('');
  const [variables, setVariables] = useState<VariableRow[]>([]);
  const [sending, setSending] = useState(false);

  const handleSend = async () => {
    if (!messageName.trim()) { 
      toast.error('Message name required'); 
      return; 
    }
    setSending(true);
    try {
      const vars = serializeVariables(variables) || {};
      const result = await correlateMessage(messageName, businessKey || undefined, vars);
      toast.success(`Message sent! Affected: ${result.length} instance(s)`);
      // Reset form on success
      setMessageName('');
      setBusinessKey('');
      setVariables([]);
      onClose();
    } catch (e) {
      toast.error('Correlation failed: ' + e);
    } finally {
      setSending(false);
    }
  };

  if (!open) return null;

  return (
    <div className="vars-dialog-overlay" onClick={onClose} style={{ zIndex: 10000 }}>
      <div className="vars-dialog" onClick={e => e.stopPropagation()} style={{ width: '600px', maxWidth: '90vw' }}>
        <h3 style={{ marginBottom: '16px' }}>📨 Correlate Message</h3>
        
        <div style={{ marginBottom: '16px' }}>
          <label style={{ display: 'block', marginBottom: '8px', fontWeight: 'bold' }}>Message Name *</label>
          <input 
            type="text" 
            className="input" 
            value={messageName} 
            onChange={(e) => setMessageName(e.target.value)} 
            placeholder="e.g. PaymentReceivedMessage"
            style={{ width: '100%', marginBottom: '16px' }}
          />

          <label style={{ display: 'block', marginBottom: '8px', fontWeight: 'bold' }}>Business Key (Optional)</label>
          <input 
            type="text" 
            className="input" 
            value={businessKey} 
            onChange={(e) => setBusinessKey(e.target.value)} 
            placeholder="e.g. order-123"
            style={{ width: '100%', marginBottom: '16px' }}
          />
        </div>

        <div style={{ marginBottom: '24px' }}>
          <label style={{ display: 'block', marginBottom: '8px', fontWeight: 'bold' }}>Message Variables</label>
          <VariableEditor variables={variables} onChange={setVariables} />
        </div>

        <div className="vars-dialog-footer">
          <button className="button button-secondary" onClick={onClose} disabled={sending}>Cancel</button>
          <button className="button" onClick={handleSend} disabled={sending}>
            {sending ? 'Sending...' : 'Send Message'}
          </button>
        </div>
      </div>
    </div>
  );
}
