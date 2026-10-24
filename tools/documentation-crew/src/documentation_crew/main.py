"""Kommandozeile für die Documentation Crew."""

from __future__ import annotations

import argparse
import logging
import os
from pathlib import Path
import sys
from typing import Any, Sequence

from dotenv import load_dotenv

from .config import ConfigError, RunConfig, load_input_file, validate_input_keys
from .crew import CrewRunError, DocumentationCrew


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="documentation-crew",
        description=(
            "Ein hierarchisches CrewAI-Team überarbeitet bestehende Dokumentation "
            "nach Diátaxis und liefert einen belegten QA-Score."
        ),
    )
    parser.add_argument(
        "--inputs",
        type=Path,
        help="JSON- oder YAML-Datei mit project_name, project_description, "
        "domain_topic, existing_docs_path, target_audience und output_path.",
    )
    parser.add_argument("--project-name")
    parser.add_argument("--project-description")
    parser.add_argument("--domain-topic")
    parser.add_argument(
        "--existing-docs",
        action="append",
        help="Pfad zu einer Datei oder einem Verzeichnis; mehrfach angebbar.",
    )
    parser.add_argument("--target-audience")
    parser.add_argument("--output")
    parser.add_argument("--repository", help="Optionaler Repository-Pfad für Codeabgleich.")
    parser.add_argument("--language", help="Dokumentationssprache, Standard: Deutsch.")
    parser.add_argument("--threshold", type=int, help="QA-Schwelle 50 bis 100.")
    parser.add_argument("--max-rounds", type=int, help="Nachbesserungsrunden 0 bis 4.")
    parser.add_argument("--model", help="Modell, Standard: deepseek/deepseek-chat.")
    parser.add_argument("--manager-model", help="Modell des Documentation Lead.")
    parser.add_argument(
        "--effort",
        choices=("low", "medium", "high", "max"),
        help="Optionaler Reasoning-Aufwand des Providers.",
    )
    parser.add_argument("--temperature", type=float, default=None)
    parser.add_argument(
        "--no-memory",
        action="store_true",
        help="Memory deaktivieren; nützlich für schnelle Testläufe.",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Inhaltsphase überspringen und Review, QA und Abschluss auf "
        "vorhandenen Dokumenten ausführen.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Konfiguration, Rollen, Aufgaben, Werkzeuge und Prompts prüfen, "
        "ohne ein Modell aufzurufen.",
    )
    parser.add_argument("--quiet", action="store_true", help="Crew-Ausgabe reduzieren.")
    return parser


def _mapping_from_args(args: argparse.Namespace) -> dict[str, Any]:
    data: dict[str, Any] = {}
    if args.inputs:
        data.update(load_input_file(args.inputs))
    direct = {
        "project_name": args.project_name,
        "project_description": args.project_description,
        "domain_topic": args.domain_topic,
        "target_audience": args.target_audience,
        "output_path": args.output,
        "repository_path": args.repository,
        "documentation_language": args.language,
        "quality_threshold": args.threshold,
        "max_revision_rounds": args.max_rounds,
    }
    data.update({key: value for key, value in direct.items() if value is not None})
    if args.existing_docs:
        data["existing_docs_path"] = args.existing_docs
    return data


def _configure_logging(quiet: bool) -> None:
    logging.basicConfig(
        level=logging.WARNING if quiet else logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )


def _print_dry_run(report) -> None:
    print("Dry-Run erfolgreich. Es wurden keine Modellaufrufe ausgeführt.")
    print(f"Projekt:       {report.project_name}")
    print(f"Ausgabe:       {report.output_path}")
    print(f"Quellen:       {report.source_files} Dateien")
    print(f"Code:          {report.code_files} Dateien")
    print(f"Agenten:       {report.agent_count}")
    print(f"Hauptaufgaben: {report.main_task_count}")
    print(f"Revision:      {report.revision_task_count}")
    print(f"Memory:        {'aktiv' if report.memory_enabled else 'aus'}")
    print(f"Modell:        {report.model}")
    print("Werkzeuge:     " + ", ".join(report.tool_names))


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    _configure_logging(args.quiet)
    project_dir = Path(__file__).resolve().parents[2]
    load_dotenv(project_dir / ".env")
    load_dotenv()

    if args.no_memory:
        os.environ["DOC_CREW_MEMORY"] = "0"
    if args.temperature is not None:
        os.environ["DOC_CREW_TEMPERATURE"] = str(args.temperature)

    try:
        data = _mapping_from_args(args)
        validate_input_keys(data)
        base_dir = args.inputs.parent if args.inputs else Path.cwd()
        config = RunConfig.from_mapping(data, base_dir=base_dir)
        crew = DocumentationCrew(
            config,
            verbose=not args.quiet,
            dry_run=args.dry_run,
            model=args.model,
            manager_model=args.manager_model,
            effort=args.effort,
            temperature=args.temperature,
        )
        if args.dry_run:
            _print_dry_run(crew.dry_run_report())
            return 0
        result = crew.run(resume=args.resume)
    except (ConfigError, CrewRunError) as exc:
        print(f"Fehler: {exc}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print("Abgebrochen.", file=sys.stderr)
        return 130
    except Exception as exc:  # noqa: BLE001 - CLI soll keine Traceback-Wand zeigen
        logging.getLogger("documentation_crew").exception("Unerwarteter Fehler")
        print(f"Unerwarteter Fehler: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1

    status = "FREIGEGEBEN" if result.approved else "NICHT FREIGEGEBEN"
    print(f"\nFertig: {status} (Score {result.score}/100)")
    print(f"Dauer:  {result.duration_seconds:.1f}s")
    print(f"Ausgabe: {result.output_path}")
    print(f"Bericht: {result.output_path / 'EXECUTIVE-SUMMARY.md'}")
    if not result.approved:
        print("Offene Blocker stehen in _qa/qa-report.json und im Abschlussbericht.")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
