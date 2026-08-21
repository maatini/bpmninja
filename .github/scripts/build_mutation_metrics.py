#!/usr/bin/env python3
"""Build mutation-metrics.json from cargo-mutants output.

Looks for outcomes.json in common locations and falls back to
caught.txt / missed.txt / timeout.txt / unviable.txt line counts.
Does not treat a present-but-zero top-level counter object as the
only source of truth when the outcomes list has mutant entries.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

OUTCOME_CAUGHT = {"caughtmutant", "caught"}
OUTCOME_MISSED = {"missedmutant", "missed"}
OUTCOME_TIMEOUT = {"timeout", "timedout", "timeoutmutant"}
OUTCOME_UNVIABLE = {"unviable", "unviablemutant"}

OUTCOMES_CANDIDATES = (
    Path("mutants.out/outcomes.json"),
    Path("engine-core/mutants.out/outcomes.json"),
)


def _label(entry: dict[str, Any]) -> str:
    return str(entry.get("summary") or entry.get("status") or "").lower()


def count_from_outcomes_list(entries: list[Any]) -> tuple[int, int, int, int]:
    caught = missed = timeout = unviable = 0
    for item in entries:
        if not isinstance(item, dict):
            continue
        label = _label(item)
        if label in OUTCOME_CAUGHT:
            caught += 1
        elif label in OUTCOME_MISSED:
            missed += 1
        elif label in OUTCOME_TIMEOUT:
            timeout += 1
        elif label in OUTCOME_UNVIABLE:
            unviable += 1
    return caught, missed, timeout, unviable


def count_from_summary_dict(data: dict[str, Any]) -> tuple[int, int, int, int] | None:
    summary = data.get("summary") if isinstance(data.get("summary"), dict) else data
    if not isinstance(summary, dict):
        return None
    keys = ("caught", "missed", "timeout", "unviable")
    if not all(k in summary for k in keys):
        return None
    return (
        int(summary.get("caught") or 0),
        int(summary.get("missed") or 0),
        int(summary.get("timeout") or 0),
        int(summary.get("unviable") or 0),
    )


def count_txt_lines(path: Path) -> int:
    if not path.is_file():
        return 0
    n = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            n += 1
    return n


def counts_from_txt_dir(directory: Path) -> tuple[int, int, int, int]:
    return (
        count_txt_lines(directory / "caught.txt"),
        count_txt_lines(directory / "missed.txt"),
        count_txt_lines(directory / "timeout.txt"),
        count_txt_lines(directory / "unviable.txt"),
    )


def find_outcomes_path(root: Path) -> Path | None:
    for rel in OUTCOMES_CANDIDATES:
        candidate = root / rel
        if candidate.is_file():
            return candidate
    matches = sorted(root.glob("**/mutants.out/outcomes.json"))
    return matches[0] if matches else None


def parse_outcomes_file(path: Path) -> tuple[int, int, int, int]:
    data = json.loads(path.read_text(encoding="utf-8"))
    list_counts = (0, 0, 0, 0)
    if isinstance(data, list):
        list_counts = count_from_outcomes_list(data)
    elif isinstance(data, dict):
        entries = data.get("outcomes", [])
        if isinstance(entries, list):
            list_counts = count_from_outcomes_list(entries)

    summary_counts = count_from_summary_dict(data) if isinstance(data, dict) else None
    list_tested = list_counts[0] + list_counts[1] + list_counts[2]
    if list_tested > 0 or list_counts[3] > 0:
        return list_counts
    if summary_counts is not None:
        summary_tested = summary_counts[0] + summary_counts[1] + summary_counts[2]
        if summary_tested > 0 or summary_counts[3] > 0:
            return summary_counts

    txt_counts = counts_from_txt_dir(path.parent)
    txt_tested = txt_counts[0] + txt_counts[1] + txt_counts[2]
    if txt_tested > 0 or txt_counts[3] > 0:
        return txt_counts
    if summary_counts is not None:
        return summary_counts
    return list_counts


def parse_from_txt_only(root: Path) -> tuple[int, int, int, int]:
    for rel in ("mutants.out", "engine-core/mutants.out"):
        counts = counts_from_txt_dir(root / rel)
        if any(counts):
            return counts
    for directory in sorted(root.glob("**/mutants.out")):
        if directory.is_dir():
            counts = counts_from_txt_dir(directory)
            if any(counts):
                return counts
    return (0, 0, 0, 0)


def build_metrics(root: Path) -> dict[str, Any]:
    metrics: dict[str, Any] = {
        "source": "core-mutation-tests",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "status": "no-data",
        "mutation_score": 0.0,
        "caught": 0,
        "missed": 0,
        "timeout": 0,
        "unviable": 0,
        "total_tested": 0,
    }

    outcomes_path = find_outcomes_path(root)
    if outcomes_path is not None:
        caught, missed, timeout, unviable = parse_outcomes_file(outcomes_path)
        metrics["outcomes_path"] = str(outcomes_path.relative_to(root)) if outcomes_path.is_relative_to(root) else str(outcomes_path)
    else:
        caught, missed, timeout, unviable = parse_from_txt_only(root)

    total_tested = caught + missed + timeout
    score = (caught / total_tested * 100.0) if total_tested else 0.0
    status = "ok" if total_tested > 0 else "no-data"
    metrics.update(
        {
            "status": status,
            "mutation_score": round(score, 1),
            "caught": caught,
            "missed": missed,
            "timeout": timeout,
            "unviable": unviable,
            "total_tested": total_tested,
        }
    )
    return metrics


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="Workspace root to search")
    parser.add_argument("-o", "--output", default="mutation-metrics.json")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    metrics = build_metrics(root)
    Path(args.output).write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metrics, indent=2))


if __name__ == "__main__":
    main()
