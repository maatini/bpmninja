# mini-bpm

An embeddable BPMN 2.0 Workflow Engine in Rust.

## Crates

* `bpmn-parser`: Parses BPMN 2.0 XML definitions into internal Rust structures.
* `engine-core`: The main workflow engine library handling tokens, state execution, and tasks.
* `persistence-nats`: (Optional) Provides NATS-based persistence and JetStream event publishing.
* `engine-server`: A standalone Axum-based HTTP server offering REST API endpoints for the engine.
* `desktop-tauri`: A Tauri desktop application interacting with the workflow engine.

## Running the Engine Server

To start the HTTP REST API server:

```bash
# Start NATS (if you plan to use persistence)
docker-compose up -d nats

# Run the engine server
cargo run -p engine-server
```

The server listens on `http://localhost:8080` by default.

### Endpoints
* `POST /api/deploy` - Deploy a BPMN definition
* `POST /api/start` - Start a new process instance
* `GET /api/tasks` - List all pending user tasks
* `POST /api/complete/:id` - Complete a user task
* `GET /api/instances` - List all process instances
* `GET /api/instances/:id` - Get details of a single instance

## Running the Desktop Application

The `mini-bpm-desktop` application can run in two modes:

1. **Embedded Engine (Default)**: The app runs its own in-memory (or NATS-backed) `WorkflowEngine` inside the Tauri backend.
   ```bash
   cargo run -p mini-bpm-desktop
   ```

2. **HTTP Backend**: The app connects to the `engine-server` instance over HTTP.
   ```bash
   cargo run -p mini-bpm-desktop --features http-backend
   ```
   *Note: Ensure `engine-server` is running before starting the app in this mode. You can configure the API endpoint with the `ENGINE_API_URL` environment variable.*

## Docker Compose

You can spin up the full infrastructure (NATS and `engine-server`) via:

```bash
docker-compose up --build
```
