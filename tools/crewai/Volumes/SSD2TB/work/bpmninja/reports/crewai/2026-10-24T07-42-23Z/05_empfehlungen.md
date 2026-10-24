{
  "generated_at": "2026-10-25T10:00:00Z",
  "focus": "all",
  "project_state": "Workspace 0.7.24, 6 Rust-Crates + Tauri; hexagonale Architektur, 0 TODO/FIXME, Hygiene abgeschlossen. Verbleibende Schwächen in Shutdown-Durability, Bootstrap-Fehlerbehandlung, Session-/Timer-Hotpaths, API-Vertragsdrifts, Security-Defaults und BPMN-Marktlücken (Multi-Instance, Tenancy, Incidents).",
  "methodology": "Vier Fachberichte (Architektur-Audit, Marktvergleich, DX-Review, Ops-/Security-/Qualitätsaudit) wurden dedupliziert, Überlappungen zusammengeführt und anhand belastbarer Code- und Quellenbelege bewertet. Scoring: impact (0-30), domain_gap (0-20), risk_reduction (0-20), effort (0-15), evidence (0-15). Generische oder bereits erledigte Punkte sind verworfen.",
  "recommendations": [
    {
      "id": "R1",
      "title": "Shutdown-Flush der Retry-Queue erzwingen",
      "category": "engine",
      "summary": "Graceful Shutdown verwirft Persist-Jobs im bounded Retry-Kanal, obwohl Logs Flush behaupten. Datenverlust genau im Störfall, für den die Queue existiert.",
      "proposed_change": "In engine-core/src/engine/retry_queue.rs den Shutdown-Pfad auf try_recv-Drain mit Zeitbudget umstellen; nach Ablauf verbleibende Jobs zählen und loggen. Logtexte in engine/mod.rs und engine-server/src/main.rs an reales Verhalten anpassen.",
      "acceptance_criteria": [
        "Bei Shutdown werden alle im Kanal vorhandenen PersistJobs verarbeitet oder als Fehler gezählt; Log enthält Anzahl verbliebener Jobs falls Budget überschritten.",
        "Unit-Test: gefüllte Queue + Shutdown → alle Jobs persistiert oder Fehlerzähler erhöht.",
        "Doku/Kommentare in engine/mod.rs sagen 'drain', nicht 'flush'."
      ],
      "risks": "Zeitbudget zu klein gewählt könnte lange Shutdowns verursachen; Budget konfigurierbar machen.",
      "effort": "S",
      "evidence": [
        {
          "path": "engine-core/src/engine/retry_queue.rs",
          "lines": "135-139",
          "note": "Worker bricht bei Shutdown sofort aus while-let-Schleife aus, ohne Kanal zu drainen."
        },
        {
          "path": "engine-core/src/engine/mod.rs",
          "lines": "136-138",
          "note": "Behauptet 'flush and exit'."
        },
        {
          "path": "engine-server/src/main.rs",
          "lines": "183-184",
          "note": "Log 'Flushing persistence queues…'."
        }
      ],
      "sources": [],
      "score": 83,
      "score_breakdown": {
        "impact": 27,
        "domain_gap": 6,
        "risk_reduction": 20,
        "effort": 15,
        "evidence": 15
      }
    },
    {
      "id": "R2",
      "title": "FetchAndLock-Eingaben validieren und per-Topic lockDuration anwenden",
      "category": "server",
      "summary": "Unvalidierte lockDuration/maxTasks können Chrono-Overflow und Lock-Amplification verursachen; per-Topic Lock-Dauer wird ignoriert und führt zu überlangen Locks.",
      "proposed_change": "In engine-server/src/server/tasks.rs lockDuration auf 1s–1h clampen, maxTasks ≤ 100, topics-Anzahl begrenzen. lock_duration je Topic durchreichen (Signatur &[(String, i64)]) und pro Task anwenden. TS-Client nutzt Topic-spezifische Dauer bei autoExtendLock.",
      "acceptance_criteria": [
        "Test mit lockDuration > 1h oder negativ → 400/422, keine Panic.",
        "maxTasks > 100 → 400.",
        "Multi-Topic-Request mit unterschiedlichen Durations liefert Tasks mit korrekten Ablaufzeiten.",
        "TS-Client autoExtendLock verwendet Optionswert des jeweiligen Topics."
      ],
      "risks": "Bestehende Worker mit hohen Werten werden abgelehnt; Rückwärtskompatibilität prüfen, ggf. Server-seitig kappen statt ablehnen.",
      "effort": "M",
      "evidence": [
        {
          "path": "engine-server/src/server/tasks.rs",
          "lines": "73-78",
          "note": "lockDuration aus erstem Topic, unvalidiert; maxTasks ungefiltert."
        },
        {
          "path": "engine-core/src/engine/service_task.rs",
          "lines": "83",
          "note": "now + TimeDelta::seconds(lock_duration) kann paniken."
        },
        {
          "path": "bpmn-ninja-external-task-client/src/ExternalTaskClient.ts",
          "lines": "286-289, 349",
          "note": "Client sendet per-Topic, autoExtend nutzt config.lockDuration."
        }
      ],
      "sources": [],
      "score": 82,
      "score_breakdown": {
        "impact": 24,
        "domain_gap": 15,
        "risk_reduction": 18,
        "effort": 10,
        "evidence": 15
      }
    },
    {
      "id": "R3",
      "title": "NATS-Bootstrap fail-closed durchsetzen",
      "category": "persistence",
      "summary": "Alle get_or_create_stream/create_key_value Fehler werden verschluckt; Readiness loggt 'Connected', obwohl KV/Streams kaputt sind. Fail-closed gilt nur für connect.",
      "proposed_change": "In persistence-nats/src/client.rs Fehler außer AlreadyExists propagieren; engine-server/src/main.rs Readiness an Bootstrap-Erfolg koppeln; StorageInfo um Status erweitern.",
      "acceptance_criteria": [
        "NATS-Account ohne JetStream-Rechte → Serverstart schlägt fehl oder /api/ready liefert 503 mit Fehler.",
        "Bereits existierende Streams/KVs gelten als ok (BucketAlreadyExists/StreamAlreadyExists).",
        "Log enthält präzise Ursache statt 'Connected to NATS'."
      ],
      "risks": "Bestehende NATS-Instanzen mit anderen Konfigurationen könnten brechen; Vorab-Prüfung der Rechte.",
      "effort": "S",
      "evidence": [
        {
          "path": "persistence-nats/src/client.rs",
          "lines": "22-29,32-49,51-128",
          "note": "Alle get_or_create_stream/create_key_value/create_object_store als let _ = ..."
        },
        {
          "path": "engine-server/src/main.rs",
          "lines": "82-116",
          "note": "Nur connect-Fehler wird als REQUIRE_NATS-Verletzung behandelt."
        },
        {
          "path": "engine-server/src/server/state.rs",
          "lines": "117-131",
          "note": "REQUIRE_NATS Default true, aber Bootstrap-Lücke."
        }
      ],
      "sources": [],
      "score": 80,
      "score_breakdown": {
        "impact": 25,
        "domain_gap": 5,
        "risk_reduction": 20,
        "effort": 15,
        "evidence": 15
      }
    },
    {
      "id": "R4",
      "title": "Timer-Fehler isolieren und Entfernen nach erfolgreicher Verarbeitung",
      "category": "engine",
      "summary": "Timer wird vor Verarbeitung gelöscht; Fehler bricht Runde ab; Doppelausführung nach Restart. Einziger inkonsistenter Timer kann Scheduler serialisieren.",
      "proposed_change": "In engine-core/src/engine/timer_processor.rs match statt ? verwenden; Fehler pro Timer isolieren. Timer erst nach erfolgreichem Batch entfernen, sonst wieder einfügen und persist_timer erzwingen. Fehlerzähler/Metrik und Incident-Vorbereitung.",
      "acceptance_criteria": [
        "Test mit NoSuchInstance: Timer bleibt erhalten oder wird als Incident markiert; andere fällige Timer derselben Runde werden verarbeitet.",
        "remove_persisted_timer nur nach erfolgreichem Batch.",
        "Metrik timer_errors erhöht bei Fehlern."
      ],
      "risks": "Dead-Letter/Incident erfordert neues Konzept, daher inkrementell: zunächst Fehler isolieren, Incident später.",
      "effort": "M",
      "evidence": [
        {
          "path": "engine-core/src/engine/timer_processor.rs",
          "lines": "32,54-56,116,175-181,237",
          "note": "Löscht Timer vor Verarbeitung; ? bricht Schleife ab; remove_persisted_timer erst am Ende."
        },
        {
          "path": "engine-server/src/main.rs",
          "lines": "144-152",
          "note": "Err nur tracing::error ohne Metrik/Incident."
        }
      ],
      "sources": [],
      "score": 78,
      "score_breakdown": {
        "impact": 25,
        "domain_gap": 10,
        "risk_reduction": 18,
        "effort": 10,
        "evidence": 15
      }
    },
    {
      "id": "R5",
      "title": "Auth fail-closed und Rollentrennung erzwingen",
      "category": "security",
      "summary": "Default keine Auth, ein einziger API-Key, keine Drosselung, kein Audit. Jeder im LAN kann Instanzen löschen oder Service-Tasks resolven.",
      "proposed_change": "Auth erforderlich, wenn nicht an Loopback gebunden; API-Key an Rollen binden (Worker vs Admin); Fehlversuche loggen und zählen; docker-compose mit Pflicht-Key.",
      "acceptance_criteria": [
        "Ohne API-Key und nicht-loopback → 401 für alle /api/...",
        "Worker-Key kann nur Service-Task-Endpunkte, Admin-Key alles.",
        "Auth-Fehlversuche im Log mit Metrik."
      ],
      "risks": "Bestehende Deployments müssen Key setzen; Übergangszeit mit Warnung.",
      "effort": "M",
      "evidence": [
        {
          "path": "engine-server/src/server/state.rs",
          "lines": "194-208",
          "note": "leer/unset → None → Auth aus."
        },
        {
          "path": "engine-server/src/server/auth.rs",
          "lines": "22-24",
          "note": "No-op ohne Key."
        },
        {
          "path": "docker-compose.yaml",
          "lines": "17-28",
          "note": "Port 8081 auf allen Interfaces, BPMNINJA_API_KEY auskommentiert."
        },
        {
          "path": "engine-server/src/main.rs",
          "lines": "163-169",
          "note": "Bind-Default 0.0.0.0."
        }
      ],
      "sources": [],
      "score": 75,
      "score_breakdown": {
        "impact": 26,
        "domain_gap": 5,
        "risk_reduction": 20,
        "effort": 10,
        "evidence": 14
      }
    },
    {
      "id": "R6",
      "title": "Incident-Modell über Job-Retries hinaus erweitern",
      "category": "engine",
      "summary": "Nur Service-Task-Retries erzeugen Incidents; Persistenz-Erschöpfung, Timer-/Condition-Fehler und ungefangene BPMN-Errors am Prozessscope nicht. Marktlücke zu Camunda.",
      "proposed_change": "Einheitlichen Incident-Typ in engine-core (z. B. in service_task.rs) definieren; Integration in Timer-Verarbeitung (R4) und Retry-Queue-Drop; Resolve-Pfad vorsehen. Erster Schritt: Incident-Struktur + Timer-/Persistenzfehler als Incident ablegen.",
      "acceptance_criteria": [
        "Persistenzfehler nach Retry-Limit erzeugen Incident mit Ursache.",
        "Timer-Auswertungsfehler erzeugen Incident statt stiller Drop.",
        "Ungefangener BPMN-Error am Prozessscope erzeugt Incident.",
        "API/UI zeigen Incidents mit cause und resolve."
      ],
      "risks": "Größeres Design, daher inkrementell: zuerst Incident-Datenmodell und Timer-Integration, dann UI/API.",
      "effort": "M",
      "evidence": [
        {
          "path": "engine-core/src/engine/service_task.rs",
          "lines": "266-464",
          "note": "Incident nur bei Service-Task-Retries=0 implementiert."
        },
        {
          "path": "engine-core/src/engine/timer_processor.rs",
          "lines": "32,54",
          "note": "Fehler führen zu stillem Abbruch ohne Incident."
        }
      ],
      "sources": [
        "https://docs.camunda.io/docs/components/concepts/incidents/"
      ],
      "score": 73,
      "score_breakdown": {
        "impact": 20,
        "domain_gap": 18,
        "risk_reduction": 15,
        "effort": 10,
        "evidence": 10
      }
    },
    {
      "id": "R7",
      "title": "JetStream-Retention und Replikation explizit konfigurieren",
      "category": "persistence",
      "summary": "WORKFLOW_EVENTS und WORKFLOW_HISTORY ohne max_age/max_bytes/num_replicas; History wächst monoton, Account läuft voll, keine Replikation.",
      "proposed_change": "In persistence-nats/src/client.rs beim Anlegen der Streams max_age, max_bytes, num_replicas setzen; Monitoring-Alert auf storage_bytes; Retention-Policy dokumentieren.",
      "acceptance_criteria": [
        "Test prüft Stream-Config auf gesetzte Limits.",
        "Doku beschreibt Aufbewahrungsdauer und Replikationsfaktor.",
        "Monitoring zeigt Alerts bei Überschreitung."
      ],
      "risks": "Bestehende Streams müssen migriert werden; Abwägung Datenhaltung vs. History-Vollständigkeit.",
      "effort": "S",
      "evidence": [
        {
          "path": "persistence-nats/src/client.rs",
          "lines": "23-29,122-128",
          "note": "Streams werden mit ..Default::default() erstellt, keine Limits."
        },
        {
          "path": "engine-server/src/log_nats.rs",
          "lines": "36-40",
          "note": "Log-Stream setzt max_messages explizit als Gegenbeispiel."
        }
      ],
      "sources": [],
      "score": 70,
      "score_breakdown": {
        "impact": 18,
        "domain_gap": 5,
        "risk_reduction": 18,
        "effort": 15,
        "evidence": 14
      }
    },
    {
      "id": "R8",
      "title": "Multi-Instance minimal implementieren",
      "category": "parser",
      "summary": "Deploy lehnt Multi-Instance hart ab (parser.rs:280-290); blockiert Camunda-Migrationen mit MI-Marker. Marktstandard.",
      "proposed_change": "Erster Schritt: Paralleles Multi-Instance für ServiceTask/UserTask mit inputCollection, loopCounter; Fehlermeldung um Migrationshinweis ergänzen. Später sequenziell/outputCollection.",
      "acceptance_criteria": [
        "BPMN mit multiInstanceLoopCharacteristics parallel (isSequential=false, loopCardinality oder inputCollection) wird deployt.",
        "Service-Task wird für jedes Element des Inputs ausgeführt.",
        "Variable loopCounter im Task-Kontext verfügbar.",
        "Fehler bei sequenziell weiterhin präzise mit Hinweis."
      ],
      "risks": "Semantik komplex; auf parallele Service-/User-Tasks begrenzen, um Akzeptanz zu testen.",
      "effort": "M",
      "evidence": [
        {
          "path": "bpmn-parser/src/parser.rs",
          "lines": "280-290",
          "note": "Deploy-Fehler 'Multi-instance is not supported'."
        },
        {
          "path": "bpmn-parser/src/tests.rs",
          "lines": "1229-1252",
          "note": "Test belegt Ablehnung."
        }
      ],
      "sources": [
        "https://docs.camunda.io/docs/components/modeler/bpmn/multi-instance/",
        "https://www.flowable.com/open-source/docs/bpmn/ch07b-BPMN-Constructs"
      ],
      "score": 65,
      "score_breakdown": {
        "impact": 20,
        "domain_gap": 20,
        "risk_reduction": 5,
        "effort": 8,
        "evidence": 12
      }
    },
    {
      "id": "R9",
      "title": "Tenant-Scoping und Rollen am API-Key einführen",
      "category": "server",
      "summary": "Kein Tenant-Begriff (0 Treffer im Code); nur globaler API-Key. Für SaaS/Mehrteam-Betrieb ungeeignet. Camunda/Temporal bieten Tenants.",
      "proposed_change": "Datenmodell um tenant_id erweitern (Definition, Instanz, Job, Timer); Key→(Tenant, Rolle) Zuordnung in state.rs; API-Filter nach Tenant. Erster Schritt: tenant_id-Feld + Migration Bestandsdaten zu default-tenant.",
      "acceptance_criteria": [
        "Mit Key A werden nur Ressourcen des Tenants A sichtbar.",
        "Admin- vs Worker-Rolle getrennt (in Kombination mit R5).",
        "Bestehende Daten werden einem Default-Tenant zugeordnet."
      ],
      "risks": "Großer Eingriff; inkrementell mit Datenmodell-Migration beginnen.",
      "effort": "L",
      "evidence": [
        {
          "path": "engine-server/src/server/state.rs",
          "lines": "117-131",
          "note": "Nur globaler API-Key, kein Tenant-Konzept."
        }
      ],
      "sources": [
        "https://docs.camunda.io/docs/components/concepts/multi-tenancy/",
        "https://docs.temporal.io/namespaces"
      ],
      "score": 63,
      "score_breakdown": {
        "impact": 18,
        "domain_gap": 20,
        "risk_reduction": 10,
        "effort": 5,
        "evidence": 10
      }
    },
    {
      "id": "R10",
      "title": "API-Vertragskonsistenz wiederherstellen",
      "category": "dx",
      "summary": "Mehrere Abweichungen zwischen OpenAPI/README und tatsächlichem Verhalten: Start/Latest-Feldname, Erfolgs-Statuscodes 200 vs 204, Fehlerobjekt nicht uniform, XML-Endpunkt-Format.",
      "proposed_change": "1) StartLatestRequest auf bpmn_process_id umstellen (alias) und business_key top-level; 2) OpenAPI auf 204 korrigieren oder Handler 200; 3) AppError um JsonRejection/LengthLimit erweitern, 413/422 dokumentieren; 4) XML-Endpunkt auf {xml}-JSON oder Spec auf text/plain, 404 liefern.",
      "acceptance_criteria": [
        "Contract-Tests schicken OpenAPI-Beispiele, keine Rejection.",
        "Statuscodes laut Erwartungstabelle (204/200).",
        "Fehlerantworten immer {error}.",
        "XML-Abruf liefert dokumentiertes Format und 404 falls fehlt."
      ],
      "risks": "Umstellung des Start-Feldnamens kann bestehende Clients brechen; alias erlaubt Übergang.",
      "effort": "M",
      "evidence": [
        {
          "path": "engine-server/src/server/instances.rs",
          "lines": "42-47",
          "note": "Erwartet bpmn_id, OpenAPI fordert bpmn_process_id."
        },
        {
          "path": "docs/openapi.yaml",
          "lines": "189-193,404,464,485,501,520,570,287",
          "note": "200 statt 204, etc."
        },
        {
          "path": "engine-server/src/server/tasks.rs",
          "lines": "44,130,160,224,246",
          "note": "Handler liefern 204."
        },
        {
          "path": "engine-server/src/server/instances.rs",
          "lines": "151,239",
          "note": "DELETE/variables 204."
        },
        {
          "path": "engine-server/src/server/tasks.rs",
          "lines": "65",
          "note": "Json<T> direkt, Axum-Rejection nicht uniform."
        },
        {
          "path": "engine-server/src/server/instances.rs",
          "lines": "28",
          "note": "Json<T> direkt."
        },
        {
          "path": "engine-server/src/server/mod.rs",
          "lines": "258",
          "note": "DefaultBodyLimit 413 nicht dokumentiert."
        },
        {
          "path": "engine-server/src/server/deploy.rs",
          "lines": "110-119",
          "note": "XML Endpoint text/plain, OpenAPI application/json."
        },
        {
          "path": "docs/openapi.yaml",
          "lines": "144-152",
          "note": "XML Format JSON {xml} erwartet."
        }
      ],
      "sources": [],
      "score": 59,
      "score_breakdown": {
        "impact": 22,
        "domain_gap": 5,
        "risk_reduction": 5,
        "effort": 12,
        "evidence": 15
      }
    }
  ],
  "discarded": [
    "async InstanceStore ohne await (F7) – kein Laufzeitfehler, API-Umbau nicht prioritär.",
    "Timer-Due-Index globaler Mutex (F8) – Performance-Risiko, aber nicht kritisch; KB-Doku-Anpassung genügt vorerst.",
    "query_history erzeugt Consumer pro Request (F4) – Performance-Hotpath, aber kein Datenverlust; zurückgestellt.",
    "Camunda-7-Kompatibilitätsmatrix und errorEventDefinition (M5) – Dokumentationslücke, kein Code; später.",
    "Migration um Event-Subscriptions erweitern (M6) – mittlere Priorität, nicht top-10.",
    "OTLP-Export und Trace-Kontext (M4) – Observability-Vorteil, aber nicht Kernfunktion.",
    "HTTP-Rand Timeouts/Concurrency-Limit (O7) – mittel, aber durch Auth und FetchAndLock höher priorisiert.",
    "Qualitätsgates Mutation/Fuzz/Persistenz-Tests (O8) – CI-Verbesserung, kein direktes Produktrisiko.",
    "Service-Task-/Incident-Liste unpaginiert (DX8) – UI-Performance, mittel.",
    "History-Filter der Desktop-UI filtert nicht (DX4) – UI-Bug, mittel.",
    "Poll-Loop TS-Client ohne Backoff (DX7) – DX-Verbesserung, mittel."
  ]
}