import re

with open("/Volumes/SSD2TB/work/antigravity/mini-bpm/desktop-tauri/src/DeployedProcesses.tsx", "r") as f:
    content = f.read()

# Imports
content = content.replace(
    "import { ScrollArea } from '@/components/ui/scroll-area';",
    "import { ScrollArea } from '@/components/ui/scroll-area';\nimport { Skeleton } from '@/components/ui/skeleton';\nimport { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';"
)

# State
content = content.replace(
    "const [viewingId, setViewingId] = useState<string | null>(null);",
    "const [viewingId, setViewingId] = useState<string | null>(null);\n  const [deleteRequest, setDeleteRequest] = useState<{defId: string, cascade: boolean, msg: string} | null>(null);"
)

# handleDelete Logic
old_delete = """  const handleDelete = async (defId: string) => {
    const relatedInstances = instances.filter(i => i.definition_key === defId);
    let cascade = false;

    if (relatedInstances.length > 0) {
      const msg = `This version has ${relatedInstances.length} associated instance(s). Deleting it will also permanently delete all associated instances.\\n\\nAre you sure?`;
      if (!window.confirm(msg)) return;
      cascade = true;
    } else {
      if (!window.confirm("Delete this version?")) return;
    }

    try {
      await deleteDefinition(defId, cascade);
      fetchDefinitions();
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Delete failed: ' + e });
    }
  };"""

new_delete = """  const handleDeleteCheck = (defId: string) => {
    const relatedInstances = instances.filter(i => i.definition_key === defId);
    let cascade = false;
    let msg = "Are you sure you want to delete this process definition version?";

    if (relatedInstances.length > 0) {
      msg = `This version has ${relatedInstances.length} associated instance(s). Deleting it will also permanently delete all associated instances.\\n\\nAre you sure?`;
      cascade = true;
    }
    setDeleteRequest({ defId, cascade, msg });
  };

  const confirmDelete = async () => {
    if (!deleteRequest) return;
    try {
      await deleteDefinition(deleteRequest.defId, deleteRequest.cascade);
      fetchDefinitions();
    } catch (e: any) {
      toast({ variant: 'destructive', description: 'Delete failed: ' + e });
    } finally {
      setDeleteRequest(null);
    }
  };"""

content = content.replace(old_delete, new_delete)

# Loading states
old_loading = """          {loading && <div className="text-muted-foreground">Loading definitions...</div>}
          {error && <div className="text-destructive font-medium">Error: {error}</div>}
          {!loading && !error && grouped.size === 0 && (
            <div className="text-muted-foreground">No deployed processes.</div>
          )}"""

new_loading = """          {loading && (
            <div className="space-y-4">
              {[1,2,3].map(i => (
                <Card key={i} className="p-4">
                  <div className="flex items-center gap-4">
                    <Skeleton className="h-10 w-10 rounded" />
                    <div className="space-y-2">
                      <Skeleton className="h-5 w-[200px]" />
                      <Skeleton className="h-4 w-[150px]" />
                    </div>
                  </div>
                </Card>
              ))}
            </div>
          )}
          {error && <div className="text-destructive font-medium">Error: {error}</div>}
          {!loading && !error && grouped.size === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-center">
              <Layers className="h-16 w-16 text-muted-foreground/30 mb-4" />
              <h3 className="text-lg font-semibold text-muted-foreground">No Deployed Processes</h3>
              <p className="text-sm text-muted-foreground/70 mt-1 max-w-sm">
                Deploy a BPMN process from the Modeler to see it listed here.
              </p>
            </div>
          )}"""

content = content.replace(old_loading, new_loading)

# onClick Delete
content = content.replace("onClick={() => handleDelete(latest.key)}", "onClick={() => handleDeleteCheck(latest.key)}")
content = content.replace("onClick={() => handleDelete(ver.key)}", "onClick={() => handleDeleteCheck(ver.key)}")

# Add Dialog at the end
dialog_end = "      </ScrollArea>\n    </div>"
alert_add = """      </ScrollArea>
      
      <AlertDialog open={!!deleteRequest} onOpenChange={open => !open && setDeleteRequest(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Confirm Deletion</AlertDialogTitle>
            <AlertDialogDescription className="whitespace-pre-wrap">
              {deleteRequest?.msg}
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


with open("/Volumes/SSD2TB/work/antigravity/mini-bpm/desktop-tauri/src/DeployedProcesses.tsx", "w") as f:
    f.write(content)

