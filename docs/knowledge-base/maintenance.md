# Maintenance

## When to Update This Knowledge Base

Update the relevant file(s) when:

- **A new crate or module is added** → Create a new folder under `modules/` with index.md, responsibility.md, dependencies.md, interfaces.md, gotchas.md.
- **A major architectural change occurs** (e.g., new execution model, persistence backend) → Update `architecture/` files and affected module docs.
- **A new BPMN element is supported** → Update `modules/engine-core/` and `cross-cutting/tags.md` if a new `@tag:bpmn-xxx` is needed.
- **A public API endpoint is added/removed** → Update `modules/engine-server/interfaces.md` and `api-spec/`.
- **A Rust dependency changes** → Check and update `modules/*/dependencies.md` for the affected crate.
- **A new cross-cutting pattern or convention emerges** → Add to `cross-cutting/shared-patterns.md`.
- **A new `@tag:xxx` is introduced** → Register in `cross-cutting/tags.md`.

## How to Update

1. Identify the affected files from the [index](index.md) directory map.
2. Edit the file directly — keep the format consistent (bullets, tables, Mermaid where helpful).
3. If adding a new module, follow the template:
   - `index.md` — one paragraph summary + bullet links to sub-files
   - `responsibility.md` — "What does this own? What must always be true?"
   - `dependencies.md` — Inbound/outbound tables + Mermaid graph if complex
   - `interfaces.md` — Public APIs, events, contracts, data models
   - `gotchas.md` — Known pitfalls, edge cases, "don't forget this"
4. Run `devbox run graphify` if available to refresh the knowledge graph.
5. Commit with a clear message, e.g., `docs(kb): update engine-core for new BPMN element`.

## Content Quality Rules

- **Progressive disclosure**: High-level summaries in index/overview files; details in sub-files.
- **Responsibilities focus**: Every module file must answer "What does this own? What is it responsible for?"
- **Dependencies explicit**: Tables with columns: Dependent | Type | Purpose | Notes
- **Ground in code**: Don't invent. If unclear, note "Needs clarification" or "Inferred from X."
- **Agent-friendly language**: Write so a coding agent can decide "Do I need to read this file before changing X?"
- **Short paragraphs + bullets + tables**: No walls of text.
- **Mermaid for diagrams**: Use it for architecture, dependency graphs, and data flows.

## File Structure Convention

```
docs/knowledge-base/
├── index.md                    # Top-level navigation + quick start
├── overview.md                 # Project purpose, architecture summary, key principles
├── maintenance.md              # This file
├── architecture/
│   ├── index.md
│   ├── components.md           # Logical components + high-level responsibilities
│   ├── dependencies.md         # Global dependency overview + Mermaid graph
│   ├── data-flows.md           # Important data flows / interactions
│   └── decisions.md            # Key architectural decisions
├── modules/
│   └── [module-name]/
│       ├── index.md
│       ├── responsibility.md
│       ├── dependencies.md
│       ├── interfaces.md
│       └── gotchas.md
├── cross-cutting/
│   ├── index.md
│   ├── tags.md                 # @tag:xxx registry
│   └── shared-patterns.md      # Reusable patterns across modules
└── maintenance.md
```
