# Data defaults and mappings (X11)

BPMNinja supports process data declarations, task-local inputs, successful task
outputs, and explicit Call Activity transfers. These are a supported subset of
BPMN/Camunda data semantics; unsupported executable forms fail deployment.

## Process data

Process-level `property` and `dataObject` declare variables using `name` (or `id`
when name is absent). A `dataObjectReference` aliases its object's variable.
Declarations without a default do not create null values or overwrite start
variables. The explicit extension `bpmninja:default` contains JSON:

```xml
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
             xmlns:bpmninja="https://bpmninja.dev/schema/bpmn">
  <process id="orders" isExecutable="true">
    <property id="Customer" name="customerId" bpmninja:default="7" />
    <dataObject id="Flags" name="flags"
                bpmninja:default="{&quot;enabled&quot;:true}" />
    <!-- start, tasks, end, and flows -->
  </process>
</definitions>
```

Defaults apply to plain, message, signal, timer, and child-process starts. Supplied
start variables override defaults. Recovery never reapplies defaults.

## Camunda activity parameters

Tasks and Call Activities accept `camunda:inputOutput`, with `inputParameter` and
`outputParameter`. Literal text is a string. `${variable}` and `#{variable}` copy
the existing JSON value, retaining its type. Variable names in expressions may
contain letters, digits, `_`, and `-`. No arithmetic, property paths, interpolation,
FEEL/JUEL, scripts, or external resources are evaluated by mappings.

```xml
<extensionElements xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
  <camunda:inputOutput>
    <camunda:inputParameter name="id">${customerId}</camunda:inputParameter>
    <camunda:inputParameter name="options">
      <camunda:map>
        <camunda:entry key="tags">
          <camunda:list><camunda:value>priority</camunda:value></camunda:list>
        </camunda:entry>
      </camunda:map>
    </camunda:inputParameter>
    <camunda:outputParameter name="receipt">${result}</camunda:outputParameter>
  </camunda:inputOutput>
</extensionElements>
```

Lists and maps nest, including lists/maps as list items. Each mapping phase reads
one source snapshot; parameters cannot depend on an earlier parameter in that
same phase. Empty or duplicate targets fail deployment. Missing runtime variables
fail execution instead of silently becoming null.

Inputs apply once per activation before start listeners. They are local to the
activation, including each loop iteration. Workers receive the effective task
variables. The process variable map hides local input names; an input shadowing an existing
process variable does not overwrite the process value.

On success, outputs apply before end listeners and outgoing conditions. When
output parameters exist, only their named writes escape the activity (plus
explicit Call Activity output transfers and end-listener writes). When there are
no output parameters, existing task writes propagate, with local input names
restored/removed. Definitions without mappings retain their existing behavior.

Activation state is persisted with the token. Worker retries and restart keep
the original inputs; they do not rerun them. A mapping/listener failure during
user/service-task completion restores the pending task for retry and preserves
its boundary subscriptions. Interrupting boundaries discard activation state
without applying success outputs. Initial synchronous mapping failures terminate
the instance with `DATA_MAPPING_ERROR`; HTTP returns a clear error. Mapping state
stores a process-variable snapshot, so mapped activations consume additional
memory and persistence space proportional to their variables.

Process-variable updates refresh the activation's process snapshot and nonlocal
variables while preserving local inputs. They do not change the already-created
worker snapshot or reevaluate inputs.

## Standard associations

Task-level `ioSpecification` accepts `dataInput`, `dataOutput`, `inputSet`, and
`outputSet`. A task `property` can also serve as a local endpoint. Each executable
association requires one source and one target:

- Input: a process property/data object/reference → task data input/property.
- Output: a task data output/property → process property/data object/reference.

IDs resolve within the enclosing process. Dangling or ambiguous references,
duplicate mapping targets, transformations, assignments, and IO collection
semantics are rejected. Data stores remain visual references, with no external
storage adapter. Process/subprocess IO and subprocess data declarations require
additional runtime-scope support (X6) and are rejected. Supported task mappings
inside a flattened subprocess can reference enclosing process data.

## Call Activities

`camunda:in` / `camunda:out` accept `source` or a variable `sourceExpression`, plus
`target`. `in` reads the effective parent activation; `out` reads completed child
variables. Adding either direction makes both transfers explicit: an omitted
direction transfers no variables. With neither direction present, legacy
transfer-all remains. `variables="all"`, local-scope transfer, business-key
mappings, and compound expressions are rejected.

Successful child transfer precedes the Call Activity's ordinary output mappings.
Failed children never apply success outputs; existing error-boundary/incident
handling applies. Child activation/parent mapping state survives persistence via
the outstanding-call token.

## Modeler and compatibility

The properties panel has **Activity inputs/outputs** JSON editors and **Child
inputs/outputs** for Call Activities. String values represent literal strings or
variable references; arrays and objects represent Camunda lists/maps. Process
selection offers **Process variable defaults**, with typed JSON. Data-object
selection offers a JSON default. Removing a process default retains its property
declaration so existing references keep their IDs.

The original deployed XML remains the export source. Older persisted definitions
and tokens deserialize with empty mappings/defaults and no activation scope.
The existing bpmn-js canvas retains standard data-association diagrams.
