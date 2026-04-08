#!/usr/bin/env bash
# External Oracle for bpmninja-engine (arXiv 2604.01687)
# Returns only PASS or FAIL + exit code. No diagnostic details.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "=== External Oracle: engine-core ==="

# Step 1: Build check
if ! cargo build -p engine-core --quiet 2>/dev/null; then
  echo "FAIL (build)"
  exit 1
fi

# Step 2: Clippy (deny all warnings)
if ! cargo clippy -p engine-core --all-targets -- -D warnings 2>/dev/null; then
  echo "FAIL (clippy)"
  exit 2
fi

# Step 3: Tests
if ! cargo test -p engine-core --quiet 2>/dev/null; then
  echo "FAIL (test)"
  exit 3
fi

echo "PASS"
exit 0
