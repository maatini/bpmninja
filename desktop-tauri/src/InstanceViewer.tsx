import { useEffect, useRef } from 'react';
import { Focus } from 'lucide-react';

// @ts-ignore
import NavigatedViewer from 'bpmn-js/lib/NavigatedViewer';

import 'bpmn-js/dist/assets/diagram-js.css';
import 'bpmn-js/dist/assets/bpmn-font/css/bpmn-embedded.css';

interface InstanceViewerProps {
  xml: string;
  activeNodeId: string;
  onNodeClick: () => void;
}

export function InstanceViewer({ xml, activeNodeId, onNodeClick }: InstanceViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<any>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const viewer = new NavigatedViewer({
      container: containerRef.current
    });
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

        // Zoom to fit
        canvas.zoom('fit-viewport', 'auto');

        // Highlight active node
        if (activeNodeId && elementRegistry.get(activeNodeId)) {
          canvas.addMarker(activeNodeId, 'highlight-node');
        }

        // Add click listener
        eventBus.on('element.click', (e: any) => {
          if (e.element.id === activeNodeId) {
            onNodeClick();
          }
        });

      } catch (err) {
        console.error('Failed to import BPMN XML for instance viewer', err);
      }
    })();

    return () => {
      isMounted = false;
    };
  }, [xml, activeNodeId, onNodeClick]);

  const handleCenter = () => {
    if (viewerRef.current) {
      viewerRef.current.get('canvas').zoom('fit-viewport', 'auto');
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
        `}
      </style>
      <div style={{ position: 'relative', marginBottom: '16px' }}>
        <div 
          ref={containerRef} 
          style={{ width: '100%', height: '300px', border: '1px solid #e2e8f0', borderRadius: '4px', backgroundColor: '#fafafa' }}
        />
        <button 
          onClick={handleCenter}
          style={{ position: 'absolute', bottom: '48px', right: '16px', zIndex: 99, padding: '6px 8px', backgroundColor: 'white', border: '1px solid #cbd5e1', borderRadius: '4px', cursor: 'pointer', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
          title="Center Workflow"
        >
          <Focus size={18} color="#475569" />
        </button>
      </div>
    </>
  );
}
