from __future__ import annotations

from pathlib import Path

from documentation_crew.qa import (
    QualityScorer,
    build_report,
    compute_score,
    extract_score,
)
from documentation_crew.workspace import DocumentationWorkspace


DIMENSIONS = {
    "completeness": 90,
    "accuracy": 95,
    "clarity": 90,
    "consistency": 90,
    "diataxis": 90,
    "usability": 90,
}


def test_weighted_score():
    assert compute_score(DIMENSIONS) == 91


def test_blockers_force_changes_requested():
    report = build_report(
        dimensions=DIMENSIONS,
        blockers=["tutorials/einstieg.md: Schritt 3 ist falsch"],
        decision="approved",
        summary="Fast fertig.",
        threshold=90,
        iteration=0,
    )
    assert report.score == 91
    assert report.decision == "changes_requested"
    assert not report.passed


def test_low_score_forces_changes_requested():
    dims = {key: 60 for key in DIMENSIONS}
    report = build_report(
        dimensions=dims,
        blockers=[],
        decision="approved",
        summary="Zu dünn.",
        threshold=90,
        iteration=1,
    )
    assert report.decision == "changes_requested"
    assert not report.passed


def test_score_roundtrip(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    scorer = QualityScorer(workspace, threshold=90)
    report = build_report(
        dimensions=DIMENSIONS,
        blockers=[],
        decision="approved",
        summary="Freigabe.",
        threshold=90,
        iteration=0,
    )
    scorer.write(report)
    loaded = scorer.read_latest()
    assert loaded is not None
    assert loaded.score == 91
    assert loaded.passed
    assert (workspace.root / "_qa" / "qa-report-round-0.json").is_file()


def test_fallback_parses_score(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    scorer = QualityScorer(workspace, threshold=90)
    report = scorer.write_fallback("Ergebnis: Gesamtscore: 94/100", iteration=1)
    assert report is not None
    assert report.score == 94
    assert report.passed
    assert extract_score("QA-Score = 77") == 77
    assert extract_score("kein score") is None
