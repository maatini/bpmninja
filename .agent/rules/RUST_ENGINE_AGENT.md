# Execution Engine Agent
- **Domain:** `engine-core/`
- **Role:** Pure state machine, token advancement, `BpmnElement` logic.
- **Rules:** No network code (no NATS). Run timers via `tokio::time`. Define traits for everything external. 
