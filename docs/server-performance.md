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
