import { isTextFieldEntryEdited, TextFieldEntry } from '@bpmn-io/properties-panel';
// @ts-ignore
import { useService } from 'bpmn-js-properties-panel';
// @ts-ignore
import { is } from 'bpmn-js/lib/util/ModelUtil';

function AssigneeProps(props: any) {
  const { element, id } = props;

  const modeling = useService('modeling');
  const translate = useService('translate');
  const debounce = useService('debounceInput');

  const getValue = () => {
    return (
      element.businessObject.get('camunda:assignee') ||
      element.businessObject.get('data-assignee') ||
      ''
    );
  };

  const setValue = (value: string) => {
    modeling.updateProperties(element, {
      'camunda:assignee': value,
      'data-assignee': value
    });
  };

  return TextFieldEntry({
    element,
    id: id + '-assignee',
    label: translate('Bearbeiter'),
    description: translate('z.B. alice oder ${assignee}'),
    getValue,
    setValue,
    debounce
  });
}

function CustomAssigneeGroup(element: any, translate: any) {
  if (!is(element, 'bpmn:UserTask')) {
    return null;
  }

  return {
    id: 'AssigneeGroup',
    label: translate('User Task'),
    shouldOpen: true,
    entries: [
      {
        id: 'userTaskAssignee',
        element,
        component: AssigneeProps,
        isEdited: isTextFieldEntryEdited
      }
    ]
  };
}

export class AssigneePropertiesProvider {
  static $inject = ['propertiesPanel', 'translate'];

  constructor(propertiesPanel: any, translate: any) {
    propertiesPanel.registerProvider(500, this);
    this.translate = translate;
  }

  translate: any;

  getGroups(element: any) {
    return (groups: any[]) => {
      const group = CustomAssigneeGroup(element, this.translate);
      if (group) {
        groups.push(group);
      }
      return groups;
    };
  }
}
