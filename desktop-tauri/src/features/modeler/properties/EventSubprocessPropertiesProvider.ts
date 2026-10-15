import { CheckboxEntry, SelectEntry, TextFieldEntry, isCheckboxEntryEdited, isSelectEntryEdited, isTextFieldEntryEdited, useStaticCallback } from '@bpmn-io/properties-panel';
// @ts-ignore
import { useService } from 'bpmn-js-properties-panel';
// @ts-ignore
import { is } from 'bpmn-js/lib/util/ModelUtil';

const references: Record<string, { property: string; type: string; value: string }> = {
  'bpmn:MessageEventDefinition': { property: 'messageRef', type: 'bpmn:Message', value: 'name' },
  'bpmn:SignalEventDefinition': { property: 'signalRef', type: 'bpmn:Signal', value: 'name' },
  'bpmn:ErrorEventDefinition': { property: 'errorRef', type: 'bpmn:Error', value: 'errorCode' },
  'bpmn:EscalationEventDefinition': { property: 'escalationRef', type: 'bpmn:Escalation', value: 'escalationCode' }
};
const timerKinds = ['timeDuration', 'timeDate', 'timeCycle'];
function eventDefinition(element: any) { return element.businessObject.eventDefinitions?.[0]; }
function definitions(element: any) {
  let parent = element.businessObject;
  while (parent && !is(parent, 'bpmn:Definitions')) parent = parent.$parent;
  return parent;
}
function validateReference(value: string) {
  if (value && !/^[A-Za-z_][A-Za-z0-9_.-]*$/.test(value)) return 'Use an XML ID: letters, digits, underscores, dots, or hyphens.';
  return undefined;
}

function Interrupting(props: any) {
  const modeling = useService('modeling');
  return CheckboxEntry({ ...props, label: 'Interrupting',
    description: 'Cancel main execution and other active handlers when triggered.',
    getValue: () => props.element.businessObject.isInterrupting !== false,
    setValue: (value: boolean) => modeling.updateProperties(props.element, { isInterrupting: value })
  });
}

function ReferenceId(props: any) {
  const { element } = props;
  const commandStack = useService('commandStack');
  const moddle = useService('moddle');
  const debounce = useService('debounceInput');
  const event = eventDefinition(element);
  const config = references[event.$type];
  const root = definitions(element);
  const validate = useStaticCallback((value: string) => {
    const error = validateReference(value);
    if (error) return error;
    if (!value && ['bpmn:Message', 'bpmn:Signal'].includes(config.type)) return 'A reference is required.';
    const existing = root.rootElements?.find((item: any) => item.id === value) || moddle.ids.assigned(value);
    if (existing && !is(existing, config.type)) return 'This ID belongs to a different BPMN element.';
    return undefined;
  });
  return TextFieldEntry({ ...props, label: 'Event reference ID', debounce,
    description: 'Reuse a declaration ID or enter a new one. Empty error/escalation references catch any code.',
    getValue: () => event[config.property]?.id || '', validate,
    setValue: (value: string) => {
      if (validate(value)) return;
      let reference = root.rootElements?.find((item: any) => item.id === value);
      const commands = [];
      if (value && !reference) {
        reference = moddle.create(config.type, { id: value, [config.value]: value });
        reference.$parent = root;
        commands.push({ cmd: 'element.updateModdleProperties', context: { element, moddleElement: root, properties: { rootElements: [...root.rootElements, reference] } } });
      }
      commands.push({ cmd: 'element.updateModdleProperties', context: { element, moddleElement: event, properties: { [config.property]: reference } } });
      // Keep declaration creation and reference binding in one undo step.
      commandStack.execute('properties-panel.multi-command-executor', commands);
    }
  });
}

function ReferenceValue(props: any) {
  const { element } = props;
  const modeling = useService('modeling');
  const debounce = useService('debounceInput');
  const event = eventDefinition(element);
  const config = references[event.$type];
  const reference = event[config.property];
  return TextFieldEntry({ ...props, debounce,
    label: config.value === 'name' ? 'Event name' : 'Event code',
    description: 'Shared by every event using this declaration.',
    disabled: !reference,
    getValue: () => reference?.[config.value] || '',
    setValue: (value: string) => reference && modeling.updateModdleProperties(element, reference, { [config.value]: value })
  });
}

function TimerKind(props: any) {
  const { element } = props;
  const modeling = useService('modeling');
  const moddle = useService('moddle');
  const event = eventDefinition(element);
  return SelectEntry({ ...props, label: 'Timer type',
    getOptions: () => [ { value: 'timeDuration', label: 'Duration (PT5M)' }, { value: 'timeDate', label: 'Date (ISO 8601)' }, { value: 'timeCycle', label: 'Cycle (R3/PT5M or cron)' } ],
    getValue: () => timerKinds.find(kind => event[kind]) || 'timeDuration',
    setValue: (kind: string) => {
      const expression = moddle.create('bpmn:FormalExpression', { body: '' });
      expression.$parent = event;
      modeling.updateModdleProperties(element, event, Object.fromEntries(timerKinds.map(key => [key, key === kind ? expression : undefined])));
    }
  });
}
function TimerValue(props: any) {
  const { element } = props;
  const modeling = useService('modeling');
  const moddle = useService('moddle');
  const debounce = useService('debounceInput');
  const event = eventDefinition(element);
  const kind = timerKinds.find(key => event[key]) || 'timeDuration';
  return TextFieldEntry({ ...props, label: 'Timer expression', debounce,
    getValue: () => event[kind]?.body || '',
    setValue: (value: string) => {
      if (event[kind]) modeling.updateModdleProperties(element, event[kind], { body: value });
      else {
        const expression = moddle.create('bpmn:FormalExpression', { body: value });
        expression.$parent = event;
        modeling.updateModdleProperties(element, event, { [kind]: expression });
      }
    }
  });
}

export class EventSubprocessPropertiesProvider {
  static $inject = ['propertiesPanel'];
  constructor(propertiesPanel: any) { propertiesPanel.registerProvider(500, this); }
  getGroups(element: any) {
    return (groups: any[]) => {
      if (!is(element, 'bpmn:StartEvent') || !element.businessObject.$parent?.triggeredByEvent) return groups;
      const event = eventDefinition(element);
      if (!event || (!references[event.$type] && !is(event, 'bpmn:TimerEventDefinition'))) return groups;
      const entries = [];
      if (!is(event, 'bpmn:ErrorEventDefinition')) entries.push({ id: 'event-subprocess-interrupting', element, component: Interrupting, isEdited: isCheckboxEntryEdited });
      if (references[event.$type]) entries.push(
        { id: 'event-subprocess-reference', element, component: ReferenceId, isEdited: isTextFieldEntryEdited },
        { id: 'event-subprocess-value', element, component: ReferenceValue, isEdited: isTextFieldEntryEdited }
      );
      else entries.push(
        { id: 'event-subprocess-timer-kind', element, component: TimerKind, isEdited: isSelectEntryEdited },
        { id: 'event-subprocess-timer-value', element, component: TimerValue, isEdited: isTextFieldEntryEdited }
      );
      groups.push({ id: 'EventSubprocessGroup', label: 'Event subprocess start', shouldOpen: true, entries });
      return groups;
    };
  }
}
