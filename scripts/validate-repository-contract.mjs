#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const requiredDocuments = [
  ".agents/context/README.md",
  ".agents/context/commands.md",
  "docs/repository-contract.md",
  "docs/implementation-examples.md",
];
const errors = [];

function read(relative) {
  const absolute = path.join(root, relative);
  if (!fs.existsSync(absolute)) {
    errors.push(`${relative}: required file is missing`);
    return "";
  }
  return fs.readFileSync(absolute, "utf8");
}

const documents = new Map(
  requiredDocuments.map((relative) => [relative, read(relative)]),
);
const router = documents.get(".agents/context/README.md") ?? "";
if (!/canonical branch[^\n]{0,80}`?main`?/i.test(router)) {
  errors.push(
    ".agents/context/README.md: canonical main branch is not declared",
  );
}

for (const [relative, contents] of documents) {
  for (const match of contents.matchAll(/!?\[[^\]]*\]\(([^)]+)\)/g)) {
    const target = match[1].trim().split(/\s+/, 1)[0].replace(/^<|>$/g, "");
    if (!target || /^(?:https?:|mailto:|#)/.test(target)) continue;
    const resolved = path.resolve(
      root,
      path.dirname(relative),
      target.split("#", 1)[0],
    );
    if (!fs.existsSync(resolved)) {
      errors.push(`${relative}: linked path does not exist: ${target}`);
    }
  }
}

let matrix;
try {
  matrix = JSON.parse(read(".agents/change-surface-matrix.json"));
} catch (error) {
  errors.push(
    `.agents/change-surface-matrix.json: invalid JSON: ${error.message}`,
  );
}

if (matrix) {
  if (matrix.schema_version !== "change-surface-matrix/v1") {
    errors.push(
      ".agents/change-surface-matrix.json: unsupported schema_version",
    );
  }
  if (!matrix.owner || !matrix.subject?.id) {
    errors.push(
      ".agents/change-surface-matrix.json: owner and subject.id are required",
    );
  }
  const reviewed = Date.parse(matrix.last_reviewed);
  const ninetyDays = 90 * 24 * 60 * 60 * 1000;
  if (!Number.isFinite(reviewed) || Date.now() - reviewed > ninetyDays) {
    errors.push(
      ".agents/change-surface-matrix.json: last_reviewed is missing or stale",
    );
  }
  const surfaces = Array.isArray(matrix.surfaces) ? matrix.surfaces : [];
  if (!surfaces.length) {
    errors.push(".agents/change-surface-matrix.json: surfaces are required");
  }
  for (const surface of surfaces) {
    if (surface.status === "not_applicable") continue;
    const operations = new Set(surface.operations ?? []);
    if (!["add", "change", "remove"].every((item) => operations.has(item))) {
      errors.push(
        `${surface.id ?? "unknown surface"}: add/change/remove coverage is incomplete`,
      );
    }
    if (!surface.owner || !surface.condition || !surface.validation?.length) {
      errors.push(
        `${surface.id ?? "unknown surface"}: owner, condition, and validation are required`,
      );
    }
  }
  for (const operation of ["add", "change", "remove"]) {
    const evidence = matrix.operation_evidence?.[operation];
    if (evidence?.status !== "passed" || !evidence.evidence?.length) {
      errors.push(
        `change-surface operation evidence is incomplete for ${operation}`,
      );
    }
  }
}

if (errors.length) {
  process.stderr.write(
    `${errors.map((error) => `contract error: ${error}`).join("\n")}\n`,
  );
  process.exit(1);
}

process.stdout.write(
  `Repository contract valid: ${requiredDocuments.length} routed documents and ${matrix.surfaces.length} change surfaces.\n`,
);
