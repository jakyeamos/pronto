import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { digestBundle, replaceAppBundle } from "./install-app-lib.mjs";

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
