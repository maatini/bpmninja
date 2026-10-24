#!/usr/bin/env node
/**
 * Propagates the version from version.json into all product manifests.
 *
 * Targeted text replacements keep the existing formatting of every file
 * intact (no JSON re-serialization). The script is idempotent; `--check`
 * reports drift instead of writing and exits non-zero.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const checkOnly = process.argv.includes("--check");

const { version } = JSON.parse(readFileSync(join(root, "version.json"), "utf8"));

if (typeof version !== "string" || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`version.json: invalid version ${JSON.stringify(version)}`);
  process.exit(1);
}

// Only the `version` key inside [package]; `version.workspace = true` is left alone.
function setCargoPackageVersion(text) {
  const lines = text.split("\n");
  let inPackage = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i].trim();
    if (line.startsWith("[") && line.endsWith("]")) {
      inPackage = line === "[package]";
      continue;
    }
    if (!inPackage) {
      continue;
    }
    const match = lines[i].match(/^(\s*version\s*=\s*")([^"]*)(".*)$/);
    if (match) {
      lines[i] = `${match[1]}${version}${match[3]}`;
      return lines.join("\n");
    }
  }
  return null;
}

function replaceFirstVersionField(text) {
  const pattern = /^(\s*"version"\s*:\s*")([^"]*)(")/m;
  if (!pattern.test(text)) {
    return null;
  }
  return text.replace(pattern, `$1${version}$3`);
}

// npm lockfiles carry the package version twice: top-level and packages[""].
function setLockfileVersions(text) {
  const packagesIndex = text.indexOf('"packages": {');
  const rootEntryIndex = text.indexOf('\n    "": {', packagesIndex);
  if (packagesIndex === -1 || rootEntryIndex === -1) {
    return null;
  }
  const head = replaceFirstVersionField(text.slice(0, packagesIndex));
  const middle = text.slice(packagesIndex, rootEntryIndex);
  const tail = replaceFirstVersionField(text.slice(rootEntryIndex));
  if (head === null || tail === null) {
    return null;
  }
  return head + middle + tail;
}

function setOpenapiInfoVersion(text) {
  const pattern = /^(\s{2}version:\s*)(\S+)\s*$/m;
  if (!pattern.test(text)) {
    return null;
  }
  return text.replace(pattern, `$1${version}`);
}

function setDocsVersion(text) {
  const pattern = /(\*\*Version:\*\*\s*)(\d+\.\d+\.\d+)/;
  if (!pattern.test(text)) {
    return null;
  }
  return text.replace(pattern, `$1${version}`);
}

// CLAUDE.md pins the product version in two places (version bullet and tag).
function setClaudeVersion(text) {
  const bullet = /(Alle Crates synchron auf Version \*\*)(\d+\.\d+\.\d+)(\*\*)/;
  const tag = /(Git-Tags:\s*`v)(\d+\.\d+\.\d+)(`)/;
  if (!bullet.test(text) || !tag.test(text)) {
    return null;
  }
  return text.replace(bullet, `$1${version}$3`).replace(tag, `$1${version}$3`);
}

// Cargo.lock carries the version of every workspace crate in its [[package]]
// block. Without this, `cargo --locked` fails after a version bump.
function setCargoLockVersions(packageNames) {
  return (text) => {
    const lines = text.split("\n");
    const found = new Set();
    let currentName = null;
    for (let i = 0; i < lines.length; i += 1) {
      const nameMatch = lines[i].match(/^name = "([^"]+)"$/);
      if (nameMatch) {
        currentName = nameMatch[1];
        continue;
      }
      if (!currentName || !packageNames.includes(currentName)) {
        continue;
      }
      const versionMatch = lines[i].match(/^(version = ")([^"]*)(")$/);
      if (versionMatch) {
        lines[i] = `${versionMatch[1]}${version}${versionMatch[3]}`;
        found.add(currentName);
      }
    }
    if (found.size !== packageNames.length) {
      return null;
    }
    return lines.join("\n");
  };
}

const targets = [
  ...["engine-core", "bpmn-parser", "persistence-nats", "persistence-memory", "agent-orchestrator", "engine-server"].map((crate) => ({
    path: `${crate}/Cargo.toml`,
    update: setCargoPackageVersion,
  })),
  {
    path: "Cargo.lock",
    update: setCargoLockVersions([
      "engine-core",
      "bpmn-parser",
      "persistence-nats",
      "persistence-memory",
      "agent-orchestrator",
      "engine-server",
    ]),
  },
  {
    path: "fuzz/Cargo.lock",
    update: setCargoLockVersions(["engine-core", "bpmn-parser", "persistence-memory"]),
  },
  { path: "desktop-tauri/src-tauri/Cargo.toml", update: setCargoPackageVersion },
  {
    path: "desktop-tauri/src-tauri/Cargo.lock",
    update: setCargoLockVersions(["bpmninja-desktop"]),
  },
  { path: "desktop-tauri/src-tauri/tauri.conf.json", update: replaceFirstVersionField },
  { path: "desktop-tauri/package.json", update: replaceFirstVersionField },
  { path: "desktop-tauri/package-lock.json", update: setLockfileVersions },
  { path: "api-spec/package.json", update: replaceFirstVersionField },
  { path: "api-spec/package-lock.json", update: setLockfileVersions },
  {
    path: "bpmn-ninja-external-task-client/package.json",
    update: replaceFirstVersionField,
  },
  {
    path: "bpmn-ninja-external-task-client/package-lock.json",
    update: setLockfileVersions,
  },
  { path: ".agent/manifest.json", update: replaceFirstVersionField },
  { path: "docs/openapi.yaml", update: setOpenapiInfoVersion },
  { path: "docs/knowledge-base/overview.md", update: setDocsVersion },
  {
    path: "docs/knowledge-base/modules/engine-core/dependencies.md",
    update: setCargoPackageVersion,
  },
  { path: "CLAUDE.md", update: setClaudeVersion },
];

const drifted = [];
const unreadable = [];

for (const target of targets) {
  const file = join(root, target.path);
  let before;
  try {
    before = readFileSync(file, "utf8");
  } catch (error) {
    unreadable.push(`${target.path} (${error.code ?? error.message})`);
    continue;
  }
  const after = target.update(before);
  if (after === null) {
    unreadable.push(`${target.path} (version marker not found)`);
    continue;
  }
  if (after === before) {
    continue;
  }
  drifted.push(target.path);
  if (!checkOnly) {
    writeFileSync(file, after);
  }
}

for (const entry of unreadable) {
  console.error(`error: ${entry}`);
}

if (checkOnly) {
  for (const path of drifted) {
    console.error(`drift: ${path} is not at ${version}`);
  }
  if (drifted.length === 0 && unreadable.length === 0) {
    console.log(`ok: all ${targets.length} files are at version ${version}`);
  }
} else {
  for (const path of drifted) {
    console.log(`updated: ${path}`);
  }
  console.log(
    `version ${version}: ${drifted.length} file(s) updated, ${targets.length - drifted.length} already in sync`,
  );
}

const failed = unreadable.length > 0 || (checkOnly && drifted.length > 0);
process.exit(failed ? 1 : 0);
