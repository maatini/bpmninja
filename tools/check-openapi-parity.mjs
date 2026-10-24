#!/usr/bin/env node
/**
 * Vergleicht die ausgelieferte OpenAPI-Spec (`docs/openapi.yaml`) mit dem
 * TypeSpec-Generat (`api-spec/tsp-output/@typespec/openapi3/openapi.yaml`).
 *
 * Geprüft wird:
 * - jede Pfad+Methode-Kombination aus dem TypeSpec-Output existiert in `docs/openapi.yaml`
 * - `info.version` der ausgelieferten Spec entspricht `version.json`; eine
 *   abweichende, echte TypeSpec-Version lässt den Check fehlschlagen
 *
 * Grenzen: Die Extraktion ist zeilenbasiert und kommt ohne YAML-Parser aus.
 * Erwartet wird die übliche Formatierung von TypeSpec und der handgepflegten
 * Spec (Pfade mit zwei Leerzeichen, Methoden mit vier Leerzeichen unter
 * `paths:`). YAML-Anchors, Merge-Keys, mehrzeilige Flow-Styles oder mehrere
 * Dokumente pro Datei werden nicht aufgelöst.
 */
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const deliveredPath = join(root, "docs/openapi.yaml");
const generatedPath = join(root, "api-spec/tsp-output/@typespec/openapi3/openapi.yaml");
const versionPath = join(root, "version.json");
const HTTP_METHODS = "get|put|post|delete|options|head|patch|trace";

const problems = [];

function readIfExists(path, label) {
  if (!existsSync(path)) {
    problems.push(`${label} fehlt: ${relative(root, path)}`);
    return null;
  }
  return readFileSync(path, "utf8");
}

// `info.version` steht mit zwei Leerzeichen unter dem Top-Level-Block `info:`.
function extractInfoVersion(text) {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((line) => /^info:\s*(?:#.*)?$/.test(line));
  if (start === -1) {
    return null;
  }
  for (let i = start + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^\S/.test(line)) {
      break;
    }
    const match = line.match(/^ {2}version:\s*["']?([^"'\s#]+)["']?\s*(?:#.*)?$/);
    if (match) {
      return match[1];
    }
  }
  return null;
}

// Liefert Map<Pfad, Set<Methode>> für alle Operationen unter `paths:`.
function extractOperations(text) {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((line) => /^paths:\s*(?:#.*)?$/.test(line));
  const operations = new Map();
  if (start === -1) {
    return operations;
  }

  let currentPath = null;
  for (let i = start + 1; i < lines.length; i += 1) {
    const line = lines[i];
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) {
      continue;
    }
    if (!line.startsWith(" ")) {
      break;
    }

    const pathMatch = line.match(/^ {2}("?)(\/[^"#]*?)\1:\s*(?:#.*)?$/);
    if (pathMatch) {
      currentPath = pathMatch[2];
      if (!operations.has(currentPath)) {
        operations.set(currentPath, new Set());
      }
      continue;
    }

    const methodMatch = line.match(
      new RegExp(`^ {4}(${HTTP_METHODS}):\\s*(?:#.*)?$`, "i"),
    );
    if (methodMatch && currentPath) {
      operations.get(currentPath).add(methodMatch[1].toUpperCase());
    }
  }
  return operations;
}

const delivered = readIfExists(deliveredPath, "Ausgelieferte Spec");
const generated = readIfExists(generatedPath, "TypeSpec-Output");
const versionFile = readIfExists(versionPath, "Versionsdatei");

if (delivered && generated) {
  const expectedVersion = versionFile
    ? JSON.parse(versionFile).version
    : null;
  const deliveredVersion = extractInfoVersion(delivered);
  const generatedVersion = extractInfoVersion(generated);

  if (!deliveredVersion) {
    problems.push("docs/openapi.yaml hat keine info.version");
  } else if (expectedVersion && deliveredVersion !== expectedVersion) {
    problems.push(
      `docs/openapi.yaml info.version ist ${deliveredVersion}, version.json erwartet ${expectedVersion}`,
    );
  }

  if (!generatedVersion) {
    problems.push("TypeSpec-Output hat keine info.version");
  } else if (
    deliveredVersion &&
    generatedVersion !== deliveredVersion &&
    generatedVersion !== "0.0.0"
  ) {
    problems.push(
      `info.version weicht ab: docs/openapi.yaml ${deliveredVersion}, TypeSpec ${generatedVersion}`,
    );
  }

  const deliveredOps = extractOperations(delivered);
  const generatedOps = extractOperations(generated);
  if (generatedOps.size === 0) {
    problems.push("TypeSpec-Output enthält keine Pfade unter paths:");
  }

  let checkedOperations = 0;
  for (const [path, methods] of generatedOps) {
    if (methods.size === 0) {
      problems.push(`TypeSpec-Pfad ohne Operation: ${path}`);
      continue;
    }
    const deliveredMethods = deliveredOps.get(path);
    if (!deliveredMethods) {
      problems.push(`Pfad fehlt in docs/openapi.yaml: ${path}`);
      continue;
    }
    for (const method of methods) {
      checkedOperations += 1;
      if (!deliveredMethods.has(method)) {
        problems.push(`Operation fehlt in docs/openapi.yaml: ${method} ${path}`);
      }
    }
  }

  // Zusätzliche Pfade in der ausgelieferten Spec sind erlaubt (z. B. Alias-Routen),
  // werden aber sichtbar gemacht, damit sie bewusst gepflegt werden.
  const extras = [...deliveredOps.keys()].filter((path) => !generatedOps.has(path));

  if (problems.length > 0) {
    console.error(`OpenAPI-Paritätscheck fehlgeschlagen (${problems.length} Problem(e)):`);
    for (const problem of problems) {
      console.error(`  - ${problem}`);
    }
    process.exit(1);
  }

  const versionNote =
    generatedVersion === "0.0.0" && expectedVersion
      ? `Version ${deliveredVersion} (TypeSpec-Platzhalter 0.0.0)`
      : `Version ${deliveredVersion}`;
  console.log(
    `ok: ${generatedOps.size} Pfad(e), ${checkedOperations} Operation(en) aus TypeSpec in docs/openapi.yaml, ${versionNote}`,
  );
  if (extras.length > 0) {
    console.log(`Hinweis: ${extras.length} zusätzlicher Pfad(e) nur in docs/openapi.yaml: ${extras.join(", ")}`);
  }
} else if (problems.length > 0) {
  console.error(`OpenAPI-Paritätscheck fehlgeschlagen (${problems.length} Problem(e)):`);
  for (const problem of problems) {
    console.error(`  - ${problem}`);
  }
  process.exit(1);
}
