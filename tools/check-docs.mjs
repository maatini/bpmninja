#!/usr/bin/env node
/**
 * Validates documentation hygiene without external dependencies:
 * - every relative Markdown link in README.md and docs/**\/*.md resolves
 * - docs/openapi.yaml `info.version` matches version.json
 * - docs/knowledge-base/overview.md `**Version:**` line mentions that version
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const { version } = JSON.parse(readFileSync(join(root, "version.json"), "utf8"));
const problems = [];

// Links inside code fences or inline code are examples, not references.
function maskCode(text) {
  return text
    .replace(/```[\s\S]*?```/g, (block) => block.replace(/[^\n]/g, " "))
    .replace(/`[^`\n]*`/g, (span) => span.replace(/[^\n]/g, " "));
}

function collectMarkdown(dir) {
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith(".") || entry.name === "node_modules") {
      continue;
    }
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectMarkdown(full));
    } else if (entry.name.endsWith(".md")) {
      files.push(full);
    }
  }
  return files;
}

function lineOf(text, index) {
  return text.slice(0, index).split("\n").length;
}

const markdownFiles = [];
for (const candidate of [join(root, "README.md"), join(root, "docs")]) {
  if (!existsSync(candidate)) {
    continue;
  }
  if (candidate.endsWith(".md")) {
    markdownFiles.push(candidate);
  } else {
    markdownFiles.push(...collectMarkdown(candidate));
  }
}

const linkPattern = /!?\[[^\]]*\]\(\s*(<[^>]*>|[^)\s]+)/g;
let linkCount = 0;

for (const file of markdownFiles) {
  const text = readFileSync(file, "utf8");
  const masked = maskCode(text);
  for (const match of masked.matchAll(linkPattern)) {
    let target = match[1];
    if (target.startsWith("<") && target.endsWith(">")) {
      target = target.slice(1, -1);
    }
    if (
      target === "" ||
      target.startsWith("#") ||
      target.startsWith("/") ||
      /^[a-z][a-z0-9+.-]*:/i.test(target)
    ) {
      continue;
    }
    const pathPart = target.split("#")[0].split("?")[0];
    if (pathPart === "") {
      continue;
    }
    let decoded = pathPart;
    try {
      decoded = decodeURIComponent(pathPart);
    } catch {
      // keep the raw path; it will simply fail the existence check
    }
    linkCount += 1;
    const resolved = resolve(dirname(file), decoded);
    if (!existsSync(resolved)) {
      const relative = file.slice(root.length + 1);
      problems.push(`${relative}:${lineOf(masked, match.index)} broken link -> ${target}`);
    }
  }
}

const openapiPath = join(root, "docs/openapi.yaml");
if (!existsSync(openapiPath)) {
  problems.push("docs/openapi.yaml is missing");
} else {
  const openapi = readFileSync(openapiPath, "utf8");
  const match = openapi.match(/^\s{2}version:\s*(\S+)\s*$/m);
  if (!match) {
    problems.push("docs/openapi.yaml has no info.version");
  } else if (match[1] !== version) {
    problems.push(`docs/openapi.yaml info.version is ${match[1]}, expected ${version}`);
  }
}

const overviewPath = join(root, "docs/knowledge-base/overview.md");
if (existsSync(overviewPath)) {
  const overview = readFileSync(overviewPath, "utf8");
  const versionLine = overview.match(/^\*\*Version:\*\*.*$/m);
  if (versionLine) {
    const escaped = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    if (!new RegExp(`(^|[^\\d.])${escaped}([^\\d.]|$)`).test(versionLine[0])) {
      problems.push(
        `docs/knowledge-base/overview.md Version line does not mention ${version}: ${versionLine[0]}`,
      );
    }
  }
}

if (problems.length > 0) {
  console.error(`docs check failed with ${problems.length} problem(s):`);
  for (const problem of problems) {
    console.error(`  - ${problem}`);
  }
  process.exit(1);
}

console.log(`ok: ${markdownFiles.length} markdown file(s), ${linkCount} relative link(s), versions at ${version}`);
