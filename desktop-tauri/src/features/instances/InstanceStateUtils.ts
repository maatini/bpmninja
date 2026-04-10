import { type ProcessInstance, type DefinitionInfo } from '../../shared/types/engine';

export function stateLabel(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'Running';
  if (state === 'Completed') return 'Completed';
  if ((state as any) === 'Errored') return 'Errored';
  if ((state as any) === 'Cancelled') return 'Cancelled';
  if (typeof state === 'object') {
    if ('Suspended' in state) return 'Suspended';
    if ('WaitingOnUserTask' in state) return 'Wait: User Task';
    if ('WaitingOnServiceTask' in state) return 'Wait: Service Task';
    if ('WaitingOnTimer' in state) return 'Wait: Timer';
    if ('WaitingOnMessage' in state) return 'Wait: Message';
    return Object.keys(state)[0]?.replace(/([A-Z])/g, ' $1').trim() || 'Unknown';
  }
  return String(state);
}

export function stateBadgeClass(state: ProcessInstance['state']): string {
  if (state === 'Running') return 'bg-blue-600 hover:bg-blue-700 text-white';
  if (state === 'Completed') return 'bg-green-600 hover:bg-green-700 text-white border-none';
  if ((state as any) === 'Errored') return 'bg-destructive/10 text-destructive hover:bg-destructive/20 border-destructive/20 outline outline-1 outline-destructive';
  if ((state as any) === 'Cancelled') return 'bg-muted text-muted-foreground hover:bg-muted/80 outline outline-1 outline-muted';
  if (typeof state === 'object') {
    if ('Suspended' in state) return 'bg-orange-500/20 text-orange-700 hover:bg-orange-500/30 border-orange-500/30 dark:text-orange-400';
    if ('WaitingOnUserTask' in state) return 'bg-amber-500/20 text-amber-700 hover:bg-amber-500/30 border-amber-500/30 dark:text-amber-400';
    if ('WaitingOnServiceTask' in state) return 'bg-purple-500/20 text-purple-700 hover:bg-purple-500/30 border-purple-500/30 dark:text-purple-400';
    if ('WaitingOnTimer' in state) return 'bg-cyan-500/20 text-cyan-700 hover:bg-cyan-500/30 border-cyan-500/30 dark:text-cyan-400';
    if ('WaitingOnMessage' in state) return 'bg-indigo-500/20 text-indigo-700 hover:bg-indigo-500/30 border-indigo-500/30 dark:text-indigo-400';
  }
  return 'bg-secondary text-secondary-foreground hover:bg-secondary/80';
}

export function groupInstances(instances: ProcessInstance[], definitions: DefinitionInfo[]) {
  const defMap = new Map<string, DefinitionInfo>();
  for (const d of definitions) defMap.set(d.key, d);

  const groups = new Map<string, ProcessInstance[]>();
  const unknownGroup: ProcessInstance[] = [];

  for (const inst of instances) {
    const def = defMap.get(inst.definition_key);
    if (def) {
      const arr = groups.get(def.bpmn_id) || [];
      arr.push(inst);
      groups.set(def.bpmn_id, arr);
    } else {
      unknownGroup.push(inst);
    }
  }

  for (const [, insts] of groups) {
    insts.sort((a, b) => {
      if (a.state === 'Completed' && b.state !== 'Completed') return 1;
      if (a.state !== 'Completed' && b.state === 'Completed') return -1;
      return a.id.localeCompare(b.id);
    });
  }

  return { groups, unknownGroup, defMap };
}
