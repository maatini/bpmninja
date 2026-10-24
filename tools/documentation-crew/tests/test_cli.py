from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys


PROJECT_ROOT = Path(__file__).resolve().parents[1]


def test_dry_run_cli(tmp_path: Path):
    docs = tmp_path / "docs"
    docs.mkdir()
    (docs / "README.md").write_text("# Demo\n\nInhalt.\n", encoding="utf-8")
    inputs = {
        "project_name": "Demo",
        "project_description": "Eine Demo.",
        "domain_topic": "Dokumentation",
        "existing_docs_path": [str(docs)],
        "target_audience": "Entwickler",
        "output_path": str(tmp_path / "out"),
        "quality_threshold": 90,
        "max_revision_rounds": 1,
    }
    input_file = tmp_path / "inputs.json"
    input_file.write_text(json.dumps(inputs), encoding="utf-8")
    env = dict(os.environ)
    env["PYTHONPATH"] = str(PROJECT_ROOT / "src")
    env["DOC_CREW_MEMORY"] = "0"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "documentation_crew.main",
            "--inputs",
            str(input_file),
            "--dry-run",
            "--quiet",
        ],
        cwd=PROJECT_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert "Dry-Run erfolgreich" in result.stdout
    assert "Hauptaufgaben: 14" in result.stdout
    assert "Memo" not in result.stdout or "aus" in result.stdout


def test_missing_inputs_returns_error(tmp_path: Path):
    env = dict(os.environ)
    env["PYTHONPATH"] = str(PROJECT_ROOT / "src")
    result = subprocess.run(
        [sys.executable, "-m", "documentation_crew.main", "--dry-run"],
        cwd=PROJECT_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 2
    assert "Fehlende Pflichtfelder" in result.stderr
