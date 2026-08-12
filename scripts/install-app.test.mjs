import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  classifyInstalledAppProcesses,
  digestBundle,
  replaceAppBundle,
} from "./install-app-lib.mjs";

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

test("distinguishes the foreground app from its launchd collector", () => {
  const executable = "/Applications/Pronto.app/Contents/MacOS/pronto";
  const processList = [
    `${executable} --skill-usage-collector`,
    executable,
    "/Applications/Other.app/Contents/MacOS/other",
  ].join("\n");

  assert.deepEqual(classifyInstalledAppProcesses(processList, executable), {
    foregroundRunning: true,
    collectorRunning: true,
  });
});

test("does not mistake the launchd collector for the foreground app", () => {
  const executable = "/Applications/Pronto.app/Contents/MacOS/pronto";

  assert.deepEqual(
    classifyInstalledAppProcesses(
      `${executable} --skill-usage-collector\n`,
      executable,
    ),
    {
      foregroundRunning: false,
      collectorRunning: true,
    },
  );
});
