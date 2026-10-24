from __future__ import annotations

from pathlib import Path

import pytest


@pytest.fixture()
def docs_tree(tmp_path: Path) -> Path:
    root = tmp_path / "docs"
    root.mkdir()
    (root / "README.md").write_text(
        "# Demo\n\nEin kurzer Einstieg.\n", encoding="utf-8"
    )
    (root / "guide.md").write_text(
        "# Anleitung\n\n## Voraussetzungen\n\nRust 1.99.\n", encoding="utf-8"
    )
    (root / "nested").mkdir()
    (root / "nested" / "reference.md").write_text(
        "# Referenz\n\nDie API.\n", encoding="utf-8"
    )
    return root


@pytest.fixture()
def config_data(tmp_path: Path, docs_tree: Path) -> dict[str, object]:
    return {
        "project_name": "Demo",
        "project_description": "Eine Demo-Dokumentation.",
        "domain_topic": "Software-Dokumentation",
        "existing_docs_path": [str(docs_tree)],
        "target_audience": ["Entwickler", "Betrieb"],
        "output_path": str(tmp_path / "out"),
        "quality_threshold": 90,
        "max_revision_rounds": 2,
    }
