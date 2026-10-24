#!/usr/bin/env bash
# Überträgt die erzeugten Qualitätsmetriken auf den Branch `metrics`.
# `main` bleibt frei von Bot-Commits; der Badge im README zeigt auf `metrics`.
# Existiert der Branch noch nicht, wird er vom aktuellen main-Stand angelegt.
# Bei abgelehntem Push: fetch + rebase und erneut versuchen. Nie force-push.
set -euo pipefail

metrics_files=(docs/quality-metrics.json docs/quality-badges.json README.md)
remote="${QUALITY_METRICS_REMOTE:-origin}"
branch="${QUALITY_METRICS_BRANCH:-metrics}"
max_attempts="${QUALITY_METRICS_PUSH_ATTEMPTS:-5}"

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

staging_dir="$(mktemp -d)"
trap 'rm -rf "$staging_dir"' EXIT

# Merge-Ergebnis sichern, bevor der Branch-Wechsel den Arbeitsbaum überschreibt.
for file in "${metrics_files[@]}"; do
  if [ -f "$file" ]; then
    mkdir -p "$staging_dir/$(dirname "$file")"
    cp "$file" "$staging_dir/$file"
  fi
done

# Arbeitsbaum bereinigen; die erzeugten Dateien liegen in $staging_dir.
git checkout -- docs README.md

if git fetch "$remote" "$branch"; then
  git checkout -B "$branch" "${remote}/${branch}"
else
  echo "Branch ${branch} existiert noch nicht; lege ihn vom aktuellen Stand an."
  git checkout -B "$branch"
fi

for file in "${metrics_files[@]}"; do
  if [ -f "$staging_dir/$file" ]; then
    mkdir -p "$(dirname "$file")"
    cp "$staging_dir/$file" "$file"
  fi
done

git add "${metrics_files[@]}"
if git diff --cached --quiet; then
  echo "Keine Änderungen an den Qualitätsmetriken."
  exit 0
fi

git commit -m "chore(docs): aktualisiere qualitaetsmetriken automatisch"

for attempt in $(seq 1 "$max_attempts"); do
  if git push "$remote" "HEAD:${branch}"; then
    exit 0
  fi
  echo "Push abgelehnt (Versuch ${attempt}/${max_attempts}); hole ${remote}/${branch} und rebase."
  git fetch "$remote" "$branch"
  if ! git rebase "${remote}/${branch}"; then
    echo "Rebase fehlgeschlagen; breche Rebase ab."
    git rebase --abort || true
    continue
  fi
done

echo "Push nach ${max_attempts} Rebase-Versuchen fehlgeschlagen."
exit 1
