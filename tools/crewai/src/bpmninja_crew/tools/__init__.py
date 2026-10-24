"""Tool-Fabriken für Analyse- und Umsetzungs-Crew."""

from .impl_tools import (
    RepoDiffTool,
    RepoReplaceTool,
    RepoRunTool,
    RepoWriteTool,
)
from .repo_tools import (
    CargoMetadataTool,
    GitLogTool,
    RepoListTool,
    RepoReadTool,
    RepoSearchTool,
    RepoStatsTool,
)
from .web_tools import FetchUrlTool, GithubSearchTool, WebSearchTool


def analysis_tools() -> list:
    return [
        RepoListTool(),
        RepoReadTool(),
        RepoSearchTool(),
        GitLogTool(),
        CargoMetadataTool(),
        RepoStatsTool(),
    ]


def research_tools() -> list:
    return [WebSearchTool(), FetchUrlTool(), GithubSearchTool()]


def implementation_tools() -> list:
    return [
        RepoListTool(),
        RepoReadTool(),
        RepoSearchTool(),
        RepoReplaceTool(),
        RepoWriteTool(),
        RepoRunTool(),
        RepoDiffTool(),
    ]


def verification_tools() -> list:
    return [
        RepoReadTool(),
        RepoSearchTool(),
        RepoRunTool(),
        RepoDiffTool(),
        RepoListTool(),
    ]


__all__ = [
    "CargoMetadataTool",
    "FetchUrlTool",
    "GithubSearchTool",
    "GitLogTool",
    "RepoDiffTool",
    "RepoListTool",
    "RepoReadTool",
    "RepoReplaceTool",
    "RepoRunTool",
    "RepoSearchTool",
    "RepoStatsTool",
    "RepoWriteTool",
    "WebSearchTool",
    "analysis_tools",
    "implementation_tools",
    "research_tools",
    "verification_tools",
]
