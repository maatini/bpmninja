#!/usr/bin/env bash
# Reproduces the Quality-Metrics workflow failure: remote main moves
# between commit and push. The shipped script must rebase and push
# without --force, then both histories must be reachable on the remote.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SHIPPED="$(cd "$SCRIPT_DIR/.." && pwd)/commit-and-push-quality-metrics.sh"

if [[ ! -x "$SHIPPED" && ! -f "$SHIPPED" ]]; then
  echo "FAIL: shipped script missing: $SHIPPED"
  exit 1
fi

if grep -E -- '--force|--force-with-lease' "$SHIPPED"; then
  echo "FAIL: shipped script must not force-push"
  exit 1
fi

WORKDIR="${1:-}"
if [[ -z "$WORKDIR" ]]; then
  echo "FAIL: workdir argument required"
  exit 1
fi
mkdir -p "$WORKDIR"
WORKDIR="$(cd "$WORKDIR" && pwd)"

REMOTE="$WORKDIR/remote.git"
WORK="$WORKDIR/work"
OTHER="$WORKDIR/other"

rm -rf "$REMOTE" "$WORK" "$OTHER"
git init --bare --initial-branch=main "$REMOTE"
git clone "$REMOTE" "$WORK"

git -C "$WORK" checkout -B main
git -C "$WORK" config user.name "tester"
git -C "$WORK" config user.email "tester@example.com"
mkdir -p "$WORK/docs"
echo '{"schemaVersion":1}' > "$WORK/docs/quality-metrics.json"
echo '{"schemaVersion":1}' > "$WORK/docs/quality-badges.json"
echo "# README" > "$WORK/README.md"
git -C "$WORK" add docs/quality-metrics.json docs/quality-badges.json README.md
git -C "$WORK" commit -m "initial"
git -C "$WORK" push -u origin main

git clone "$REMOTE" "$OTHER"
git -C "$OTHER" config user.name "other"
git -C "$OTHER" config user.email "other@example.com"
echo "remote moved" >> "$OTHER/README.md"
git -C "$OTHER" add README.md
git -C "$OTHER" commit -m "remote advances main"
git -C "$OTHER" push origin main
OTHER_SHA="$(git -C "$OTHER" rev-parse HEAD)"

echo '{"schemaVersion":1,"updated":true}' > "$WORK/docs/quality-metrics.json"

set +e
(cd "$WORK" && bash "$SHIPPED")
RC=$?
set -e

if [[ $RC -ne 0 ]]; then
  echo "FAIL: shipped script exited $RC when remote was ahead"
  exit 1
fi

git -C "$OTHER" pull --ff-only origin main
METRICS_MSG="$(git -C "$OTHER" log --oneline --grep='aktualisiere qualitaetsmetriken')"
if [[ -z "$METRICS_MSG" ]]; then
  echo "FAIL: metrics commit not on remote after script"
  exit 1
fi

if ! git -C "$OTHER" merge-base --is-ancestor "$OTHER_SHA" HEAD; then
  echo "FAIL: remote-ahead commit was rewritten (force-push?)"
  exit 1
fi

echo "OK: script rebased and pushed; both commits reachable"
