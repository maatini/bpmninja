# BPMNinja Desktop (Tauri + React + bpmn-js)

Desktop-Anwendung für BPMNinja mit BPMN-Modeler, Live-Instanzansichten und Monitoring.

## Voraussetzungen

- Node.js >= 18
- Rust Toolchain (für Tauri)
- Laufender Backend-Stack (`engine-server` + NATS), standardmäßig unter `http://localhost:8081`

## Entwicklung

```bash
cd desktop-tauri
npm install
npm run tauri dev
```

Alternativ im Repository-Root:

```bash
devbox run ui:dev
```

## Qualitätssicherung

```bash
cd desktop-tauri
npm run lint
npm run build
npm run test:e2e
```

## Zentrale Features

- BPMN-Modeler auf Basis von `bpmn-js` inkl. Properties Panel
- Live-Updates über SSE-Events des Backends
- Instanz- und Definitionsansichten mit Token/Task-Kontext
- Monitoring-Ansichten inkl. Logs, Storage-Status und Engine-Metriken

## Flow Conditions (Camunda-7-kompatibel)

Der Modeler unterstützt Flow Conditions analog zum Camunda Modeler:

- Condition-Gruppe nur bei Sequence Flows von Exclusive/Inclusive Gateways
- Keine Condition-Eingabe bei als Default markierten Flows
- Condition Type: `None`, `Expression`, `Script`
- XML-Ausgabe:
  - Expression: `<conditionExpression xsi:type="bpmn:tFormalExpression">${...}</conditionExpression>`
  - Script: `<conditionExpression xsi:type="bpmn:tFormalExpression" language="rhai">...</conditionExpression>`
