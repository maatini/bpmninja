#!/usr/bin/env node
/**
 * Fails when the Rust Tauri crates and the locked npm @tauri-apps packages
 * disagree on major.minor. The bundle ships JS bindings, so a drifted minor
 * means the webview calls commands the installed crate does not provide.
 */
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const pairs = [
  ["tauri", "@tauri-apps/api"],
  ["tauri-plugin-dialog", "@tauri-apps/plugin-dialog"],
  ["tauri-plugin-fs", "@tauri-apps/plugin-fs"],
];

const lockPath = join(root, "package-lock.json");
if (!existsSync(lockPath)) {
  console.error(
    "package-lock.json missing: run `npm install --package-lock-only` in desktop-tauri first",
  );
  process.exit(1);
}

const lock = JSON.parse(readFileSync(lockPath, "utf8"));
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const cargo = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");

function cargoDependencies() {
  const deps = new Map();
  let inDeps = false;
  for (const line of cargo.split(/\r?\n/)) {
    const header = line.match(/^\s*\[([^\]]+)\]\s*$/);
    if (header) {
      inDeps = header[1].trim() === "dependencies";
      continue;
    }
    if (!inDeps) {
      continue;
    }
    const kv = line.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*(.+?)\s*$/);
    if (kv) {
      deps.set(kv[1], kv[2]);
    }
  }
  return deps;
}

function cargoConstraint(raw, crate) {
  if (!raw) {
    throw new Error(`src-tauri/Cargo.toml [dependencies] has no '${crate}'`);
  }
  const spec = raw.startsWith("{")
    ? raw.match(/version\s*=\s*"([^"]+)"/)?.[1]
    : raw.match(/^"([^"]+)"$/)?.[1];
  if (!spec) {
    throw new Error(`src-tauri/Cargo.toml '${crate}' has no parseable version: ${raw}`);
  }
  return spec;
}

function minorOf(version, label) {
  const m = String(version).match(/^v?(\d+)\.(\d+)/);
  if (!m) {
    throw new Error(`unparseable ${label} version: ${version}`);
  }
  return `${m[1]}.${m[2]}`;
}

function npmResolved(name, declared) {
  const locked = lock.packages?.[`node_modules/${name}`]?.version;
  const pinned = declared && /^\d+\.\d+\.\d+$/.test(declared);
  if (!pinned && !locked) {
    throw new Error(
      `${name} is declared as '${declared ?? "(missing)"}' and has no package-lock.json resolution`,
    );
  }
  return { version: pinned ? declared : locked, source: pinned ? "pinned" : "lockfile" };
}

const problems = [];

try {
  const deps = cargoDependencies();

  for (const [crate, npmName] of pairs) {
    const crateMinor = minorOf(cargoConstraint(deps.get(crate), crate), crate);
    const { version: npmVersion, source } = npmResolved(npmName, pkg.dependencies?.[npmName]);
    const npmMinor = minorOf(npmVersion, npmName);
    const status = crateMinor === npmMinor ? "ok" : "MISMATCH";

    console.log(`${status} ${crate} ${crateMinor} <-> ${npmName} ${npmMinor} (${npmVersion}, ${source})`);
    if (status !== "ok") {
      problems.push(`${crate} ${crateMinor} vs ${npmName} ${npmMinor} (${npmVersion})`);
    }

    // npm tolerates a drifted copy nested under its dependents, which is what
    // broke the v0.7.24 bundle; a top-level-only comparison would miss it.
    for (const [path, entry] of Object.entries(lock.packages ?? {})) {
      if (!path.endsWith(`/node_modules/${npmName}`) || !entry.version) {
        continue;
      }
      const nestedMinor = minorOf(entry.version, path);
      if (nestedMinor !== npmMinor) {
        console.log(`MISMATCH nested ${npmName} ${nestedMinor} (${entry.version}, ${path})`);
        problems.push(`nested ${path} ${entry.version} vs ${npmName} ${npmMinor}`);
      }
    }
  }
} catch (error) {
  console.error(`cannot verify Tauri parity: ${error.message}`);
  process.exit(1);
}

if (problems.length > 0) {
  console.error(`Tauri parity broken:\n  ${problems.join("\n  ")}`);
  process.exit(1);
}
