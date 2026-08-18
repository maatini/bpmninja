#!/usr/bin/env bash
# Commit quality-metrics files and push. Used by update-quality-metrics.yml.
# If main moved after checkout, pull --rebase and retry. Never force-push.
set -euo pipefail

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

git add docs/quality-metrics.json docs/quality-badges.json README.md
if git diff --cached --quiet; then
  echo "No metrics updates."
  exit 0
fi
git -c user.name="github-actions[bot]" -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
  commit -m "chore(docs): aktualisiere qualitaetsmetriken automatisch"

branch="$(git rev-parse --abbrev-ref HEAD)"
remote="${QUALITY_METRICS_REMOTE:-origin}"
max_attempts="${QUALITY_METRICS_PUSH_ATTEMPTS:-5}"

for attempt in $(seq 1 "$max_attempts"); do
  if git push "$remote" "HEAD:${branch}"; then
    exit 0
  fi
  echo "Push rejected (attempt ${attempt}/${max_attempts}); fetching and rebasing onto ${remote}/${branch}"
  git fetch "$remote" "$branch"
  if ! git rebase "${remote}/${branch}"; then
    echo "Rebase failed; aborting rebase state"
    git rebase --abort || true
    continue
  fi
done

echo "Failed to push after ${max_attempts} rebase attempts"
exit 1
