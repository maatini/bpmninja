"""Reproduzierbare Qualitätsbewertung und Score-Persistenz."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
import json
import re
from pathlib import Path
from typing import Any, Mapping

from .workspace import DocumentationWorkspace, WorkspaceError


DIMENSION_WEIGHTS: dict[str, float] = {
    "completeness": 0.20,
    "accuracy": 0.25,
    "clarity": 0.15,
    "consistency": 0.15,
    "diataxis": 0.15,
    "usability": 0.10,
}

DECISIONS = {"approved", "changes_requested"}


class QualityError(ValueError):
    """Ungültiger Qualitätsdatensatz."""


def compute_score(dimensions: Mapping[str, int]) -> int:
    """Gewichteten Gesamtscore berechnen; fehlende Dimension ist ein Fehler."""
    missing = [name for name in DIMENSION_WEIGHTS if name not in dimensions]
    if missing:
        raise QualityError("Fehlende Dimensionen: " + ", ".join(missing))
    total = 0.0
    for name, weight in DIMENSION_WEIGHTS.items():
        value = dimensions[name]
        if not isinstance(value, int) or not 0 <= value <= 100:
            raise QualityError(f"Dimension '{name}' muss zwischen 0 und 100 liegen.")
        total += value * weight
    return int(round(total))


@dataclass(frozen=True)
class QualityReport:
    score: int
    threshold: int
    dimensions: dict[str, int]
    blockers: list[str]
    decision: str
    summary: str
    iteration: int
    created_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )

    @property
    def passed(self) -> bool:
        return self.decision == "approved" and not self.blockers and self.score >= self.threshold

    def to_dict(self) -> dict[str, Any]:
        return {
            "score": self.score,
            "threshold": self.threshold,
            "dimensions": dict(self.dimensions),
            "blockers": list(self.blockers),
            "decision": self.decision,
            "passed": self.passed,
            "summary": self.summary,
            "iteration": self.iteration,
            "created_at": self.created_at,
            "weights": dict(DIMENSION_WEIGHTS),
        }


def build_report(
    *,
    dimensions: Mapping[str, int],
    blockers: list[str],
    decision: str,
    summary: str,
    threshold: int,
    iteration: int,
) -> QualityReport:
    clean_blockers = [str(item).strip() for item in blockers if str(item).strip()]
    if decision not in DECISIONS:
        raise QualityError("decision muss 'approved' oder 'changes_requested' sein.")
    score = compute_score(dimensions)
    effective = decision
    if clean_blockers or score < threshold:
        effective = "changes_requested"
    return QualityReport(
        score=score,
        threshold=threshold,
        dimensions={name: int(value) for name, value in dimensions.items()},
        blockers=clean_blockers,
        decision=effective,
        summary=summary.strip() or "Keine Begründung angegeben.",
        iteration=iteration,
    )


class QualityScorer:
    """Schreibt Score-Dateien in den Ausgabe-Sandbox."""

    def __init__(self, workspace: DocumentationWorkspace, threshold: int) -> None:
        self.workspace = workspace
        self.threshold = threshold

    @property
    def latest_path(self) -> Path:
        return self.workspace.resolve("_qa/qa-report.json", {".json"})

    def write(self, report: QualityReport) -> QualityReport:
        payload = report.to_dict()
        latest = self.latest_path
        latest.parent.mkdir(parents=True, exist_ok=True)
        latest.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        round_path = self.workspace.resolve(
            f"_qa/qa-report-round-{report.iteration}.json", {".json"}
        )
        round_path.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return report

    def read_latest(self) -> QualityReport | None:
        path = self.latest_path
        if not path.is_file():
            return None
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return None
        try:
            return report_from_dict(payload)
        except QualityError:
            return None

    def write_fallback(self, raw: str, iteration: int) -> QualityReport | None:
        """Score aus Freitext retten, wenn der QA-Agent das Werkzeug nicht nutzte."""
        score = extract_score(raw)
        if score is None:
            return None
        report = QualityReport(
            score=score,
            threshold=self.threshold,
            dimensions={},
            blockers=(
                []
                if score >= self.threshold
                else ["QA-Werkzeug wurde nicht aufgerufen; Score aus Text geparst."]
            ),
            decision="approved" if score >= self.threshold else "changes_requested",
            summary="Fallback: Gesamtscore aus der QA-Ausgabe geparst.",
            iteration=iteration,
        )
        return self.write(report)


def report_from_dict(payload: Mapping[str, Any]) -> QualityReport:
    try:
        dimensions = {
            str(key): int(value)
            for key, value in dict(payload.get("dimensions") or {}).items()
        }
        score = int(payload["score"])
        threshold = int(payload["threshold"])
        iteration = int(payload.get("iteration", 0))
        decision = str(payload.get("decision", "changes_requested"))
        summary = str(payload.get("summary", ""))
        blockers = [str(item) for item in payload.get("blockers") or []]
    except (KeyError, TypeError, ValueError) as exc:
        raise QualityError(f"Ungültiger QA-Datensatz: {exc}") from exc
    if decision not in DECISIONS:
        decision = "changes_requested"
    if blockers or score < threshold:
        decision = "changes_requested"
    return QualityReport(
        score=score,
        threshold=threshold,
        dimensions=dimensions,
        blockers=blockers,
        decision=decision,
        summary=summary,
        iteration=iteration,
        created_at=str(payload.get("created_at") or datetime.now(timezone.utc).isoformat()),
    )


def extract_score(text: str) -> int | None:
    pattern = re.compile(
        r"(?:QA[-\s]?Score|Gesamtscore|Gesamtpunktzahl|Overall\s+Score)\s*[:=]\s*"
        r"(\d{1,3})(?:\s*/\s*100)?",
        re.IGNORECASE,
    )
    match = pattern.search(text)
    if not match:
        return None
    value = int(match.group(1))
    return value if 0 <= value <= 100 else None


def quality_history(reports: list[QualityReport]) -> str:
    if not reports:
        return "Noch keine Messung."
    return "\n".join(
        f"- Runde {report.iteration}: {report.score}/100, "
        f"{report.decision}, {len(report.blockers)} Blocker"
        for report in reports
    )


def qa_summary(report: QualityReport | None) -> str:
    if report is None:
        return "Kein maschinenlesbarer QA-Report vorhanden."
    return json.dumps(report.to_dict(), ensure_ascii=False, indent=2)


def archive_report(workspace: DocumentationWorkspace, report: QualityReport) -> None:
    """Zusätzliche Kopie, falls ein Agent die Latest-Datei überschreibt."""
    target = workspace.resolve(f"_qa/round-{report.iteration}.json", {".json"})
    data = json.dumps(report.to_dict(), ensure_ascii=False, indent=2) + "\n"
    try:
        target.write_text(data, encoding="utf-8")
    except OSError as exc:
        raise WorkspaceError(f"QA-Archiv nicht schreibbar: {exc}") from exc
