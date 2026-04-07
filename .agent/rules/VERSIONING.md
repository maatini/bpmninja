---
trigger: file_match
file_patterns: ["**/Cargo.toml", "**/package.json", "**/tauri.conf.json"]
---

# Versioning Convention

All version numbers across the project MUST stay in sync. When bumping a version, update ALL of these files:

## Files to Update
| File | Field |
|---|---|
| `engine-core/Cargo.toml` | `version` |
| `bpmn-parser/Cargo.toml` | `version` |
| `persistence-nats/Cargo.toml` | `version` |
| `engine-server/Cargo.toml` | `version` |
| `agent-orchestrator/Cargo.toml` | `version` |
| `desktop-tauri/src-tauri/Cargo.toml` | `version` |
| `desktop-tauri/package.json` | `version` |
| `desktop-tauri/src-tauri/tauri.conf.json` | `version` |
| `.agent/manifest.json` | `version` |

## Rules
- Follow **Semantic Versioning** (SemVer): `MAJOR.MINOR.PATCH`
  - MAJOR: Breaking public API changes
  - MINOR: New features, backward-compatible
  - PATCH: Bug fixes, backward-compatible
- Never bump only a single file — always update all files listed above.
- The `/verify` and `/verify-ui` workflows do NOT check version sync — verify manually.
