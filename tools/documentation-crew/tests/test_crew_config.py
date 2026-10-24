from __future__ import annotations

from pathlib import Path

from crewai.tools.agent_tools.base_agent_tools import BaseAgentTool

from documentation_crew.config import RunConfig
from documentation_crew.crew import DocumentationCrew


def test_dry_run_builds_all_crews(tmp_path: Path, config_data: dict, monkeypatch):
    monkeypatch.setenv("DOC_CREW_MEMORY", "0")
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    crew = DocumentationCrew(config, verbose=False, dry_run=True)
    report = crew.dry_run_report()
    assert report.agent_count == 10
    assert report.main_task_count == 14
    assert report.revision_task_count == 3
    assert report.source_files == 3
    assert report.memory_enabled is False
    assert "record_quality_score" in report.tool_names


def test_hierarchical_main_crew_excludes_manager(
    tmp_path: Path, config_data: dict, monkeypatch
):
    monkeypatch.setenv("DOC_CREW_MEMORY", "0")
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    crew = DocumentationCrew(config, verbose=False, dry_run=True)
    main, tasks = crew._main_crew()
    assert main.process.value == "hierarchical"
    assert main.manager_agent is not None
    assert main.manager_agent.role not in [agent.role for agent in main.agents]
    # CrewAI erlaubt am Manager nur Agent-Tools; ohne sie gäbe es keine Delegation.
    assert main.manager_agent.tools
    assert all(isinstance(tool, BaseAgentTool) for tool in main.manager_agent.tools)
    assert len(main.agents) == 9
    assert len(tasks) == 14
    names = {task.name for task in tasks}
    assert "quality_assurance_task" in names
    assert "executive_summary_task" not in names


def test_task_callback_persists_missing_artifact_once(
    tmp_path: Path, config_data: dict, monkeypatch
):
    monkeypatch.setenv("DOC_CREW_MEMORY", "0")
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    crew = DocumentationCrew(config, verbose=False, dry_run=True)
    _, tasks = crew._main_crew()
    task = next(item for item in tasks if item.name == "domain_research_task")

    class Output:
        raw = "# Recherche\n\nInhalt der Recherche."

    target = config.output_path / "_research" / "domain-research.md"
    assert not target.exists()
    task.callback(Output())
    assert target.read_text(encoding="utf-8").startswith("# Recherche")

    class Later:
        raw = "# Überschrieben"

    task.callback(Later())
    assert "Überschrieben" not in target.read_text(encoding="utf-8")


def test_bundle_callback_writes_all_content_files_and_skips_audit_bundles(
    tmp_path: Path, config_data: dict, monkeypatch
):
    monkeypatch.setenv("DOC_CREW_MEMORY", "0")
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    crew = DocumentationCrew(config, verbose=False, dry_run=True)
    _, tasks = crew._main_crew()
    tutorial = next(item for item in tasks if item.name == "tutorial_creation_task")
    audit = next(item for item in tasks if item.name == "documentation_audit_task")

    class Output:
        raw = (
            "## Datei: `tutorials/start.md`\n# Start\n\nInhalt.\n\n"
            "## Datei: `how-to/deploy.md`\n# Deploy\n\nInhalt.\n"
        )

    tutorial.callback(Output())
    assert (config.output_path / "tutorials" / "start.md").exists()
    assert (config.output_path / "how-to" / "deploy.md").exists()

    class AuditOutput:
        raw = "## Datei: `reference/erfunden.md`\n# Nicht schreiben\n"

    audit.callback(AuditOutput())
    assert not (config.output_path / "reference" / "erfunden.md").exists()


def test_final_crew_is_sequential(tmp_path: Path, config_data: dict, monkeypatch):
    monkeypatch.setenv("DOC_CREW_MEMORY", "0")
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    crew = DocumentationCrew(config, verbose=False, dry_run=True)
    final, tasks = crew._final_crew()
    assert final.process.value == "sequential"
    assert len(final.agents) == 1
    assert final.agents[0].tools
    assert tasks[0].name == "executive_summary_task"


def test_resume_without_documents_raises(tmp_path: Path, config_data: dict, monkeypatch):
    import pytest

    from documentation_crew.crew import CrewRunError

    monkeypatch.setenv("DOC_CREW_MEMORY", "0")
    config = RunConfig.from_mapping(config_data, base_dir=tmp_path)
    crew = DocumentationCrew(config, verbose=False, dry_run=False)
    with pytest.raises(CrewRunError, match="Resume"):
        crew.run(resume=True)
