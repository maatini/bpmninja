// Run the same workflow workload under the server's selected global allocator.
// Keep fixtures and Criterion settings identical to the engine baseline.
#[path = "../src/allocator.rs"]
mod allocator;

include!("../../engine-core/benches/engine_bench.rs");
