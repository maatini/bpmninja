//! Fuzz target: Rhai script engine
//!
//! Feeds arbitrary script strings into the sandboxed Rhai engine to verify
//! that the operation limit (`set_max_operations`) correctly prevents runaway
//! scripts and that no input causes a panic or memory safety violation.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(script) = std::str::from_utf8(data) {
        // Create a sandboxed engine matching the production configuration
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(10_000);   // Same limit as production
        engine.set_max_string_size(4_096);   // Prevent memory bombs
        engine.set_max_array_size(1_024);     // Prevent allocation bombs
        engine.set_max_map_size(256);         // Prevent allocation bombs
        engine.set_max_expr_depths(32, 32);  // Prevent stack overflow

        let mut scope = rhai::Scope::new();
        // Seed a few variables so the fuzzer can discover variable-interaction bugs
        scope.push("x", 42_i64);
        scope.push("name", "fuzz".to_string());
        scope.push("flag", true);

        // We don't care about the result — only that it doesn't panic or crash.
        // Rhai errors (parsing, runtime, too many operations) are expected and OK.
        let _ = engine.eval_with_scope::<rhai::Dynamic>(&mut scope, script);
    }
});
