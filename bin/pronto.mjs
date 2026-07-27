#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const cargoCandidates = [
  globalThis.process.env.PRONTO_CARGO,
  "/opt/homebrew/bin/cargo",
  "/usr/local/bin/cargo",
  "cargo",
].filter((candidate) => {
  if (!candidate || !candidate.includes("/")) return true;
  return existsSync(candidate);
});
const cargo = cargoCandidates[0] ?? "cargo";
const environment = { ...globalThis.process.env };
if (existsSync("/opt/homebrew/bin")) {
  environment.PATH = `/opt/homebrew/bin:${environment.PATH ?? ""}`;
}
const result = spawnSync(
  cargo,
  [
    "run",
    "--manifest-path",
    join(projectRoot, "src-tauri", "Cargo.toml"),
    "--bin",
    "pronto_cli",
    "--",
    ...globalThis.process.argv.slice(2),
  ],
  { cwd: projectRoot, env: environment, stdio: "inherit" },
);

globalThis.process.exit(result.status ?? 1);
