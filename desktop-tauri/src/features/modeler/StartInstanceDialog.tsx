import { useState, useEffect } from 'react';
import { Play } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { VariableEditor, type VariableRow, serializeVariables } from '../../shared/components/VariableEditor';
import { uploadInstanceFile } from '../../shared/lib/tauri';
import { useToast } from '@/hooks/use-toast';

interface StartInstanceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onStartConfigured: (variables: Record<string, unknown>, pendingFiles: VariableRow[], businessKey: string) => Promise<string | undefined>;
}

export function StartInstanceDialog({ open, onOpenChange, onStartConfigured }: StartInstanceDialogProps) {
  const { toast } = useToast();
  const [businessKey, setBusinessKey] = useState('');
  const [startVariables, setStartVariables] = useState<VariableRow[]>([]);
  const [isStarting, setIsStarting] = useState(false);

  // Reset state when opening
  useEffect(() => {
    if (open) {
      setBusinessKey('');
      setStartVariables([]);
      setIsStarting(false);
    }
  }, [open]);

  const handleStartConfirm = async () => {
    const serialized = serializeVariables(startVariables);
    if (serialized === null) {
      toast({ variant: 'destructive', description: 'Invalid variables format. Please check JSON or Numbers.' });
      return;
    }

    if (businessKey.trim() !== '') {
      serialized.business_key = businessKey.trim();
    }

    // Collect pending file rows for deferred upload
    const pendingFiles = startVariables.filter(v => v.type === 'File' && v.pendingFilePath);

    setIsStarting(true);
    try {
      const instanceId = await onStartConfigured(serialized, pendingFiles, businessKey);
      
      // Upload pending files after instance creation if successful
      if (instanceId) {
        for (const pf of pendingFiles) {
          if (pf.pendingFilePath && pf.name.trim()) {
            try {
              await uploadInstanceFile(instanceId, pf.name.trim(), pf.pendingFilePath);
            } catch (uploadErr) {
              console.error(`Failed to upload file '${pf.name}':`, uploadErr);
            }
          }
        }
      }
      onOpenChange(false);
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Failed to start process: ' + e });
    } finally {
      setIsStarting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="vars-dialog sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Start Process Instance</DialogTitle>
          <DialogDescription>
            Provide an optional business key and initial process variables.
          </DialogDescription>
        </DialogHeader>

        <div className="py-2 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="businessKey">Business Key (optional)</Label>
            <Input
              id="businessKey"
              type="text"
              value={businessKey}
              onChange={(e: any) => setBusinessKey(e.target.value)}
              placeholder="e.g. ORDER-1000"
              autoCapitalize="off"
              autoCorrect="off"
              spellCheck={false}
            />
          </div>
          
          <div className="space-y-2">
            <Label>Process Variables</Label>
            <div className="bg-muted/30 border rounded-md p-3">
              <VariableEditor
                variables={startVariables}
                onChange={setStartVariables}
                allowPendingFiles={true}
              />
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button 
            onClick={handleStartConfirm} 
            disabled={isStarting} 
            className="bg-green-600 hover:bg-green-700 text-white gap-2"
          >
             {isStarting ? (
               <>Deploying & Starting…</>
             ) : (
               <><Play className="h-4 w-4"/> Start</>
             )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
