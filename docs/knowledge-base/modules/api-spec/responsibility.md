# api-spec — Responsibilities

## What It Owns

1. **@tag:api-spec** — OpenAPI 3.0 specification for all 38 REST endpoints.
2. TypeSpec source (`main.tsp`) compiled to OpenAPI YAML.
3. Generated Redoc portal hosted via GitHub Pages.

**Invariants:**
- Must stay in sync with `engine-server/src/server/mod.rs` route definitions
- Must stay in sync with `AppError` HTTP status codes

## Regeneration

```bash
cd api-spec
npm install
npx tsp compile .
# Output: tsp-output/@typespec/openapi3/openapi.yaml
# Copy to: docs/openapi.yaml
```
