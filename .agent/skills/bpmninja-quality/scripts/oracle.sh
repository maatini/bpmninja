#!/usr/bin/env bash
# External Oracle for bpmninja-quality (arXiv 2604.01687)
# Full workspace verification: build + clippy + test
# Returns only PASS or FAIL + exit code. No diagnostic details.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "=== External Oracle: workspace quality ==="

# Step 1: Full workspace build
if ! cargo build --workspace --quiet 2>/dev/null; then
  echo "FAIL (build)"
  exit 1
fi

# Step 2: Workspace clippy
if ! cargo clippy --workspace --all-targets -- -D warnings 2>/dev/null; then
  echo "FAIL (clippy)"
  exit 2
fi

# Step 3: Workspace tests
if ! cargo test --workspace --quiet 2>/dev/null; then
  echo "FAIL (test)"
  exit 3
fi

echo "PASS"
exit 0
