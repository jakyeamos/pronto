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

function fail(message) {
  globalThis.console.error(message);
  globalThis.process.exit(1);
}

function matchesSource() {
  return (
    existsSync(source) &&
    existsSync(target) &&
    readFileSync(source).equals(readFileSync(target)) &&
    (statSync(target).mode & 0o077) === 0
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
    "Papercuts CLI installation did not match the repository release binary.",
  );
}
globalThis.console.log(`Installed the current Papercuts CLI at ${target}.`);
