"""CrewAI-Werkzeuge für die Repository-Analyse."""

from __future__ import annotations

from pydantic import BaseModel, Field

from crewai.tools import BaseTool

from .. import repo


class RepoListInput(BaseModel):
    glob: str = Field(default="**/*", description="Glob relativ zur Repo-Wurzel")
    max_results: int = Field(default=400, ge=1, le=2000)


class RepoListTool(BaseTool):
    name: str = "repo_list"
    description: str = (
        "Listet Textdateien im BPMNinja-Repository. Build-Artefakte, node_modules "
        "und target werden automatisch übersprungen. Nutze enge Globs wie "
        "'engine-core/src/**/*.rs' oder 'docs/**/*.md'."
    )
    args_schema: type[BaseModel] = RepoListInput

    def _run(self, glob: str = "**/*", max_results: int = 400) -> str:
        return repo.list_files(glob=glob, max_results=max_results)


class RepoReadInput(BaseModel):
    path: str = Field(description="Pfad relativ zur Repo-Wurzel")
    start_line: int = Field(default=1, ge=1)
    max_lines: int = Field(default=400, ge=1, le=2000)


class RepoReadTool(BaseTool):
    name: str = "repo_read"
    description: str = (
        "Liest eine Textdatei mit Zeilennummern. Nutze start_line/max_lines für "
        "große Dateien. Maximal 2000 Zeilen pro Aufruf."
    )
    args_schema: type[BaseModel] = RepoReadInput

    def _run(self, path: str, start_line: int = 1, max_lines: int = 400) -> str:
        return repo.read_file(path=path, start_line=start_line, max_lines=max_lines)


class RepoSearchInput(BaseModel):
    pattern: str = Field(description="Regex, z. B. 'unwrap\\(|expect\\('")
    glob: str = Field(default="**/*.rs", description="Datei-Glob")
    max_results: int = Field(default=80, ge=1, le=400)
    ignore_case: bool = Field(default=False)
    context: int = Field(default=0, ge=0, le=5)


class RepoSearchTool(BaseTool):
    name: str = "repo_search"
    description: str = (
        "Regex-Suche über Textdateien. Liefert path:line: text. Ideal für "
        "Bestandsaufnahmen von unwrap, panic!, unsafe, TODO, allow-Attributen "
        "oder API-Routen."
    )
    args_schema: type[BaseModel] = RepoSearchInput

    def _run(
        self,
        pattern: str,
        glob: str = "**/*.rs",
        max_results: int = 80,
        ignore_case: bool = False,
        context: int = 0,
    ) -> str:
        return repo.search(
            pattern=pattern,
            glob=glob,
            max_results=max_results,
            ignore_case=ignore_case,
            context=context,
        )


class GitLogInput(BaseModel):
    limit: int = Field(default=20, ge=1, le=100)
    path: str = Field(default="", description="Optionaler Pfadfilter")


class GitLogTool(BaseTool):
    name: str = "git_log"
    description: str = (
        "Zeigt die letzten Commits und den Status des Arbeitsbaums. Damit "
        "erkennst du bereits erledigte Arbeiten und unfertige Änderungen."
    )
    args_schema: type[BaseModel] = GitLogInput

    def _run(self, limit: int = 20, path: str = "") -> str:
        return repo.git_log(limit=limit, path=path)


class CargoMetadataInput(BaseModel):
    pass


class CargoMetadataTool(BaseTool):
    name: str = "cargo_metadata"
    description: str = (
        "Zeigt Workspace-Crates und ihre Abhängigkeiten (inklusive Pfad- und "
        "Dev-Abhängigkeiten) aus cargo metadata. Nutze das für Architektur- und "
        "Dependency-Analysen."
    )
    args_schema: type[BaseModel] = CargoMetadataInput

    def _run(self) -> str:
        return repo.cargo_metadata()


class RepoStatsInput(BaseModel):
    pass


class RepoStatsTool(BaseTool):
    name: str = "repo_stats"
    description: str = (
        "Liefert Kennzahlen: Rust-Dateien/-Zeilen, Anzahl Tests, TODO-Marker und "
        "Doku-Umfang. Guter Einstieg für einen schnellen Projektüberblick."
    )
    args_schema: type[BaseModel] = RepoStatsInput

    def _run(self) -> str:
        return repo.repo_stats()
