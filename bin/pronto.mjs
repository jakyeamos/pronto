#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
// Bare `cargo` intentionally resolves through the guarded Codex wrapper.
// PRONTO_CARGO remains an explicit operator escape hatch for non-Codex use.
const cargo = globalThis.process.env.PRONTO_CARGO || "cargo";
const jsonRequested = globalThis.process.argv.includes("--json");
const result = spawnSync(
  cargo,
  [
    "run",
    ...(jsonRequested ? ["--quiet"] : []),
    "--manifest-path",
    join(projectRoot, "src-tauri", "Cargo.toml"),
    "--bin",
    "pronto_cli",
    "--",
    ...globalThis.process.argv.slice(2),
  ],
  { cwd: projectRoot, env: globalThis.process.env, stdio: "inherit" },
);

globalThis.process.exit(result.status ?? 1);
