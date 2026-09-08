import process from "node:process";
import { spawnSync } from "node:child_process";

// Bounded proof for the Compass pilot; no installation, refresh, or execution queue.
const commands = [
  [
    "cargo",
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "project_compass",
      "--lib",
      "--quiet",
    ],
  ],
  [
    "cargo",
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "route_projection",
      "--lib",
      "--quiet",
    ],
  ],
  [
    "pnpm",
    [
      "exec",
      "vitest",
      "run",
      "src/renderer/src/components/CompassFamilyDetail.test.tsx",
    ],
  ],
];
for (const [command, args] of commands) {
  const result = spawnSync(command, args, { stdio: "inherit", timeout: 90000 });
  if (result.error || result.status !== 0) {
    console.error(
      result.error?.message ?? `${command} exited ${result.status}`,
    );
    process.exit(1);
  }
}
