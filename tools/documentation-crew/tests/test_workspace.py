from __future__ import annotations

from pathlib import Path

import pytest

from documentation_crew.workspace import (
    DocumentationWorkspace,
    TextCorpus,
    WorkspaceError,
    extract_markdown_bundle,
)


def test_write_and_read_document(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    record = workspace.write_document(
        "tutorials/einstieg.md", "# Einstieg\n\nLos geht es.\n"
    )
    assert record["path"] == "tutorials/einstieg.md"
    assert record["words"] >= 4
    text = workspace.read_document("tutorials/einstieg.md")
    assert "# Einstieg" in text
    assert "tutorials/einstieg.md" in workspace.inventory()


def test_write_rejects_traversal_and_wrong_suffix(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    with pytest.raises(WorkspaceError, match="verlässt"):
        workspace.write_document("../ausbruch.md", "# Nein\n")
    with pytest.raises(WorkspaceError, match="endung"):
        workspace.write_document("tutorials/script.sh", "echo nein")


def test_write_rejects_symlink_escape(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    outside = tmp_path / "outside"
    outside.mkdir()
    link = workspace.root / "tutorials" / "link"
    link.symlink_to(outside, target_is_directory=True)
    with pytest.raises(WorkspaceError, match="verlässt"):
        workspace.write_document("tutorials/link/datei.md", "# Nein\n")


def test_check_links_reports_missing_target(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    workspace.write_document(
        "README.md",
        "# Start\n\n[Gut](tutorials/einstieg.md) und [Schlecht](fehlt.md).\n",
    )
    workspace.write_document("tutorials/einstieg.md", "# Einstieg\n")
    report = workspace.check_links()
    assert "1 defekte" in report
    assert "fehlt.md" in report


def test_verify_mermaid_accepts_known_types(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    workspace.write_document(
        "explanation/architektur.md",
        "# Architektur\n\n```mermaid\nflowchart TD\n  A --> B\n```\n",
    )
    assert "keine Syntaxbefunde" in workspace.verify_mermaid()


def test_verify_mermaid_flags_unknown_type(tmp_path: Path):
    workspace = DocumentationWorkspace(tmp_path / "out")
    workspace.write_document(
        "explanation/architektur.md",
        "# Architektur\n\n```mermaid\nnotADiagram\n  A --> B\n```\n",
    )
    report = workspace.verify_mermaid()
    assert "unbekannter Diagrammtyp" in report


def test_source_corpus_reads_and_searches(docs_tree: Path):
    corpus = TextCorpus([docs_tree], {".md"})
    assert corpus.size == 3
    labels = corpus.labels()
    assert "README.md" in labels
    text = corpus.read("README.md")
    assert "kurzer Einstieg" in text
    hits = corpus.search("Rust")
    assert "guide.md" in hits
    with pytest.raises(WorkspaceError, match="Unbekannte Quelle"):
        corpus.read("nicht-da.md")


def test_extract_markdown_bundle_splits_sections():
    bundle = """## Datei: `tutorials/start.md`
# Start

```bash
echo hallo
```

## Datei: `reference/api.md`
# API

| Feld | Typ |
|---|---|
| id | string |
"""
    documents = extract_markdown_bundle(bundle)
    assert set(documents) == {"tutorials/start.md", "reference/api.md"}
    assert documents["tutorials/start.md"].startswith("# Start")
    assert "echo hallo" in documents["tutorials/start.md"]
    assert documents["reference/api.md"].startswith("# API")


def test_extract_markdown_bundle_rejects_unsafe_paths():
    bundle = """## Datei: `../ausbruch.md`
# Nein

## Datei: `/etc/passwd.md`
# Nein

## Datei: `bild.png`
# Nein
"""
    assert extract_markdown_bundle(bundle) == {}


def test_document_read_schema_accepts_small_max_chars():
    from documentation_crew.tools.docs_tools import DocumentReadInput

    payload = DocumentReadInput(path="README.md", max_chars=500)
    assert payload.max_chars == 500
