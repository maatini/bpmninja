"""Sichere Dateisystem- und Markdown-Hilfen für die Documentation Crew.

Die Crew bekommt keine uneingeschränkten Dateiwerkzeuge. Alle Lesezugriffe
laufen über ein Quellen- oder Code-Korpus, alle Schreibzugriffe über den
Ausgabe-Sandbox. Das verhindert Pfadausbrüche, versehentliche Änderungen am
Repository und das Schreiben von Nicht-Markdown-Artefakten.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
from typing import Iterable, Sequence

import yaml


DOC_EXTENSIONS = {".md", ".mdx", ".rst", ".txt", ".adoc"}
CODE_EXTENSIONS = {
    ".rs",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".py",
    ".go",
    ".java",
    ".kt",
    ".kts",
    ".cs",
    ".rb",
    ".php",
    ".sql",
    ".proto",
    ".toml",
    ".yaml",
    ".yml",
    ".json",
    ".sh",
    ".zsh",
    ".bash",
    ".dockerfile",
    ".tf",
    ".hcl",
    ".md",
}
SKIP_DIRS = {
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "venv",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "coverage",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "graphify-out",
}
MAX_FILE_BYTES = 1_048_576
MAX_MANIFEST_FILES = 300
MAX_DOCUMENT_BYTES = 2_097_152
ALLOWED_DOC_SUFFIXES = {".md", ".mmd"}


class WorkspaceError(ValueError):
    """Ungültiger Pfad oder unzulässiger Inhalt."""


def _is_hidden_or_skipped(path: Path) -> bool:
    return any(part in SKIP_DIRS for part in path.parts)


def iter_text_files(
    roots: Sequence[Path],
    extensions: set[str],
    *,
    limit: int = MAX_MANIFEST_FILES,
    max_bytes: int = MAX_FILE_BYTES,
) -> list[tuple[str, Path, int]]:
    """Textdateien unter den Wurzeln einsammeln.

    Rückgabe ist eine sortierte Liste aus Anzeigename, absolutem Pfad und
    Dateigröße. Der Anzeigename bleibt innerhalb der Wurzel relativ, wird bei
    mehreren Wurzeln aber mit dem Wurzelnamen präfixiert, damit er eindeutig
    bleibt.
    """
    seen: set[Path] = set()
    entries: list[tuple[str, Path, int]] = []
    multiple_roots = len(roots) > 1
    for root in roots:
        root = root.expanduser()
        if root.is_file():
            if root.suffix.lower() not in extensions:
                continue
            resolved = root.resolve()
            if resolved in seen or resolved.stat().st_size > max_bytes:
                continue
            seen.add(resolved)
            label = root.name if not multiple_roots else f"{root.parent.name}/{root.name}"
            entries.append((label, resolved, resolved.stat().st_size))
            continue
        if not root.is_dir():
            continue
        for candidate in sorted(root.rglob("*")):
            if len(entries) >= limit:
                break
            if not candidate.is_file() or candidate.is_symlink():
                continue
            relative = candidate.relative_to(root)
            if _is_hidden_or_skipped(relative):
                continue
            if candidate.suffix.lower() not in extensions:
                continue
            try:
                size = candidate.stat().st_size
            except OSError:
                continue
            if size > max_bytes:
                continue
            resolved = candidate.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            label = relative.as_posix()
            if multiple_roots:
                label = f"{root.name}/{label}"
            entries.append((label, resolved, size))
    entries.sort(key=lambda item: item[0].lower())
    return entries[:limit]


def _first_heading(path: Path) -> str:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for _ in range(80):
                line = handle.readline()
                if not line:
                    break
                stripped = line.strip()
                if stripped.startswith("#"):
                    return stripped.lstrip("#").strip() or path.name
    except OSError:
        pass
    return path.name


def _word_count(path: Path) -> int:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return 0
    return len(re.findall(r"\w+", text))


def build_source_manifest(roots: Sequence[Path], limit: int = MAX_MANIFEST_FILES) -> str:
    """Ein lesbares Inventar für Prompt und Audit erzeugen."""
    entries = iter_text_files(roots, DOC_EXTENSIONS, limit=limit)
    lines = [
        "# Quellenmanifest",
        "",
        f"Wurzeln: {', '.join(str(root) for root in roots)}",
        f"Gefundene Textdateien: {len(entries)} (Limit {limit})",
        "",
        "| Anzeigename | Wörter | Titel |",
        "|---|---:|---|",
    ]
    for label, path, _size in entries:
        title = _first_heading(path).replace("|", "\\|")
        lines.append(f"| `{label}` | {_word_count(path)} | {title} |")
    if not entries:
        lines.append("| _keine Textdateien gefunden_ | 0 | – |")
    if len(entries) >= limit:
        lines.extend(
            [
                "",
                f"> Das Manifest wurde bei {limit} Dateien abgeschnitten. "
                "Nutze source_search für gezielte Treffer außerhalb des Manifests.",
            ]
        )
    return "\n".join(lines) + "\n"


FILE_SECTION = re.compile(
    r"^#{1,4}\s+(?:Datei|File)\s*(?:\d+)?\s*[—–:=-]\s*`?([^`\n]+?)`?\s*$"
)


def extract_markdown_bundle(text: str) -> dict[str, str]:
    """Liest Dateiabschnitte im Format ``## Datei: `pfad.md```.

    Der Inhalt reicht bis zum nächsten Dateiabschnitt. Überschriften innerhalb
    von Code-Fences werden ignoriert, damit Beispiele die Aufteilung nicht
    stören. Absolute Pfade, ``..``-Ausbrüche und fremde Endungen werden
    verworfen; die eigentliche Sandbox-Prüfung erfolgt beim Schreiben.
    """
    sections: list[tuple[str, list[str]]] = []
    current: str | None = None
    body: list[str] = []
    in_fence = False

    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("```"):
            in_fence = not in_fence
        match = None if in_fence else FILE_SECTION.match(stripped)
        if match:
            if current:
                sections.append((current, body))
            current = match.group(1).strip().strip("`").strip()
            body = []
            continue
        if current:
            body.append(line)
    if current:
        sections.append((current, body))

    documents: dict[str, str] = {}
    for raw_path, lines in sections:
        candidate = raw_path.replace("\\", "/").strip()
        while candidate.startswith("./"):
            candidate = candidate[2:]
        if not candidate or candidate.startswith("/"):
            continue
        if any(part == ".." for part in Path(candidate).parts):
            continue
        if Path(candidate).suffix.lower() not in ALLOWED_DOC_SUFFIXES:
            continue
        content = "\n".join(lines).strip()
        # Manche Modelle umschließen den Dateiinhalt zusätzlich mit einem
        # Markdown-Fence. Das ist kein Teil der Datei und wird entfernt.
        if content.startswith("```"):
            first_newline = content.find("\n")
            if first_newline != -1:
                content = content[first_newline + 1 :]
        if content.rstrip().endswith("```"):
            content = content.rstrip()[:-3].rstrip()
        if content:
            documents[candidate] = content
    return documents


@dataclass(frozen=True)
class CorpusEntry:
    label: str
    path: Path
    size: int


class TextCorpus:
    """Lesender, auf Wurzeln beschränkter Zugriff auf Textdateien."""

    def __init__(
        self,
        roots: Sequence[Path],
        extensions: set[str],
        *,
        limit: int = MAX_MANIFEST_FILES,
    ) -> None:
        self.roots = tuple(root.expanduser().resolve() for root in roots)
        self.extensions = extensions
        self._entries = iter_text_files(self.roots, extensions, limit=limit)
        self._by_label = {entry[0]: entry for entry in self._entries}

    @property
    def size(self) -> int:
        return len(self._entries)

    def labels(self) -> list[str]:
        return sorted(self._by_label)

    def manifest(self) -> str:
        if not self.roots:
            return "Kein Quellen- oder Codepfad angegeben.\n"
        lines = [
            "# Verfügbare Textdateien",
            "",
            "| Anzeigename | Wörter | Titel |",
            "|---|---:|---|",
        ]
        for label, path, _size in self._entries:
            title = _first_heading(path).replace("|", "\\|")
            lines.append(f"| `{label}` | {_word_count(path)} | {title} |")
        if not self._entries:
            lines.append("| _keine passenden Dateien_ | 0 | – |")
        lines.extend(
            [
                "",
                "Lies Dateien mit dem passenden read-Werkzeug über den Anzeigenamen.",
                "Nutze die Suche, wenn eine Datei nicht im Manifest steht.",
            ]
        )
        return "\n".join(lines) + "\n"

    def resolve(self, label: str) -> CorpusEntry:
        cleaned = label.strip().strip("`")
        entry = self._by_label.get(cleaned)
        if entry is None and "/" not in cleaned:
            matches = [
                value for key, value in self._by_label.items() if key.endswith("/" + cleaned)
            ]
            if len(matches) == 1:
                entry = matches[0]
        if entry is None:
            raise WorkspaceError(
                f"Unbekannte Quelle '{label}'. Nutze das Inventar oder die Suche."
            )
        return CorpusEntry(*entry)

    def read(self, label: str, max_chars: int = 40_000) -> str:
        entry = self.resolve(label)
        try:
            text = entry.path.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            raise WorkspaceError(f"Quelle '{entry.label}' nicht lesbar: {exc}") from exc
        header = f"# Quelle: {entry.label}\n\n"
        if len(text) > max_chars:
            text = text[:max_chars] + "\n\n… gekürzt; gezielt mit der Suche nachlesen."
        return header + text

    def search(
        self,
        query: str,
        *,
        max_results: int = 20,
        glob: str | None = None,
    ) -> str:
        query = query.strip()
        if len(query) < 2:
            raise WorkspaceError("Suchbegriff muss mindestens zwei Zeichen haben.")
        try:
            pattern = re.compile(re.escape(query), re.IGNORECASE)
        except re.error as exc:  # pragma: no cover - re.escape should not fail
            raise WorkspaceError(f"Ungültige Suche: {exc}") from exc
        hits: list[str] = []
        for label, path, _size in self._entries:
            if glob and not path.match(glob):
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for number, line in enumerate(text.splitlines(), start=1):
                if pattern.search(line):
                    snippet = line.strip()
                    if len(snippet) > 300:
                        snippet = snippet[:300] + "…"
                    hits.append(f"- `{label}:{number}` {snippet}")
                    if len(hits) >= max_results:
                        return (
                            f"# Treffer für '{query}' ({len(hits)} angezeigt)\n\n"
                            + "\n".join(hits)
                            + "\n\nWeitere Treffer mit einem engeren Begriff suchen."
                        )
        if not hits:
            return f"Keine Treffer für '{query}' in {len(self._entries)} Dateien."
        return f"# Treffer für '{query}'\n\n" + "\n".join(hits)


class DocumentationWorkspace:
    """Sandbox für die erzeugte Dokumentation."""

    DIATAXIS = ("tutorials", "how-to", "reference", "explanation")
    INTERNAL = ("_research", "_audit", "_architecture", "_review", "_qa")

    def __init__(self, root: Path, *, create: bool = True) -> None:
        self.root = root.expanduser().resolve()
        if create:
            self.root.mkdir(parents=True, exist_ok=True)
            for name in (*self.DIATAXIS, *self.INTERNAL):
                (self.root / name).mkdir(exist_ok=True)

    def resolve(self, relative: str, suffixes: set[str] | None = None) -> Path:
        cleaned = relative.strip().strip("`")
        if not cleaned:
            raise WorkspaceError("Leerer Dateipfad.")
        candidate_path = Path(cleaned)
        if candidate_path.is_absolute():
            raise WorkspaceError("Absolute Pfade sind nicht erlaubt.")
        candidate = (self.root / candidate_path).resolve()
        if not candidate.is_relative_to(self.root):
            raise WorkspaceError(f"Pfad verlässt das Ausgabeverzeichnis: {relative}")
        if suffixes is not None and candidate.suffix.lower() not in suffixes:
            erlaubt = ", ".join(sorted(suffixes))
            raise WorkspaceError(
                f"Dateiendung '{candidate.suffix}' ist nicht erlaubt ({erlaubt})."
            )
        return candidate

    def write_document(self, relative: str, content: str) -> dict[str, object]:
        if not isinstance(content, str) or not content.strip():
            raise WorkspaceError("Dokumentinhalt ist leer.")
        encoded = content.encode("utf-8")
        if len(encoded) > MAX_DOCUMENT_BYTES:
            raise WorkspaceError(
                f"Dokument ist größer als {MAX_DOCUMENT_BYTES // 1024} KiB."
            )
        target = self.resolve(relative, ALLOWED_DOC_SUFFIXES)
        target.parent.mkdir(parents=True, exist_ok=True)
        normalised = content.replace("\r\n", "\n").replace("\r", "\n")
        if not normalised.endswith("\n"):
            normalised += "\n"
        target.write_text(normalised, encoding="utf-8")
        return {
            "path": target.relative_to(self.root).as_posix(),
            "bytes": len(normalised.encode("utf-8")),
            "words": len(re.findall(r"\w+", normalised)),
        }

    def read_document(self, relative: str, max_chars: int = 40_000) -> str:
        target = self.resolve(relative, ALLOWED_DOC_SUFFIXES)
        if not target.is_file():
            raise WorkspaceError(f"Dokument nicht gefunden: {relative}")
        text = target.read_text(encoding="utf-8", errors="replace")
        header = f"# Dokument: {target.relative_to(self.root).as_posix()}\n\n"
        if len(text) > max_chars:
            text = text[:max_chars] + "\n\n… gekürzt; gezielt mit check_links prüfen."
        return header + text

    def documents(self) -> list[dict[str, object]]:
        records: list[dict[str, object]] = []
        for path in sorted(self.root.rglob("*.md")):
            if not path.is_file() or "_qa/memory" in path.as_posix():
                continue
            relative = path.relative_to(self.root)
            text = path.read_text(encoding="utf-8", errors="replace")
            front_matter = _read_front_matter(text)
            records.append(
                {
                    "path": relative.as_posix(),
                    "words": len(re.findall(r"\w+", text)),
                    "bytes": len(text.encode("utf-8")),
                    "title": _first_heading(path),
                    "content_type": relative.parts[0] if len(relative.parts) > 1 else "root",
                    "status": front_matter.get("status"),
                    "audience": front_matter.get("audience"),
                }
            )
        return records

    def inventory(self) -> str:
        records = self.documents()
        lines = [
            "# Dokumentationsinventar",
            "",
            f"Ausgabewurzel: {self.root}",
            f"Markdown-Dateien: {len(records)}",
            "",
            "| Pfad | Typ | Wörter | Status | Titel |",
            "|---|---|---:|---|---|",
        ]
        for record in records:
            title = str(record["title"]).replace("|", "\\|")
            status = str(record["status"] or "–").replace("|", "\\|")
            lines.append(
                f"| `{record['path']}` | {record['content_type']} | "
                f"{record['words']} | {status} | {title} |"
            )
        if not records:
            lines.append("| _keine Dokumente_ | – | 0 | – | – |")
        return "\n".join(lines) + "\n"

    def check_links(self) -> str:
        broken: list[str] = []
        checked = 0
        link_pattern = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+[^)]*)?\)")
        for path in sorted(self.root.rglob("*.md")):
            if not path.is_file():
                continue
            relative = path.relative_to(self.root)
            text = path.read_text(encoding="utf-8", errors="replace")
            for match in link_pattern.finditer(text):
                raw = match.group(1).strip().strip("<>")
                if raw.startswith(("http://", "https://", "mailto:", "tel:", "#")):
                    continue
                target_part = raw.split("#", 1)[0]
                if not target_part:
                    continue
                checked += 1
                target = (path.parent / target_part).resolve()
                if not target.is_relative_to(self.root) or not target.exists():
                    line = text.count("\n", 0, match.start()) + 1
                    broken.append(f"- `{relative}:{line}` → `{raw}`")
        if broken:
            return (
                f"# Linkprüfung: {len(broken)} defekte von {checked} internen Links\n\n"
                + "\n".join(broken)
            )
        return f"# Linkprüfung: {checked} interne Links geprüft, keine Defekte."

    def verify_mermaid(self) -> str:
        issues: list[str] = []
        diagrams = 0
        allowed = re.compile(
            r"^(flowchart|graph|sequenceDiagram|stateDiagram(?:-v2)?|classDiagram|"
            r"erDiagram|journey|gantt|pie|mindmap|timeline|quadrantChart|gitGraph|"
            r"C4Context|C4Container|C4Component|C4Dynamic|C4Deployment|block-beta|"
            r"sankey-beta|xychart-beta|packet-beta|architecture-beta)\b"
        )
        fence = re.compile(r"```mermaid[ \t]*\n(.*?)\n```", re.DOTALL)
        for path in sorted(self.root.rglob("*.md")):
            if not path.is_file():
                continue
            relative = path.relative_to(self.root)
            text = path.read_text(encoding="utf-8", errors="replace")
            for match in fence.finditer(text):
                diagrams += 1
                body = match.group(1)
                line = text.count("\n", 0, match.start()) + 1
                first = next((row.strip() for row in body.splitlines() if row.strip()), "")
                if not first:
                    issues.append(f"- `{relative}:{line}` leerer Mermaid-Block")
                    continue
                if not allowed.match(first):
                    issues.append(
                        f"- `{relative}:{line}` unbekannter Diagrammtyp: `{first[:60]}`"
                    )
                if "\t" in body:
                    issues.append(f"- `{relative}:{line}` Tabulator im Mermaid-Block")
        if issues:
            return (
                f"# Mermaid-Prüfung: {len(issues)} Befunde in {diagrams} Diagrammen\n\n"
                + "\n".join(issues)
            )
        return f"# Mermaid-Prüfung: {diagrams} Diagramme, keine Syntaxbefunde."


def _read_front_matter(text: str) -> dict[str, object]:
    if not text.startswith("---\n"):
        return {}
    end = text.find("\n---\n", 4)
    if end < 0:
        return {}
    try:
        data = yaml.safe_load(text[4:end])
    except yaml.YAMLError:
        return {}
    return data if isinstance(data, dict) else {}


def iter_existing(paths: Iterable[Path]) -> list[Path]:
    """Nur existierende Pfade zurückgeben, für frühe Validierung."""
    return [path for path in paths if path.exists()]
