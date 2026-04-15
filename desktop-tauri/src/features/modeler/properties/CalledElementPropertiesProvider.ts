import { isTextFieldEntryEdited, TextFieldEntry } from '@bpmn-io/properties-panel';
// @ts-ignore
import { useService } from 'bpmn-js-properties-panel';
// @ts-ignore
import { is } from 'bpmn-js/lib/util/ModelUtil';

function CalledElementProps(props: any) {
  const { element, id } = props;

  const modeling = useService('modeling');
  const translate = useService('translate');
  const debounce = useService('debounceInput');

  const getValue = () => {
    return element.businessObject.get('calledElement') || '';
  };

  const setValue = (value: string) => {
    modeling.updateProperties(element, {
      calledElement: value
    });
  };

  return TextFieldEntry({
    element,
    id: id + '-calledElement',
    label: translate('Aufgerufener Prozess'),
    description: translate('Prozess-ID des aufzurufenden Workflows (z.B. child-process)'),
    getValue,
    setValue,
    debounce
  });
}

function CalledElementGroup(element: any, translate: any) {
  if (!is(element, 'bpmn:CallActivity')) {
    return null;
  }

  return {
    id: 'CalledElementGroup',
    label: translate('Call Activity'),
    shouldOpen: true,
    entries: [
      {
        id: 'calledElement',
        element,
        component: CalledElementProps,
        isEdited: isTextFieldEntryEdited
      }
    ]
  };
}

export class CalledElementPropertiesProvider {
  static $inject = ['propertiesPanel', 'translate'];

  constructor(propertiesPanel: any, translate: any) {
    propertiesPanel.registerProvider(500, this);
    this.translate = translate;
  }

  translate: any;

  getGroups(element: any) {
    return (groups: any[]) => {
      const group = CalledElementGroup(element, this.translate);
      if (group) {
        groups.push(group);
      }
      return groups;
    };
  }
}
