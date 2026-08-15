import assert from "node:assert/strict";
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { URL } from "node:url";

const installer = new URL("./install-papercuts-hook.mjs", import.meta.url);

function runInstaller(target, ...args) {
  return spawnSync(process.execPath, [installer.pathname, ...args], {
    encoding: "utf8",
    env: { ...process.env, PAPERCUTS_HOOK_TARGET: target },
  });
}

test("installs and verifies the entrypoint with its private runtime package", (t) => {
  const root = mkdtempSync(join(tmpdir(), "pronto-papercuts-hook-"));
  t.after(() => rmSync(root, { recursive: true }));
  const target = join(root, "papercuts-capture.py");

  assert.equal(runInstaller(target).status, 0);
  assert.equal(runInstaller(target, "--check").status, 0);
  assert.match(
    readFileSync(join(root, "papercuts_capture", "runtime.py"), "utf8"),
    /def persist/,
  );
  const contract = spawnSync("/usr/bin/python3", [target, "contract"], {
    encoding: "utf8",
  });
  assert.equal(contract.status, 0);
  assert.equal(
    JSON.parse(contract.stdout).schema_version,
    "pronto-papercuts-observation/v1",
  );
});

test("check rejects a stale runtime module even when the entrypoint matches", (t) => {
  const root = mkdtempSync(join(tmpdir(), "pronto-papercuts-hook-"));
  t.after(() => rmSync(root, { recursive: true }));
  const target = join(root, "papercuts-capture.py");

  assert.equal(runInstaller(target).status, 0);
  const runtime = join(root, "papercuts_capture", "runtime.py");
  writeFileSync(runtime, "# stale\n", "utf8");
  chmodSync(runtime, 0o600);

  const check = runInstaller(target, "--check");
  assert.equal(check.status, 1);
  assert.match(check.stderr, /missing, stale, or not private/);
});
