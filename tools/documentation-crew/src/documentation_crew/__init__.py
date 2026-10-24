"""CrewAI-Team zur professionellen Überarbeitung technischer Dokumentation."""

from __future__ import annotations

import os

# CrewAI sendet standardmäßig Telemetrie. Für ein Enterprise-Werkzeug ist das
# opt-in, nicht opt-out.
os.environ.setdefault("CREWAI_TELEMETRY_OPT_OUT", "true")

from .config import ConfigError, RunConfig  # noqa: E402
from .crew import DocumentationCrew, DryRunReport, RunResult  # noqa: E402

__all__ = [
    "ConfigError",
    "DocumentationCrew",
    "DryRunReport",
    "RunConfig",
    "RunResult",
]

__version__ = "0.1.0"
