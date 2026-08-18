#!/usr/bin/env node
/**
 * Fails if the locked TypeScript version is outside typescript-eslint's
 * declared peer range. This is the contract `npm ci` enforces in CI.
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

const packages = lock.packages ?? {};
const tsEntry = packages["node_modules/typescript"];
const teslint = packages["node_modules/typescript-eslint"];

if (!tsEntry?.version) {
  console.error("package-lock.json has no node_modules/typescript entry");
  process.exit(1);
}
if (!teslint?.peerDependencies?.typescript) {
  console.error("package-lock.json has no typescript-eslint typescript peer");
  process.exit(1);
}

const lockedTs = tsEntry.version;
const declaredTs = pkg.devDependencies?.typescript ?? "";
const peerRange = teslint.peerDependencies.typescript;

function parseVer(v) {
  const m = String(v).match(/^(\d+)\.(\d+)\.(\d+)/);
  if (!m) {
    throw new Error(`unparseable version: ${v}`);
  }
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

function cmp(a, b) {
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) {
      return a[i] < b[i] ? -1 : 1;
    }
  }
  return 0;
}

function satisfies(version, range) {
  const ver = parseVer(version);
  const parts = range.trim().split(/\s+/);
  return parts.every((part) => {
    const m = part.match(/^(>=|>|<=|<|=)?(.+)$/);
    if (!m) {
      throw new Error(`unparseable range part: ${part}`);
    }
    const op = m[1] || "=";
    const bound = parseVer(m[2]);
    const c = cmp(ver, bound);
    switch (op) {
      case ">=":
        return c >= 0;
      case ">":
        return c > 0;
      case "<=":
        return c <= 0;
      case "<":
        return c < 0;
      default:
        return c === 0;
    }
  });
}

if (!satisfies(lockedTs, peerRange)) {
  console.error(
    `locked typescript@${lockedTs} does not satisfy typescript-eslint peer '${peerRange}'`,
  );
  process.exit(1);
}

const declaredNumeric = declaredTs.replace(/^[~^]/, "");
if (declaredNumeric && !satisfies(declaredNumeric, peerRange)) {
  console.error(
    `package.json typescript '${declaredTs}' does not satisfy typescript-eslint peer '${peerRange}'`,
  );
  process.exit(1);
}

console.log(
  `ok: typescript@${lockedTs} (declared ${declaredTs}) satisfies peer '${peerRange}'`,
);
