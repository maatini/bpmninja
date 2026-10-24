#!/usr/bin/env node
// Erzeugt eine SHA256-Prüfsummendatei für die Tauri-Bundles eines Ziels.
// Aufruf: node tools/release-checksums.mjs --target <triple> --name <asset-name>
// Die Ausgabe landet als bpmninja-<name>.sha256 im Workspace-Root.

import { createHash } from "node:crypto";
import { createReadStream, readFileSync } from "node:fs";
import { readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const BUNDLE_EXTENSIONS = [".deb", ".rpm", ".AppImage", ".msi", ".exe", ".dmg", ".app.tar.gz"];
const DEFAULT_ROOT = path.join("desktop-tauri", "src-tauri", "target");

// Tauri legt das Updater-Bundle auf macOS als `bpmninja-desktop.app.tar.gz` ab,
// lädt es aber als `bpmninja-desktop_<version>_<arch>.app.tar.gz` hoch (Archivname
// aus Produktname, Version und Architektur). Die Prüfsummenzeile muss den
// veröffentlichten Namen nennen, sonst findet `sha256sum -c` die Datei nicht.
const APP_TAR_SUFFIX = ".app.tar.gz";
const MACOS_ARCH = {
  "aarch64-apple-darwin": "aarch64",
  "x86_64-apple-darwin": "x64",
};

function publishedBundleName(basename, target, version) {
  if (!basename.endsWith(APP_TAR_SUFFIX)) {
    return basename;
  }
  const arch = MACOS_ARCH[target];
  if (!arch) {
    return basename;
  }
  return `bpmninja-desktop_${version}_${arch}${APP_TAR_SUFFIX}`;
}

function parseArgs(argv) {
  const options = { target: "", name: "", root: DEFAULT_ROOT };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    const value = argv[i + 1];
    if (arg === "--target" && value) {
      options.target = value;
      i += 1;
    } else if (arg === "--name" && value) {
      options.name = value;
      i += 1;
    } else if (arg === "--root" && value) {
      options.root = value;
      i += 1;
    } else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: node tools/release-checksums.mjs --target <triple> [--name <asset>] [--root <target-dir>]",
      );
      process.exit(0);
    } else {
      console.error(`Unbekanntes Argument: ${arg}`);
      process.exit(2);
    }
  }
  return options;
}

function isBundleFile(fileName) {
  return BUNDLE_EXTENSIONS.some((extension) => fileName.endsWith(extension));
}

async function collectBundles(directory) {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch {
    return [];
  }

  const files = [];
  for (const entry of entries) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectBundles(fullPath)));
    } else if (entry.isFile() && isBundleFile(entry.name)) {
      files.push(fullPath);
    }
  }
  return files;
}

function sha256(filePath) {
  return new Promise((resolve, reject) => {
    const hash = createHash("sha256");
    createReadStream(filePath)
      .on("error", reject)
      .on("data", (chunk) => hash.update(chunk))
      .on("end", () => resolve(hash.digest("hex")));
  });
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (!options.target) {
    console.error("--target ist erforderlich.");
    process.exit(2);
  }

  const name = options.name || options.target;
  const bundleDirs = [
    path.join(options.root, options.target, "release", "bundle"),
    path.join(options.root, "release", "bundle"),
  ];

  let files = [];
  let usedDir = "";
  for (const directory of bundleDirs) {
    const info = await stat(directory).catch(() => null);
    if (!info?.isDirectory()) {
      continue;
    }
    files = await collectBundles(directory);
    if (files.length > 0) {
      usedDir = directory;
      break;
    }
  }

  if (files.length === 0) {
    console.error(
      `::error::Keine Bundle-Dateien (${BUNDLE_EXTENSIONS.join(", ")}) unter ${bundleDirs.join(" oder ")} gefunden.`,
    );
    process.exit(1);
  }

  files.sort();
  const version = JSON.parse(
    readFileSync(new URL("../version.json", import.meta.url), "utf8"),
  ).version;
  const lines = [];
  for (const file of files) {
    const published = publishedBundleName(path.basename(file), options.target, version);
    lines.push(`${await sha256(file)}  ${published}`);
  }

  const outputFileName = `bpmninja-${name}.sha256`;
  await writeFile(outputFileName, `${lines.join("\n")}\n`, "utf8");

  console.log(`Prüfsummen aus ${usedDir}:`);
  for (const line of lines) {
    console.log(`  ${line}`);
  }
  console.log(`Geschrieben: ${outputFileName}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
