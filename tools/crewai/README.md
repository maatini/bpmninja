# BPMNinja CrewAI

CrewAI-Crew zur Analyse und zur bestätigten Umsetzung von Verbesserungen am
BPMNinja-Projekt. Die Analyse ist rein lesend. Änderungen laufen über einen
separaten Lauf mit expliziter Bestätigung und einer engen Werkzeug-Allowlist.

## Aufbau

| Agent | Aufgabe | Werkzeuge |
|---|---|---|
| `repo_analyst` | Rust-Architektur, tote Lasten, Testlücken, Konsistenz | Repo lesen/suchen, cargo metadata, git log |
| `domain_researcher` | BPMN-2.0-Markt, Camunda/Zeebe/Flowable/Temporal, Standards | Repo + Websuche, URL-Fetch, GitHub |
| `product_reviewer` | REST-API, TypeScript-Client, Desktop-UX, Doku | Repo + Websuche |
| `quality_auditor` | Panics, Limits, Auth, Persistenz, Dependency-Risiken | Repo + Websuche |
| `prioritizer` | Dedupe und Rating 1–100 | keine |

Die Umsetzungs-Crew besteht aus `rust_implementer`, `verifier` und `reviewer`.
Sie bearbeitet maximal drei bestätigte Empfehlungen pro Lauf.

## Einrichtung

```bash
cd tools/crewai
cp .env.example .env
# DEEPSEEK_API_KEY in .env eintragen
uv sync
uv run bpmninja-crew --help
```

Der Key kann auch als Umgebungsvariable gesetzt sein. `tools/crewai/.env`
wird nicht versioniert.

## Analyse starten

```bash
# gesamtes Projekt
uv run bpmninja-crew analyze --focus all

# einzelne Bereiche
uv run bpmninja-crew analyze --focus engine
uv run bpmninja-crew analyze --focus server --max-recommendations 8

# letzten Lauf erneut anzeigen
uv run bpmninja-crew list
```

Fokuswerte: `all`, `engine`, `parser`, `persistence`, `server`, `desktop`,
`docs`, `quality`, `security`.

Modelle und Denkaufwand sind über Umgebungsvariablen steuerbar:

```bash
BPMNINJA_CREW_MODEL_SCAN=deepseek/deepseek-flash
BPMNINJA_CREW_EFFORT_SCAN=high
BPMNINJA_CREW_MODEL_SYNTH=deepseek/deepseek-v4-pro
BPMNINJA_CREW_EFFORT_SYNTH=high
```

Die schnellen Fachagenten laufen standardmäßig mit `deepseek-flash`, die
Synthese mit `deepseek-v4-pro`. Beide Modelle unterstützen die Stufen `low`,
`high` und `max`.

## Rating 1–100

Jede Empfehlung trägt eine nachvollziehbare Aufteilung:

| Kriterium | Punkte | Bedeutung |
|---|---:|---|
| `impact` | 0–30 | Wirkung auf Nutzer, Betrieb oder Produktreife |
| `domain_gap` | 0–20 | Relevanz im BPMN-/Workflow-Markt, belegt durch Quellen |
| `risk_reduction` | 0–20 | Gewinn bei Stabilität, Security oder Wartbarkeit |
| `effort` | 0–15 | 15 = kleiner Aufwand, 0 = großer Umbau |
| `evidence` | 0–15 | Stärke der Code- und Quellenbelege |

Der Score ist die Summe und wird beim Einlesen gegen die Aufteilung geprüft.
Empfehlungen unter 40 Punkten werden von der Crew verworfen.

## Ergebnisse

Jeder Lauf schreibt nach `reports/crewai/<Zeitstempel>/`:

- `01_repo_analyse.md` – Architektur- und Code-Funde
- `02_domaenen_recherche.md` – Marktvergleich mit Quellen
- `03_produkt_dx.md` – API-, Client- und UI-Funde
- `04_qualitaet_security.md` – Risiko- und Testlücken
- `05_empfehlungen.md` – strukturierter Bericht der Crew
- `report.json` – maschinenlesbarer `RecommendationReport`
- `EMPFEHLUNGEN.md` – aufbereitetes Ranking mit Evidenz und Abnahmekriterien
- `run.json` – Modelle, Effort, Fokus und Zeitstempel

`reports/` ist lokal ignoriert. Nach einer Bestätigung können ausgewählte
Empfehlungen in `docs/` übernommen oder committet werden.

## Umsetzung nach Bestätigung

Zuerst den Trockenlauf ausführen:

```bash
uv run bpmninja-crew implement \
  --run reports/crewai/2026-10-24T10-00-00Z \
  --ids R1,R3
```

Ohne `--confirm` wird nur der Plan angezeigt. Erst damit startet die
Umsetzungs-Crew:

```bash
uv run bpmninja-crew implement \
  --run reports/crewai/2026-10-24T10-00-00Z \
  --ids R1,R3 --confirm
```

Die Crew darf:

- Textdateien im Repo lesen und durchsuchen,
- bestehende Dateien über exakte Ersetzungen ändern,
- neue Textdateien anlegen,
- `cargo fmt/check/test/clippy/build`, `git status/diff/log/show`,
  `npm run build/lint/test`, `tsc` und `pytest` ausführen.

Sie darf nicht:

- Commits oder Pushes erzeugen,
- `HANDOFF.md`, `plan.md`, `improve1026.md`, `Cargo.lock` oder `.env` ändern,
- beliebige Shell-Kommandos ausführen,
- Verzeichnisse wie `.git`, `target`, `node_modules` oder `.venv` anfassen.

Nach dem Lauf die vollständige Verifikation aus dem Repo ausführen:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Tests

```bash
cd tools/crewai
uv run pytest
```

## Grenzen

- Die Websuche nutzt Bing-RSS mit DuckDuckGo-Fallback. Bei Rate-Limits oder
  Layoutänderungen können Quellen fehlen; der Bericht markiert das.
- LLM-Antworten können falsche Dateibezüge enthalten. Jede Empfehlung nennt
  Evidenz, die vor der Umsetzung geprüft werden muss.
- Die Umsetzungs-Crew ist bewusst auf kleine, bestätigte Schritte begrenzt.
  Große Architekturänderungen gehören in einen eigenen Plan.
- Der Lauf benötigt Netzwerkzugriff und einen DeepSeek-API-Key. Die Kosten
  hängen vom gewählten Modell und Effort ab.
