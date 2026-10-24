"""Sandbox-Werkzeuge für Quellen, Code, Dokumente und Qualitätsscore."""

from __future__ import annotations

import json
from typing import Literal

from crewai.tools import BaseTool
from pydantic import BaseModel, ConfigDict, Field

from ..qa import QualityScorer, build_report
from ..workspace import DocumentationWorkspace, TextCorpus, WorkspaceError


class EmptyInput(BaseModel):
    """Werkzeug ohne Parameter."""


class SourceInventoryInput(BaseModel):
    """Keine Parameter: liefert das vollständige Quellenmanifest."""


class SourceReadInput(BaseModel):
    path: str = Field(description="Anzeigename aus dem Quellenmanifest")
    max_chars: int = Field(default=40_000, description="Maximale Zeichenzahl")


class SourceSearchInput(BaseModel):
    query: str = Field(description="Suchbegriff, mindestens zwei Zeichen")
    max_results: int = Field(default=20, description="Maximale Trefferzahl")
    glob: str | None = Field(
        default=None,
        description="Optionaler Pfadfilter, zum Beispiel **/*.md",
    )


class DocumentListInput(BaseModel):
    """Keine Parameter: liefert das Inventar der erzeugten Dokumentation."""


class DocumentReadInput(BaseModel):
    path: str = Field(description="Relativer Pfad unterhalb des Ausgabeverzeichnisses")
    max_chars: int = Field(default=40_000, description="Maximale Zeichenzahl")


class DocumentWriteInput(BaseModel):
    path: str = Field(
        description="Relativer Markdown-Pfad, zum Beispiel tutorials/einstieg.md"
    )
    content: str = Field(
        min_length=1,
        description="Vollständiger Markdown-Inhalt inklusive Front Matter",
    )


class LinkCheckInput(BaseModel):
    """Keine Parameter: prüft alle internen Links."""


class MermaidCheckInput(BaseModel):
    """Keine Parameter: prüft alle Mermaid-Blöcke."""


class QualityScoreInput(BaseModel):
    completeness: int = Field(ge=0, le=100)
    accuracy: int = Field(ge=0, le=100)
    clarity: int = Field(ge=0, le=100)
    consistency: int = Field(ge=0, le=100)
    diataxis: int = Field(ge=0, le=100)
    usability: int = Field(ge=0, le=100)
    blockers: list[str] = Field(
        default_factory=list,
        description="Konkrete Blocker mit Datei und Stelle; leer nur bei Freigabe",
    )
    decision: Literal["approved", "changes_requested"] = Field(
        description="Empfehlung; das Werkzeug erzwingt changes_requested bei Blockern"
    )
    summary: str = Field(description="Kurze Begründung des Ergebnisses")
    iteration: int = Field(default=0, ge=0, le=20)


def _clamp_chars(value: int) -> int:
    """Begrenzt eine vom Modell gelieferte Zeichenzahl auf den gültigen Bereich."""
    try:
        return max(1_000, min(int(value), 80_000))
    except (TypeError, ValueError):
        return 40_000


def _clamp_results(value: int) -> int:
    """Begrenzt eine vom Modell gelieferte Trefferzahl auf den gültigen Bereich."""
    try:
        return max(1, min(int(value), 50))
    except (TypeError, ValueError):
        return 20


def _safe_call(callable_, *args, **kwargs) -> str:
    try:
        return str(callable_(*args, **kwargs))
    except WorkspaceError as exc:
        return f"FEHLER: {exc}"
    except Exception as exc:  # noqa: BLE001 - Werkzeugfehler als Text zurückgeben
        return f"FEHLER: {type(exc).__name__}: {exc}"


class SourceInventoryTool(BaseTool):
    name: str = "source_inventory"
    description: str = (
        "Listet alle lesbaren Quelltexte mit Anzeigename, Wortzahl und Titel. "
        "Immer zuerst aufrufen, bevor Dokumente gelesen oder auditiert werden."
    )
    args_schema: type[BaseModel] = SourceInventoryInput
    corpus: TextCorpus = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self) -> str:
        return self.corpus.manifest()


class SourceReadTool(BaseTool):
    name: str = "source_read"
    description: str = (
        "Liest eine bestehende Quelle über ihren Anzeigenamen aus dem Manifest. "
        "Große Dateien werden gekürzt; für Details die Suche nutzen."
    )
    args_schema: type[BaseModel] = SourceReadInput
    corpus: TextCorpus = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self, path: str, max_chars: int = 40_000) -> str:
        return _safe_call(self.corpus.read, path, _clamp_chars(max_chars))


class SourceSearchTool(BaseTool):
    name: str = "source_search"
    description: str = (
        "Durchsucht alle Quellen case-insensitiv und liefert Datei, Zeile und "
        "Textausschnitt. Ideal für Terminologie, Versionsangaben und Widersprüche."
    )
    args_schema: type[BaseModel] = SourceSearchInput
    corpus: TextCorpus = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(
        self, query: str, max_results: int = 20, glob: str | None = None
    ) -> str:
        return _safe_call(
            self.corpus.search,
            query,
            max_results=_clamp_results(max_results),
            glob=glob,
        )


class CodeReadTool(BaseTool):
    name: str = "code_read"
    description: str = (
        "Liest eine Datei aus dem angegebenen Repository zur Prüfung technischer "
        "Aussagen. Ohne repository_path liefert das Werkzeug einen Hinweis."
    )
    args_schema: type[BaseModel] = SourceReadInput
    corpus: TextCorpus = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self, path: str, max_chars: int = 40_000) -> str:
        if not self.corpus.roots:
            return "Kein repository_path angegeben; Codeabgleich nicht möglich."
        return _safe_call(self.corpus.read, path, _clamp_chars(max_chars))


class CodeSearchTool(BaseTool):
    name: str = "code_search"
    description: str = (
        "Durchsucht den angegebenen Repository-Code nach Symbolen, Statuscodes "
        "und Konfigurationswerten. Ohne repository_path nicht verfügbar."
    )
    args_schema: type[BaseModel] = SourceSearchInput
    corpus: TextCorpus = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(
        self, query: str, max_results: int = 20, glob: str | None = None
    ) -> str:
        if not self.corpus.roots:
            return "Kein repository_path angegeben; Codeabgleich nicht möglich."
        return _safe_call(
            self.corpus.search,
            query,
            max_results=_clamp_results(max_results),
            glob=glob,
        )


class DocumentListTool(BaseTool):
    name: str = "doc_list"
    description: str = (
        "Listet alle erzeugten Markdown-Dateien mit Content-Typ, Wortzahl und "
        "Status. Vor Reviews und QA immer aufrufen."
    )
    args_schema: type[BaseModel] = DocumentListInput
    workspace: DocumentationWorkspace = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self) -> str:
        return self.workspace.inventory()


class DocumentReadTool(BaseTool):
    name: str = "doc_read"
    description: str = (
        "Liest ein erzeugtes Markdown-Dokument über seinen relativen Pfad. "
        "Große Dateien werden gekürzt."
    )
    args_schema: type[BaseModel] = DocumentReadInput
    workspace: DocumentationWorkspace = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self, path: str, max_chars: int = 40_000) -> str:
        return _safe_call(
            self.workspace.read_document, path, _clamp_chars(max_chars)
        )


class DocumentWriteTool(BaseTool):
    name: str = "doc_write"
    description: str = (
        "Schreibt eine Markdown-Datei unterhalb des Ausgabeverzeichnisses. "
        "Nur .md und .mmd sind erlaubt; absolute Pfade und .. werden abgewiesen. "
        "Der Inhalt muss vollständig sein und mit Front Matter beginnen."
    )
    args_schema: type[BaseModel] = DocumentWriteInput
    workspace: DocumentationWorkspace = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self, path: str, content: str) -> str:
        try:
            record = self.workspace.write_document(path, content)
        except WorkspaceError as exc:
            return f"FEHLER: {exc}"
        return json.dumps(record, ensure_ascii=False)


class CheckLinksTool(BaseTool):
    name: str = "check_links"
    description: str = (
        "Prüft alle relativen internen Links und Bildpfade in den erzeugten "
        "Markdown-Dateien. Anchors werden nicht geprüft."
    )
    args_schema: type[BaseModel] = LinkCheckInput
    workspace: DocumentationWorkspace = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self) -> str:
        return self.workspace.check_links()


class VerifyMermaidTool(BaseTool):
    name: str = "verify_mermaid"
    description: str = (
        "Prüft Mermaid-Blöcke auf bekannte Diagrammtypen, leere Blöcke und "
        "Tabulatoren. Eine echte Rendering-Prüfung ersetzt das nicht."
    )
    args_schema: type[BaseModel] = MermaidCheckInput
    workspace: DocumentationWorkspace = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(self) -> str:
        return self.workspace.verify_mermaid()


class RecordQualityScoreTool(BaseTool):
    name: str = "record_quality_score"
    description: str = (
        "Berechnet aus sechs Dimensionen einen gewichteten QA-Score, erzwingt "
        "changes_requested bei Blockern oder Unterschreitung des Schwellenwerts "
        "und schreibt _qa/qa-report.json sowie eine Rundenkopie. Genau einmal "
        "pro QA-Durchlauf aufrufen."
    )
    args_schema: type[BaseModel] = QualityScoreInput
    scorer: QualityScorer = Field(exclude=True)
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _run(
        self,
        completeness: int,
        accuracy: int,
        clarity: int,
        consistency: int,
        diataxis: int,
        usability: int,
        blockers: list[str] | None = None,
        decision: Literal["approved", "changes_requested"] = "changes_requested",
        summary: str = "",
        iteration: int = 0,
    ) -> str:
        dimensions = {
            "completeness": completeness,
            "accuracy": accuracy,
            "clarity": clarity,
            "consistency": consistency,
            "diataxis": diataxis,
            "usability": usability,
        }
        try:
            report = build_report(
                dimensions=dimensions,
                blockers=blockers or [],
                decision=decision,
                summary=summary,
                threshold=self.scorer.threshold,
                iteration=iteration,
            )
            self.scorer.write(report)
        except Exception as exc:  # noqa: BLE001 - Fehler als Text an den Agenten
            return f"FEHLER: {type(exc).__name__}: {exc}"
        return (
            f"QA-Report geschrieben: score={report.score}/100, "
            f"threshold={report.threshold}, decision={report.decision}, "
            f"blockers={len(report.blockers)}\n"
            + json.dumps(report.to_dict(), ensure_ascii=False, indent=2)
        )
