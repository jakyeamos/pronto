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
const runtimeSource = join(projectRoot, "scripts", "papercuts_capture");
const runtimeFiles = ["__init__.py", "common.py", "intake.py", "runtime.py"];
const target = globalThis.process.env.PAPERCUTS_HOOK_TARGET
  ? globalThis.process.env.PAPERCUTS_HOOK_TARGET
  : join(homedir(), ".codex", "hooks", "papercuts-capture.py");
const checkOnly = globalThis.process.argv.includes("--check");
const runtimeTarget = join(dirname(target), "papercuts_capture");

function matchesSource() {
  return (
    existsSync(target) &&
    readFileSync(source).equals(readFileSync(target)) &&
    (statSync(target).mode & 0o077) === 0 &&
    runtimeFiles.every((file) => {
      const deployed = join(runtimeTarget, file);
      return (
        existsSync(deployed) &&
        readFileSync(join(runtimeSource, file)).equals(
          readFileSync(deployed),
        ) &&
        (statSync(deployed).mode & 0o077) === 0
      );
    })
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
mkdirSync(runtimeTarget, { recursive: true, mode: 0o700 });
chmodSync(runtimeTarget, 0o700);
try {
  for (const file of runtimeFiles) {
    const deployed = join(runtimeTarget, file);
    const runtimeTemporary = join(
      runtimeTarget,
      `.${file}.${globalThis.process.pid}.tmp`,
    );
    try {
      copyFileSync(join(runtimeSource, file), runtimeTemporary);
      chmodSync(runtimeTemporary, 0o600);
      renameSync(runtimeTemporary, deployed);
    } finally {
      rmSync(runtimeTemporary, { force: true });
    }
  }
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
