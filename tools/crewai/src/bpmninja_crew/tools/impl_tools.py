"""Schreib- und Prüfwerkzeuge für die Umsetzungs-Crew.

Die Werkzeuge sind absichtlich eng: keine freie Shell, keine Commits, keine
Änderungen an Nutzer-Notizen oder Secrets. Jede Änderung bleibt im
Arbeitsbaum und wird anschließend vom Verifier geprüft.
"""

from __future__ import annotations

import os
import shlex
import subprocess
from pathlib import Path

from pydantic import BaseModel, Field

from crewai.tools import BaseTool

from .. import repo

BLOCKED_PARTS = {
    ".git",
    ".venv",
    "venv",
    "node_modules",
    "target",
    "gen",
    "dist",
    "build",
    ".playwright",
    "playwright-report",
    "test-results",
    "graphify-out",
    "mutants.out",
}
BLOCKED_FILES = {
    ".env",
    ".env.local",
    "HANDOFF.md",
    "improve1026.md",
    "plan.md",
    "Cargo.lock",
    "TASK_MEMORY.md",
}
SHELL_METACHARACTERS = set(";&|><`$\n")


def _writable_target(path: str, root: Path | None = None) -> Path:
    root = (root or repo.get_repo_root()).resolve()
    target = (root / path).resolve()
    try:
        relative = target.relative_to(root)
    except ValueError as exc:
        raise ValueError(f"Pfad liegt außerhalb des Repos: {path}") from exc
    if any(part in BLOCKED_PARTS for part in relative.parts):
        raise ValueError(f"Pfad ist für die Crew gesperrt: {relative}")
    if target.name in BLOCKED_FILES:
        raise ValueError(f"Datei ist für die Crew gesperrt: {relative}")
    if target.suffix.lower() not in repo.TEXT_EXTENSIONS and target.name not in repo.TEXT_NAMES:
        raise ValueError(f"Nur Textdateien sind erlaubt: {relative}")
    if not target.parent.is_dir():
        raise ValueError(f"Elternverzeichnis fehlt: {relative.parent}")
    return target


def _command_allowed(args: list[str]) -> bool:
    if not args:
        return False
    if args[0] == "cargo" and len(args) >= 2:
        return args[1] in {"fmt", "check", "test", "clippy", "build", "metadata"}
    if args[0] == "git" and len(args) >= 2:
        return args[1] in {"status", "diff", "log", "show"}
    if args[0] == "npm" and len(args) >= 3 and args[1] == "run":
        return args[2] in {"build", "lint", "test", "test:deps", "test:e2e"}
    if args[0] in {"npx", "./node_modules/.bin/tsc"}:
        return "tsc" in args or args[0].endswith("tsc")
    if args[0] in {"pytest", "python"}:
        return args[0] == "pytest" or args[1:3] == ["-m", "pytest"]
    return False


def replace_text(
    path: str,
    old_string: str,
    new_string: str,
    replace_all: bool = False,
    root: Path | None = None,
) -> str:
    """Exakten Text ersetzen; Anzahl der Treffer wird geprüft."""
    target = _writable_target(path, root)
    if not target.is_file():
        raise ValueError(f"Datei nicht gefunden: {path}")
    content = target.read_text(encoding="utf-8")
    occurrences = content.count(old_string)
    if occurrences == 0:
        raise ValueError("old_string kommt in der Datei nicht vor.")
    if occurrences > 1 and not replace_all:
        raise ValueError(
            f"old_string kommt {occurrences}× vor; ergänze Kontext oder setze replace_all."
        )
    updated = content.replace(old_string, new_string) if replace_all else content.replace(
        old_string, new_string, 1
    )
    if updated == content:
        raise ValueError("Ersetzung ändert nichts.")
    target.write_text(updated, encoding="utf-8")
    return f"Ersetzt in {path}: {occurrences if replace_all else 1} Treffer."


def write_new_file(
    path: str,
    content: str,
    overwrite: bool = False,
    root: Path | None = None,
) -> str:
    """Neue Textdatei anlegen; Überschreiben nur explizit."""
    target = _writable_target(path, root)
    if target.exists() and not overwrite:
        raise ValueError("Datei existiert; nutze repo_replace oder overwrite=true.")
    if len(content) > 250_000:
        raise ValueError("Inhalt zu groß (max. 250k Zeichen).")
    existed = target.exists()
    target.write_text(content, encoding="utf-8")
    action = "Überschrieben" if existed else "Angelegt"
    return f"{action}: {path} ({len(content)} Zeichen)."


def run_command(
    command: str,
    cwd: str = ".",
    timeout: int = 900,
    root: Path | None = None,
) -> str:
    """Allowlist-Kommando ohne Shell ausführen."""
    root = (root or repo.get_repo_root()).resolve()
    if any(char in command for char in SHELL_METACHARACTERS):
        raise ValueError("Shell-Metazeichen sind nicht erlaubt.")
    args = shlex.split(command)
    if not _command_allowed(args):
        raise ValueError(
            "Kommando nicht erlaubt. Erlaubt sind cargo fmt/check/test/clippy/build, "
            "git status/diff/log/show, npm run build/lint/test, tsc und pytest."
        )
    workdir = (root / cwd).resolve()
    try:
        workdir.relative_to(root)
    except ValueError as exc:
        raise ValueError("cwd liegt außerhalb des Repos.") from exc
    if not workdir.is_dir():
        raise ValueError(f"cwd fehlt: {cwd}")
    timeout = max(30, min(timeout, 1800))
    env = os.environ.copy()
    env.update({"CI": "1", "CARGO_TERM_COLOR": "never", "NO_COLOR": "1"})
    try:
        result = subprocess.run(
            args,
            cwd=workdir,
            capture_output=True,
            text=True,
            timeout=timeout,
            env=env,
        )
    except FileNotFoundError as exc:
        raise ValueError(f"Kommando nicht gefunden: {args[0]}") from exc
    output = (result.stdout + "\n" + result.stderr).strip()
    tail = output[-14_000:] if len(output) > 14_000 else output
    return f"$ {command}\nexit={result.returncode}\n{tail}"


def git_diff(path: str = "", root: Path | None = None) -> str:
    """Arbeitsbaum-Diff anzeigen."""
    root = (root or repo.get_repo_root()).resolve()
    status = subprocess.run(
        ["git", "-C", str(root), "status", "--short"],
        capture_output=True,
        text=True,
        timeout=30,
    )
    diff_command = ["git", "-C", str(root), "diff", "--stat"]
    if path:
        diff_command.extend(["--", path])
    diff = subprocess.run(diff_command, capture_output=True, text=True, timeout=60)
    return (
        "## Status\n"
        + (status.stdout.strip() or "clean")
        + "\n## Diffstat\n"
        + (diff.stdout.strip() or "keine Änderungen")
    )


class RepoReplaceInput(BaseModel):
    path: str = Field(description="Bestehende Textdatei relativ zur Repo-Wurzel")
    old_string: str = Field(description="Exakter vorhandener Text inklusive Einrückung")
    new_string: str = Field(description="Ersatztext")
    replace_all: bool = Field(default=False)


class RepoReplaceTool(BaseTool):
    name: str = "repo_replace"
    description: str = (
        "Ersetzt exakten Text in einer bestehenden Datei. Bevorzugtes Werkzeug "
        "für chirurgische Änderungen. old_string muss eindeutig sein, sonst "
        "mehr Kontext mitgeben oder replace_all setzen."
    )
    args_schema: type[BaseModel] = RepoReplaceInput

    def _run(
        self, path: str, old_string: str, new_string: str, replace_all: bool = False
    ) -> str:
        return replace_text(path, old_string, new_string, replace_all)


class RepoWriteInput(BaseModel):
    path: str = Field(description="Neue Datei relativ zur Repo-Wurzel")
    content: str
    overwrite: bool = Field(default=False)


class RepoWriteTool(BaseTool):
    name: str = "repo_write"
    description: str = (
        "Legt eine neue Textdatei an. Für bestehende Dateien repo_replace nutzen. "
        "Überschreiben nur mit overwrite=true und triftigem Grund."
    )
    args_schema: type[BaseModel] = RepoWriteInput

    def _run(self, path: str, content: str, overwrite: bool = False) -> str:
        return write_new_file(path, content, overwrite)


class RepoRunInput(BaseModel):
    command: str = Field(description="Allowlist-Kommando, z. B. 'cargo test -p engine-core'")
    cwd: str = Field(default=".", description="Arbeitsverzeichnis relativ zur Repo-Wurzel")
    timeout: int = Field(default=900, ge=30, le=1800)


class RepoRunTool(BaseTool):
    name: str = "repo_run"
    description: str = (
        "Führt ein erlaubtes Prüfkommando aus (cargo fmt/check/test/clippy/build, "
        "git status/diff/log/show, npm run build/lint/test, tsc, pytest). Keine "
        "Shell, keine Commits, kein Push."
    )
    args_schema: type[BaseModel] = RepoRunInput

    def _run(self, command: str, cwd: str = ".", timeout: int = 900) -> str:
        return run_command(command, cwd, timeout)


class RepoDiffInput(BaseModel):
    path: str = Field(default="", description="Optionaler Pfadfilter")


class RepoDiffTool(BaseTool):
    name: str = "repo_diff"
    description: str = (
        "Zeigt git status und Diffstat des Arbeitsbaums. Nutze es vor und nach "
        "Änderungen, um Scope Creep zu erkennen."
    )
    args_schema: type[BaseModel] = RepoDiffInput

    def _run(self, path: str = "") -> str:
        return git_diff(path)
