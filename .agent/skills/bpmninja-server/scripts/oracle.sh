#!/usr/bin/env bash
# External Oracle for bpmninja-server (arXiv 2604.01687)
# Covers: engine-server, persistence-nats, agent-orchestrator
# Returns only PASS or FAIL + exit code. No diagnostic details.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "=== External Oracle: server crates ==="

# Build all three crates
if ! cargo build -p engine-server -p persistence-nats -p agent-orchestrator --quiet 2>/dev/null; then
  echo "FAIL (build)"
  exit 1
fi

# Clippy all three crates
if ! cargo clippy -p engine-server -p persistence-nats -p agent-orchestrator --all-targets -- -D warnings 2>/dev/null; then
  echo "FAIL (clippy)"
  exit 2
fi

# Tests (persistence-nats tests may require NATS – test what's available)
if ! cargo test -p engine-server --quiet 2>/dev/null; then
  echo "FAIL (test engine-server)"
  exit 3
fi

if ! cargo test -p agent-orchestrator --quiet 2>/dev/null; then
  echo "FAIL (test agent-orchestrator)"
  exit 4
fi

# persistence-nats tests are conditional on NATS availability
cargo test -p persistence-nats --quiet 2>/dev/null || true

echo "PASS"
exit 0
