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
const source = join(projectRoot, "scripts", "papercuts-capture.py");
const target = globalThis.process.env.PAPERCUTS_HOOK_TARGET
  ? globalThis.process.env.PAPERCUTS_HOOK_TARGET
  : join(homedir(), ".codex", "hooks", "papercuts-capture.py");
const checkOnly = globalThis.process.argv.includes("--check");

function matchesSource() {
  return (
    existsSync(target) &&
    readFileSync(source).equals(readFileSync(target)) &&
    (statSync(target).mode & 0o077) === 0
  );
}

if (checkOnly) {
  if (!matchesSource()) {
    globalThis.console.error(
      `Papercuts hook is missing, stale, or not private at ${target}. Run pnpm papercuts:hook:install.`,
    );
    globalThis.process.exit(1);
  }
  globalThis.console.log(
    `Papercuts hook matches the repository source at ${target}.`,
  );
  globalThis.process.exit(0);
}

const targetDirectory = dirname(target);
const temporary = join(
  targetDirectory,
  `.papercuts-capture.${globalThis.process.pid}.tmp`,
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
  globalThis.console.error(
    "Papercuts hook installation did not match the repository source.",
  );
  globalThis.process.exit(1);
}
globalThis.console.log(`Installed the current Papercuts hook at ${target}.`);
