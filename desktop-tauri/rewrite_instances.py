import re

with open("/Volumes/SSD2TB/work/antigravity/mini-bpm/desktop-tauri/src/Instances.tsx", "r") as f:
    content = f.read()

# Imports
content = re.sub(
    r"import \{ Dialog, DialogContent, DialogHeader, DialogTitle \} from '@/components/ui/dialog';",
    "import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';\nimport { Skeleton } from '@/components/ui/skeleton';\nimport { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';",
    content
)

content = content.replace(
    "import { RefreshCw, Activity, CheckCircle, Clock, Trash, FileCode2, Network, ScrollText } from 'lucide-react';",
    "import { RefreshCw, Activity, CheckCircle, Clock, Trash, FileCode2, Network, ScrollText, Layers } from 'lucide-react';"
)


# Functions
old_functions = """function stateLabel(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'Running';
  if (state === 'Completed') return 'Completed';
  if (typeof state === 'object' && 'WaitingOnUserTask' in state) return 'Wait: User Task';
  if (typeof state === 'object' && 'WaitingOnServiceTask' in state) return 'Wait: Service Task';
  if (typeof state === 'object' && 'WaitingOnTimer' in state) return 'Wait: Timer';
  if (typeof state === 'object' && 'WaitingOnMessage' in state) return 'Wait: Message';
  return String(state);
}

function stateBadgeVariant(state: ProcessInstance['state']): \"default\" | \"secondary\" | \"destructive\" | \"outline\" {
  if (state === 'Running') return 'default';
  if (state === 'Completed') return 'outline'; // Or custom a green via class
  return 'secondary';
}"""

new_functions = """function stateLabel(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'Running';
  if (state === 'Completed') return 'Completed';
  if (state === 'Errored') return 'Errored';
  if (state === 'Cancelled') return 'Cancelled';
  if (typeof state === 'object') {
    if ('WaitingOnUserTask' in state) return 'Wait: User Task';
    if ('WaitingOnServiceTask' in state) return 'Wait: Service Task';
    if ('WaitingOnTimer' in state) return 'Wait: Timer';
    if ('WaitingOnMessage' in state) return 'Wait: Message';
    return Object.keys(state)[0]?.replace(/([A-Z])/g, ' $1').trim() || 'Unknown';
  }
  return String(state);
}

function stateBadgeClass(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'bg-blue-600 hover:bg-blue-700 text-white';
  if (state === 'Completed') return 'bg-green-600 hover:bg-green-700 text-white border-none';
  if (state === 'Errored') return 'bg-red-600 hover:bg-red-700 text-white border-none';
  if (state === 'Cancelled') return 'bg-gray-500 hover:bg-gray-600 text-white border-none';
  if (typeof state === 'object') {
    if ('WaitingOnUserTask' in state) return 'bg-amber-500/20 text-amber-700 hover:bg-amber-500/30 border-amber-500/30 dark:text-amber-400';
    if ('WaitingOnServiceTask' in state) return 'bg-purple-500/20 text-purple-700 hover:bg-purple-500/30 border-purple-500/30 dark:text-purple-400';
    if ('WaitingOnTimer' in state) return 'bg-cyan-500/20 text-cyan-700 hover:bg-cyan-500/30 border-cyan-500/30 dark:text-cyan-400';
    if ('WaitingOnMessage' in state) return 'bg-indigo-500/20 text-indigo-700 hover:bg-indigo-500/30 border-indigo-500/30 dark:text-indigo-400';
  }
  return 'bg-secondary text-secondary-foreground hover:bg-secondary/80';
}"""

content = content.replace(old_functions, new_functions)


# Dialog State
content = content.replace("const [showNodeDetails, setShowNodeDetails] = useState(true);", "const [showNodeDetails, setShowNodeDetails] = useState(true);\n  const [instanceToDelete, setInstanceToDelete] = useState<string | null>(null);")

# handleDelete
old_delete = """  const handleDelete = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    if (!window.confirm("Are you sure you want to delete this process instance?")) return;
    try {
      await deleteInstance(id);
      if (selected?.id === id) {
        handleClose();
      }
      fetchData();
    } catch (err) {
      toast({ variant: 'destructive', description: "Failed to delete instance: " + err });
    }
  };"""

new_delete = """  const handleDeleteRequest = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setInstanceToDelete(id);
  };

  const confirmDelete = async () => {
    if (!instanceToDelete) return;
    try {
      await deleteInstance(instanceToDelete);
      if (selected?.id === instanceToDelete) {
        handleClose();
      }
      fetchData();
    } catch (err) {
      toast({ variant: 'destructive', description: "Failed to delete instance: " + err });
    } finally {
      setInstanceToDelete(null);
    }
  };"""

content = content.replace(old_delete, new_delete)

# Loading states
old_loading = """          {loading && <div className="text-muted-foreground">Loading instances...</div>}
          {error && <div className="text-destructive font-medium">Error: {error}</div>}
          {!loading && !error && instances.length === 0 && (
            <div className="text-muted-foreground">No instances found.</div>
          )}"""

new_loading = """          {loading && (
            <div className="space-y-4">
              {[1,2,3].map(i => (
                <Card key={i} className="p-4">
                  <div className="flex items-center gap-4">
                    <Skeleton className="h-6 w-[120px] rounded-full" />
                    <Skeleton className="h-5 w-[200px]" />
                    <Skeleton className="h-5 w-[80px] ml-auto" />
                  </div>
                </Card>
              ))}
            </div>
          )}
          {error && <div className="text-destructive font-medium">Error: {error}</div>}
          {!loading && !error && instances.length === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-center">
              <Layers className="h-16 w-16 text-muted-foreground/30 mb-4" />
              <h3 className="text-lg font-semibold text-muted-foreground">No Instances Yet</h3>
              <p className="text-sm text-muted-foreground/70 mt-1 max-w-sm">
                Deploy a BPMN process and start your first instance from the Modeler.
              </p>
            </div>
          )}"""

content = content.replace(old_loading, new_loading)


# Badge fix 1
old_badge1 = """                              <Badge variant={stateBadgeVariant(inst.state)} className={cn(
                                "flex items-center justify-center gap-1.5 w-full",
                                inst.state === 'Running' && "bg-blue-600 hover:bg-blue-700",
                                inst.state === 'Completed' && "bg-green-600 hover:bg-green-700 border-none text-white",
                                typeof inst.state === 'object' && "bg-amber-500/20 text-amber-700 hover:bg-amber-500/30 border-amber-500/30 dark:text-amber-400"
                              )}>"""

new_badge1 = """                              <Badge className={cn(
                                "flex items-center justify-center gap-1.5 w-full",
                                stateBadgeClass(inst.state)
                              )}>"""

content = content.replace(old_badge1, new_badge1)

# handle delete calls
content = content.replace("onClick={(e) => handleDelete(e, inst.id)}", "onClick={(e) => handleDeleteRequest(e, inst.id)}")
content = content.replace("selected && handleDelete(e as any, selected.id)", "selected && handleDeleteRequest(e as any, selected.id)")

# unknown group badge
content = content.replace("<Badge variant={stateBadgeVariant(inst.state)}>{stateLabel(inst.state)}</Badge>", "<Badge className={stateBadgeClass(inst.state)}>{stateLabel(inst.state)}</Badge>")

# details group badge
content = content.replace("<Badge variant={stateBadgeVariant(selected.state)} className=\"w-fit\">", "<Badge className={cn(\"w-fit, border-none\", stateBadgeClass(selected.state))}>")

# Add alert dialog to bottom
dialog_end = "      </Dialog>\n    </div>"
alert_add = """      </Dialog>
      <AlertDialog open={!!instanceToDelete} onOpenChange={open => !open && setInstanceToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Instance</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this process instance? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmDelete} className="bg-destructive hover:bg-destructive/90 text-destructive-foreground">Delete</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>"""

content = content.replace(dialog_end, alert_add)


with open("/Volumes/SSD2TB/work/antigravity/mini-bpm/desktop-tauri/src/Instances.tsx", "w") as f:
    f.write(content)

