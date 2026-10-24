from __future__ import annotations

from pathlib import Path

import pytest

from documentation_crew.config import ConfigError, RunConfig, validate_input_keys
from documentation_crew.workspace import build_source_manifest


def test_config_resolves_paths_and_builds_inputs(tmp_path: Path, config_data: dict):
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    assert config.project_name == "Demo"
    assert config.target_audience == "Entwickler, Betrieb"
    assert config.output_path == (tmp_path / "out").resolve()
    assert config.quality_threshold == 90
    assert len(config.existing_docs_paths) == 1
    inputs = config.base_inputs()
    for key in (
        "project_name",
        "project_description",
        "domain_topic",
        "target_audience",
        "output_path",
        "repository_path",
        "existing_docs_path",
        "source_manifest",
        "documentation_language",
        "quality_threshold",
        "qa_threshold",
        "iteration",
        "qa_feedback",
        "qa_summary",
        "quality_history",
    ):
        assert key in inputs
    assert "README.md" in inputs["source_manifest"]


def test_output_path_may_not_exist_yet(tmp_path: Path, docs_tree: Path):
    missing = tmp_path / "neu" / "ausgabe"
    config = RunConfig.from_mapping(
        {
            "project_name": "Demo",
            "project_description": "Demo",
            "domain_topic": "Doku",
            "existing_docs_path": [str(docs_tree)],
            "target_audience": "Entwickler",
            "output_path": str(missing),
        }
    )
    assert config.output_path == missing.resolve()


def test_missing_source_is_rejected(tmp_path: Path, config_data: dict):
    config_data["existing_docs_path"] = [str(tmp_path / "fehlt")]
    with pytest.raises(ConfigError, match="existiert nicht"):
        RunConfig.from_mapping(config_data, base_dir=tmp_path)


def test_invalid_threshold_is_rejected(tmp_path: Path, config_data: dict):
    config_data["quality_threshold"] = 120
    with pytest.raises(ConfigError, match="zwischen"):
        RunConfig.from_mapping(config_data, base_dir=tmp_path)


def test_missing_required_fields():
    with pytest.raises(ConfigError, match="project_name"):
        validate_input_keys(
            {
                "project_description": "x",
                "domain_topic": "x",
                "existing_docs_path": ["x"],
                "target_audience": "x",
                "output_path": "x",
            }
        )


def test_manifest_contains_titles_and_limits(docs_tree: Path):
    manifest = build_source_manifest([docs_tree])
    assert "# Quellenmanifest" in manifest
    assert "README.md" in manifest
    assert "Demo" in manifest
    assert "nested/reference.md" in manifest


def test_repository_path_must_exist(tmp_path: Path, config_data: dict):
    config_data["repository_path"] = str(tmp_path / "kein-repo")
    with pytest.raises(ConfigError, match="repository_path"):
        RunConfig.from_mapping(config_data, base_dir=tmp_path)
