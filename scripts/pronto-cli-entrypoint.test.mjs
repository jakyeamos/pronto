import assert from "node:assert/strict";
import {
  chmod,
  mkdtemp,
  mkdir,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, test } from "node:test";
import { spawnSync } from "node:child_process";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcher = join(projectRoot, "bin", "pronto.mjs");
const temporaryRoots = [];

afterEach(async () => {
  await Promise.all(
    temporaryRoots
      .splice(0)
      .map((path) => rm(path, { force: true, recursive: true })),
  );
});

async function createFakeCargo(root) {
  const path = join(root, "fake-cargo.mjs");
  await writeFile(
    path,
    `#!/usr/bin/env node
import { writeFileSync } from "node:fs";
writeFileSync(process.env.PRONTO_LAUNCH_CAPTURE, JSON.stringify({
  args: process.argv.slice(2),
  cwd: process.cwd(),
}));
process.exit(Number(process.env.PRONTO_FAKE_EXIT_CODE ?? 0));
`,
  );
  await chmod(path, 0o755);
  return path;
}

test("launches the checkout-native CLI independently of the caller pnpm version", async () => {
  const root = await mkdtemp(join(tmpdir(), "pronto-cli-entrypoint-"));
  temporaryRoots.push(root);
  const caller = join(root, "caller");
  const capture = join(root, "capture.json");
  await mkdir(caller);
  await writeFile(
    join(caller, "package.json"),
    JSON.stringify({ packageManager: "pnpm@11.20.0" }),
  );
  const cargo = await createFakeCargo(root);

  const result = spawnSync(
    globalThis.process.execPath,
    [launcher, "route", "/example/repository", "--json"],
    {
      cwd: caller,
      encoding: "utf8",
      env: {
        ...globalThis.process.env,
        PRONTO_CARGO: cargo,
        PRONTO_LAUNCH_CAPTURE: capture,
      },
    },
  );

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, "");
  const invocation = JSON.parse(await readFile(capture, "utf8"));
  assert.equal(invocation.cwd, projectRoot);
  assert.deepEqual(invocation.args, [
    "run",
    "--quiet",
    "--manifest-path",
    join(projectRoot, "src-tauri", "Cargo.toml"),
    "--bin",
    "pronto_cli",
    "--",
    "route",
    "/example/repository",
    "--json",
  ]);
});

test("keeps human-readable commands unsuppressed", async () => {
  const root = await mkdtemp(join(tmpdir(), "pronto-cli-entrypoint-"));
  temporaryRoots.push(root);
  const capture = join(root, "capture.json");
  const cargo = await createFakeCargo(root);

  const result = spawnSync(globalThis.process.execPath, [launcher, "help"], {
    cwd: root,
    encoding: "utf8",
    env: {
      ...globalThis.process.env,
      PRONTO_CARGO: cargo,
      PRONTO_LAUNCH_CAPTURE: capture,
    },
  });

  assert.equal(result.status, 0, result.stderr);
  const invocation = JSON.parse(await readFile(capture, "utf8"));
  assert.equal(invocation.args.includes("--quiet"), false);
  assert.deepEqual(invocation.args.slice(-2), ["--", "help"]);
});

test("propagates a native CLI failure", async () => {
  const root = await mkdtemp(join(tmpdir(), "pronto-cli-entrypoint-"));
  temporaryRoots.push(root);
  const capture = join(root, "capture.json");
  const cargo = await createFakeCargo(root);

  const result = spawnSync(
    globalThis.process.execPath,
    [launcher, "doctor", "--json"],
    {
      cwd: root,
      encoding: "utf8",
      env: {
        ...globalThis.process.env,
        PRONTO_CARGO: cargo,
        PRONTO_FAKE_EXIT_CODE: "23",
        PRONTO_LAUNCH_CAPTURE: capture,
      },
    },
  );

  assert.equal(result.status, 23);
  const invocation = JSON.parse(await readFile(capture, "utf8"));
  assert.deepEqual(invocation.args.slice(-3), ["--", "doctor", "--json"]);
});
