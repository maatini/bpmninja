# RTK - Rust Token Killer (Google Antigravity)

**Usage**: Token-optimized CLI proxy for shell commands.

## Rule

Use `rtk` for shell commands when available and stable in the current environment.
If `rtk` is unavailable or breaks command semantics, run the original command directly.

Examples:

```bash
rtk git status
rtk cargo test
rtk ls src/
rtk rg "pattern" src/
rtk rg --files -g "*.rs"
rtk docker ps
rtk gh pr list
```

## Meta Commands

```bash
rtk gain              # Show token savings
rtk gain --history    # Command history with savings
rtk discover          # Find missed RTK opportunities
rtk proxy <cmd>       # Run raw (no filtering, for debugging)
```

## Why

RTK filters and compresses command output before it reaches the LLM context, often saving 60-90% tokens on common operations.
Prefer `rtk <cmd>` where possible, but do not block task execution if raw commands are required.
