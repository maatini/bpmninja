# reports/ — lokale Lauf-Artefakte

Dieses Verzeichnis sammelt die Ausgaben der Analyse- und Dokumentations-Crews.
Die Inhalte sind Arbeitsartefakte und bleiben **lokal**: Sie werden nicht mehr
versioniert (siehe `.gitignore`), nur diese README ist im Repo.

## Bündel

| Verzeichnis | Inhalt |
|---|---|
| `documentation-crew-run/` | Vollständiger Dokumentations-Lauf: Diátaxis-Struktur (`tutorials/`, `how-to/`, `reference/`, `explanation/`), Audit, QA- und Abschlussberichte |
| `documentation-crew-run-abbruch-20261024T101317Z/` | Abgebrochener Lauf desselben Teams, aufbewahrt zur Nachvollziehbarkeit |
| `crewai/` | Läufe der Verbesserungs-Crew (Analyse, priorisierte Empfehlungen, ggf. Umsetzung) |

Jeder Lauf schreibt in ein eigenes, zeitgestempeltes Ausgabeverzeichnis; alte
Läufe werden nicht überschrieben.

## Reproduzieren

Dokumentations-Crew:

```bash
cd tools/documentation-crew
uv sync
uv run documentation-crew --inputs inputs.example.json \
  --output ../../reports/documentation-crew-run
```

Verbesserungs-Crew:

```bash
cd tools/crewai
uv sync
uv run bpmninja-crew analyze --focus all
```

Details zu Aufrufen und Konfiguration stehen in den READMEs der beiden Tools.
Ältere Stände der Artefakte bleiben in der Git-Historie einsehbar; neue Läufe
werden bewusst nicht mehr eingecheckt.
