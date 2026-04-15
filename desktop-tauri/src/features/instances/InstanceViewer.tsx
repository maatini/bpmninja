import { useEffect, useRef, memo } from 'react';
import { Focus } from 'lucide-react';
import { Button } from '@/components/ui/button';

// @ts-ignore
import NavigatedViewer from 'bpmn-js/lib/NavigatedViewer';

import 'bpmn-js/dist/assets/diagram-js.css';
import 'bpmn-js/dist/assets/bpmn-font/css/bpmn-embedded.css';

interface InstanceViewerProps {
  xml: string;
  activeNodeIds: string[];
  onNodeClick: (nodeId: string) => void;
  timerStartNodeId?: string;
}

export const InstanceViewer = memo(function InstanceViewer({ xml, activeNodeIds, onNodeClick, timerStartNodeId }: InstanceViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<any>(null);
  const importAbortRef = useRef<boolean>(false);

  // Create viewer once on mount, destroy on unmount
  useEffect(() => {
    if (!containerRef.current) return;

    const viewer = new NavigatedViewer({
      container: containerRef.current
    });
    viewerRef.current = viewer;

    return () => {
      importAbortRef.current = true;
      viewer.destroy();
      viewerRef.current = null;
    };
  }, []);

  // Re-import XML whenever it or the active nodes change
  useEffect(() => {
    const viewer = viewerRef.current;
    if (!viewer || !xml) return;

    importAbortRef.current = false;
    let cancelled = false;

    (async () => {
      try {
        // Clear existing diagram before re-importing to avoid "element already exists"
        try { viewer.clear(); } catch (_) { /* ignore if not yet initialized */ }

        await viewer.importXML(xml);

        if (cancelled || importAbortRef.current) return;

        const canvas = viewer.get('canvas');
        const elementRegistry = viewer.get('elementRegistry');
        const eventBus = viewer.get('eventBus');

        // Zoom to fit
        try { canvas.zoom('fit-viewport', 'auto'); } catch (_) { /* ignore */ }

        // Highlight all active nodes (supports parallel execution)
        for (const nodeId of activeNodeIds) {
          if (nodeId && elementRegistry.get(nodeId)) {
            canvas.addMarker(nodeId, 'highlight-node');
          }
        }

        // Highlight timer start event if cycle is still active
        if (timerStartNodeId && !activeNodeIds.includes(timerStartNodeId) && elementRegistry.get(timerStartNodeId)) {
          canvas.addMarker(timerStartNodeId, 'highlight-timer-active');
        }

        // Add click listener — re-register each time to use fresh activeNodeIds closure
        eventBus.off('element.click');
        eventBus.on('element.click', (e: any) => {
          if (activeNodeIds.includes(e.element.id)) {
            onNodeClick(e.element.id);
          }
        });

      } catch (err) {
        if (!cancelled && !importAbortRef.current) {
          console.error('Failed to import BPMN XML for instance viewer', err);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [xml, activeNodeIds, onNodeClick, timerStartNodeId]);

  const handleCenter = () => {
    if (viewerRef.current) {
      try { viewerRef.current.get('canvas').zoom('fit-viewport', 'auto'); } catch (_) { /* ignore */ }
    }
  };

  return (
    <>
      <style>
        {`
          .highlight-node:not(.djs-connection) .djs-visual > :nth-child(1) {
            stroke: #10b981 !important; /* Emerald green border */
            stroke-width: 4px !important;
            fill: rgba(16, 185, 129, 0.2) !important; /* Light emerald fill */
          }
          .highlight-timer-active:not(.djs-connection) .djs-visual > :nth-child(1) {
            stroke: #f59e0b !important; /* Amber border – timer still firing */
            stroke-width: 4px !important;
            fill: rgba(245, 158, 11, 0.15) !important;
            animation: timer-pulse 2s ease-in-out infinite;
          }
          @keyframes timer-pulse {
            0%, 100% { fill-opacity: 0.15; }
            50% { fill-opacity: 0.35; }
          }
        `}
      </style>
      <div className="relative w-full h-full min-h-[300px] border border-border rounded-md bg-muted/20">
        <div
          ref={containerRef}
          className="w-full h-full flex-1 min-h-[300px] bg-background"
        />
        <Button
          variant="outline"
          size="icon"
          onClick={handleCenter}
          className="absolute bottom-12 right-4 z-10 shadow-md bg-background/90 backdrop-blur"
          title="Center Workflow"
        >
          <Focus className="h-5 w-5 text-muted-foreground" />
        </Button>
      </div>
    </>
  );
});
