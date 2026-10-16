# Server allocator and release optimization experiments

`engine-server` uses the system allocator by default. Build with
`--features allocator-mimalloc` to select mimalloc for the server executable.
The library does not install a global allocator in applications that embed it.
The same allocator module is used by the server and its benchmark executable.

```sh
cargo build --locked --release -p engine-server --features allocator-mimalloc
```

## Compare allocators and LTO

The server benchmark reuses the existing engine workload fixtures, including
linear service tasks, parallel gateways, scripts, and conditions. Compare the
same workload on the same machine, with no other load. For example:

```sh
cargo bench --locked -p engine-server --bench allocator_bench --profile release -- 'Linear Process Execution/100' --save-baseline system-thin
cargo bench --locked -p engine-server --bench allocator_bench --profile release --features allocator-mimalloc -- 'Linear Process Execution/100' --baseline system-thin
cargo bench --locked -p engine-server --bench allocator_bench --profile release-fat --features allocator-mimalloc -- 'Linear Process Execution/100' --baseline system-thin
```

`release-fat` inherits the normal release profile and changes only LTO from
`thin` to `fat`. Production builds still use thin LTO. Criterion output is under
`target/criterion`; verify that the filter selected a benchmark. Record build
time and executable size alongside latency and throughput. These engine-only
benchmarks omit HTTP and NATS I/O; validate representative HTTP workloads with
NATS, peak RSS, and tail latency before choosing production settings.
Allocator changes can affect memory use as well as speed. A feature-enabled
`cargo test` checks library code; the benchmark and `cargo check --all-targets`
also compile the executable that actually installs the allocator.

## Evaluate profile-guided optimization on Linux

Follow the [rustc PGO workflow](https://doc.rust-lang.org/rustc/profile-guided-optimization.html).
Keep target, features, profile, source revision, and non-PGO compiler flags
identical for training and optimization. Use the matching Rust toolchain's
`llvm-profdata`, and a fresh directory for every training run:

```sh
rustup component add llvm-tools-preview
PGO_DIR=$(mktemp -d)
PGO_TARGET=x86_64-unknown-linux-gnu
RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --locked -p engine-server --profile release-fat --target "$PGO_TARGET" --features allocator-mimalloc
LLVM_PROFILE_FILE="$PGO_DIR/server-%m-%p.profraw" target/$PGO_TARGET/release-fat/engine-server
```

While the instrumented server runs, exercise a representative mix of deployment,
process starts, worker fetch/complete operations, timers, scripts, and instance
queries against a disposable NATS instance. Shut the server down normally to
flush profiles, then merge and rebuild:

```sh
LLVM_PROFDATA="$(rustc --print sysroot)/lib/rustlib/$PGO_TARGET/bin/llvm-profdata"
"$LLVM_PROFDATA" merge -sparse "$PGO_DIR"/*.profraw -o "$PGO_DIR/merged.profdata"
RUSTFLAGS="-Cprofile-use=$PGO_DIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" cargo build --locked -p engine-server --profile release-fat --target "$PGO_TARGET" --features allocator-mimalloc
```

Run the optimized binary against a separate validation workload and compare it
with an uninstrumented build. Training profiles are workload- and
architecture-specific; do not reuse them across unrelated targets or revisions.
No production allocator, LTO, or PGO performance improvement is claimed yet.

## HTTP-/NATS-Vergleich mit RSS und Tail-Latenz

`engine-server/benchmarks/http_load.py` benötigt nur Python 3. Jeder Worker nutzt
eine persistente HTTP-Verbindung und führt eine Transaktion aus: Start einer
Rhai-Script-Kette, anschließend Lesen und Prüfung der abgeschlossenen Instanz
und des Script-Ergebnisses. Die gemessene Latenz umfasst beide Requests. Der
Test meldet Durchsatz, p50/p95/p99/max und Fehler; HTTP-, Ergebnis-, RSS- und
Persistenzfehler führen zu einem Exit-Code ungleich null. Warmup und Deployment
liegen außerhalb der Messung. Standardmäßig ist NATS-Persistenz erforderlich;
`--allow-in-memory` ist für Smoke-Tests vorgesehen.

Für jeden Vergleich denselben Quellstand, dieselbe Maschine, Anzahl Requests,
Concurrency, Script-Schritte und Payload verwenden. Server und **disposable**
NATS mit leerem Datenverzeichnis für jede Wiederholung neu starten: Der Test
legt Definitionen und Instanzen an und behält sie für die Nachprüfung. Nie auf
einem produktiven Server messen. Beispiel für Windows/PowerShell:

```powershell
# NATS separat starten, mit einem neuen Datenverzeichnis je Wiederholung:
nats-server -js -p 14222 -sd target/http-bench/run-01/nats

# In einem zweiten Terminal:
$env:NATS_URL = "nats://127.0.0.1:14222"
$env:REQUIRE_NATS = "true"
$env:PORT = "18081"
$env:LOG_FILE = "off"
$env:RUST_LOG = "error"
$env:MODEL_CHECK = "off"
cargo build --locked --release -p engine-server
target/release/engine-server.exe

# In einem dritten Terminal: tatsächliche Server-PID einsetzen.
python -B engine-server/benchmarks/http_load.py --base-url http://127.0.0.1:18081 --requests 1000 --warmup 100 --concurrency 8 --steps 10 --payload-bytes 1024 --server-pid 12345 --label system-thin-REVISION --output target/http-bench/system-thin.json
```

Für mimalloc `--features allocator-mimalloc` beim Build ergänzen. Für fat-LTO
`--release` durch `--profile release-fat` ersetzen und das Binary unter
`target/release-fat/` starten. Unter Linux identische Umgebungsvariablen per
`export` setzen und die Linux-Binaries ohne `.exe` starten. API-Key bei Bedarf
auch für den Client über `BPMNINJA_API_KEY` setzen; er wird nicht ausgegeben.

`--server-pid` misst den lokalen Serverprozess, unter Linux RSS und unter Windows
Working Set. Das Sampling erfolgt alle 100 ms, ergänzt um Anfangs-/Endwerte;
`sampled_peak` ist deshalb kein garantierter absoluter Peak. Für entfernte
Server PID weglassen und RSS separat auf dem Server messen. Der Client arbeitet
closed-loop: langsame Antworten reduzieren die angebotene Last. p99 beschreibt
die erfolgreichen Transaktionen unter dieser Concurrency, keine feste externe
Ankunftsrate. Bei Fehlern sind die Erfolgs-Perzentile keine bestandene Messung.

Mehrere Wiederholungen in wechselnder Reihenfolge ausführen. Label mit
Allocator, LTO, Revision und ggf. PGO versehen; Builddauer und Binary-Größe
separat erfassen. Der Script-/Start-/Lesetest ergänzt die Engine-Benches, deckt
aber Worker-fetch/complete, Timer und Deployment unter Last nicht ab. Für PGO
zusätzliche produktionsnahe Trainings- und unabhängige Validierungslasten nutzen.

Regressionstests des Messwerkzeugs:

```sh
python -B -m unittest discover -s engine-server/benchmarks -p test_http_load.py -v
```

## Initial local allocator result (2026-10-13)

Windows x64, Rust 1.99.0, normal release profile (thin LTO), linear workflow with
100 service tasks. A confirmation run used the already compiled executables,
2 seconds of warmup, 3 seconds of measurement, and 20 samples:

| Allocator | Criterion time estimate | Estimated interval |
| --- | --- | --- |
| System | 5.83 ms | 5.58–6.12 ms |
| mimalloc | 1.90 ms | 1.74–2.13 ms |

Criterion reported a roughly 68% reduction on this workload (p < 0.05). The first
shorter run was slower for both builds (9.67 vs 2.90 ms), so absolute latency is
sensitive to local conditions. This is evidence for keeping the opt-in experiment,
not a production HTTP/NATS result. Repeat on deployment hardware and record RSS
and tail latency before changing defaults. HTTP-/NATS-Ergebnisse einschließlich
fat-LTO folgen unten; PGO wurde noch nicht gemessen.

## Lokaler HTTP-/NATS-Vergleich (2026-10-13)

Windows x64, Rust 1.99.0, NATS 2.11.9, Server-Quellstand `668a6d6`, Default-
Persistenzformat. Je Lauf: frischer Server und NATS-Datenbestand, 100 Warmup-
Transaktionen, danach 1.000 Transaktionen, acht Worker, zehn Script-Schritte und
1.024 Bytes zusätzliche Variablen-Payload. Deployment-Modellprüfung und
Datei-Logging waren deaktiviert (`MODEL_CHECK=off`, `LOG_FILE=off`, `RUST_LOG=error`).

| Build / Lauf | Transaktionen/s | p95 (ms) | p99 (ms) | Höchste Working-Set-Stichprobe (MiB) |
| --- | ---: | ---: | ---: | ---: |
| System / thin / 1 | 770,9 | 17,34 | 21,91 | 20,77 |
| System / thin / 2 | 905,1 | 12,86 | 15,57 | 21,27 |
| mimalloc / thin / 1 | 922,9 | 12,72 | 16,22 | 22,52 |
| mimalloc / thin / 2 | 706,7 | 24,80 | 37,58 | 22,81 |
| mimalloc / fat / 1 | 926,0 | 12,10 | 33,37 | 22,39 |
| mimalloc / fat / 2 | 920,8 | 12,99 | 19,83 | 21,43 |

Alle aufgeführten Transaktionen und Ergebnisprüfungen waren erfolgreich;
keine neuen Persistenzfehler, Readiness- oder RSS-Fehler. Rohberichte liegen
lokal unter `target/http-bench/`. Die Läufe dauern nur etwa 1–1,5 Sekunden,
erfolgten je Build nacheinander und streuen deutlich. Sie belegen die Funktion
des Messwerkzeugs, keinen statistisch gesicherten Allocator-Vorteil. Das höhere
Working Set von mimalloc ist ebenfalls nur ein Befund für diesen kleinen Test.
Für belastbare Entscheidungen längere, abwechselnde Wiederholungen und die
Zielhardware verwenden; der Standard bleibt System-Allocator mit thin-LTO.

Binary-Größen: System/thin 18.062.336 Bytes, mimalloc/thin 18.014.720 Bytes,
mimalloc/fat 17.228.288 Bytes.
Buildzeiten: System/thin 5m37s (mit Dependency-Rebuild), mimalloc/thin 3m12s
(inkrementell), mimalloc/fat 8m44s (neuer Profil-Cache).
Wegen unterschiedlicher Cache-Zustände sind diese Buildzeiten
kein fairer Vergleich der Compilerprofile. PGO wurde hier nicht gemessen;
`llvm-tools-preview` ist in der lokalen Rust-Toolchain nicht installiert.
