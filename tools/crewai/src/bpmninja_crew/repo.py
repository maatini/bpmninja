"""Read-only Zugriff auf das BPMNinja-Repository.

Die Crew bekommt bewusst keine freie Shell. Alle Analysewerkzeuge laufen
über diese Funktionen, damit Pfade, Dateigrößen und Laufzeiten begrenzt
bleiben.
"""

from __future__ import annotations

import fnmatch
import json
import os
import re
import subprocess
from pathlib import Path

TEXT_EXTENSIONS = {
    ".rs",
    ".toml",
    ".md",
    ".yaml",
    ".yml",
    ".json",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".css",
    ".html",
    ".sql",
    ".sh",
    ".py",
    ".tsp",
    ".xml",
    ".bpmn",
    ".txt",
    ".conf",
    ".ini",
    ".c",
    ".h",
    ".graphql",
}

TEXT_NAMES = {
    "Dockerfile",
    "Makefile",
    "LICENSE",
    "README",
    "CHANGELOG",
    ".gitignore",
    ".dockerignore",
    ".env.example",
    "rust-toolchain",
}

IGNORED_DIRS = {
    ".git",
    ".venv",
    "venv",
    "node_modules",
    "target",
    "dist",
    "build",
    "gen",
    ".playwright",
    "playwright-report",
    "test-results",
    "graphify-out",
    "mutants.out",
    "tarpaulin-report",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".cargo",
    ".idea",
    ".vscode",
    "data",
    "nats-data",
}

MAX_TEXT_BYTES = 2_000_000


def get_repo_root() -> Path:
    """Repo-Wurzel aus Umgebung oder Dateipfad bestimmen."""
    override = os.getenv("BPMNINJA_ROOT")
    if override:
        return Path(override).expanduser().resolve()
    return Path(__file__).resolve().parents[4]


def _resolve(root: Path, relative: str) -> Path:
    candidate = (root / relative).resolve()
    try:
        candidate.relative_to(root)
    except ValueError as exc:
        raise ValueError(f"Pfad liegt außerhalb des Repos: {relative}") from exc
    return candidate


def is_text_file(path: Path) -> bool:
    return path.suffix.lower() in TEXT_EXTENSIONS or path.name in TEXT_NAMES


def _walk_text_files(root: Path):
    for current, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in IGNORED_DIRS and not d.startswith(".git")]
        for name in files:
            path = Path(current) / name
            if is_text_file(path):
                yield path


def list_files(
    glob: str = "**/*",
    max_results: int = 400,
    root: Path | None = None,
) -> str:
    """Dateien auflisten, ohne Build-Artefakte und Abhängigkeiten."""
    root = (root or get_repo_root()).resolve()
    matches: list[str] = []
    for path in _walk_text_files(root):
        relative = path.relative_to(root).as_posix()
        if fnmatch.fnmatch(relative, glob) or fnmatch.fnmatch(path.name, glob):
            matches.append(relative)
            if len(matches) >= max_results:
                break
    matches.sort()
    suffix = f"\n… gekürzt bei {max_results} Treffern" if len(matches) >= max_results else ""
    return "\n".join(matches) + suffix if matches else "Keine Treffer."


def read_file(
    path: str,
    start_line: int = 1,
    max_lines: int = 400,
    max_chars: int = 30_000,
    root: Path | None = None,
) -> str:
    """Textdatei mit Zeilennummern lesen."""
    root = (root or get_repo_root()).resolve()
    target = _resolve(root, path)
    if not target.is_file():
        raise ValueError(f"Datei nicht gefunden: {path}")
    if not is_text_file(target):
        raise ValueError(f"Kein Textformat: {path}")
    if target.stat().st_size > MAX_TEXT_BYTES:
        raise ValueError(f"Datei zu groß für die Analyse: {path}")
    start_line = max(1, start_line)
    max_lines = max(1, min(max_lines, 2000))
    lines = target.read_text(encoding="utf-8", errors="replace").splitlines()
    chunk = lines[start_line - 1 : start_line - 1 + max_lines]
    numbered = [
        f"{start_line + offset:>5}→{line}" for offset, line in enumerate(chunk)
    ]
    body = "\n".join(numbered)
    if len(body) > max_chars:
        body = body[:max_chars] + "\n… gekürzt"
    return f"# {path} (Zeilen {start_line}–{start_line + len(chunk) - 1} von {len(lines)})\n{body}"


def search(
    pattern: str,
    glob: str = "**/*",
    max_results: int = 80,
    ignore_case: bool = False,
    context: int = 0,
    root: Path | None = None,
) -> str:
    """Regex-Suche über Textdateien, Ergebnis als path:line: text."""
    root = (root or get_repo_root()).resolve()
    flags = re.IGNORECASE if ignore_case else 0
    try:
        regex = re.compile(pattern, flags)
    except re.error as exc:
        raise ValueError(f"Ungültiger Regex: {exc}") from exc
    hits: list[str] = []
    for path in _walk_text_files(root):
        relative = path.relative_to(root).as_posix()
        if not (fnmatch.fnmatch(relative, glob) or fnmatch.fnmatch(path.name, glob)):
            continue
        if path.stat().st_size > MAX_TEXT_BYTES:
            continue
        try:
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        for number, line in enumerate(lines, start=1):
            if regex.search(line):
                hits.append(f"{relative}:{number}: {line.strip()[:300]}")
                for offset in range(1, context + 1):
                    if number - 1 + offset < len(lines):
                        extra = lines[number - 1 + offset].strip()[:300]
                        hits.append(f"{relative}:{number + offset}: {extra}")
                if len(hits) >= max_results:
                    suffix = f"\n… gekürzt bei {max_results} Treffern"
                    return "\n".join(hits) + suffix
    return "\n".join(hits) if hits else "Keine Treffer."


def git_log(limit: int = 20, path: str = "", root: Path | None = None) -> str:
    """Letzte Commits und Arbeitsbaum-Status anzeigen."""
    root = (root or get_repo_root()).resolve()
    limit = max(1, min(limit, 100))
    command = ["git", "-C", str(root), "log", f"-{limit}", "--oneline", "--decorate"]
    if path:
        command.extend(["--", path])
    log = subprocess.run(command, capture_output=True, text=True, timeout=30)
    status = subprocess.run(
        ["git", "-C", str(root), "status", "--short"],
        capture_output=True,
        text=True,
        timeout=30,
    )
    return (
        "## Commits\n"
        + (log.stdout.strip() or log.stderr.strip() or "keine Ausgabe")
        + "\n## Arbeitsbaum\n"
        + (status.stdout.strip() or "clean")
    )


def cargo_metadata(root: Path | None = None) -> str:
    """Workspace-Crates und Abhängigkeiten über cargo metadata zusammenfassen."""
    root = (root or get_repo_root()).resolve()
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        capture_output=True,
        text=True,
        timeout=180,
    )
    if result.returncode != 0:
        return f"cargo metadata fehlgeschlagen:\n{result.stderr[-3000:]}"
    data = json.loads(result.stdout)
    package_by_id = {pkg["id"]: pkg for pkg in data.get("packages", [])}
    members = data.get("workspace_members", [])
    lines = ["## Workspace-Crates"]
    for member in members:
        pkg = package_by_id.get(member, {})
        lines.append(
            f"- {pkg.get('name', '?')} {pkg.get('version', '?')} "
            f"({Path(pkg.get('manifest_path', '')).parent})"
        )
    lines.append("\n## Abhängigkeiten der Workspace-Crates")
    external: set[str] = set()
    for member in members:
        pkg = package_by_id.get(member, {})
        lines.append(f"### {pkg.get('name', '?')}")
        for dep in pkg.get("dependencies", []):
            kind = dep.get("kind") or "normal"
            target = dep.get("path") or dep.get("req") or ""
            line = f"- {dep.get('name')} ({kind}) {target}"
            if dep.get("path"):
                line += " [workspace]"
            else:
                external.add(dep.get("name", ""))
            lines.append(line)
    lines.append(f"\nExterne Abhängigkeiten (eindeutig): {len(external)}")
    text = "\n".join(lines)
    return text[:16_000] + ("\n… gekürzt" if len(text) > 16_000 else "")


def repo_stats(root: Path | None = None) -> str:
    """Kennzahlen sammeln: Rust-Dateien, Tests, Doku, offene Markierungen."""
    root = (root or get_repo_root()).resolve()
    rust_files = [
        p for p in _walk_text_files(root) if p.suffix == ".rs" and "fuzz" not in p.parts
    ]
    rust_lines = 0
    test_attributes = 0
    todo_markers = 0
    for path in rust_files:
        try:
            content = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        rust_lines += content.count("\n") + 1
        test_attributes += len(re.findall(r"#\[(?:tokio::)?test\]", content))
        todo_markers += len(re.findall(r"\b(TODO|FIXME|XXX|HACK)\b", content))
    docs = list((root / "docs").rglob("*.md")) if (root / "docs").exists() else []
    return (
        f"Rust-Dateien: {len(rust_files)}\n"
        f"Rust-Zeilen: {rust_lines}\n"
        f"Rust-Testattribute: {test_attributes}\n"
        f"TODO/FIXME/HACK in Rust: {todo_markers}\n"
        f"Doku-Dateien unter docs/: {len(docs)}"
    )
