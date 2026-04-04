import { useState, useEffect, useRef } from 'react';
import { getPendingServiceTasks, type PendingServiceTask } from './lib/tauri';
import { useToast } from '@/hooks/use-toast';
import { AlertTriangle, RefreshCw, ExternalLink } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardContent, CardFooter } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';

export function IncidentsView({ onViewInstance }: { onViewInstance?: (id: string) => void }) {
  const { toast } = useToast();
  const [incidents, setIncidents] = useState<PendingServiceTask[]>([]);
  const [loading, setLoading] = useState(true);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchIncidents = async () => {
    try {
      const all = await getPendingServiceTasks();
      // Filter: incidents = service tasks with retries <= 0
      setIncidents(all.filter(t => t.retries <= 0));
      setLoading(false);
    } catch (e: any) { 
      toast({ variant: 'destructive', description: 'Failed to load incidents: ' + e }); 
    }
  };

  useEffect(() => {
    fetchIncidents();
    intervalRef.current = setInterval(fetchIncidents, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  return (
    <div className="flex flex-col h-full bg-background">
      <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
        <h2 className="text-2xl font-bold tracking-tight flex items-center gap-2">
          <AlertTriangle className="h-6 w-6 text-destructive" /> Incidents
        </h2>
        <div className="flex items-center gap-4">
          <span className="text-xs text-muted-foreground bg-muted px-2 py-1 rounded-md">Auto-refreshing</span>
          <Button onClick={fetchIncidents} variant="outline" size="sm" className="gap-2">
            <RefreshCw className="h-4 w-4" /> Refresh
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1 p-6">
        {loading && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {[1,2,3,4].map(i => (
              <Card key={i} className="flex flex-col h-[200px]">
                <div className="p-4 flex-1 space-y-4">
                  <Skeleton className="h-6 w-[140px]" />
                  <Skeleton className="h-4 w-full" />
                  <Skeleton className="h-10 w-full" />
                </div>
              </Card>
            ))}
          </div>
        )}
        
        {!loading && incidents.length === 0 && (
          <Card className="border-green-500/50 bg-green-500/10 dark:bg-green-500/5">
            <CardContent className="flex flex-col items-center justify-center p-12 text-green-700 dark:text-green-500 space-y-4">
              <AlertTriangle className="h-12 w-12" />
              <div className="text-lg font-medium">No incidents — all systems operational.</div>
            </CardContent>
          </Card>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {incidents.map(inc => (
            <Card key={inc.id} className="border-destructive/50 flex flex-col">
              <CardHeader className="pb-3 border-b bg-destructive/5">
                <CardTitle className="text-lg flex items-center gap-2 text-destructive font-mono">
                  <AlertTriangle className="h-5 w-5" /> {inc.node_id}
                </CardTitle>
              </CardHeader>
              <CardContent className="pt-4 space-y-4 flex-1">
                <div className="flex flex-col gap-2 text-sm">
                  <div className="flex justify-between items-center">
                     <span className="text-muted-foreground">Topic</span>
                     <Badge variant="outline" className="font-mono">{inc.topic}</Badge>
                  </div>
                  <div className="flex justify-between items-center">
                     <span className="text-muted-foreground">Instance</span>
                     <span className="font-mono bg-muted px-2 py-0.5 rounded text-xs">{inc.instance_id.substring(0, 8)}</span>
                  </div>
                </div>
                
                <div className="bg-destructive/10 p-3 rounded-md border border-destructive/20 text-sm">
                  <strong className="text-destructive block mb-1">Error:</strong> 
                  <span className="text-foreground">{inc.error_message || 'No error message'}</span>
                </div>
                
                {inc.error_details && (
                  <pre className="bg-muted border text-foreground p-3 rounded-md text-xs overflow-x-auto whitespace-pre-wrap font-mono">
                    {inc.error_details}
                  </pre>
                )}
              </CardContent>
              <CardFooter className="pt-2">
                {onViewInstance && (
                  <Button 
                    variant="outline" 
                    className="w-full gap-2" 
                    onClick={() => onViewInstance(inc.instance_id)}
                  >
                    <ExternalLink className="h-4 w-4" /> View Instance
                  </Button>
                )}
              </CardFooter>
            </Card>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
