import { isTextAreaEntryEdited, TextAreaEntry } from '@bpmn-io/properties-panel';
// @ts-ignore
import { useService } from 'bpmn-js-properties-panel';
// @ts-ignore
import { is } from 'bpmn-js/lib/util/ModelUtil';

type MappingValue = string | MappingValue[] | { [key: string]: MappingValue };

function validateValue(value: unknown): asserts value is MappingValue {
  if (typeof value === 'string') {
    if ((value.includes('${') || value.includes('#{')) && !/^[#$]\{[\p{L}\p{N}_-]+\}$/u.test(value.trim())) {
      throw new Error('Use a literal or ${variable}; expression interpolation is unsupported.');
    }
  } else if (Array.isArray(value)) {
    value.forEach(validateValue);
  } else if (value && typeof value === 'object') {
    Object.values(value).forEach(validateValue);
  } else {
    throw new Error('Parameter values must be strings, lists, or maps. Use process defaults for typed JSON values.');
  }
}

export function parseMappingEditor(value: string): Record<string, MappingValue> {
  const parsed = JSON.parse(value || '{}');
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('Enter a JSON object.');
  for (const [name, item] of Object.entries(parsed)) {
    if (!name.trim()) throw new Error('A parameter name cannot be empty.');
    validateValue(item);
  }
  return parsed;
}

function readValue(parameter: any): MappingValue {
  const definition = parameter.definition || (is(parameter, 'camunda:List') || is(parameter, 'camunda:Map') ? parameter : undefined);
  if (!definition) {
    if (parameter.value !== undefined || is(parameter, 'camunda:Value')) return parameter.value || '';
    if (is(parameter, 'camunda:Script')) throw new Error('Unsupported parameter script.');
    return '';
  }
  if (is(definition, 'camunda:List')) return (definition.items || []).map(readValue);
  if (is(definition, 'camunda:Map')) return Object.fromEntries((definition.entries || []).map((entry: any) => [entry.key, readValue(entry)]));
  throw new Error('This parameter uses an unsupported script. Edit it in the XML source.');
}

function valueProperties(moddle: any, value: MappingValue): any {
  if (typeof value === 'string') return { value };
  if (Array.isArray(value)) {
    const definition = moddle.create('camunda:List');
    definition.items = value.map(item => {
      const properties = valueProperties(moddle, item);
      const child = properties.definition || moddle.create('camunda:Value', properties);
      child.$parent = definition;
      return child;
    });
    return { definition };
  }
  const definition = moddle.create('camunda:Map');
  definition.entries = Object.entries(value).map(([key, item]) => {
    const child = moddle.create('camunda:Entry', { key, ...valueProperties(moddle, item) });
    child.$parent = definition;
    if (child.definition) child.definition.$parent = child;
    return child;
  });
  return { definition };
}

function MappingEditor(props: any) {
  const { element, id, direction, transfer } = props;
  const modeling = useService('modeling');
  const moddle = useService('moddle');
  const debounce = useService('debounceInput');
  const getValue = () => {
    const extensions = element.businessObject.extensionElements?.values || [];
    if (transfer) {
      const mappings = extensions.filter((item: any) => is(item, direction === 'input' ? 'camunda:In' : 'camunda:Out'));
      // Preserve unsupported transfer-all/local forms; never overwrite them through this editor.
      if (mappings.some((item: any) => !item.target || (!item.source && !item.sourceExpression) || item.variables || item.local || item.businessKey)) return '';
      return JSON.stringify(Object.fromEntries(mappings.map((item: any) => [item.target, item.sourceExpression || '${' + item.source + '}'])), null, 2);
    }
    const io = extensions.find((item: any) => is(item, 'camunda:InputOutput'));
    try {
      return JSON.stringify(Object.fromEntries((io?.[direction === 'input' ? 'inputParameters' : 'outputParameters'] || []).map((item: any) => [item.name, readValue(item)])), null, 2);
    } catch {
      return '';
    }
  };
  const validate = (value: string) => {
    try {
      const mappings = parseMappingEditor(value);
      if (transfer && Object.values(mappings).some(item => typeof item !== 'string' || !/^[#$]\{[\p{L}\p{N}_-]+\}$/u.test(item.trim()))) {
        return 'Call transfers require ${variable} values.';
      }
      return undefined;
    } catch (error) { return (error as Error).message; }
  };
  const setValue = (value: string) => {
    if (validate(value)) return;
    const mappings = parseMappingEditor(value);
    const bo = element.businessObject;
    const extensions = bo.extensionElements || moddle.create('bpmn:ExtensionElements', { values: [] });
    const existing = extensions.values || [];
    let values: any[];
    if (transfer) {
      const type = direction === 'input' ? 'camunda:In' : 'camunda:Out';
      if (existing.some((item: any) => is(item, type) && (item.variables || item.local || item.businessKey || !item.target))) return;
      values = existing.filter((item: any) => !is(item, type));
      for (const [target, sourceExpression] of Object.entries(mappings)) values.push(moddle.create(type, { target, sourceExpression }));
    } else {
      const old = existing.find((item: any) => is(item, 'camunda:InputOutput'));
      // Clone both directions, retaining the unedited values and undo semantics.
      const io = moddle.create('camunda:InputOutput', {
        inputParameters: [...(old?.inputParameters || [])],
        outputParameters: [...(old?.outputParameters || [])]
      });
      const property = direction === 'input' ? 'inputParameters' : 'outputParameters';
      try { (old?.[property] || []).forEach(readValue); } catch { return; }
      io[property] = Object.entries(mappings).map(([name, item]) => {
        const parameter = moddle.create(direction === 'input' ? 'camunda:InputParameter' : 'camunda:OutputParameter', { name, ...valueProperties(moddle, item) });
        parameter.$parent = io;
        if (parameter.definition) parameter.definition.$parent = parameter;
        return parameter;
      });
      values = existing.filter((item: any) => !is(item, 'camunda:InputOutput'));
      values.push(io);
    }
    // Replace the extension container through a single undoable modeling command.
    const replacement = moddle.create('bpmn:ExtensionElements', { values });
    replacement.$parent = bo;
    for (const child of values) child.$parent = replacement;
    modeling.updateProperties(element, { extensionElements: replacement });
  };
  return TextAreaEntry({
    element, id, label: transfer ? (direction === 'input' ? 'Child inputs' : 'Child outputs') : (direction === 'input' ? 'Activity inputs' : 'Activity outputs'),
    description: transfer ? 'JSON: {"target": "${source}"}. Adding either direction makes both transfers explicit.' : 'JSON: {"id": "${customerId}"}. Values: literal strings, references, lists, maps.',
    getValue, setValue, validate, debounce
  });
}

function DefaultsEditor(props: any) {
  const { element, id } = props;
  const modeling = useService('modeling');
  const moddle = useService('moddle');
  const debounce = useService('debounceInput');
  const bo = element.businessObject;
  const getValue = () => {
    const values: Record<string, unknown> = {};
    for (const property of bo.properties || []) {
      const value = property.get('bpmninja:default');
      if (value !== undefined) {
        try { values[property.name || property.id] = JSON.parse(value); }
        catch { return ''; }
      }
    }
    return JSON.stringify(values, null, 2);
  };
  const validate = (value: string) => {
    try {
      const parsed = JSON.parse(value || '{}');
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return 'Enter a JSON object of process variable defaults.';
      if (Object.keys(parsed).some(name => !name.trim())) return 'A variable name cannot be empty.';
      return undefined;
    } catch { return 'Enter valid JSON.'; }
  };
  const setValue = (value: string) => {
    if (validate(value)) return;
    const defaults: Record<string, unknown> = JSON.parse(value || '{}');
    const remaining = new Set(Object.keys(defaults));
    const properties = (bo.properties || []).map((old: any) => {
      const name = old.name || old.id;
      if (!remaining.has(name) && old.get('bpmninja:default') === undefined) return old;
      const property = moddle.create('bpmn:Property');
      for (const descriptor of old.$descriptor.properties) {
        if (descriptor.name !== 'bpmninja:default' && !descriptor.isVirtual && old.get(descriptor.name) !== undefined) {
          property.set(descriptor.name, old.get(descriptor.name));
        }
      }
      property.$attrs = { ...old.$attrs };
      property.$parent = bo;
      if (remaining.has(name)) {
        property.set('bpmninja:default', JSON.stringify(defaults[name]));
        remaining.delete(name);
      }
      return property;
    });
    for (const name of remaining) {
      const property = moddle.create('bpmn:Property', { id: 'Property_' + crypto.randomUUID().replaceAll('-', ''), name });
      property.set('bpmninja:default', JSON.stringify(defaults[name]));
      property.$parent = bo;
      properties.push(property);
    }
    modeling.updateProperties(element, { properties });
  };
  return TextAreaEntry({ element, id, label: 'Process variable defaults', description: 'Typed JSON defaults, applied only at start. Start variables override these values.', getValue, setValue, validate, debounce });
}

function ObjectDefaultEditor(props: any) {
  const { element, id } = props;
  const modeling = useService('modeling');
  const debounce = useService('debounceInput');
  const object = element.businessObject.dataObjectRef || element.businessObject;
  return TextAreaEntry({
    element, id, label: 'Data object default (JSON)',
    description: 'Leave empty for a declaration without a value.',
    getValue: () => object.get('bpmninja:default') || '',
    validate: (value: string) => { try { if (value) JSON.parse(value); return undefined; } catch { return 'Enter valid JSON.'; } },
    setValue: (value: string) => {
      try { if (value) JSON.parse(value); } catch { return; }
      modeling.updateModdleProperties(element, object, { 'bpmninja:default': value || undefined });
    }, debounce
  });
}

export class DataMappingsPropertiesProvider {
  static $inject = ['propertiesPanel'];
  constructor(propertiesPanel: any) { propertiesPanel.registerProvider(500, this); }
  getGroups(element: any) {
    return (groups: any[]) => {
      if (is(element, 'bpmn:Process')) {
        groups.push({ id: 'processData', label: 'Process data', entries: [{ id: 'process-data-defaults', element, component: DefaultsEditor, isEdited: isTextAreaEntryEdited }] });
        return groups;
      }
      if (is(element, 'bpmn:DataObject') || is(element, 'bpmn:DataObjectReference')) {
        groups.push({ id: 'objectData', label: 'Process data', entries: [{ id: 'data-object-default', element, component: ObjectDefaultEditor, isEdited: isTextAreaEntryEdited }] });
        return groups;
      }
      if (!is(element, 'bpmn:Task') && !is(element, 'bpmn:CallActivity')) return groups;
      // Receive tasks execute as catch events and cannot have activation mappings yet.
      if (is(element, 'bpmn:ReceiveTask')) return groups;
      const entries = ['input', 'output'].map(direction => ({
        id: 'data-' + direction, element, component: (props: any) => MappingEditor({ ...props, direction, transfer: false }), isEdited: isTextAreaEntryEdited
      }));
      if (is(element, 'bpmn:CallActivity')) for (const direction of ['input', 'output']) entries.push({
        id: 'call-data-' + direction, element, component: (props: any) => MappingEditor({ ...props, direction, transfer: true }), isEdited: isTextAreaEntryEdited
      });
      groups.push({ id: 'dataMappings', label: 'Data mappings', entries });
      return groups;
    };
  }
}
