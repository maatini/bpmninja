import { useState } from 'react';
import { Mail } from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import { VariableEditor, serializeVariables } from './VariableEditor';
import type { VariableRow } from './VariableEditor';
import { correlateMessage } from './lib/tauri';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface MessageDialogProps {
  open: boolean;
  onClose: () => void;
}

export function MessageDialog({ open, onClose }: MessageDialogProps) {
  const { toast } = useToast();
  const [messageName, setMessageName] = useState('');
  const [businessKey, setBusinessKey] = useState('');
  const [variables, setVariables] = useState<VariableRow[]>([]);
  const [sending, setSending] = useState(false);

  const handleSend = async () => {
    if (!messageName.trim()) { 
      toast({ variant: 'destructive', description: 'Message name required' }); 
      return; 
    }
    setSending(true);
    try {
      const vars = serializeVariables(variables) || {};
      const result = await correlateMessage(messageName, businessKey || undefined, vars);
      toast({ description: `Message sent! Affected: ${result.length} instance(s)` });
      // Reset form on success
      setMessageName('');
      setBusinessKey('');
      setVariables([]);
      onClose();
    } catch (e) {
      toast({ variant: 'destructive', description: 'Correlation failed: ' + e });
    } finally {
      setSending(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Mail className="h-5 w-5" /> Correlate Message
          </DialogTitle>
        </DialogHeader>
        
        <div className="grid gap-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="messageName">Message Name *</Label>
            <Input 
              id="messageName"
              value={messageName} 
              onChange={(e) => setMessageName(e.target.value)} 
              placeholder="e.g. PaymentReceivedMessage"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="businessKey">Business Key (Optional)</Label>
            <Input 
              id="businessKey"
              value={businessKey} 
              onChange={(e) => setBusinessKey(e.target.value)} 
              placeholder="e.g. order-123"
            />
          </div>

          <div className="space-y-2 mt-2">
            <Label>Message Variables</Label>
            <div className="rounded-md border bg-card text-card-foreground p-3">
              <VariableEditor variables={variables} onChange={setVariables} />
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={sending}>Cancel</Button>
          <Button onClick={handleSend} disabled={sending}>
            {sending ? 'Sending...' : 'Send Message'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
