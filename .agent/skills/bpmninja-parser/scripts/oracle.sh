#!/usr/bin/env bash
# External Oracle for bpmninja-parser (arXiv 2604.01687)
# Returns only PASS or FAIL + exit code. No diagnostic details.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "=== External Oracle: bpmn-parser ==="

if ! cargo build -p bpmn-parser --quiet 2>/dev/null; then
  echo "FAIL (build)"
  exit 1
fi

if ! cargo clippy -p bpmn-parser --all-targets -- -D warnings 2>/dev/null; then
  echo "FAIL (clippy)"
  exit 2
fi

if ! cargo test -p bpmn-parser --quiet 2>/dev/null; then
  echo "FAIL (test)"
  exit 3
fi

echo "PASS"
exit 0
