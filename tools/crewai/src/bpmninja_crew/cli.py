"""Kommandozeile für Analyse, Anzeige und bestätigte Umsetzung."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import traceback
from datetime import datetime, timezone
from pathlib import Path

os.environ.setdefault("CREWAI_TELEMETRY_OPT_OUT", "true")
os.environ.setdefault("OTEL_SDK_DISABLED", "true")
os.environ.setdefault("CREWAI_DISABLE_TELEMETRY", "true")

from dotenv import load_dotenv  # noqa: E402
from pydantic import ValidationError  # noqa: E402

from .models import Recommendation, RecommendationReport  # noqa: E402
from .repo import get_repo_root  # noqa: E402

FOCUS_PATHS = {
    "all": (
        "engine-core/src, bpmn-parser/src, persistence-nats/src, "
        "persistence-memory/src, engine-server/src, desktop-tauri/src, docs"
    ),
    "engine": "engine-core/src/**/*.rs, engine-core/tests/**/*.rs",
    "parser": "bpmn-parser/src/**/*.rs, bpmn-parser/tests/**/*.rs",
    "persistence": "persistence-nats/src/**/*.rs, persistence-memory/src/**/*.rs",
    "server": "engine-server/src/**/*.rs, engine-server/tests/**/*.rs, api-spec",
    "desktop": "desktop-tauri/src/**/*.tsx, desktop-tauri/src/**/*.ts, desktop-tauri/src-tauri/src",
    "docs": "README.md, docs/**/*.md, AGENTS.md, api-spec",
    "quality": (
        "Cargo.toml, .github/workflows, fuzz, engine-core/src, "
        "engine-server/src, persistence-nats/src"
    ),
    "security": (
        "engine-server/src, persistence-nats/src, engine-core/src, "
        ".github/workflows, Dockerfile"
    ),
}
FOCUS_CHOICES = sorted(FOCUS_PATHS)


def _configure_env() -> None:
    root = get_repo_root()
    load_dotenv(root / "tools" / "crewai" / ".env")
    load_dotenv(root / ".env")
    load_dotenv()


def _run_stamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _require_api_key() -> None:
    if not os.getenv("DEEPSEEK_API_KEY"):
        raise SystemExit(
            "DEEPSEEK_API_KEY fehlt. Lege tools/crewai/.env an "
            "(Vorlage: tools/crewai/.env.example) oder exportiere den Key."
        )


def _extract_report(result, run_dir: Path) -> RecommendationReport:
    candidate = getattr(result, "pydantic", None)
    if isinstance(candidate, RecommendationReport):
        return candidate
    if isinstance(candidate, dict):
        return RecommendationReport.model_validate(candidate)

    data = getattr(result, "json_dict", None)
    if isinstance(data, dict) and "recommendations" in data:
        return RecommendationReport.model_validate(data)

    raw = getattr(result, "raw", "") or ""
    last_raw = ""
    last_error = ""
    for text in (raw, _read_if_exists(run_dir / "05_empfehlungen.md")):
        parsed = _parse_json_block(text)
        if not parsed or "recommendations" not in parsed:
            continue
        try:
            return RecommendationReport.model_validate(parsed)
        except ValidationError as exc:
            last_raw, last_error = text, str(exc)

    if last_raw:
        print(
            "Synthese war nicht schema-konform, starte eine Reparatur mit DeepSeek.",
            file=sys.stderr,
        )
        repaired = _repair_report(last_raw, last_error)
        if repaired is not None:
            return repaired

    raise RuntimeError(
        "Die Crew hat keinen auswertbaren RecommendationReport geliefert. "
        f"Rohausgabe liegt in {run_dir / '05_empfehlungen.md'}."
    )


def _repair_report(raw: str, error: str) -> RecommendationReport | None:
    """Einmaliger Reparaturversuch, wenn die Synthese kein valides JSON liefert."""
    from .crew import build_llm, model_settings

    _, synth_model, _, synth_effort = model_settings()
    schema = json.dumps(RecommendationReport.model_json_schema(), ensure_ascii=False)
    llm = build_llm(
        synth_model,
        synth_effort,
        temperature=0.0,
        max_tokens=16384,
        response_format={"type": "json_object"},
    )
    messages = [
        {
            "role": "system",
            "content": (
                "Du korrigierst JSON-Ausgaben. Antworte ausschließlich mit "
                "einem einzigen validen JSON-Objekt, ohne Markdown."
            ),
        },
        {
            "role": "user",
            "content": (
                f"JSON-Schema:\n{schema}\n\n"
                f"Validierungsfehler:\n{error}\n\n"
                f"Ursprüngliche Ausgabe:\n{raw[:80000]}"
            ),
        },
    ]
    try:
        text = llm.call(messages)
        parsed = _parse_json_block(str(text))
        if parsed:
            return RecommendationReport.model_validate(parsed)
    except Exception as exc:  # noqa: BLE001 - Reparatur ist ein Best-Effort-Schritt
        print(f"Reparatur fehlgeschlagen: {exc}", file=sys.stderr)
    return None


def _read_if_exists(path: Path) -> str:
    return path.read_text(encoding="utf-8") if path.is_file() else ""


def _write_intermediate(result, run_dir: Path) -> None:
    """Rohberichte der Fachaufgaben sichern, falls CrewAI sie nicht schreibt."""
    names = [
        "01_repo_analyse.md",
        "02_domaenen_recherche.md",
        "03_produkt_dx.md",
        "04_qualitaet_security.md",
        "05_empfehlungen.md",
    ]
    outputs = getattr(result, "tasks_output", None) or []
    for task_output, name in zip(outputs, names):
        path = run_dir / name
        if path.is_file():
            continue
        raw = getattr(task_output, "raw", "") or ""
        if raw.strip():
            path.write_text(raw, encoding="utf-8")


def _parse_json_block(text: str) -> dict | None:
    fenced = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", text, re.DOTALL)
    candidates = [fenced.group(1)] if fenced else []
    candidates.extend(_balanced_objects(text))
    for candidate in candidates:
        try:
            value = json.loads(candidate)
            if isinstance(value, dict):
                return value
        except json.JSONDecodeError:
            continue
    return None


def _balanced_objects(text: str):
    """Alle vollständigen JSON-Objekte im Text, auch mit Text davor/danach."""
    start = text.find("{")
    while start != -1:
        depth = 0
        in_string = False
        escaped = False
        for index in range(start, len(text)):
            char = text[index]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
                continue
            if char == '"':
                in_string = True
            elif char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    yield text[start : index + 1]
                    break
        start = text.find("{", start + 1)


def _report_markdown(report: RecommendationReport) -> str:
    lines = [
        "# CrewAI-Empfehlungen für BPMNinja",
        "",
        f"- Erstellt: {report.generated_at or 'n/a'}",
        f"- Fokus: {report.focus}",
        "",
        "## Projektzustand",
        "",
        report.project_state or "nicht angegeben",
        "",
        "## Ranking",
        "",
        "| ID | Score | Aufwand | Kategorie | Titel |",
        "|---|---:|---|---|---|",
    ]
    for rec in report.recommendations:
        lines.append(
            f"| {rec.id} | {rec.score} | {rec.effort} | {rec.category} | {rec.title} |"
        )
    lines.extend(["", "## Details", ""])
    for rec in report.recommendations:
        breakdown = rec.score_breakdown
        lines.extend(
            [
                f"### {rec.id} — {rec.title}",
                "",
                f"**Score {rec.score}/100** (Wirkung {breakdown.impact}/30, "
                f"Domänenlücke {breakdown.domain_gap}/20, "
                f"Risiko {breakdown.risk_reduction}/20, "
                f"Aufwand {breakdown.effort}/15, Evidenz {breakdown.evidence}/15) · "
                f"Kategorie {rec.category} · Aufwand {rec.effort}",
                "",
                rec.summary,
                "",
                "**Vorgeschlagene Änderung**",
                "",
                rec.proposed_change,
                "",
                "**Abnahmekriterien**",
                "",
            ]
        )
        lines.extend(f"- {item}" for item in rec.acceptance_criteria)
        lines.extend(["", "**Evidenz**", ""])
        lines.extend(
            f"- `{item.path}` {item.lines}: {item.note}" for item in rec.evidence
        )
        if rec.risks:
            lines.extend(["", "**Risiken**", "", rec.risks])
        if rec.sources:
            lines.extend(["", "**Quellen**", ""])
            lines.extend(f"- {source}" for source in rec.sources)
        lines.append("")
    if report.discarded:
        lines.extend(["## Verworfene Ideen", ""])
        lines.extend(f"- {item}" for item in report.discarded)
    lines.extend(
        [
            "",
            "## Umsetzung",
            "",
            "Nichts wurde automatisch umgesetzt. Nach Prüfung können einzelne IDs "
            "bestätigt und mit `bpmninja-crew implement` an die Umsetzungs-Crew "
            "übergeben werden.",
            "",
        ]
    )
    return "\n".join(lines)


def _print_ranking(report: RecommendationReport, run_dir: Path) -> None:
    print("\nCrewAI-Empfehlungen (absteigend nach Rating):\n")
    print(f"{'ID':<5}{'Score':>6}  {'Aufwand':<8}{'Kategorie':<14}Titel")
    print("-" * 100)
    for rec in report.recommendations:
        print(
            f"{rec.id:<5}{rec.score:>6}  {rec.effort:<8}{rec.category:<14}{rec.title}"
        )
    print(f"\nBericht: {run_dir / 'EMPFEHLUNGEN.md'}")
    print(f"Maschinenlesbar: {run_dir / 'report.json'}")
    if report.recommendations:
        ids = ",".join(rec.id for rec in report.recommendations[:3])
        print(
            "\nNächster Schritt (erst nach Bestätigung):\n"
            f"  bpmninja-crew implement --run {run_dir} --ids {ids} --confirm"
        )


def _load_report(run: Path) -> RecommendationReport:
    path = run if run.is_file() else run / "report.json"
    if not path.is_file():
        raise SystemExit(f"report.json nicht gefunden: {path}")
    return RecommendationReport.model_validate_json(path.read_text(encoding="utf-8"))


def cmd_analyze(args: argparse.Namespace) -> int:
    _configure_env()
    _require_api_key()
    root = get_repo_root()
    out = Path(args.out)
    if not out.is_absolute():
        out = root / out
    run_dir = out / (args.run_name or _run_stamp())
    if run_dir.exists():
        run_dir = out / f"{run_dir.name}-{os.getpid()}"
    run_dir.mkdir(parents=True, exist_ok=True)

    from .crew import AnalysisCrew, model_settings

    scan_model, synth_model, scan_effort, synth_effort = model_settings()
    metadata = {
        "started_at": datetime.now(timezone.utc).isoformat(),
        "focus": args.focus,
        "scan_model": scan_model,
        "synthesis_model": synth_model,
        "scan_effort": scan_effort,
        "synthesis_effort": synth_effort,
        "max_recommendations": args.max_recommendations,
    }
    (run_dir / "run.json").write_text(
        json.dumps(metadata, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    print(
        f"Starte CrewAI-Analyse: Fokus={args.focus}, "
        f"Scan={scan_model} ({scan_effort}), Synthese={synth_model} ({synth_effort})"
    )
    print(f"Artefakte: {run_dir}\n")

    crew = AnalysisCrew(run_dir=run_dir, verbose=not args.no_verbose)
    result = crew.crew().kickoff(
        inputs={
            "focus": args.focus,
            "focus_paths": FOCUS_PATHS[args.focus],
            "run_dir": str(run_dir),
            "max_recommendations": str(args.max_recommendations),
            "report_schema": json.dumps(
                RecommendationReport.model_json_schema(), ensure_ascii=False
            ),
        }
    )
    _write_intermediate(result, run_dir)
    report = _extract_report(result, run_dir)
    report.generated_at = report.generated_at or datetime.now(timezone.utc).isoformat()
    report.focus = report.focus or args.focus
    report.recommendations = report.recommendations[: args.max_recommendations]
    if not report.recommendations:
        print("Warnung: Die Crew hat keine Empfehlungen geliefert.")
    report.methodology = report.methodology or (
        "Vier Fachagenten (Architektur, Domäne, Produkt/DX, Qualität/Security) "
        "mit anschließender Priorisierung nach einem 100-Punkte-Raster."
    )
    (run_dir / "report.json").write_text(
        report.model_dump_json(indent=2), encoding="utf-8"
    )
    (run_dir / "EMPFEHLUNGEN.md").write_text(
        _report_markdown(report), encoding="utf-8"
    )
    metadata["finished_at"] = datetime.now(timezone.utc).isoformat()
    (run_dir / "run.json").write_text(
        json.dumps(metadata, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    _print_ranking(report, run_dir)
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    _configure_env()
    root = get_repo_root()
    if args.run:
        run_dir = Path(args.run)
        if not run_dir.is_absolute():
            run_dir = root / run_dir
    else:
        base = root / "reports" / "crewai"
        runs = sorted(p for p in base.glob("*/report.json")) if base.exists() else []
        if not runs:
            raise SystemExit("Kein Lauf gefunden. Starte zuerst `bpmninja-crew analyze`.")
        run_dir = runs[-1].parent
    report = _load_report(run_dir)
    _print_ranking(report, run_dir)
    return 0


def cmd_implement(args: argparse.Namespace) -> int:
    _configure_env()
    _require_api_key()
    root = get_repo_root()
    run_dir = Path(args.run)
    if not run_dir.is_absolute():
        run_dir = root / run_dir
    report = _load_report(run_dir)
    wanted = [item.strip().upper() for item in args.ids.split(",") if item.strip()]
    by_id = {rec.id: rec for rec in report.recommendations}
    missing = [item for item in wanted if item not in by_id]
    if missing:
        raise SystemExit(
            f"Unbekannte IDs: {', '.join(missing)}. "
            f"Verfügbar: {', '.join(by_id) or 'keine'}"
        )
    selected: list[Recommendation] = [by_id[item] for item in wanted]
    if len(selected) > 3:
        raise SystemExit("Maximal 3 Empfehlungen pro Umsetzungslauf, bitte aufteilen.")

    print("Bestätigte Empfehlungen:")
    for rec in selected:
        print(f"  {rec.id} — {rec.title} (Score {rec.score}, Aufwand {rec.effort})")
    if not args.confirm:
        print(
            "\nTrockenlauf: Es wurde nichts geändert. "
            "Zum Starten dieselbe Anfrage mit --confirm wiederholen."
        )
        return 2

    impl_dir = Path(args.out) if args.out else run_dir / f"umsetzung-{_run_stamp()}"
    if not impl_dir.is_absolute():
        impl_dir = root / impl_dir
    impl_dir.mkdir(parents=True, exist_ok=True)

    from .crew import ImplementationCrew

    print(f"\nStarte Umsetzungs-Crew. Artefakte: {impl_dir}\n")
    crew = ImplementationCrew(
        run_dir=impl_dir, recommendations=selected, verbose=not args.no_verbose
    )
    result = crew.crew().kickoff()
    print("\n=== Ergebnis der Umsetzungs-Crew ===\n")
    print(getattr(result, "raw", result))
    print("\nBitte anschließend die vollständige Verifikation im Repo ausführen.")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="bpmninja-crew",
        description="CrewAI-Analyse und bestätigte Umsetzung für BPMNinja",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    analyze = sub.add_parser("analyze", help="Analyse-Crew starten")
    analyze.add_argument("--focus", default="all", choices=FOCUS_CHOICES)
    analyze.add_argument("--max-recommendations", type=int, default=10)
    analyze.add_argument(
        "--out", default="reports/crewai", help="Basisverzeichnis für Läufe"
    )
    analyze.add_argument("--run-name", default="")
    analyze.add_argument("--no-verbose", action="store_true")
    analyze.set_defaults(func=cmd_analyze)

    list_cmd = sub.add_parser("list", help="Letzten oder benannten Lauf anzeigen")
    list_cmd.add_argument("--run", default="")
    list_cmd.set_defaults(func=cmd_list)

    implement = sub.add_parser(
        "implement", help="Bestätigte Empfehlungen umsetzen (Safety-Gate)"
    )
    implement.add_argument("--run", required=True)
    implement.add_argument("--ids", required=True, help="z. B. R1,R4 (max. 3)")
    implement.add_argument(
        "--confirm",
        action="store_true",
        help="Ohne dieses Flag wird nur der Plan angezeigt",
    )
    implement.add_argument("--out", default="")
    implement.add_argument("--no-verbose", action="store_true")
    implement.set_defaults(func=cmd_implement)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except ValidationError as exc:
        print(f"Antwort der Crew war nicht schema-konform:\n{exc}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("\nAbgebrochen.", file=sys.stderr)
        return 130
    except Exception as exc:  # noqa: BLE001 - CLI soll verständlich scheitern
        print(f"Fehler: {exc}", file=sys.stderr)
        if os.getenv("BPMNINJA_CREW_DEBUG"):
            traceback.print_exc()
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
