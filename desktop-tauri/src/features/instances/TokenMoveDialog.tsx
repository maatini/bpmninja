import { useState, useEffect, useRef, useCallback, memo } from 'react';
import { AlertTriangle, Focus, ArrowRightLeft, Loader2 } from 'lucide-react';
import { type ProcessInstance } from '../../shared/types/engine';
import { moveToken } from '../../shared/lib/tauri';
import { VariableEditor, type VariableRow, parseVariables, serializeVariables } from '../../shared/components/VariableEditor';
import { useToast } from '@/hooks/use-toast';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
// @ts-ignore
import NavigatedViewer from 'bpmn-js/lib/NavigatedViewer';
import 'bpmn-js/dist/assets/diagram-js.css';
import 'bpmn-js/dist/assets/bpmn-font/css/bpmn-embedded.css';

interface TokenMoveDialogProps {
  instance: ProcessInstance | null;
  xml: string | null;
  open: boolean;
  onClose: () => void;
  onMoved: () => void;
}

const DiagramPicker = memo(function DiagramPicker({
  xml,
  activeNodeId,
  selectedNodeId,
  onNodeSelect,
}: {
  xml: string;
  activeNodeId: string;
  selectedNodeId: string | null;
  onNodeSelect: (nodeId: string, nodeName: string | null) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<any>(null);
  const prevSelectedRef = useRef<string | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const viewer = new NavigatedViewer({ container: containerRef.current });
    viewerRef.current = viewer;
    return () => {
      viewer.destroy();
      viewerRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!viewerRef.current || !xml) return;

    let isMounted = true;
    (async () => {
      try {
        await viewerRef.current.importXML(xml);
        if (!isMounted) return;

        const canvas = viewerRef.current.get('canvas');
        const elementRegistry = viewerRef.current.get('elementRegistry');
        const eventBus = viewerRef.current.get('eventBus');

        canvas.zoom('fit-viewport', 'auto');

        // Highlight current token position
        if (activeNodeId && elementRegistry.get(activeNodeId)) {
          canvas.addMarker(activeNodeId, 'token-current');
        }

        // Click listener for node selection
        eventBus.on('element.click', (e: any) => {
          const el = e.element;
          // Only allow clicking on actual BPMN elements (not the root/canvas)
          if (el.type === 'bpmn:Process' || el.type === 'label') return;
          // Skip sequence flows
          if (el.type === 'bpmn:SequenceFlow') return;

          const nodeId = el.id;
          const nodeName = el.businessObject?.name || null;

          // Remove previous selection marker
          if (prevSelectedRef.current && elementRegistry.get(prevSelectedRef.current)) {
            canvas.removeMarker(prevSelectedRef.current, 'token-target');
          }

          // Add new selection marker
          if (elementRegistry.get(nodeId)) {
            canvas.addMarker(nodeId, 'token-target');
          }
          prevSelectedRef.current = nodeId;

          onNodeSelect(nodeId, nodeName);
        });
      } catch (err) {
        console.error('Failed to import BPMN XML for token move', err);
      }
    })();

    return () => { isMounted = false; };
  }, [xml, activeNodeId, onNodeSelect]);

  // Update selection marker when selectedNodeId changes externally (e.g. reset)
  useEffect(() => {
    if (!viewerRef.current) return;
    const canvas = viewerRef.current.get('canvas');
    const elementRegistry = viewerRef.current.get('elementRegistry');

    // Remove old marker
    if (prevSelectedRef.current && elementRegistry.get(prevSelectedRef.current)) {
      canvas.removeMarker(prevSelectedRef.current, 'token-target');
    }
    // Add new marker
    if (selectedNodeId && elementRegistry.get(selectedNodeId)) {
      canvas.addMarker(selectedNodeId, 'token-target');
    }
    prevSelectedRef.current = selectedNodeId;
  }, [selectedNodeId]);

  const handleCenter = () => {
    if (viewerRef.current) {
      viewerRef.current.get('canvas').zoom('fit-viewport', 'auto');
    }
  };

  return (
    <>
      <style>
        {`
          .token-current:not(.djs-connection) .djs-visual > :nth-child(1) {
            stroke: #10b981 !important;
            stroke-width: 4px !important;
            fill: rgba(16, 185, 129, 0.2) !important;
          }
          .token-target:not(.djs-connection) .djs-visual > :nth-child(1) {
            stroke: #3b82f6 !important;
            stroke-width: 4px !important;
            fill: rgba(59, 130, 246, 0.25) !important;
            stroke-dasharray: 6 3 !important;
          }
        `}
      </style>
      <div className="relative w-full h-full min-h-[300px] border border-border rounded-md bg-muted/20">
        <div ref={containerRef} className="w-full h-full flex-1 min-h-[300px] bg-background" />
        <Button
          variant="outline"
          size="icon"
          onClick={handleCenter}
          className="absolute bottom-3 right-3 z-10 shadow-md bg-background/90 backdrop-blur"
          title="Center"
        >
          <Focus className="h-5 w-5 text-muted-foreground" />
        </Button>
        {/* Legend */}
        <div className="absolute top-3 left-3 z-10 flex gap-3 text-xs bg-background/90 backdrop-blur px-3 py-1.5 rounded-md border shadow-sm">
          <span className="flex items-center gap-1.5">
            <span className="w-3 h-3 rounded-sm border-2 border-emerald-500 bg-emerald-500/20" />
            Aktuell
          </span>
          <span className="flex items-center gap-1.5">
            <span className="w-3 h-3 rounded-sm border-2 border-blue-500 bg-blue-500/25 border-dashed" />
            Ziel
          </span>
        </div>
      </div>
    </>
  );
});

export function TokenMoveDialog({ instance, xml, open, onClose, onMoved }: TokenMoveDialogProps) {
  const { toast } = useToast();
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedNodeName, setSelectedNodeName] = useState<string | null>(null);
  const [cancelCurrent, setCancelCurrent] = useState(true);
  const [variables, setVariables] = useState<VariableRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);

  // Reset state when dialog opens/instance changes
  useEffect(() => {
    if (open && instance) {
      setSelectedNodeId(null);
      setSelectedNodeName(null);
      setCancelCurrent(true);
      setVariables(parseVariables(instance.variables));
      setShowConfirm(false);
    }
  }, [open, instance?.id]);

  const handleNodeSelect = useCallback((nodeId: string, nodeName: string | null) => {
    setSelectedNodeId(nodeId);
    setSelectedNodeName(nodeName);
    setShowConfirm(false);
  }, []);

  const handleExecute = async () => {
    if (!instance || !selectedNodeId) return;
    setLoading(true);
    try {
      const vars = serializeVariables(variables, new Set());
      await moveToken(instance.id, {
        target_node_id: selectedNodeId,
        variables: vars ?? undefined,
        cancel_current: cancelCurrent,
      });
      toast({
        description: `Token verschoben nach '${selectedNodeName || selectedNodeId}'.`,
      });
      onMoved();
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Token Move fehlgeschlagen: ' + e });
    } finally {
      setLoading(false);
    }
  };

  const isSameNode = selectedNodeId === instance?.current_node;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-[80vw] w-full max-h-[90vh] flex flex-col p-0 overflow-hidden bg-background">
        <DialogHeader className="px-6 py-4 border-b shrink-0">
          <DialogTitle className="text-lg flex items-center gap-2">
            <ArrowRightLeft className="h-5 w-5 text-blue-500" />
            Token Move — Modify Process Instance
          </DialogTitle>
        </DialogHeader>

        {instance && xml && (
          <div className="flex-1 overflow-y-auto min-h-0 p-6 space-y-5">
            {/* Diagram */}
            <div className="h-[400px]">
              <DiagramPicker
                xml={xml}
                activeNodeId={instance.current_node}
                selectedNodeId={selectedNodeId}
                onNodeSelect={handleNodeSelect}
              />
            </div>

            {/* Selection info */}
            <div className="flex items-center gap-4 text-sm">
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">Aktueller Knoten:</span>
                <Badge variant="outline" className="font-mono border-emerald-500 text-emerald-700 dark:text-emerald-400">
                  {instance.current_node}
                </Badge>
              </div>
              <span className="text-muted-foreground">→</span>
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">Ziel:</span>
                {selectedNodeId ? (
                  <Badge variant="outline" className="font-mono border-blue-500 text-blue-700 dark:text-blue-400">
                    {selectedNodeName ? `${selectedNodeName} (${selectedNodeId})` : selectedNodeId}
                  </Badge>
                ) : (
                  <span className="text-muted-foreground italic">Knoten im Diagramm anklicken…</span>
                )}
              </div>
            </div>

            {isSameNode && selectedNodeId && (
              <div className="flex items-center gap-2 text-sm text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-800 rounded-md px-3 py-2">
                <AlertTriangle className="h-4 w-4 shrink-0" />
                Der Zielknoten ist identisch mit dem aktuellen Knoten. Der Token wird neu gestartet.
              </div>
            )}

            {/* Options */}
            <div className="border rounded-lg p-4 space-y-4">
              <h4 className="font-semibold text-sm">Optionen</h4>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="cancel-current"
                  checked={cancelCurrent}
                  onChange={(e) => setCancelCurrent(e.target.checked)}
                  className="h-4 w-4 rounded border-border"
                />
                <Label htmlFor="cancel-current" className="text-sm">
                  Aktuelle Token(s) canceln (empfohlen)
                </Label>
              </div>
              <p className="text-xs text-muted-foreground">
                Entfernt alle laufenden User Tasks, Service Tasks, Timer und Message-Catches dieser Instanz vor dem Move.
              </p>
            </div>

            {/* Variables */}
            <div className="border rounded-lg p-4 space-y-3">
              <h4 className="font-semibold text-sm">Variablen (optional anpassen)</h4>
              <p className="text-xs text-muted-foreground">
                Diese Variablen werden dem neuen Token mitgegeben. Du kannst Werte ändern oder neue hinzufügen.
              </p>
              <VariableEditor
                variables={variables}
                onChange={setVariables}
                readOnlyNames={false}
              />
            </div>

            {/* Confirmation */}
            {showConfirm && selectedNodeId && (
              <div className="bg-destructive/10 border border-destructive/30 rounded-lg p-4 space-y-3">
                <div className="flex items-start gap-2 text-sm">
                  <AlertTriangle className="h-5 w-5 text-destructive shrink-0 mt-0.5" />
                  <div>
                    <p className="font-semibold text-destructive">Achtung: Token Move ist eine mächtige Operation!</p>
                    <p className="text-muted-foreground mt-1">
                      Der Token wird von <strong>{instance.current_node}</strong> nach <strong>{selectedNodeId}</strong> verschoben.
                      {cancelCurrent && ' Alle aktiven Wait-States werden gecancelt.'}
                      {' '}Dieser Vorgang kann nicht rückgängig gemacht werden.
                    </p>
                  </div>
                </div>
                <div className="flex justify-end gap-2">
                  <Button variant="outline" size="sm" onClick={() => setShowConfirm(false)}>
                    Abbrechen
                  </Button>
                  <Button
                    variant="destructive"
                    size="sm"
                    className="gap-2"
                    disabled={loading}
                    onClick={handleExecute}
                  >
                    {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <ArrowRightLeft className="h-4 w-4" />}
                    Token jetzt verschieben
                  </Button>
                </div>
              </div>
            )}
          </div>
        )}

        <DialogFooter className="px-6 py-4 border-t shrink-0">
          <Button variant="outline" onClick={onClose}>Schließen</Button>
          {!showConfirm && (
            <Button
              disabled={!selectedNodeId || loading}
              className="gap-2"
              onClick={() => setShowConfirm(true)}
            >
              <ArrowRightLeft className="h-4 w-4" />
              Token verschieben…
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
