"""CrewAI-Werkzeuge für Domänen- und Wettbewerbsrecherche."""

from __future__ import annotations

from pydantic import BaseModel, Field

from crewai.tools import BaseTool

from .. import web


class WebSearchInput(BaseModel):
    query: str = Field(description="Suchanfrage, möglichst konkret")
    max_results: int = Field(default=6, ge=1, le=10)


class WebSearchTool(BaseTool):
    name: str = "web_search"
    description: str = (
        "Sucht im Web (Bing-RSS, DuckDuckGo-Fallback) und liefert Titel, URL und "
        "Snippet. Für BPMN-Spezifikation, Camunda/Zeebe/Flowable/Temporal, "
        "Sicherheits- und Rust-Praktiken. Immer mit Quellen-URL zitieren."
    )
    args_schema: type[BaseModel] = WebSearchInput

    def _run(self, query: str, max_results: int = 6) -> str:
        results = web.web_search(query, max_results=max_results)
        lines = [f"# Suche: {query}"]
        for item in results:
            lines.append(
                f"- {item['title']}\n  {item['url']}\n  {item['snippet']}"
            )
        return "\n".join(lines)


class FetchUrlInput(BaseModel):
    url: str = Field(description="Öffentliche http(s)-URL")
    max_chars: int = Field(default=12_000, ge=1_000, le=40_000)


class FetchUrlTool(BaseTool):
    name: str = "fetch_url"
    description: str = (
        "Lädt eine öffentliche Webseite und gibt den lesbaren Text zurück. "
        "Nutze sie, um Primärquellen (BPMN-Spezifikation, Herstellerdocs, "
        "Rust-Security-Advisories) tatsächlich zu prüfen."
    )
    args_schema: type[BaseModel] = FetchUrlInput

    def _run(self, url: str, max_chars: int = 12_000) -> str:
        return web.fetch_url(url, max_chars=max_chars)


class GithubSearchInput(BaseModel):
    query: str = Field(description="GitHub-Suchquery")
    kind: str = Field(default="repositories", description="repositories, issues oder code")
    max_results: int = Field(default=5, ge=1, le=10)


class GithubSearchTool(BaseTool):
    name: str = "github_search"
    description: str = (
        "Sucht GitHub-Repositories oder Issues. Gut für Wettbewerbsvergleich, "
        "Referenzimplementierungen und bekannte Fehlerbilder. Code-Suche "
        "benötigt GITHUB_TOKEN."
    )
    args_schema: type[BaseModel] = GithubSearchInput

    def _run(self, query: str, kind: str = "repositories", max_results: int = 5) -> str:
        return web.github_search(query, kind=kind, max_results=max_results)
