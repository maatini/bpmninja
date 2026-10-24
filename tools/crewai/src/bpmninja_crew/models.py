"""Datenmodelle für den CrewAI-Analyselauf.

Das Rating ist bewusst nachvollziehbar: Jede Empfehlung trägt eine
Punktaufteilung, deren Summe den Gesamtwert (1–100) ergibt.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field, model_validator

Category = Literal[
    "engine",
    "parser",
    "persistence",
    "server",
    "desktop",
    "docs",
    "quality",
    "security",
    "dx",
    "performance",
]
Effort = Literal["S", "M", "L", "XL"]


class ScoreBreakdown(BaseModel):
    """Punktaufteilung des Ratings. Höhere Werte sind besser."""

    impact: int = Field(ge=0, le=30, description="Wirkung auf Nutzer/Betrieb")
    domain_gap: int = Field(ge=0, le=20, description="Relevanz im BPMN-/Workflow-Markt")
    risk_reduction: int = Field(ge=0, le=20, description="Gewinn bei Stabilität/Security")
    effort: int = Field(ge=0, le=15, description="15 = kleiner Aufwand, 0 = sehr groß")
    evidence: int = Field(ge=0, le=15, description="Belegstärke durch Code/Quellen")

    @property
    def total(self) -> int:
        return (
            self.impact
            + self.domain_gap
            + self.risk_reduction
            + self.effort
            + self.evidence
        )


class Evidence(BaseModel):
    path: str = Field(description="Repo-Pfad, z. B. engine-core/src/engine/gateway.rs")
    lines: str = Field(default="", description="Zeilenspanne oder Symbol")
    note: str = Field(default="", description="Was der Fund belegt")


class Recommendation(BaseModel):
    id: str = Field(default="", description="R1, R2, ...")
    title: str
    category: Category
    summary: str
    proposed_change: str
    acceptance_criteria: list[str] = Field(default_factory=list)
    risks: str = ""
    effort: Effort = "M"
    evidence: list[Evidence] = Field(default_factory=list)
    sources: list[str] = Field(default_factory=list)
    score: int = Field(default=0, ge=0, le=100)
    score_breakdown: ScoreBreakdown

    @model_validator(mode="after")
    def _sync_score(self) -> "Recommendation":
        self.score = max(1, min(100, self.score_breakdown.total))
        return self


class RecommendationReport(BaseModel):
    generated_at: str = ""
    focus: str = "all"
    project_state: str = ""
    methodology: str = ""
    recommendations: list[Recommendation] = Field(default_factory=list)
    discarded: list[str] = Field(default_factory=list)

    @model_validator(mode="after")
    def _normalize(self) -> "RecommendationReport":
        self.recommendations.sort(key=lambda rec: rec.score, reverse=True)
        seen: set[str] = set()
        for index, rec in enumerate(self.recommendations, start=1):
            rec.id = f"R{index}"
            seen.add(rec.id)
        if self.discarded and len(self.discarded) > 50:
            self.discarded = self.discarded[:50]
        return self


class ImplementationOutcome(BaseModel):
    recommendation_id: str
    status: Literal["done", "blocked", "failed"]
    files_changed: list[str] = Field(default_factory=list)
    commands: list[str] = Field(default_factory=list)
    notes: str = ""
