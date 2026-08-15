#!/usr/bin/env node

import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const source = join(
  projectRoot,
  "src-tauri",
  "target",
  "release",
  "pronto_cli",
);
const target = globalThis.process.env.PAPERCUTS_CLI_TARGET
  ? globalThis.process.env.PAPERCUTS_CLI_TARGET
  : join(homedir(), ".codex", "bin", "pronto-papercuts");
const checkOnly = globalThis.process.argv.includes("--check");
const hookSource = join(projectRoot, "scripts", "papercuts-capture.py");

function fail(message) {
  globalThis.console.error(message);
  globalThis.process.exit(1);
}

function readContract(command, arguments_) {
  const result = spawnSync(command, arguments_, {
    encoding: "utf8",
    timeout: 3000,
  });
  if (result.status !== 0) return null;
  try {
    return JSON.parse(result.stdout);
  } catch {
    return null;
  }
}

function canonicalJson(value) {
  if (Array.isArray(value)) return value.map(canonicalJson);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, item]) => [key, canonicalJson(item)]),
    );
  }
  return value;
}

function contractsMatch(executable) {
  const hookContract = readContract("/usr/bin/python3", [
    hookSource,
    "contract",
  ]);
  const cliContract = readContract(executable, [
    "papercuts",
    "contract",
    "--json",
  ]);
  return (
    hookContract !== null &&
    cliContract !== null &&
    JSON.stringify(canonicalJson(hookContract)) ===
      JSON.stringify(canonicalJson(cliContract))
  );
}

function matchesSource() {
  return (
    existsSync(source) &&
    existsSync(target) &&
    readFileSync(source).equals(readFileSync(target)) &&
    (statSync(target).mode & 0o077) === 0 &&
    contractsMatch(target)
  );
}

if (!existsSync(source)) {
  fail(
    `Papercuts CLI build is missing at ${source}. Run pnpm papercuts:cli:build.`,
  );
}
if (checkOnly) {
  if (!matchesSource()) {
    fail(
      `Papercuts CLI is missing, stale, or not private at ${target}. Run pnpm papercuts:cli:install.`,
    );
  }
  globalThis.console.log(
    `Papercuts CLI matches the repository release binary at ${target}.`,
  );
  globalThis.process.exit(0);
}

const targetDirectory = dirname(target);
const temporary = join(
  targetDirectory,
  `.pronto-papercuts.${globalThis.process.pid}.tmp`,
);
mkdirSync(targetDirectory, { recursive: true, mode: 0o700 });
chmodSync(targetDirectory, 0o700);
try {
  copyFileSync(source, temporary);
  chmodSync(temporary, 0o700);
  renameSync(temporary, target);
} finally {
  rmSync(temporary, { force: true });
}

if (!matchesSource()) {
  fail(
    "Papercuts CLI installation did not match the release binary and observation contract.",
  );
}
globalThis.console.log(`Installed the current Papercuts CLI at ${target}.`);
