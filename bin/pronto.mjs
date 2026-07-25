#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const result = spawnSync(
  "cargo",
  [
    "run",
    "--manifest-path",
    join(projectRoot, "src-tauri", "Cargo.toml"),
    "--bin",
    "pronto_cli",
    "--",
    ...globalThis.process.argv.slice(2),
  ],
  { cwd: projectRoot, stdio: "inherit" },
);

globalThis.process.exit(result.status ?? 1);
