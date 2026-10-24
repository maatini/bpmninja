"""CrewAI-Definitionen für Analyse und bestätigte Umsetzung."""

from __future__ import annotations

import os
from pathlib import Path

from crewai import Agent, Crew, LLM, Process, Task
from crewai.project import CrewBase, agent, crew, task

from .models import Recommendation
from .tools import (
    analysis_tools,
    implementation_tools,
    research_tools,
    verification_tools,
)

CONFIG_DIR = Path(__file__).resolve().parent / "config"

DEFAULT_SCAN_MODEL = "deepseek/deepseek-flash"
DEFAULT_SYNTH_MODEL = "deepseek/deepseek-v4-pro"
DEFAULT_SCAN_EFFORT = "high"
DEFAULT_SYNTH_EFFORT = "high"


def build_llm(
    model: str,
    effort: str,
    temperature: float = 0.2,
    max_tokens: int = 8192,
    response_format: dict | None = None,
) -> LLM:
    """DeepSeek über den OpenAI-kompatiblen CrewAI-Provider konfigurieren.

    DeepSeek unterstützt nur ``json_object``, kein ``json_schema``. Für die
    Synthese wird deshalb der JSON-Modus gesetzt und lokal validiert.
    """
    return LLM(
        model=model,
        reasoning_effort=effort,
        temperature=temperature,
        max_tokens=max_tokens,
        timeout=1200,
        response_format=response_format,
    )


def model_settings() -> tuple[str, str, str, str]:
    return (
        os.getenv("BPMNINJA_CREW_MODEL_SCAN", DEFAULT_SCAN_MODEL),
        os.getenv("BPMNINJA_CREW_MODEL_SYNTH", DEFAULT_SYNTH_MODEL),
        os.getenv("BPMNINJA_CREW_EFFORT_SCAN", DEFAULT_SCAN_EFFORT),
        os.getenv("BPMNINJA_CREW_EFFORT_SYNTH", DEFAULT_SYNTH_EFFORT),
    )


@CrewBase
class AnalysisCrew:
    """Vier Fachagenten plus Priorisierer; rein lesend."""

    agents_config = str(CONFIG_DIR / "agents.yaml")
    tasks_config = str(CONFIG_DIR / "tasks.yaml")

    def __init__(self, run_dir: Path, verbose: bool = True) -> None:
        self.run_dir = Path(run_dir)
        self.run_dir.mkdir(parents=True, exist_ok=True)
        scan_model, synth_model, scan_effort, synth_effort = model_settings()
        self.scan_llm = build_llm(scan_model, scan_effort)
        self.synth_llm = build_llm(
            synth_model,
            synth_effort,
            temperature=0.1,
            max_tokens=16384,
            response_format={"type": "json_object"},
        )
        self.verbose = verbose
        self._analysis = analysis_tools()
        self._research = research_tools()

    @agent
    def repo_analyst(self) -> Agent:
        return Agent(
            config=self.agents_config["repo_analyst"],
            llm=self.scan_llm,
            tools=self._analysis,
            verbose=self.verbose,
            max_iter=18,
            allow_delegation=False,
        )

    @agent
    def domain_researcher(self) -> Agent:
        return Agent(
            config=self.agents_config["domain_researcher"],
            llm=self.scan_llm,
            tools=self._analysis + self._research,
            verbose=self.verbose,
            max_iter=24,
            allow_delegation=False,
        )

    @agent
    def product_reviewer(self) -> Agent:
        return Agent(
            config=self.agents_config["product_reviewer"],
            llm=self.scan_llm,
            tools=self._analysis + self._research,
            verbose=self.verbose,
            max_iter=24,
            allow_delegation=False,
        )

    @agent
    def quality_auditor(self) -> Agent:
        return Agent(
            config=self.agents_config["quality_auditor"],
            llm=self.scan_llm,
            tools=self._analysis + self._research,
            verbose=self.verbose,
            max_iter=24,
            allow_delegation=False,
        )

    @agent
    def prioritizer(self) -> Agent:
        return Agent(
            config=self.agents_config["prioritizer"],
            llm=self.synth_llm,
            verbose=self.verbose,
            max_iter=6,
            allow_delegation=False,
        )

    @task
    def repo_analysis_task(self) -> Task:
        return Task(
            config=self.tasks_config["repo_analysis_task"],
            agent=self.repo_analyst(),
            output_file=str(self.run_dir / "01_repo_analyse.md"),
        )

    @task
    def domain_research_task(self) -> Task:
        return Task(
            config=self.tasks_config["domain_research_task"],
            agent=self.domain_researcher(),
            context=[self.repo_analysis_task()],
            output_file=str(self.run_dir / "02_domaenen_recherche.md"),
        )

    @task
    def product_review_task(self) -> Task:
        return Task(
            config=self.tasks_config["product_review_task"],
            agent=self.product_reviewer(),
            context=[self.repo_analysis_task()],
            output_file=str(self.run_dir / "03_produkt_dx.md"),
        )

    @task
    def quality_audit_task(self) -> Task:
        return Task(
            config=self.tasks_config["quality_audit_task"],
            agent=self.quality_auditor(),
            context=[self.repo_analysis_task()],
            output_file=str(self.run_dir / "04_qualitaet_security.md"),
        )

    @task
    def synthesis_task(self) -> Task:
        return Task(
            config=self.tasks_config["synthesis_task"],
            agent=self.prioritizer(),
            context=[
                self.repo_analysis_task(),
                self.domain_research_task(),
                self.product_review_task(),
                self.quality_audit_task(),
            ],
            output_file=str(self.run_dir / "05_empfehlungen.md"),
        )

    @crew
    def crew(self) -> Crew:
        return Crew(
            agents=[
                self.repo_analyst(),
                self.domain_researcher(),
                self.product_reviewer(),
                self.quality_auditor(),
                self.prioritizer(),
            ],
            tasks=[
                self.repo_analysis_task(),
                self.domain_research_task(),
                self.product_review_task(),
                self.quality_audit_task(),
                self.synthesis_task(),
            ],
            process=Process.sequential,
            verbose=self.verbose,
            memory=False,
        )


def recommendation_brief(recommendations: list[Recommendation]) -> str:
    blocks: list[str] = []
    for rec in recommendations:
        evidence = "\n".join(
            f"  - {item.path}:{item.lines} {item.note}" for item in rec.evidence
        )
        criteria = "\n".join(f"  - {item}" for item in rec.acceptance_criteria)
        blocks.append(
            f"### {rec.id} — {rec.title} (Score {rec.score}, Aufwand {rec.effort})\n"
            f"Kategorie: {rec.category}\n"
            f"Zusammenfassung: {rec.summary}\n"
            f"Vorgeschlagene Änderung: {rec.proposed_change}\n"
            f"Evidenz:\n{evidence or '  - keine'}\n"
            f"Abnahmekriterien:\n{criteria or '  - keine'}\n"
            f"Risiken: {rec.risks}"
        )
    return "\n\n".join(blocks)


@CrewBase
class ImplementationCrew:
    """Umsetzung, Verifikation und Review für bestätigte Empfehlungen."""

    agents_config = str(CONFIG_DIR / "agents.yaml")
    tasks_config = str(CONFIG_DIR / "tasks.yaml")

    def __init__(
        self,
        run_dir: Path,
        recommendations: list[Recommendation],
        verbose: bool = True,
    ) -> None:
        self.run_dir = Path(run_dir)
        self.run_dir.mkdir(parents=True, exist_ok=True)
        self.recommendations = recommendations
        self.brief = recommendation_brief(recommendations)
        scan_model, _, scan_effort, _ = model_settings()
        self.llm = build_llm(scan_model, scan_effort)
        self.verbose = verbose

    @agent
    def rust_implementer(self) -> Agent:
        return Agent(
            config=self.agents_config["rust_implementer"],
            llm=self.llm,
            tools=implementation_tools(),
            verbose=self.verbose,
            max_iter=30,
            allow_delegation=False,
        )

    @agent
    def verifier(self) -> Agent:
        return Agent(
            config=self.agents_config["verifier"],
            llm=self.llm,
            tools=verification_tools(),
            verbose=self.verbose,
            max_iter=16,
            allow_delegation=False,
        )

    @agent
    def reviewer(self) -> Agent:
        return Agent(
            config=self.agents_config["reviewer"],
            llm=self.llm,
            tools=verification_tools(),
            verbose=self.verbose,
            max_iter=10,
            allow_delegation=False,
        )

    @task
    def implement_task(self) -> Task:
        return Task(
            config=self.tasks_config["implement_task"],
            agent=self.rust_implementer(),
            output_file=str(self.run_dir / "10_umsetzung.md"),
        )

    @task
    def verify_task(self) -> Task:
        return Task(
            config=self.tasks_config["verify_task"],
            agent=self.verifier(),
            context=[self.implement_task()],
            output_file=str(self.run_dir / "11_verifikation.md"),
        )

    @task
    def review_task(self) -> Task:
        return Task(
            config=self.tasks_config["review_task"],
            agent=self.reviewer(),
            context=[self.implement_task(), self.verify_task()],
            output_file=str(self.run_dir / "12_review.md"),
        )

    @crew
    def crew(self) -> Crew:
        return Crew(
            agents=[
                self.rust_implementer(),
                self.verifier(),
                self.reviewer(),
            ],
            tasks=[
                self.implement_task(),
                self.verify_task(),
                self.review_task(),
            ],
            process=Process.sequential,
            verbose=self.verbose,
            memory=False,
        )
