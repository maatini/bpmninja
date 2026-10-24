"""Eingaben validieren und in stabile Laufzeitkonfiguration überführen."""

from __future__ import annotations

from dataclasses import dataclass, field
import json
import os
from pathlib import Path
from typing import Any, Mapping, Sequence

import yaml

from .workspace import build_source_manifest


class ConfigError(ValueError):
    """Ungültige oder unvollständige Crew-Eingaben."""


DEFAULT_LANGUAGE = "Deutsch"
DEFAULT_THRESHOLD = 90
DEFAULT_MAX_ROUNDS = 2
MAX_ROUNDS_LIMIT = 4


def _as_text(value: Any, key: str) -> str:
    if value is None:
        raise ConfigError(f"Eingabe '{key}' fehlt.")
    if isinstance(value, (list, tuple)):
        text = ", ".join(str(item).strip() for item in value if str(item).strip())
    else:
        text = str(value).strip()
    if not text:
        raise ConfigError(f"Eingabe '{key}' darf nicht leer sein.")
    return text


def _as_path_list(value: Any, key: str, base_dir: Path) -> tuple[Path, ...]:
    if value is None:
        raise ConfigError(f"Eingabe '{key}' fehlt.")
    raw_values: list[str]
    if isinstance(value, (list, tuple)):
        raw_values = [str(item) for item in value if str(item).strip()]
    elif isinstance(value, str):
        raw_values = [row.strip() for row in value.splitlines() if row.strip()]
    else:
        raise ConfigError(f"Eingabe '{key}' muss ein Pfad oder eine Liste sein.")
    if not raw_values:
        raise ConfigError(f"Eingabe '{key}' enthält keine Pfade.")
    paths: list[Path] = []
    for raw in raw_values:
        path = Path(raw).expanduser()
        if not path.is_absolute():
            path = base_dir / path
        path = path.resolve()
        if not path.exists():
            raise ConfigError(f"Pfad aus '{key}' existiert nicht: {path}")
        if path not in paths:
            paths.append(path)
    return tuple(paths)


def _as_optional_path(value: Any, key: str, base_dir: Path) -> Path | None:
    if value in (None, "", []):
        return None
    if isinstance(value, (list, tuple)):
        raise ConfigError(f"Eingabe '{key}' muss ein einzelner Pfad sein.")
    path = Path(str(value)).expanduser()
    if not path.is_absolute():
        path = base_dir / path
    return path.resolve()


def _as_output_path(value: Any, key: str, base_dir: Path) -> Path:
    path = _as_optional_path(value, key, base_dir)
    if path is None:
        raise ConfigError(f"Eingabe '{key}' fehlt.")
    if path.exists() and not path.is_dir():
        raise ConfigError(f"Eingabe '{key}' ist kein Verzeichnis: {path}")
    return path


def _as_int(value: Any, key: str, default: int, minimum: int, maximum: int) -> int:
    if value in (None, ""):
        return default
    try:
        number = int(value)
    except (TypeError, ValueError) as exc:
        raise ConfigError(f"Eingabe '{key}' muss eine ganze Zahl sein.") from exc
    if not minimum <= number <= maximum:
        raise ConfigError(
            f"Eingabe '{key}' muss zwischen {minimum} und {maximum} liegen."
        )
    return number


def _env_int(name: str, default: int, minimum: int, maximum: int) -> int:
    return _as_int(os.getenv(name), name, default, minimum, maximum)


def load_input_file(path: Path) -> dict[str, Any]:
    """JSON oder YAML laden; Fehler mit Dateibezug melden."""
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ConfigError(f"Eingabedatei nicht lesbar: {path}: {exc}") from exc
    try:
        if path.suffix.lower() in {".yaml", ".yml"}:
            data = yaml.safe_load(text)
        else:
            data = json.loads(text)
    except (json.JSONDecodeError, yaml.YAMLError) as exc:
        raise ConfigError(f"Eingabedatei ist ungültig: {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise ConfigError("Eingabedatei muss ein Objekt mit benannten Feldern enthalten.")
    return data


@dataclass(frozen=True)
class RunConfig:
    """Validierte, unveränderliche Laufzeitkonfiguration."""

    project_name: str
    project_description: str
    domain_topic: str
    existing_docs_paths: tuple[Path, ...]
    target_audience: str
    output_path: Path
    repository_path: Path | None = None
    documentation_language: str = DEFAULT_LANGUAGE
    quality_threshold: int = DEFAULT_THRESHOLD
    max_revision_rounds: int = DEFAULT_MAX_ROUNDS
    source_manifest: str = field(default="", compare=False)

    @classmethod
    def from_mapping(
        cls,
        data: Mapping[str, Any],
        *,
        base_dir: Path | None = None,
        overrides: Mapping[str, Any] | None = None,
    ) -> "RunConfig":
        merged = dict(data)
        if overrides:
            merged.update({key: value for key, value in overrides.items() if value is not None})
        base = (base_dir or Path.cwd()).expanduser().resolve()

        project_name = _as_text(merged.get("project_name"), "project_name")
        project_description = _as_text(
            merged.get("project_description"), "project_description"
        )
        domain_topic = _as_text(merged.get("domain_topic"), "domain_topic")
        target_audience = _as_text(merged.get("target_audience"), "target_audience")
        sources = _as_path_list(
            merged.get("existing_docs_path"), "existing_docs_path", base
        )
        output_path = _as_output_path(merged.get("output_path"), "output_path", base)
        if any(source == output_path for source in sources):
            raise ConfigError("output_path darf keine Quelle überschreiben.")

        repository_path = _as_optional_path(
            merged.get("repository_path"), "repository_path", base
        )
        if repository_path is not None and not repository_path.exists():
            raise ConfigError(
                f"Pfad aus 'repository_path' existiert nicht: {repository_path}"
            )
        language = str(
            merged.get("documentation_language") or DEFAULT_LANGUAGE
        ).strip() or DEFAULT_LANGUAGE
        threshold = _as_int(
            merged.get("quality_threshold", os.getenv("DOC_CREW_QUALITY_THRESHOLD")),
            "quality_threshold",
            DEFAULT_THRESHOLD,
            50,
            100,
        )
        max_rounds = _as_int(
            merged.get("max_revision_rounds", os.getenv("DOC_CREW_MAX_REVISION_ROUNDS")),
            "max_revision_rounds",
            _env_int("DOC_CREW_MAX_REVISION_ROUNDS", DEFAULT_MAX_ROUNDS, 0, MAX_ROUNDS_LIMIT),
            0,
            MAX_ROUNDS_LIMIT,
        )

        manifest = build_source_manifest(sources)
        return cls(
            project_name=project_name,
            project_description=project_description,
            domain_topic=domain_topic,
            existing_docs_paths=sources,
            target_audience=target_audience,
            output_path=output_path,
            repository_path=repository_path,
            documentation_language=language,
            quality_threshold=threshold,
            max_revision_rounds=max_rounds,
            source_manifest=manifest,
        )

    @property
    def slug(self) -> str:
        slug = "".join(
            character.lower() if character.isalnum() else "-"
            for character in self.project_name
        )
        return "-".join(part for part in slug.split("-") if part) or "project"

    def base_inputs(self) -> dict[str, Any]:
        """Alle Platzhalter, die in den YAML-Aufgaben vorkommen."""
        return {
            "project_name": self.project_name,
            "project_description": self.project_description,
            "domain_topic": self.domain_topic,
            "target_audience": self.target_audience,
            "output_path": str(self.output_path),
            "repository_path": str(self.repository_path) if self.repository_path else "nicht angegeben",
            "existing_docs_path": "\n".join(str(path) for path in self.existing_docs_paths),
            "source_manifest": self.source_manifest,
            "documentation_language": self.documentation_language,
            "quality_threshold": self.quality_threshold,
            "qa_threshold": self.quality_threshold,
            "iteration": "0",
            "qa_feedback": "Noch kein QA-Feedback vorhanden.",
            "qa_summary": "Noch kein QA-Report vorhanden.",
            "quality_history": "Noch keine Messung.",
        }


def validate_input_keys(data: Mapping[str, Any]) -> None:
    """Frühe, verständliche Fehlermeldung für Pflichtfelder."""
    required = (
        "project_name",
        "project_description",
        "domain_topic",
        "existing_docs_path",
        "target_audience",
        "output_path",
    )
    missing = [key for key in required if not data.get(key)]
    if missing:
        raise ConfigError("Fehlende Pflichtfelder: " + ", ".join(missing))


def as_paths(value: str | Sequence[str]) -> tuple[str, ...]:
    """Bequemer Helfer für Tests und externe Aufrufer."""
    if isinstance(value, str):
        return (value,)
    return tuple(value)
