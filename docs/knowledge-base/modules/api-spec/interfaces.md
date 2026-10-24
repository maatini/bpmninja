# api-spec — Interfaces

## TypeSpec Source

Entry point: `main.tsp` — generates OpenAPI 3.0 YAML.

## Deployed Artifacts

| File | Purpose |
|------|---------|
| `docs/openapi.yaml` | Hand-authored/completed OpenAPI spec (authoritative) |
| `api-spec/tsp-output/@typespec/openapi3/openapi.yaml` | Generated from TypeSpec |
| `docs/index.html` | Redoc portal HTML |

`tools/check-openapi-parity.mjs` compares both files (paths, methods, `info.version`) as part of the `typespec` CI job. The TypeSpec output carries the placeholder version `0.0.0`; the delivered spec must match `version.json`.
