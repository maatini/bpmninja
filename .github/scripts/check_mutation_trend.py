#!/usr/bin/env python3
"""Prüft Mutation-Score und Trend gegenüber dem letzten Wochenlauf.

Liest den frisch erzeugten mutation-metrics.json und den vorherigen Stand
aus docs/quality-metrics.json. Fehlt die Datei lokal, wird sie per
`git show origin/metrics:docs/quality-metrics.json` nachgeladen.

Exit 1 bei Score unter der Schwelle oder bei einem Abfall von mehr als
der erlaubten Punktzahl. Ohne auswertbare Mutationsdaten endet das Skript
erfolgreich, damit ein abgebrochener cargo-mutants-Lauf nicht zusätzlich
fehlschlägt.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

DEFAULT_THRESHOLD = 65.0
DEFAULT_MAX_DROP = 2.0
PREVIOUS_FALLBACK_REF = "origin/metrics:docs/quality-metrics.json"


def load_json(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None
    return data if isinstance(data, dict) else None


def extract_mutation(data: dict[str, Any] | None) -> dict[str, Any] | None:
    """Akzeptiert mutation-metrics.json und docs/quality-metrics.json."""
    if not data:
        return None
    mutation = data.get("mutation")
    if isinstance(mutation, dict):
        return mutation
    if "mutation_score" in data:
        return data
    return None


def has_signal(mutation: dict[str, Any] | None) -> bool:
    if not mutation:
        return False
    return mutation.get("status") == "ok" and float(mutation.get("total_tested") or 0) > 0


def load_previous(path: Path) -> tuple[dict[str, Any] | None, str]:
    """Lädt den vorherigen Stand aus Datei oder origin/metrics."""
    from_file = extract_mutation(load_json(path))
    if has_signal(from_file):
        return from_file, str(path)

    try:
        result = subprocess.run(
            ["git", "show", PREVIOUS_FALLBACK_REF],
            capture_output=True,
            text=True,
            check=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError) as error:
        print(f"Hinweis: Vorheriger Stand nicht verfügbar ({error}).")
        return None, "nicht verfügbar"

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        print("Hinweis: Vorheriger Stand ist kein gültiges JSON.")
        return None, "nicht verfügbar"

    mutation = extract_mutation(data if isinstance(data, dict) else None)
    if not has_signal(mutation):
        return None, "nicht verfügbar"
    return mutation, PREVIOUS_FALLBACK_REF


def trend_label(delta: float) -> str:
    if abs(delta) < 0.05:
        return "stabil"
    return "verbessert" if delta > 0 else "verschlechtert"


def write_summary(lines: list[str], failed: bool) -> None:
    body = "\n".join(lines) + "\n"
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as handle:
            handle.write(body)
    print(body)
    if failed:
        print("::error::Mutation-Score oder Trend verfehlt die Vorgaben.")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--current", default="mutation-metrics.json")
    parser.add_argument("--previous", default="docs/quality-metrics.json")
    parser.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD)
    parser.add_argument("--max-drop", type=float, default=DEFAULT_MAX_DROP)
    args = parser.parse_args()

    current = extract_mutation(load_json(Path(args.current)))
    if not has_signal(current):
        lines = [
            "## Mutation Score",
            "",
            f"Keine auswertbaren Mutationsdaten in `{args.current}`. "
            "Schwelle und Trend werden nicht geprüft.",
        ]
        write_summary(lines, failed=False)
        return 0

    score = float(current.get("mutation_score") or 0.0)
    previous, previous_source = load_previous(Path(args.previous))
    delta: float | None = None
    if previous is not None:
        delta = round(score - float(previous.get("mutation_score") or 0.0), 1)

    below_threshold = score < args.threshold
    trend_violation = delta is not None and delta < -args.max_drop
    failed = below_threshold or trend_violation

    lines = [
        "## Mutation Score",
        "",
        f"- Aktueller Score: **{score:.1f}%** (Schwelle {args.threshold:.1f}%)",
    ]
    if delta is None:
        lines.append(f"- Vorheriger Stand: {previous_source} → kein Vergleich möglich")
    else:
        previous_score = float(previous.get("mutation_score") or 0.0)
        sign = "+" if delta > 0 else ""
        lines.append(
            f"- Vorheriger Stand: **{previous_score:.1f}%** "
            f"({previous_source}) → Delta {sign}{delta:.1f} Punkte ({trend_label(delta)})"
        )
    lines.append(
        f"- Erlaubter Abfall: {args.max_drop:.1f} Punkte; "
        f"Ergebnis: {'❌ verfehlt' if failed else '✅ erfüllt'}"
    )
    lines.append("")

    write_summary(lines, failed=failed)

    if delta is not None and delta < 0:
        print(f"::warning::Mutation-Score ist um {abs(delta):.1f} Punkte gefallen ({score:.1f}%).")
    if below_threshold:
        print(f"::error::Mutation-Score {score:.1f}% liegt unter {args.threshold:.1f}%.")
    if trend_violation:
        print(
            f"::error::Mutation-Score ist um {abs(delta or 0):.1f} Punkte gefallen "
            f"(erlaubt: {args.max_drop:.1f})."
        )

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
