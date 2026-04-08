#!/usr/bin/env bash
# External Oracle for bpmninja-desktop (arXiv 2604.01687)
# Covers: desktop-tauri (Rust backend + TypeScript frontend)
# Returns only PASS or FAIL + exit code. No diagnostic details.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "=== External Oracle: desktop-tauri ==="

# Step 1: Rust backend build
if ! cargo build -p desktop-tauri --quiet 2>/dev/null; then
  echo "FAIL (rust-build)"
  exit 1
fi

# Step 2: Rust clippy
if ! cargo clippy -p desktop-tauri --all-targets -- -D warnings 2>/dev/null; then
  echo "FAIL (clippy)"
  exit 2
fi

# Step 3: Rust tests
if ! cargo test -p desktop-tauri --quiet 2>/dev/null; then
  echo "FAIL (rust-test)"
  exit 3
fi

# Step 4: TypeScript build check
cd desktop-tauri
if [ -f "package.json" ]; then
  if ! npm run build 2>/dev/null; then
    echo "FAIL (ts-build)"
    exit 4
  fi
fi

echo "PASS"
exit 0
