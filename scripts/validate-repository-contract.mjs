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
  "docs/ci-gate-profiles.md",
  "docs/implementation-examples.md",
  "docs/agent-usability-maturity.md",
];
const evidenceContractPath = ".agents/environment-legibility.json";
const agentUsabilityContractPath = ".agents/agent-usability.json";
const automationFiles = new Set([
  ".circleci/config.yml",
  ".circleci/config.yaml",
  ".gitlab-ci.yml",
  ".gitlab-ci.yaml",
  ".github/CODEOWNERS",
  ".pre-commit-config.yaml",
  ".pre-commit-config.yml",
  "CODEOWNERS",
  "Jenkinsfile",
  "azure-pipelines.yml",
  "azure-pipelines.yaml",
]);
const validationRoots = new Set([
  "bin",
  "script",
  "scripts",
  "spec",
  "specs",
  "test",
  "tests",
  "tools",
]);
const requiredDimensionHeadings = {
  architecture_boundaries: "## Architecture and ownership boundaries",
  coding_conventions: "## Coding conventions",
  context_routing: "Read the deeper file only when the task matches it:",
  security_constraints: "## Security and credential constraints",
  failure_modes: "## Common failure modes and recovery",
  implementation_examples: "## Add or change an evidence projection",
  definition_of_done: "## Definition of done",
  approval_gated_paths: "## Approval-gated paths and operations",
  deployment_rollback: "## Installation, release, and rollback",
};
const errors = [];

function read(relative) {
  const absolute = path.join(root, relative);
  if (!fs.existsSync(absolute)) {
    errors.push(`${relative}: required file is missing`);
    return "";
  }
  return fs.readFileSync(absolute, "utf8");
}

function readContractPath(relative, label) {
  if (typeof relative !== "string" || !relative.trim()) {
    errors.push(`${label}: path is missing`);
    return "";
  }
  const parts = relative.split(/[\\/]+/);
  if (path.isAbsolute(relative) || parts.includes("..")) {
    errors.push(`${label}: path is outside the repository: ${relative}`);
    return "";
  }
  let current = root;
  for (const part of parts) {
    current = path.join(current, part);
    if (fs.existsSync(current) && fs.lstatSync(current).isSymbolicLink()) {
      errors.push(`${label}: symlinked paths are not allowed: ${relative}`);
      return "";
    }
  }
  const absolute = path.resolve(root, relative);
  if (!absolute.startsWith(`${root}${path.sep}`) || !fs.existsSync(absolute)) {
    errors.push(`${label}: referenced file is missing or unsafe: ${relative}`);
    return "";
  }
  if (!fs.statSync(absolute).isFile()) {
    errors.push(`${label}: referenced path is not a file: ${relative}`);
    return "";
  }
  return fs.readFileSync(absolute, "utf8");
}

function isValidationSurface(relative) {
  const normalized = relative.replaceAll("\\", "/");
  const parts = normalized.split("/");
  const name = parts.at(-1)?.toLowerCase() ?? "";
  return (
    validationRoots.has(parts[0]?.toLowerCase()) ||
    name.startsWith("test_") ||
    name.endsWith("_test.py") ||
    name.includes(".test.") ||
    name.includes(".spec.") ||
    normalized.startsWith(".github/actions/")
  );
}

function isAutomationSurface(relative) {
  const normalized = relative.replaceAll("\\", "/");
  return (
    automationFiles.has(normalized) ||
    /^\.github\/workflows\/[^/]+\.ya?ml$/.test(normalized)
  );
}

const documents = new Map(
  requiredDocuments.map((relative) => [relative, read(relative)]),
);
const router = documents.get(".agents/context/README.md") ?? "";
const repositoryContract = documents.get("docs/repository-contract.md") ?? "";
const implementationExamples =
  documents.get("docs/implementation-examples.md") ?? "";
if (!/canonical branch[^\n]{0,80}`?main`?/i.test(router)) {
  errors.push(
    ".agents/context/README.md: canonical main branch is not declared",
  );
}

for (const [dimension, heading] of Object.entries(requiredDimensionHeadings)) {
  const contents =
    dimension === "implementation_examples"
      ? implementationExamples
      : dimension === "context_routing"
        ? router
        : repositoryContract;
  if (!contents.includes(heading)) {
    errors.push(
      `${dimension}: required contract heading is missing: ${heading}`,
    );
  }
}

for (const heading of [
  "## Add or change an evidence projection",
  "## Add or change a quality or remediation rule",
  "## Add or change a repository surface",
  "## Preserve before folding",
]) {
  if (!implementationExamples.includes(heading)) {
    errors.push(
      `implementation examples: required heading is missing: ${heading}`,
    );
  }
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

let evidenceContract;
try {
  evidenceContract = JSON.parse(read(evidenceContractPath));
} catch (error) {
  errors.push(`${evidenceContractPath}: invalid JSON: ${error.message}`);
}

if (evidenceContract) {
  if (
    evidenceContract.schema_version !==
    "quality-runner-environment-legibility/v1"
  ) {
    errors.push(`${evidenceContractPath}: unsupported schema_version`);
  }
  if (!evidenceContract.owner) {
    errors.push(`${evidenceContractPath}: owner is required`);
  }
  const reviewed = Date.parse(evidenceContract.last_reviewed);
  const ninetyDays = 90 * 24 * 60 * 60 * 1000;
  if (
    !Number.isFinite(reviewed) ||
    reviewed > Date.now() ||
    Date.now() - reviewed > ninetyDays
  ) {
    errors.push(
      `${evidenceContractPath}: last_reviewed is missing, future-dated, or stale`,
    );
  }

  const dimensions = evidenceContract.dimensions ?? {};
  for (const dimension of Object.keys(requiredDimensionHeadings)) {
    const entry = dimensions[dimension];
    if (!entry) {
      errors.push(`${evidenceContractPath}: ${dimension} is missing`);
      continue;
    }
    for (const key of ["evidence", "validation", "automation"]) {
      if (!Array.isArray(entry[key]) || entry[key].length === 0) {
        errors.push(
          `${evidenceContractPath}: ${dimension}.${key} must be non-empty`,
        );
      }
    }
    for (const evidencePath of entry.evidence ?? []) {
      readContractPath(evidencePath, `${dimension}.evidence`);
    }
    for (const key of ["validation", "automation"]) {
      for (const assertion of entry[key] ?? []) {
        if (
          !assertion ||
          typeof assertion !== "object" ||
          typeof assertion.path !== "string"
        ) {
          errors.push(
            `${evidenceContractPath}: ${dimension}.${key} assertion is invalid`,
          );
          continue;
        }
        const allowed =
          key === "validation"
            ? isValidationSurface(assertion.path)
            : isAutomationSurface(assertion.path);
        if (!allowed) {
          errors.push(
            `${evidenceContractPath}: ${dimension}.${key} path is not an allowed surface: ${assertion.path}`,
          );
          continue;
        }
        const contents = readContractPath(
          assertion.path,
          `${dimension}.${key}`,
        );
        if (
          !Array.isArray(assertion.contains) ||
          assertion.contains.length === 0
        ) {
          errors.push(
            `${evidenceContractPath}: ${dimension}.${key} assertion terms are missing`,
          );
          continue;
        }
        for (const term of assertion.contains) {
          if (typeof term !== "string" || !contents.includes(term)) {
            errors.push(
              `${evidenceContractPath}: ${dimension}.${key} assertion did not match ${assertion.path}: ${term}`,
            );
          }
        }
      }
    }
  }
}

let agentUsabilityContract;
try {
  agentUsabilityContract = JSON.parse(read(agentUsabilityContractPath));
} catch (error) {
  errors.push(`${agentUsabilityContractPath}: invalid JSON: ${error.message}`);
}

if (agentUsabilityContract) {
  if (agentUsabilityContract.schema !== "agent-usability/v1") {
    errors.push(`${agentUsabilityContractPath}: unsupported schema`);
  }
  const reviewed = Date.parse(agentUsabilityContract.reviewed_at);
  const ninetyDays = 90 * 24 * 60 * 60 * 1000;
  if (
    !Number.isFinite(reviewed) ||
    reviewed > Date.now() ||
    Date.now() - reviewed > ninetyDays
  ) {
    errors.push(
      `${agentUsabilityContractPath}: reviewed_at is missing, future-dated, or stale`,
    );
  }
  const skills = Array.isArray(agentUsabilityContract.skills)
    ? agentUsabilityContract.skills
    : [];
  const skillIds = new Set();
  for (const skill of skills) {
    if (!skill?.id || skillIds.has(skill.id)) {
      errors.push(`${agentUsabilityContractPath}: skill IDs must be unique`);
      continue;
    }
    skillIds.add(skill.id);
    if (!skill.family || !["hosted", "projected"].includes(skill.source)) {
      errors.push(
        `${skill.id}: family and hosted/projected source are required`,
      );
    }
    if (skill.source === "hosted") {
      readContractPath(skill.contract_path, `${skill.id}.contract_path`);
    }
  }
  const tools = Array.isArray(agentUsabilityContract.tools)
    ? agentUsabilityContract.tools
    : [];
  if (!tools.length) {
    errors.push(`${agentUsabilityContractPath}: tools are required`);
  }
  const toolIds = new Set();
  for (const tool of tools) {
    if (!tool?.id || toolIds.has(tool.id)) {
      errors.push(`${agentUsabilityContractPath}: tool IDs must be unique`);
      continue;
    }
    toolIds.add(tool.id);
    if (!Array.isArray(tool.documentation) || !tool.documentation.length) {
      errors.push(`${tool.id}: documentation references are required`);
    }
    for (const documentPath of tool.documentation ?? []) {
      readContractPath(documentPath, `${tool.id}.documentation`);
    }
    if (!Array.isArray(tool.skills) || !tool.skills.length) {
      errors.push(`${tool.id}: skill references are required`);
    }
    for (const skillId of tool.skills ?? []) {
      if (!skillIds.has(skillId)) {
        errors.push(`${tool.id}: unknown skill reference: ${skillId}`);
      }
    }
    if (!Array.isArray(tool.behavior_evidence)) {
      errors.push(`${tool.id}: behavior_evidence must be an array`);
    }
    for (const item of tool.behavior_evidence ?? []) {
      const evidencePath = typeof item === "string" ? item : item?.path;
      readContractPath(evidencePath, `${tool.id}.behavior_evidence`);
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
  `Repository contract valid: ${requiredDocuments.length} routed documents, ${Object.keys(requiredDimensionHeadings).length} maturity dimensions, and ${matrix.surfaces.length} change surfaces.\n`,
);
