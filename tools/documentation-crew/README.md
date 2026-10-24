# Documentation Crew

Ein hierarchisches CrewAI-Team, das bestehende technische Dokumentation
analysiert, nach Diátaxis neu ordnet, überarbeitet und mit einem
reproduzierbaren Qualitätsscore freigibt oder zurückweist.

Das Projekt ist eigenständig und liegt unter `tools/documentation-crew/`. Es
verändert die Dokumentation im Quellbestand nicht. Alle Ergebnisse landen in
einem separaten Ausgabeverzeichnis.

## Was das Team liefert

- belastbare Domänenrecherche mit Quellen und Vergleichsprojekten
- Audit der bestehenden Dokumentation mit priorisierten Lücken
- Informationsarchitektur nach Diátaxis: `tutorials/`, `how-to/`,
  `reference/`, `explanation/`
- überarbeitete Markdown-Seiten mit Front Matter, Beispielen, Voraussetzungen
  und Fehlerfällen
- technischen Review, Style Guide, Glossar und UX-Review
- geprüfte Mermaid-Diagramme mit Alternativtexten
- QA-Report mit sechs Dimensionen und gewichtetem Gesamtscore
- Abschlussbericht mit Freigabeentscheidung, Restrisiken und
  Übernahme-Checkliste

## Team

Der Documentation Lead ist Manager im hierarchischen Prozess. Die neun
Spezialisten führen die Aufgaben aus.

| Agent | Rolle | Memory | Delegation |
|---|---|---:|---:|
| `documentation_lead` | Manager, Konfliktentscheidung, Freigabe | ja | ja |
| `domain_researcher` | Domäne, Standards, Vergleichsprojekte | ja | nein |
| `documentation_auditor` | Audit, Lücken, Baseline-Score | ja | nein |
| `information_architect` | Diátaxis, Nutzerreisen, Dateiplan | ja | ja |
| `senior_technical_writer` | Inhalte schreiben und überarbeiten | ja | ja |
| `technical_accuracy_reviewer` | Technische Korrektheit, Codeabgleich | nein | nein |
| `style_terminology_editor` | Terminologie, Ton, Formatierung | nein | nein |
| `audience_ux_specialist` | Onboarding, Auffindbarkeit, UX | nein | nein |
| `visual_diagram_specialist` | Mermaid, Visualisierung, Alternativtexte | nein | nein |
| `quality_assurance_reviewer` | Ganzheitliche QA und Score | ja | nein |

Die vollständigen Rollen, Ziele und Backstories stehen in
`src/documentation_crew/config/agents.yaml`, die Aufgaben und Abhängigkeiten in
`src/documentation_crew/config/tasks.yaml`.

## Ablauf

```text
Domain Research ─┐
                 ├─> Information Architecture ─> Content Creation
Documentation ───┘                                      │
Audit                                                   ├─> Technical Review
                                                        ├─> Style & Glossary
                                                        ├─> UX Optimization
                                                        └─> Diagram Integration
                                                                 │
                                                                 v
                                                        Quality Assurance
                                                                 │
                                      Score >= Schwelle? ── nein ──> Revision
                                                                 │        │
                                                                 ja       v
                                                                 │   Technical Recheck
                                                                 │        │
                                                                 │   QA erneut
                                                                 v
                                                     Executive Summary (Lead)
```

Der QA-Agent bewertet `completeness`, `accuracy`, `clarity`, `consistency`,
`diataxis` und `usability`. Das Werkzeug `record_quality_score` berechnet daraus
den Gesamtscore und erzwingt `changes_requested`, sobald ein Blocker offen ist
oder der Score unter der Schwelle liegt. Bei Bedarf folgen bis zu vier
Nachbesserungsrunden. Der Lead schreibt danach den Abschlussbericht in einem
eigenen sequenziellen Lauf.

## Einrichtung

Voraussetzungen: Python 3.11 bis 3.13, ein API-Key für das gewählte Modell.

```bash
cd tools/documentation-crew
cp .env.example .env
# DEEPSEEK_API_KEY in .env eintragen oder als Umgebungsvariable setzen

uv venv --python 3.12
uv pip install -r requirements.txt
```

Alternativ mit `pyproject.toml`:

```bash
uv sync
```

Ohne `uv`:

```bash
python3.12 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## Eingaben

Die Crew wird ausschließlich über Eingaben gesteuert. Pflichtfelder:

| Feld | Typ | Bedeutung |
|---|---|---|
| `project_name` | String | Name des Projekts |
| `project_description` | String | Kurzbeschreibung, Zweck und Kontext |
| `domain_topic` | String | Fachdomäne für die Recherche |
| `existing_docs_path` | String oder Liste | Dateien oder Verzeichnisse der bestehenden Dokumentation |
| `target_audience` | String oder Liste | Zielgruppen |
| `output_path` | String | Zielverzeichnis; wird angelegt |

Optionale Felder:

| Feld | Standard | Bedeutung |
|---|---|---|
| `repository_path` | – | Repository für den technischen Codeabgleich |
| `documentation_language` | `Deutsch` | Sprache der erzeugten Dokumentation |
| `quality_threshold` | `90` | QA-Schwelle von 50 bis 100 |
| `max_revision_rounds` | `2` | Nachbesserungsrunden von 0 bis 4 |

`existing_docs_path` akzeptiert eine Liste oder einen String mit einem Pfad pro
Zeile. Relative Pfade werden relativ zur Eingabedatei aufgelöst, bei
Kommandozeilenaufrufen relativ zum aktuellen Arbeitsverzeichnis.

Beispiel: `inputs.example.json`.

```json
{
  "project_name": "BPMNinja",
  "project_description": "BPMN-2.0-Workflow-Engine in Rust.",
  "domain_topic": "BPMN 2.0 und Prozessautomatisierung",
  "existing_docs_path": ["../../README.md", "../../docs"],
  "target_audience": ["Einsteiger", "Entwickler", "Betrieb"],
  "output_path": "../../reports/documentation-crew-run",
  "repository_path": "../..",
  "documentation_language": "Deutsch",
  "quality_threshold": 90,
  "max_revision_rounds": 2
}
```

## Nutzung

Zuerst die Konfiguration ohne Modellaufruf prüfen:

```bash
PYTHONPATH=src .venv/bin/python -m documentation_crew --inputs inputs.example.json --dry-run
```

Der Dry-Run validiert Pflichtfelder, Pfade, YAML, Rollen, Werkzeuge,
Task-Abhängigkeiten und alle Prompt-Platzhalter. Er ruft kein Modell auf.

Dann den Lauf starten:

```bash
PYTHONPATH=src .venv/bin/python -m documentation_crew.main \
  --inputs inputs.example.json
```

Oder nach Installation des Pakets:

```bash
uv run documentation-crew --inputs inputs.example.json
```

Alle Felder lassen sich auch über Kommandozeilenparameter setzen:

```bash
PYTHONPATH=src .venv/bin/python -m documentation_crew.main \
  --project-name "BPMNinja" \
  --project-description "BPMN-2.0-Engine in Rust" \
  --domain-topic "BPMN 2.0" \
  --existing-docs ../../README.md \
  --existing-docs ../../docs \
  --target-audience "Entwickler und Betriebsteams" \
  --repository ../.. \
  --output ../../reports/documentation-crew-run \
  --threshold 92 \
  --max-rounds 2
```

Nützliche Schalter:

```text
--dry-run              nur validieren
--no-memory            Memory deaktivieren
--quiet                Crew-Ausgabe reduzieren
--model MODELL         Standard: deepseek/deepseek-chat
--manager-model MODELL Modell des Documentation Lead
--effort low|medium|high|max
--temperature 0.2
--threshold 90
--max-rounds 2
```

## Programmatische Nutzung

```python
from pathlib import Path

from documentation_crew.config import RunConfig
from documentation_crew.crew import DocumentationCrew

config = RunConfig.from_mapping(
    {
        "project_name": "Mein Projekt",
        "project_description": "Beschreibung",
        "domain_topic": "Fachdomäne",
        "existing_docs_path": ["README.md", "docs"],
        "target_audience": ["Entwickler", "Betrieb"],
        "output_path": "reports/doku-lauf",
        "repository_path": ".",
        "quality_threshold": 90,
        "max_revision_rounds": 2,
    },
    base_dir=Path.cwd(),
)

crew = DocumentationCrew(config, verbose=True)
result = crew.kickoff()  # Alias für run(), CrewAI-konform

print(result.approved, result.score)
print(result.output_path / "EXECUTIVE-SUMMARY.md")
```

Für einen eigenen Ablauf ohne Dateisystem kann `RunConfig.from_mapping`
direkt mit absoluten Pfaden verwendet werden. `dry_run=True` baut die Crew,
führt aber keine Modellaufrufe aus.

## Ausgabestruktur

```text
<output_path>/
├── README.md                     Einstieg und Navigation
├── EXECUTIVE-SUMMARY.md          Abschlussbericht des Lead
├── tutorials/                    Lernen
├── how-to/                       Aufgaben erledigen
├── reference/                    Nachschlagen
├── explanation/                  Verstehen
├── _research/
│   ├── domain-research.md
│   └── sources.md
├── _audit/
│   ├── audit-report.md
│   ├── inventory.md
│   └── gap-matrix.md
├── _architecture/
│   ├── information-architecture.md
│   ├── content-plan.md
│   ├── migration-map.md
│   └── diagram-plan.md
├── _review/
│   ├── technical-review.md
│   ├── style-guide.md
│   ├── glossary.md
│   ├── style-report.md
│   └── ux-review.md
└── _qa/
    ├── content-manifest.md
    ├── qa-report.md
    ├── qa-report.json
    ├── qa-report-round-<n>.json
    ├── revision-round-<n>.md
    ├── final-approval.md
    ├── run.json
    └── memory/                   lokale CrewAI-Memory
```

`run.json` enthält Dauer, Modelle, Scores, Iterationen, Dokumente und
Token-Nutzung. API-Keys werden nicht protokolliert.

## Qualitätsscore

| Dimension | Gewicht | Inhalt |
|---|---:|---|
| `accuracy` | 25 % | Technische Korrektheit, Quellen, Codeabgleich |
| `completeness` | 20 % | Voraussetzungen, Ergebnisse, Fehlerfälle, Betrieb |
| `clarity` | 15 % | Aufgabe, Zielgruppe, nächster Schritt |
| `consistency` | 15 % | Terminologie, Front Matter, Formate |
| `diataxis` | 15 % | Saubere Trennung der vier Content-Typen |
| `usability` | 10 % | Einstieg, Navigation, Lesepfade, Barrierefreiheit |

Der Score ist die gewichtete Summe. Ein Lauf gilt nur als freigegeben, wenn
`decision = approved`, `blockers` leer und `score >= quality_threshold` ist.
Der Lead darf diese Regel im Abschlussbericht nicht überschreiben.

## Werkzeuge und Sicherheit

| Werkzeug | Zweck |
|---|---|
| `web_search` | Serper bei `SERPER_API_KEY`, sonst Bing-RSS mit DuckDuckGo-Fallback |
| `fetch_url` | Öffentliche Webseiten; private/localhost-Adressen und Redirects werden geprüft |
| `source_inventory`, `source_read`, `source_search` | Ausschließlich lesender Zugriff auf den Quellenbestand |
| `code_read`, `code_search` | Optionaler Codeabgleich über `repository_path` |
| `doc_list`, `doc_read` | Inventar und Lesen der erzeugten Dokumentation |
| `doc_write` | Schreiben ausschließlich unter `output_path`, nur `.md` und `.mmd` |
| `check_links` | Relative interne Links und Bildpfade |
| `verify_mermaid` | Diagrammtyp, leere Blöcke, Tabulatoren |
| `record_quality_score` | Score berechnen, Blocker erzwingen, JSON schreiben |

Sicherheitsregeln:

- Alle Dateizugriffe laufen über einen Sandbox-Pfad. `..`, absolute Pfade und
  Symlink-Ausbrüche werden abgewiesen.
- Schreibzugriffe sind auf 2 MiB pro Datei und `.md`/`.mmd` begrenzt.
- Quellen werden nur bis 1 MiB je Datei indiziert; große Dateien werden beim
  Lesen gekürzt.
- Webabrufe sind auf 2 MiB begrenzt und blockieren private, lokale und
  reservierte Adressen.
- Die Crew bekommt keine Shell, keine Git-Schreibrechte und keinen Zugriff auf
  `.env`, `.git`, `target`, `node_modules` oder `.venv`.
- CrewAI-Telemetrie ist per `CREWAI_TELEMETRY_OPT_OUT=true` abgeschaltet.

## Memory

Memory ist für Lead, Researcher, Auditor, Architect, Writer und QA aktiv. Der
Standard-Embedder ist das lokale ONNX-Modell `all-MiniLM-L6-v2`; beim ersten
Speichern lädt Chroma etwa 80 MB. Ein OpenAI-Key ist dafür nicht nötig.

```bash
# Memory vollständig deaktivieren
DOC_CREW_MEMORY=0

# OpenAI-Embedder statt ONNX
DOC_CREW_EMBEDDER=openai

# Eigene Embedder-Konfiguration
DOC_CREW_EMBEDDER='{"provider":"onnx","config":{}}'
```

Memory kostet zusätzliche Modellaufrufe. Für schnelle Testläufe oder
Kostengrenzen `DOC_CREW_MEMORY=0` setzen.

## Modellkonfiguration

```bash
DOC_CREW_MODEL=deepseek/deepseek-chat
DOC_CREW_MANAGER_MODEL=deepseek/deepseek-chat
DOC_CREW_EFFORT=high
DOC_CREW_TEMPERATURE=0.2
DOC_CREW_TOOL_FAILURE_POLICY=warn
```

Jedes LiteLLM-kompatible Modell funktioniert, sofern der passende Provider-Key
gesetzt ist. Für DeepSeek wird `DEEPSEEK_API_KEY` benötigt. Der Manager kann
ein stärkeres Modell verwenden als die Fachagenten.

## Tests

```bash
cd tools/documentation-crew
PYTHONPATH=src .venv/bin/python -m pytest
PYTHONPATH=src .venv/bin/python -m documentation_crew.main \
  --inputs inputs.example.json --dry-run
```

Die Tests benötigen keinen API-Key. Sie prüfen Eingabevalidierung,
Pfadsandbox, Quelleninventar, Link- und Mermaid-Prüfung, Score-Berechnung,
YAML-Konsistenz und den Dry-Run der Crew.

## Grenzen

- Die Qualitätsbewertung ist modellbasiert. Ein menschlicher Review bleibt für
  Veröffentlichungen erforderlich.
- `verify_mermaid` prüft Syntaxmerkmale, nicht das gerenderte Layout. Für
  visuelle Abnahme zusätzlich die Mermaid-CLI oder die Zielplattform nutzen.
- `check_links` prüft Dateipfade, keine Anchors und keine externen URLs.
- Websuche kann durch Rate-Limits oder Layoutänderungen eingeschränkt sein.
  Der Research-Agent dokumentiert fehlende Quellen.
- Das Ausgabeverzeichnis wird nicht automatisch geleert. Für reproduzierbare
  Läufe pro Lauf ein eigenes `output_path` verwenden; alte QA-Berichte werden
  beim Start archiviert.
- Die Crew ersetzt keinen Static-Site-Generator. Sie liefert Markdown und
  Mermaid, keine fertige Website.
- Ohne `repository_path` kann der Technical Reviewer nur gegen vorhandene
  Dokumente und Quellen prüfen. Nicht verifizierbare Aussagen werden markiert.

## Projektstruktur

```text
tools/documentation-crew/
├── README.md
├── requirements.txt
├── pyproject.toml
├── .env.example
├── inputs.example.json
├── src/documentation_crew/
│   ├── __init__.py
│   ├── config.py            Eingaben und Pfadvalidierung
│   ├── crew.py              Agenten, Aufgaben, Crews, Iterationsloop
│   ├── main.py              CLI und Dry-Run
│   ├── qa.py                Score-Modell und Persistenz
│   ├── web.py               keyless Websuche und sicherer URL-Abruf
│   ├── workspace.py         Pfadsandbox, Quellenkorpus, Link-/Mermaid-Prüfung
│   ├── config/
│   │   ├── agents.yaml
│   │   └── tasks.yaml
│   └── tools/
│       ├── docs_tools.py
│       └── web_tools.py
└── tests/
```
