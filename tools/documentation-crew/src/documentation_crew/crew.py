"""Orchestrierung der Documentation Crew.

Der Hauptlauf ist hierarchisch: Der Documentation Lead ist Manager, die neun
Spezialisten führen die Aufgaben aus. Nach dem QA-Lauf entscheidet ein
reproduzierbarer Score, ob eine oder mehrere Nachbesserungsrunden folgen. Der
Abschlussbericht wird anschließend in einem kleinen sequenziellen Lauf vom Lead
selbst geschrieben.
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from datetime import datetime, timezone
import json
import logging
import os
from pathlib import Path
import re
import shutil
import time
from typing import Any, Mapping, Sequence

import yaml

from crewai import Agent, Crew, LLM, Process, Task
from crewai.memory.unified_memory import Memory
from crewai.tools.agent_tools.agent_tools import AgentTools
from crewai.tools.tool_failure import ToolFailurePolicy
from crewai.utilities import agent_utils as _crewai_agent_utils

# CrewAI 1.15 beendet den gesamten Crew-Lauf, sobald ein Agent denselben
# Werkzeugaufruf fünfmal wiederholt. Bei langen Dokumentationsläufen ist ein
# Abbruch teurer als ein weiterer Iterationsschritt; max_iter begrenzt den
# Agenten bereits. Die Schwelle liegt deshalb über den üblichen max_iter-Werten.
_crewai_agent_utils.MAX_REPEATED_TOOL_CALLS = 40

from .config import ConfigError, RunConfig
from .qa import (
    QualityReport,
    QualityScorer,
    archive_report,
    qa_summary,
    quality_history,
)
from .tools import (
    CheckLinksTool,
    CodeReadTool,
    CodeSearchTool,
    DocumentListTool,
    DocumentReadTool,
    DocumentWriteTool,
    FetchUrlTool,
    RecordQualityScoreTool,
    SourceInventoryTool,
    SourceReadTool,
    SourceSearchTool,
    VerifyMermaidTool,
    build_web_search_tool,
)
from .workspace import (
    ALLOWED_DOC_SUFFIXES,
    CODE_EXTENSIONS,
    DOC_EXTENSIONS,
    DocumentationWorkspace,
    TextCorpus,
    WorkspaceError,
    extract_markdown_bundle,
)


LOGGER = logging.getLogger("documentation_crew")
CONFIG_DIR = Path(__file__).resolve().parent / "config"
AGENTS_FILE = CONFIG_DIR / "agents.yaml"
TASKS_FILE = CONFIG_DIR / "tasks.yaml"

DEFAULT_MODEL = "deepseek/deepseek-chat"
MEMORY_AGENTS = {
    "documentation_lead",
    "domain_researcher",
    "documentation_auditor",
    "information_architect",
    "senior_technical_writer",
    "quality_assurance_reviewer",
}
MANAGER = "documentation_lead"
PLACEHOLDER = re.compile(r"\{([A-Za-z_][A-Za-z0-9_]*)\}")

# Der Hauptlauf ist in zwei Crews geteilt. So kann die Review-Phase auch dann
# laufen, wenn eine einzelne Inhaltsaufgabe ihr Iterationslimit erreicht.
CONTENT_TASK_NAMES = (
    "domain_research_task",
    "documentation_audit_task",
    "information_architecture_task",
    "tutorial_creation_task",
    "howto_creation_task",
    "reference_creation_task",
    "explanation_creation_task",
    "landing_page_task",
    "content_manifest_task",
)
REVIEW_TASK_NAMES = (
    "technical_review_task",
    "style_consistency_task",
    "ux_optimization_task",
    "diagram_integration_task",
    "quality_assurance_task",
)

# Nur diese Aufgaben dürfen ganze Datei-Bundles anlegen. Berichtsaufgaben
# liefern Empfehlungen und dürfen keine Dokumentationsdateien erzeugen.
BUNDLE_TASKS = {
    "tutorial_creation_task",
    "howto_creation_task",
    "reference_creation_task",
    "explanation_creation_task",
    "landing_page_task",
    "content_manifest_task",
    "revision_task",
}

# CrewAI bricht den gesamten Lauf ab, wenn ein Agent denselben Werkzeugaufruf
# wiederholt. Diese Regel gilt für jede Aufgabe und hält Agenten in Bewegung.
TOOL_DISCIPLINE = (
    "\n\nWerkzeugdisziplin: Rufe kein Werkzeug mit identischen Argumenten erneut "
    "auf. Lies eine Datei höchstens einmal. Wenn eine Datei nicht existiert, "
    "überspringe sie und dokumentiere die Lücke, statt den Aufruf zu wiederholen."
)

# Primärartefakt je Aufgabe. Erreicht ein Agent sein Iterationslimit, bevor er
# doc_write aufruft, wird seine finale Antwort hier gesichert. Existierende
# Dateien werden nie überschrieben.
TASK_FALLBACKS: dict[str, str] = {
    "domain_research_task": "_research/domain-research.md",
    "documentation_audit_task": "_audit/audit-report.md",
    "information_architecture_task": "_architecture/information-architecture.md",
    "tutorial_creation_task": "tutorials/erster-prozess.md",
    "howto_creation_task": "how-to/external-task-worker-schreiben.md",
    "reference_creation_task": "reference/README.md",
    "explanation_creation_task": "explanation/architektur.md",
    "landing_page_task": "README.md",
    "content_manifest_task": "_qa/content-manifest.md",
    "technical_review_task": "_review/technical-review.md",
    "style_consistency_task": "_review/style-report.md",
    "ux_optimization_task": "_review/ux-review.md",
    "diagram_integration_task": "_architecture/diagram-plan.md",
    "quality_assurance_task": "_qa/qa-report.md",
    "revision_task": "_qa/revision-round-{iteration}.md",
    "revision_technical_task": "_review/technical-review-round-{iteration}.md",
    "executive_summary_task": "EXECUTIVE-SUMMARY.md",
}


class CrewRunError(RuntimeError):
    """Ein Lauf konnte nicht gestartet oder abgeschlossen werden."""


@dataclass(frozen=True)
class RunResult:
    approved: bool
    score: int | None
    reports: list[QualityReport]
    output_path: Path
    duration_seconds: float
    final_summary: str


@dataclass(frozen=True)
class DryRunReport:
    project_name: str
    output_path: Path
    source_files: int
    code_files: int
    agent_count: int
    main_task_count: int
    revision_task_count: int
    tool_names: list[str]
    memory_enabled: bool
    model: str


def load_yaml_file(path: Path) -> dict[str, Any]:
    try:
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
    except (OSError, yaml.YAMLError) as exc:
        raise ConfigError(f"Konfiguration nicht lesbar: {path}: {exc}") from exc
    if not isinstance(data, dict) or not data:
        raise ConfigError(f"Konfiguration ist leer oder kein Objekt: {path}")
    for name, value in data.items():
        if not isinstance(value, dict):
            raise ConfigError(f"Eintrag '{name}' in {path.name} muss ein Objekt sein.")
    return data


def _memory_enabled() -> bool:
    raw = os.getenv("DOC_CREW_MEMORY", "1").strip().lower()
    return raw not in {"0", "false", "no", "off"}


def _tool_failure_policy() -> ToolFailurePolicy:
    raw = os.getenv("DOC_CREW_TOOL_FAILURE_POLICY", "warn").strip().lower()
    return {
        "raise": ToolFailurePolicy.RAISE,
        "warn": ToolFailurePolicy.WARN,
        "ignore": ToolFailurePolicy.IGNORE,
    }.get(raw, ToolFailurePolicy.WARN)


def _embedder_spec() -> dict[str, Any] | None:
    raw = os.getenv("DOC_CREW_EMBEDDER", "onnx").strip()
    if not raw or raw.lower() == "openai":
        return None
    if raw.startswith("{"):
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise ConfigError(f"DOC_CREW_EMBEDDER ist kein gültiges JSON: {exc}") from exc
        if not isinstance(value, dict):
            raise ConfigError("DOC_CREW_EMBEDDER muss ein JSON-Objekt sein.")
        return value
    if raw.lower() == "onnx":
        return {"provider": "onnx", "config": {}}
    raise ConfigError(
        "DOC_CREW_EMBEDDER muss 'onnx', 'openai' oder ein JSON-Objekt sein."
    )


def build_llm(
    model: str,
    *,
    effort: str = "",
    temperature: float = 0.2,
    max_tokens: int = 8192,
    dry_run: bool = False,
) -> LLM:
    """LLM bauen und bei fehlendem Key eine verständliche Meldung liefern."""
    kwargs: dict[str, Any] = {
        "model": model,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "timeout": 1800,
    }
    if effort:
        kwargs["reasoning_effort"] = effort
    try:
        return LLM(**kwargs)
    except Exception as exc:
        if dry_run:
            try:
                return LLM(api_key="dry-run", **kwargs)
            except Exception as retry_exc:  # pragma: no cover - defensiv
                raise ConfigError(
                    f"LLM konnte auch im Dry-Run nicht erzeugt werden: {retry_exc}"
                ) from retry_exc
        raise ConfigError(
            f"LLM '{model}' konnte nicht initialisiert werden. Prüfe den "
            "API-Key (für DeepSeek: DEEPSEEK_API_KEY) und das Modell. "
            f"Originalfehler: {exc}"
        ) from exc


class DocumentationCrew:
    """Wiederverwendbare Crew-Fabrik mit Haupt-, Revisions- und Abschlusslauf."""

    def __init__(
        self,
        config: RunConfig,
        *,
        verbose: bool = True,
        dry_run: bool = False,
        model: str | None = None,
        manager_model: str | None = None,
        effort: str | None = None,
        temperature: float | None = None,
    ) -> None:
        self.config = config
        self.verbose = verbose
        self.dry_run = dry_run
        self.model = model or os.getenv("DOC_CREW_MODEL", DEFAULT_MODEL)
        self.manager_model = manager_model or os.getenv(
            "DOC_CREW_MANAGER_MODEL", self.model
        )
        self.effort = effort if effort is not None else os.getenv("DOC_CREW_EFFORT", "")
        self.temperature = (
            temperature
            if temperature is not None
            else float(os.getenv("DOC_CREW_TEMPERATURE", "0.2"))
        )
        self.llm = build_llm(
            self.model,
            effort=self.effort,
            temperature=self.temperature,
            dry_run=dry_run,
        )
        self.manager_llm = (
            self.llm
            if self.manager_model == self.model
            else build_llm(
                self.manager_model,
                effort=self.effort,
                temperature=self.temperature,
                dry_run=dry_run,
            )
        )

        self.agents_config = load_yaml_file(AGENTS_FILE)
        self.tasks_config = load_yaml_file(TASKS_FILE)
        self.workspace = DocumentationWorkspace(config.output_path)
        self.sources = TextCorpus(config.existing_docs_paths, DOC_EXTENSIONS)
        self.code = TextCorpus(
            (config.repository_path,) if config.repository_path else (),
            CODE_EXTENSIONS,
        )
        self.scorer = QualityScorer(self.workspace, config.quality_threshold)
        self.memory_enabled = _memory_enabled()
        self.embedder = _embedder_spec()
        self.tool_failure_policy = _tool_failure_policy()
        self._tools = self._build_tools()
        self._memories: dict[str, Memory] = {}
        # Zeitstempel des aktuellen Laufs: Dateien älterer Läufe werden ersetzt,
        # Dateien aus diesem Lauf bleiben unangetastet.
        self._run_started_epoch: float = 0.0
        self.crew_memory: Memory | None = (
            self._memory_for("crew") if self.memory_enabled else None
        )

        self.main_task_names = [
            name
            for name in self.tasks_config
            if not name.startswith("revision_") and name != "executive_summary_task"
        ]
        self.revision_task_names = [
            name for name in self.tasks_config if name.startswith("revision_")
        ]
        self._validate_configuration()

    # ------------------------------------------------------------------ tools

    def _build_tools(self) -> dict[str, Any]:
        return {
            "web_search": build_web_search_tool(),
            "fetch_url": FetchUrlTool(),
            "source_inventory": SourceInventoryTool(corpus=self.sources),
            "source_read": SourceReadTool(corpus=self.sources),
            "source_search": SourceSearchTool(corpus=self.sources),
            "code_read": CodeReadTool(corpus=self.code),
            "code_search": CodeSearchTool(corpus=self.code),
            "doc_list": DocumentListTool(workspace=self.workspace),
            "doc_read": DocumentReadTool(workspace=self.workspace),
            "doc_write": DocumentWriteTool(workspace=self.workspace),
            "check_links": CheckLinksTool(workspace=self.workspace),
            "verify_mermaid": VerifyMermaidTool(workspace=self.workspace),
            "record_quality_score": RecordQualityScoreTool(scorer=self.scorer),
        }

    def _validate_configuration(self) -> None:
        if MANAGER not in self.agents_config:
            raise ConfigError(f"Agent '{MANAGER}' fehlt in agents.yaml.")
        for name, agent in self.agents_config.items():
            for tool_name in agent.get("tools", []) or []:
                if tool_name not in self._tools:
                    raise ConfigError(
                        f"Agent '{name}' referenziert unbekanntes Werkzeug "
                        f"'{tool_name}'."
                    )
        order = list(self.tasks_config)
        for index, name in enumerate(order):
            task = self.tasks_config[name]
            agent_name = task.get("agent")
            if agent_name not in self.agents_config:
                raise ConfigError(f"Aufgabe '{name}' verweist auf unbekannten Agenten.")
            if name != "executive_summary_task" and agent_name == MANAGER:
                raise ConfigError(
                    f"Aufgabe '{name}' darf nicht dem Manager zugewiesen sein; "
                    "der Manager koordiniert im hierarchischen Prozess."
                )
            for context_name in task.get("context", []) or []:
                if context_name not in self.tasks_config:
                    raise ConfigError(
                        f"Aufgabe '{name}' hat unbekannten Kontext '{context_name}'."
                    )
                if order.index(context_name) >= index:
                    raise ConfigError(
                        f"Aufgabe '{name}' verweist auf einen späteren oder "
                        f"zyklischen Kontext '{context_name}'."
                    )
        if not self.main_task_names:
            raise ConfigError("Keine Hauptaufgaben in tasks.yaml gefunden.")
        phased = set(CONTENT_TASK_NAMES) | set(REVIEW_TASK_NAMES)
        unknown = set(self.main_task_names) - phased
        if unknown:
            raise ConfigError(
                "Hauptaufgaben ohne Phase: " + ", ".join(sorted(unknown))
            )
        missing_phases = phased - set(self.tasks_config)
        if missing_phases:
            raise ConfigError(
                "Fehlende Phasenaufgaben: " + ", ".join(sorted(missing_phases))
            )
        expected = {
            "domain_research_task",
            "documentation_audit_task",
            "information_architecture_task",
            "tutorial_creation_task",
            "howto_creation_task",
            "reference_creation_task",
            "explanation_creation_task",
            "landing_page_task",
            "content_manifest_task",
            "technical_review_task",
            "style_consistency_task",
            "ux_optimization_task",
            "diagram_integration_task",
            "quality_assurance_task",
            "executive_summary_task",
        }
        missing = expected - set(self.tasks_config)
        if missing:
            raise ConfigError("Fehlende Kernaufgaben: " + ", ".join(sorted(missing)))
        if self.tasks_config["executive_summary_task"].get("agent") != MANAGER:
            raise ConfigError(
                "Die Abschlussaufgabe muss dem Documentation Lead zugewiesen sein."
            )

    # ---------------------------------------------------------------- memory

    def _memory_for(self, scope: str) -> Memory:
        if scope not in self._memories:
            safe_scope = re.sub(r"[^a-zA-Z0-9_.-]+", "-", scope).strip("-") or "scope"
            storage = self.config.output_path / "_qa" / "memory" / safe_scope
            storage.mkdir(parents=True, exist_ok=True)
            self._memories[scope] = Memory(
                llm=self.llm,
                embedder=self.embedder,
                storage=str(storage),
                root_scope=f"/documentation/{self.config.slug}/{safe_scope}",
            )
        return self._memories[scope]

    # ---------------------------------------------------------------- agents

    def _build_agent(
        self,
        name: str,
        *,
        as_manager: bool = False,
        force_no_delegation: bool = False,
        memory_scope: str | None = None,
    ) -> Agent:
        data = dict(self.agents_config[name])
        tool_names = data.pop("tools", []) or []
        data.pop("memory", None)
        data["llm"] = self.manager_llm if as_manager else self.llm
        # CrewAI 1.x verbietet Werkzeuge am Manager im hierarchischen Prozess.
        # Der Lead koordiniert dort ausschließlich über Delegation; im finalen
        # sequenziellen Lauf (as_manager=False) behält er seine Werkzeuge.
        data["tools"] = (
            [] if as_manager else [self._tools[tool_name] for tool_name in tool_names]
        )
        data["verbose"] = bool(data.get("verbose", True)) and self.verbose
        data["allow_delegation"] = (
            False
            if force_no_delegation
            else True
            if as_manager
            else bool(data.get("allow_delegation", False))
        )
        data["tool_failure_policy"] = self.tool_failure_policy
        # memory=True würde CrewAIs OpenAI-Standardmemory aktivieren. Stattdessen
        # bekommt jeder Schlüsselagent ein explizit konfiguriertes Memory mit
        # lokalem ONNX-Embedder und eigenem Scope.
        if self.memory_enabled and name in MEMORY_AGENTS:
            data["memory"] = self._memory_for(memory_scope or name)
        return Agent(**data)

    def _agent_for(self, agent_name: str, *, as_manager: bool = False) -> Agent:
        return self._build_agent(agent_name, as_manager=as_manager)

    def _build_workers(self, task_names: Sequence[str]) -> dict[str, Agent]:
        names = sorted(
            {self.tasks_config[name]["agent"] for name in task_names if name in self.tasks_config}
        )
        return {name: self._build_agent(name) for name in names if name != MANAGER}

    # ----------------------------------------------------------------- tasks

    def _task_output_callback(self, task_name: str, iteration: int | None) -> Any:
        """Liefert einen Task-Callback, der das Primärartefakt notfalls anlegt.

        Der Callback läuft direkt nach der Aufgabe, also bevor abhängige
        Aufgaben die Datei lesen. Vorhandene Dateien bleiben unangetastet.
        """
        target = TASK_FALLBACKS.get(task_name)
        if target and iteration is not None and "{iteration}" in target:
            target = target.format(iteration=iteration)
        final_target = target

        def _persist(output: Any) -> None:
            try:
                self._save_task_output(
                    task_name,
                    getattr(output, "raw", "") or "",
                    final_target,
                    overwrite=iteration is not None,
                )
            except Exception:  # noqa: BLE001 - Callback darf den Lauf nicht abbrechen
                LOGGER.exception("Fallback für Aufgabe '%s' fehlgeschlagen.", task_name)

        return _persist

    def _save_task_output(
        self, task_name: str, raw_text: str, target: str | None, *, overwrite: bool = False
    ) -> None:
        """Sichert Datei-Bundles und ersatzweise die finale Antwort einer Aufgabe."""
        raw = raw_text.strip()
        if not raw:
            return

        def _is_current(path: Any) -> bool:
            if not path.exists():
                return False
            if overwrite:
                return False
            return path.stat().st_mtime >= self._run_started_epoch

        written: list[str] = []
        documents = (
            extract_markdown_bundle(raw) if task_name in BUNDLE_TASKS else {}
        )
        for relative, content in documents.items():
            try:
                path = self.workspace.resolve(relative, ALLOWED_DOC_SUFFIXES)
            except WorkspaceError:
                LOGGER.warning(
                    "Aufgabe '%s' nannte einen ungültigen Pfad: %s", task_name, relative
                )
                continue
            if _is_current(path):
                continue
            self.workspace.write_document(relative, content)
            written.append(relative)
        if written:
            LOGGER.info(
                "Aufgabe '%s' lieferte %s Datei(en) im Bundle: %s",
                task_name,
                len(written),
                ", ".join(written),
            )
            return

        if not target:
            return
        try:
            path = self.workspace.resolve(target, ALLOWED_DOC_SUFFIXES)
        except WorkspaceError:
            LOGGER.warning("Fallback-Ziel '%s' ist ungültig.", target)
            return
        if _is_current(path):
            return
        info = self.workspace.write_document(target, raw)
        LOGGER.info(
            "Aufgabe '%s' schrieb keine Datei; finale Antwort nach '%s' gesichert (%s Wörter).",
            task_name,
            target,
            info.get("words", "?"),
        )

    def _build_tasks(
        self,
        task_names: Sequence[str],
        agents: Mapping[str, Agent],
        *,
        iteration: int | None = None,
        allow_external_context: bool = False,
    ) -> list[Task]:
        built: dict[str, Task] = {}
        tasks: list[Task] = []
        for name in task_names:
            data = dict(self.tasks_config[name])
            agent_name = data.pop("agent")
            context_names = list(data.pop("context", []) or [])
            if agent_name not in agents:
                raise ConfigError(f"Agent '{agent_name}' für Aufgabe '{name}' fehlt.")
            missing_context = [item for item in context_names if item not in built]
            if missing_context and not allow_external_context:
                raise ConfigError(
                    f"Aufgabe '{name}' hat nicht gebaute Kontexte: "
                    + ", ".join(missing_context)
                )
            data["name"] = name
            data["agent"] = agents[agent_name]
            data["context"] = [built[item] for item in context_names if item in built]
            data["description"] = (
                data.get("description", "").rstrip() + TOOL_DISCIPLINE
            )
            data["callback"] = self._task_output_callback(name, iteration)
            task = Task(**data)
            built[name] = task
            tasks.append(task)
        return tasks

    # ----------------------------------------------------------------- crews

    def _manager_with_delegation(self, workers: Mapping[str, Agent]) -> Agent:
        """Baut den Lead als Manager und gibt ihm ausschließlich Delegationswerkzeuge.

        CrewAI erlaubt einem Manager im hierarchischen Prozess keine regulären
        Werkzeuge. Zulässig sind nur die Agent-Tools (delegate_work,
        ask_question); ohne sie könnte der Lead keine Aufgabe abgeben.
        """
        manager = self._build_agent(MANAGER, as_manager=True)
        manager.tools = AgentTools(agents=list(workers.values())).tools()
        return manager

    def _main_crew(
        self, task_names: Sequence[str] | None = None
    ) -> tuple[Crew, list[Task]]:
        names = list(task_names or self.main_task_names)
        workers = self._build_workers(names)
        manager = self._manager_with_delegation(workers)
        tasks = self._build_tasks(names, workers, allow_external_context=True)
        crew = Crew(
            agents=list(workers.values()),
            tasks=tasks,
            process=Process.hierarchical,
            manager_agent=manager,
            memory=self.crew_memory,
            verbose=self.verbose,
            name=f"documentation-main-{self.config.slug}",
            tool_failure_policy=self.tool_failure_policy,
        )
        return crew, tasks

    def _revision_crew(self, iteration: int | None = None) -> tuple[Crew, list[Task]]:
        workers = self._build_workers(self.revision_task_names)
        manager = self._manager_with_delegation(workers)
        tasks = self._build_tasks(
            self.revision_task_names, workers, iteration=iteration
        )
        crew = Crew(
            agents=list(workers.values()),
            tasks=tasks,
            process=Process.hierarchical,
            manager_agent=manager,
            memory=self.crew_memory,
            verbose=self.verbose,
            name=f"documentation-revision-{self.config.slug}",
            tool_failure_policy=self.tool_failure_policy,
        )
        return crew, tasks

    def _final_crew(self) -> tuple[Crew, list[Task]]:
        lead = self._build_agent(
            MANAGER,
            as_manager=False,
            force_no_delegation=True,
            memory_scope="lead-final",
        )
        tasks = self._build_tasks(["executive_summary_task"], {MANAGER: lead})
        crew = Crew(
            agents=[lead],
            tasks=tasks,
            process=Process.sequential,
            memory=self.crew_memory,
            verbose=self.verbose,
            name=f"documentation-final-{self.config.slug}",
            tool_failure_policy=self.tool_failure_policy,
        )
        return crew, tasks

    # ------------------------------------------------------------------ run

    def _inputs(self, extra: Mapping[str, Any] | None = None) -> dict[str, Any]:
        values = self.config.base_inputs()
        if extra:
            values.update(extra)
        return values

    def validate_prompts(self, inputs: Mapping[str, Any] | None = None) -> None:
        values = dict(inputs or self._inputs())
        missing_report: list[str] = []
        for name, task in self.tasks_config.items():
            text = f"{task.get('description', '')}\n{task.get('expected_output', '')}"
            unknown = sorted(
                {
                    match.group(1)
                    for match in PLACEHOLDER.finditer(text)
                    if match.group(1) not in values
                }
            )
            if unknown:
                missing_report.append(f"{name}: {', '.join(unknown)}")
        if missing_report:
            raise ConfigError(
                "Unbekannte Prompt-Platzhalter: " + "; ".join(missing_report)
            )

    def dry_run_report(self) -> DryRunReport:
        self.validate_prompts()
        # Crew-Aufbau prüft Manager-Regeln, Werkzeugzuordnung und Task-Kontexte.
        self._main_crew()
        self._revision_crew()
        self._final_crew()
        return DryRunReport(
            project_name=self.config.project_name,
            output_path=self.config.output_path,
            source_files=self.sources.size,
            code_files=self.code.size,
            agent_count=len(self.agents_config),
            main_task_count=len(self.main_task_names),
            revision_task_count=len(self.revision_task_names),
            tool_names=sorted(self._tools),
            memory_enabled=self.memory_enabled,
            model=self.model,
        )

    def _archive_stale_qa(self) -> None:
        latest = self.scorer.latest_path
        if not latest.is_file():
            return
        target_dir = self.workspace.root / "_qa" / "previous"
        target_dir.mkdir(parents=True, exist_ok=True)
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        shutil.move(str(latest), target_dir / f"qa-report-{stamp}.json")

    def _report_from_tasks(
        self,
        tasks: Sequence[Task],
        expected_iteration: int,
    ) -> QualityReport | None:
        report = self.scorer.read_latest()
        if report is None:
            for task in tasks:
                if task.name in {"quality_assurance_task", "revision_qa_task"}:
                    raw = getattr(getattr(task, "output", None), "raw", None)
                    if raw:
                        report = self.scorer.write_fallback(raw, expected_iteration)
                    break
        if report is None:
            return None
        if report.iteration != expected_iteration:
            report = replace(report, iteration=expected_iteration)
            self.scorer.write(report)
        archive_report(self.workspace, report)
        return report

    def kickoff(self) -> RunResult:
        """CrewAI-konformer Einstiegspunkt für den vollständigen Lauf."""
        return self.run()

    def run(self, *, resume: bool = False) -> RunResult:
        if self.dry_run:
            raise CrewRunError("run() ist im Dry-Run-Modus nicht verfügbar.")
        self.validate_prompts()
        if resume and not self.workspace.documents():
            raise CrewRunError(
                "Resume nicht möglich: keine Dokumente im Ausgabeverzeichnis."
            )
        self._archive_stale_qa()
        self._run_started_epoch = time.time()
        start = time.monotonic()
        started_at = datetime.now(timezone.utc)
        reports: list[QualityReport] = []

        LOGGER.info("Starte Inhaltsphase für %s", self.config.project_name)
        content_result: Any = None
        if resume:
            LOGGER.info("Resume: Inhaltsphase übersprungen, vorhandene Dokumente werden geprüft.")
        else:
            content_crew, content_tasks = self._main_crew(CONTENT_TASK_NAMES)
            try:
                content_result = content_crew.kickoff(inputs=self._inputs())
            except Exception:  # noqa: BLE001 - Review soll mit vorhandenen Dateien laufen
                LOGGER.exception(
                    "Inhaltsphase fehlgeschlagen; vorhandene Dokumente werden weiterverarbeitet."
                )
                if not self.workspace.documents():
                    raise
            else:
                self._persist_missing_outputs(content_tasks, content_result)

        LOGGER.info("Starte Review-Phase für %s", self.config.project_name)
        review_crew, review_tasks = self._main_crew(REVIEW_TASK_NAMES)
        review_result = review_crew.kickoff(inputs=self._inputs())
        self._persist_missing_outputs(review_tasks, review_result)
        report = self._report_from_tasks(review_tasks, 0)
        if report is None:
            LOGGER.warning(
                "Der QA-Agent hat keinen maschinenlesbaren Score geschrieben. "
                "Der Lauf wird ohne Nachbesserungsloop fortgesetzt."
            )
        else:
            reports.append(report)

        for round_number in range(1, self.config.max_revision_rounds + 1):
            if report is None or report.passed:
                break
            feedback = json.dumps(report.to_dict(), ensure_ascii=False, indent=2)
            LOGGER.info(
                "QA-Score %s unter Schwelle %s; starte Nachbesserungsrunde %s",
                report.score,
                report.threshold,
                round_number,
            )
            revision_crew, revision_tasks = self._revision_crew(iteration=round_number)
            revision_result = revision_crew.kickoff(
                inputs=self._inputs(
                    {
                        "iteration": str(round_number),
                        "qa_feedback": feedback,
                    }
                )
            )
            self._persist_missing_outputs(
                revision_tasks, revision_result, iteration=round_number
            )
            new_report = self._report_from_tasks(revision_tasks, round_number)
            if new_report is None:
                LOGGER.warning("Runde %s lieferte keinen QA-Score.", round_number)
                break
            reports.append(new_report)
            if new_report.score <= report.score and not new_report.passed:
                LOGGER.warning(
                    "Runde %s verbesserte den Score nicht (%s -> %s).",
                    round_number,
                    report.score,
                    new_report.score,
                )
            report = new_report

        latest = reports[-1] if reports else None
        final_inputs = self._inputs(
            {
                "qa_summary": qa_summary(latest),
                "quality_history": quality_history(reports),
            }
        )
        LOGGER.info("Erstelle Abschlussbericht und Freigabeempfehlung")
        final_crew, _final_tasks = self._final_crew()
        final_result = final_crew.kickoff(inputs=final_inputs)
        self._persist_missing_outputs(_final_tasks, final_result)

        duration = time.monotonic() - start
        summary = self._read_final_summary()
        self._write_run_report(
            started_at=started_at,
            duration=duration,
            reports=reports,
            content_result=content_result,
            review_result=review_result,
            final_result=final_result,
        )
        return RunResult(
            approved=bool(latest and latest.passed),
            score=latest.score if latest else None,
            reports=reports,
            output_path=self.config.output_path,
            duration_seconds=duration,
            final_summary=summary,
        )

    def _persist_missing_outputs(
        self,
        tasks: Sequence[Task],
        result: Any,
        *,
        iteration: int | None = None,
    ) -> None:
        """Sichert finale Task-Antworten, wenn der Agent sein Primärartefakt nicht schrieb.

        Hintergrund: Erreicht ein Agent ``max_iter``, liefert er nur noch Text zurück.
        Ohne diese Sicherung fehlt der Datei nachfolgenden Aufgaben und dem Bericht.
        """
        outputs = getattr(result, "tasks_output", None) or []
        for task, output in zip(tasks, outputs):
            target = TASK_FALLBACKS.get(task.name)
            if target and iteration is not None and "{iteration}" in target:
                target = target.format(iteration=iteration)
            raw = getattr(output, "raw", "") or ""
            if raw.strip():
                self._save_task_output(
                    task.name, raw, target, overwrite=iteration is not None
                )

    def _read_final_summary(self) -> str:
        path = self.workspace.root / "EXECUTIVE-SUMMARY.md"
        if path.is_file():
            return path.read_text(encoding="utf-8", errors="replace")
        return "Der Documentation Lead hat keinen Abschlussbericht geschrieben."

    def _write_run_report(
        self,
        *,
        started_at: datetime,
        duration: float,
        reports: list[QualityReport],
        content_result: Any,
        review_result: Any,
        final_result: Any,
    ) -> None:
        payload = {
            "schema_version": 1,
            "project": self.config.project_name,
            "started_at": started_at.isoformat(),
            "finished_at": datetime.now(timezone.utc).isoformat(),
            "duration_seconds": round(duration, 2),
            "model": self.model,
            "manager_model": self.manager_model,
            "quality_threshold": self.config.quality_threshold,
            "max_revision_rounds": self.config.max_revision_rounds,
            "approved": bool(reports and reports[-1].passed),
            "final_score": reports[-1].score if reports else None,
            "iterations": [report.to_dict() for report in reports],
            "documents": self.workspace.documents(),
            "token_usage": {
                "content": _usage(content_result),
                "review": _usage(review_result),
                "final": _usage(final_result),
            },
        }
        target = self.workspace.resolve("_qa/run.json", {".json"})
        target.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )


def _usage(result: Any) -> dict[str, Any] | None:
    usage = getattr(result, "token_usage", None)
    if usage is None:
        return None
    if hasattr(usage, "model_dump"):
        try:
            return usage.model_dump()
        except Exception:  # noqa: BLE001 - Telemetrie darf den Lauf nicht stören
            return None
    if isinstance(usage, dict):
        return usage
    return None
