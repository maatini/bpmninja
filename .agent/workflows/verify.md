---
description: Verify the project
---
// turbo-all
1. cargo build --workspace
2. cargo clippy --workspace --all-targets --all-features -- -D warnings
3. cargo test --workspace
