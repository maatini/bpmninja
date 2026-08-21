#!/usr/bin/env python3
"""Merge incoming fuzz/mutation metric artifacts into docs/quality-metrics.json."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def find_incoming(name: str) -> Path | None:
    direct = Path("incoming-metrics") / name
    if direct.is_file():
        return direct
    matches = sorted(Path("incoming-metrics").glob(f"**/{name}"))
    return matches[0] if matches else None


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    return data if isinstance(data, dict) else {}


def mutation_has_signal(block: dict[str, Any]) -> bool:
    if block.get("status") == "ok" and int(block.get("total_tested") or 0) > 0:
        return True
    return int(block.get("total_tested") or 0) > 0


def merge_metrics(
    existing: dict[str, Any],
    fuzz: dict[str, Any] | None,
    mutation: dict[str, Any] | None,
) -> dict[str, Any]:
    metrics = dict(existing)
    metrics.setdefault(
        "fuzz",
        {
            "status": "no-data",
            "targets_total": 0,
            "targets_passed": 0,
            "targets_failed": 0,
            "targets": [],
        },
    )
    metrics.setdefault(
        "mutation",
        {
            "status": "no-data",
            "mutation_score": 0.0,
            "caught": 0,
            "missed": 0,
            "timeout": 0,
            "unviable": 0,
            "total_tested": 0,
        },
    )

    if fuzz:
        metrics["fuzz"] = fuzz

    if mutation:
        previous = metrics.get("mutation") if isinstance(metrics.get("mutation"), dict) else {}
        incoming_empty = not mutation_has_signal(mutation)
        previous_good = mutation_has_signal(previous)
        if incoming_empty and previous_good:
            # Keep last successful mutation signal instead of publishing 0.0%.
            kept = dict(previous)
            kept["retained"] = True
            kept["incoming_status"] = mutation.get("status", "no-data")
            metrics["mutation"] = kept
        else:
            metrics["mutation"] = mutation

    metrics["updated_at"] = datetime.now(timezone.utc).isoformat()
    return metrics


def render_readme_block(metrics: dict[str, Any]) -> str:
    mutation = metrics.get("mutation") or {}
    fuzz = metrics.get("fuzz") or {}
    mutation_score = float(mutation.get("mutation_score") or 0.0)
    fuzz_ok = fuzz.get("status") == "ok"
    fuzz_message = f"{fuzz.get('targets_passed', 0)}/{fuzz.get('targets_total', 0)} targets"
    start = "<!-- QUALITY_METRICS:START -->"
    end = "<!-- QUALITY_METRICS:END -->"
    return (
        f"{start}\n"
        f"- Letztes Update (UTC): `{metrics['updated_at']}`\n"
        f"- Mutation Score: **{mutation_score:.1f}%** "
        f"(caught: {mutation.get('caught', 0)}, "
        f"missed: {mutation.get('missed', 0)}, "
        f"timeout: {mutation.get('timeout', 0)})\n"
        f"- Fuzzing: **{fuzz_message}** "
        f"({'ok' if fuzz_ok else 'failed'})\n"
        f"{end}"
    )


def patch_readme(readme: str, block: str) -> str:
    start_marker = "<!-- QUALITY_METRICS:START -->"
    end_marker = "<!-- QUALITY_METRICS:END -->"
    if start_marker in readme and end_marker in readme:
        before = readme.split(start_marker)[0]
        after = readme.split(end_marker, 1)[1]
        return before + block + after
    return readme + "\n## Quality Metrics (Auto)\n\n" + block + "\n"


def write_badges(path: Path, mutation_score: float) -> None:
    color = "success" if mutation_score >= 75 else ("yellow" if mutation_score >= 60 else "red")
    badges = {
        "schemaVersion": 1,
        "label": "Mutation Score",
        "message": f"{mutation_score:.1f}%",
        "color": color,
        "namedLogo": "rust",
        "links": ["https://github.com/maatini/bpmninja/actions/workflows/mutation-tests.yml"],
    }
    path.write_text(json.dumps(badges, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--metrics", default="docs/quality-metrics.json")
    parser.add_argument("--badges", default="docs/quality-badges.json")
    parser.add_argument("--readme", default="README.md")
    parser.add_argument("--incoming-fuzz", default="incoming-metrics/fuzz-metrics.json")
    parser.add_argument("--incoming-mutation", default="incoming-metrics/mutation-metrics.json")
    args = parser.parse_args()

    metrics_path = Path(args.metrics)
    existing = load_json(metrics_path)
    fuzz_path = Path(args.incoming_fuzz)
    mutation_path = Path(args.incoming_mutation)
    if not fuzz_path.is_file():
        found = find_incoming("fuzz-metrics.json")
        fuzz_path = found or fuzz_path
    if not mutation_path.is_file():
        found = find_incoming("mutation-metrics.json")
        mutation_path = found or mutation_path
    fuzz = load_json(fuzz_path) or None
    mutation = load_json(mutation_path) or None
    if fuzz == {}:
        fuzz = None
    if mutation == {}:
        mutation = None

    metrics = merge_metrics(existing, fuzz, mutation)
    metrics_path.parent.mkdir(parents=True, exist_ok=True)
    metrics_path.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")

    mutation_score = float((metrics.get("mutation") or {}).get("mutation_score") or 0.0)
    write_badges(Path(args.badges), mutation_score)

    readme_path = Path(args.readme)
    readme = readme_path.read_text(encoding="utf-8") if readme_path.is_file() else ""
    readme_path.write_text(patch_readme(readme, render_readme_block(metrics)), encoding="utf-8")


if __name__ == "__main__":
    main()
