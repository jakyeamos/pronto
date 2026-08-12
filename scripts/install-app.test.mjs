import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  digestBundle,
  findExecutableProcessIds,
  replaceAppBundle,
} from "./install-app-lib.mjs";

test("selects only the exact installed app process for termination", () => {
  const executable = "/Applications/Pronto.app/Contents/MacOS/pronto";
  const processList = `
  41 ${executable}
  42 ${executable} --skill-usage-collector
  43 ${executable} --skill-usage-collector --verbose
  44 ${executable} --diagnostic
  45 /tmp/Pronto.app/Contents/MacOS/pronto
  46 ${executable}-helper
  `;

  assert.deepEqual(
    findExecutableProcessIds(processList, executable, [
      "--skill-usage-collector",
    ]),
    [41, 44],
  );
  assert.deepEqual(
    findExecutableProcessIds(processList, executable),
    [41, 42, 43, 44],
  );
});

function writeBundle(bundlePath, executableContents) {
  mkdirSync(join(bundlePath, "Contents", "MacOS"), { recursive: true });
  writeFileSync(
    join(bundlePath, "Contents", "MacOS", "pronto"),
    executableContents,
  );
  writeFileSync(join(bundlePath, "Contents", "Info.plist"), "plist");
}

test("replaces the installed bundle instead of retaining obsolete files", () => {
  const root = mkdtempSync(join(tmpdir(), "pronto-install-test-"));
  const sourceApp = join(root, "source", "Pronto.app");
  const targetApp = join(root, "Applications", "Pronto.app");

  try {
    writeBundle(sourceApp, "current executable");
    writeBundle(targetApp, "stale executable");
    mkdirSync(join(targetApp, "Contents", "_CodeSignature"), {
      recursive: true,
    });
    writeFileSync(
      join(targetApp, "Contents", "_CodeSignature", "CodeResources"),
      "obsolete signature",
    );

    replaceAppBundle(sourceApp, targetApp);

    assert.equal(digestBundle(targetApp), digestBundle(sourceApp));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
