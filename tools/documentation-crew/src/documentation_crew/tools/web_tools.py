"""Websuche und URL-Abruf als CrewAI-Werkzeuge."""

from __future__ import annotations

import os

from crewai.tools import BaseTool
from pydantic import BaseModel, Field

from .. import web


class WebSearchInput(BaseModel):
    query: str = Field(description="Konkrete Suchanfrage")
    max_results: int = Field(default=6, ge=1, le=10)


class FetchUrlInput(BaseModel):
    url: str = Field(description="Öffentliche http(s)-URL")
    max_chars: int = Field(default=12_000, ge=1_000, le=40_000)


class WebSearchTool(BaseTool):
    name: str = "web_search"
    description: str = (
        "Sucht im Web (Bing-RSS, DuckDuckGo-Fallback) und liefert Titel, URL und "
        "Snippet. Für Standards, Best Practices, Vergleichsprojekte und "
        "Domänenwissen. Quellen immer mit URL und Zugriffsdatum zitieren."
    )
    args_schema: type[BaseModel] = WebSearchInput

    def _run(self, query: str, max_results: int = 6) -> str:
        try:
            results = web.web_search(query, max_results=max_results)
        except web.WebError as exc:
            return f"FEHLER: {exc}"
        lines = [f"# Suche: {query}"]
        for item in results:
            lines.append(
                f"- {item['title']}\n  {item['url']}\n  {item['snippet']}"
            )
            if item.get("published"):
                lines.append(f"  Veröffentlicht: {item['published']}")
        return "\n".join(lines)


class FetchUrlTool(BaseTool):
    name: str = "fetch_url"
    description: str = (
        "Lädt eine öffentliche Webseite und gibt den lesbaren Text zurück. "
        "Private und lokale Adressen sind gesperrt; Redirects werden geprüft. "
        "Nutze das Werkzeug, um Primärquellen tatsächlich zu lesen."
    )
    args_schema: type[BaseModel] = FetchUrlInput

    def _run(self, url: str, max_chars: int = 12_000) -> str:
        try:
            return web.fetch_url(url, max_chars=max_chars)
        except Exception as exc:  # noqa: BLE001 - Toolfehler als Text
            return f"FEHLER: {type(exc).__name__}: {exc}"


def build_web_search_tool() -> BaseTool:
    """Serper bei vorhandenem Key, sonst keyless DuckDuckGo/Bing."""
    if os.getenv("SERPER_API_KEY"):
        try:
            from crewai_tools import SerperDevTool

            tool = SerperDevTool(n_results=8)
            tool.name = "web_search"
            tool.description = (
                "Sucht im Web über Serper und liefert Titel, URL und Snippet. "
                "Quellen immer mit URL und Zugriffsdatum zitieren."
            )
            return tool
        except Exception:  # noqa: BLE001 - Fallback ist wichtiger als der Anbieter
            pass
    return WebSearchTool()
