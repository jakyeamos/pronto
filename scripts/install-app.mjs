#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, lstatSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  digestBundle,
  digestFile,
  findExecutableProcessIds,
  replaceAppBundle,
} from "./install-app-lib.mjs";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const sourceApp = join(
  projectRoot,
  "src-tauri",
  "target",
  "release",
  "bundle",
  "macos",
  "Pronto.app",
);
const targetApp = "/Applications/Pronto.app";
const sourceExecutable = join(sourceApp, "Contents", "MacOS", "pronto");
const targetExecutable = join(targetApp, "Contents", "MacOS", "pronto");
const checkOnly = globalThis.process.argv.includes("--check");
const appBundleIdentifier = "com.pronto.desktop";
const collectorArgument = "--skill-usage-collector";
const collectorLabel = "com.pronto.skill-usage-collector";
const userId = globalThis.process.getuid?.();
const collectorService =
  userId == null ? null : `gui/${userId}/${collectorLabel}`;

function fail(message) {
  globalThis.console.error(
    `Pronto app ${checkOnly ? "check" : "install"} failed: ${message}`,
  );
  globalThis.process.exitCode = 1;
}

function assertBundle(path, label) {
  if (!existsSync(path)) {
    fail(
      `${label} is missing at ${path}. Run ${checkOnly ? "pnpm build first" : "pnpm build before installing"}.`,
    );
    return false;
  }
  if (lstatSync(path).isSymbolicLink()) {
    fail(`${label} must be a real app bundle, not a symlink: ${path}`);
    return false;
  }
  return true;
}

function installedAppProcessIds({ includeCollector = true } = {}) {
  const processList = execFileSync("/bin/ps", ["-ax", "-o", "pid=,command="], {
    encoding: "utf8",
  });
  return findExecutableProcessIds(
    processList,
    targetExecutable,
    includeCollector ? [] : [collectorArgument],
  );
}

function waitForInstalledAppToQuit(timeoutMilliseconds = 10_000) {
  const deadline = Date.now() + timeoutMilliseconds;
  const waitState = new Int32Array(new SharedArrayBuffer(4));
  while (installedAppProcessIds({ includeCollector: false }).length > 0) {
    if (Date.now() >= deadline) {
      throw new Error(
        "Pronto did not quit within 10 seconds; the installed bundle was not replaced.",
      );
    }
    Atomics.wait(waitState, 0, 0, 100);
  }
}

function collectorServiceIsLoaded() {
  if (collectorService == null) return false;
  try {
    execFileSync("/bin/launchctl", ["print", collectorService], {
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

function restartCollectorService() {
  if (collectorService == null) {
    throw new Error(
      "A macOS user ID is required to restart the Pronto collector.",
    );
  }
  execFileSync("/bin/launchctl", ["kickstart", "-k", collectorService]);
}

function launchInstalledApp() {
  execFileSync("/usr/bin/open", ["-b", appBundleIdentifier]);
}

if (!assertBundle(sourceApp, "The local release bundle"))
  globalThis.process.exit(1);
if (!assertBundle(sourceExecutable, "The local Pronto executable"))
  globalThis.process.exit(1);

const sourceBundleDigest = digestBundle(sourceApp);
const sourceExecutableDigest = digestFile(sourceExecutable);

if (checkOnly) {
  if (!assertBundle(targetApp, "The installed Pronto bundle"))
    globalThis.process.exit(1);
  if (!assertBundle(targetExecutable, "The installed Pronto executable"))
    globalThis.process.exit(1);

  const targetBundleDigest = digestBundle(targetApp);
  const targetExecutableDigest = digestFile(targetExecutable);
  globalThis.console.log(`Local bundle:     ${sourceApp}`);
  globalThis.console.log(`Installed bundle: ${targetApp}`);
  globalThis.console.log(
    `Local bundle hash: ${sourceBundleDigest.slice(0, 12)}`,
  );
  globalThis.console.log(
    `Installed hash:    ${targetBundleDigest.slice(0, 12)}`,
  );
  globalThis.console.log(
    `Local executable:   ${sourceExecutableDigest.slice(0, 12)}`,
  );
  globalThis.console.log(
    `Installed exe:      ${targetExecutableDigest.slice(0, 12)}`,
  );
  if (sourceBundleDigest !== targetBundleDigest) {
    fail(
      "Applications is stale. Run pnpm app to build and install the current checkout.",
    );
    globalThis.process.exit(1);
  }
  globalThis.console.log(
    "Applications matches the current local release bundle.",
  );
  globalThis.process.exit(0);
}

if (existsSync(targetApp) && lstatSync(targetApp).isSymbolicLink()) {
  fail(
    `The installation target must be a real app bundle, not a symlink: ${targetApp}`,
  );
  globalThis.process.exit(1);
}

let installedAppWasRunning = false;
try {
  installedAppWasRunning =
    installedAppProcessIds({ includeCollector: false }).length > 0;
  const collectorWasLoaded = collectorServiceIsLoaded();
  if (installedAppWasRunning) {
    execFileSync("/usr/bin/osascript", [
      "-e",
      `tell application id "${appBundleIdentifier}" to quit`,
    ]);
    waitForInstalledAppToQuit();
  }
  replaceAppBundle(sourceApp, targetApp);
  if (collectorWasLoaded) restartCollectorService();
} catch (error) {
  if (installedAppWasRunning && existsSync(targetApp)) {
    try {
      launchInstalledApp();
    } catch {
      // Preserve the original installation failure as the actionable error.
    }
  }
  fail(
    `Could not replace the local release bundle at ${targetApp}: ${
      error instanceof Error ? error.message : String(error)
    }`,
  );
  globalThis.process.exit(1);
}

if (!assertBundle(targetExecutable, "The installed Pronto executable"))
  globalThis.process.exit(1);
const installedBundleDigest = digestBundle(targetApp);
const installedExecutableDigest = digestFile(targetExecutable);
if (installedBundleDigest !== sourceBundleDigest) {
  fail("The installed app bundle does not match the local release bundle.");
  globalThis.process.exit(1);
}

if (installedAppWasRunning) {
  try {
    launchInstalledApp();
  } catch (error) {
    fail(
      `The app was replaced but could not be reopened: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
    globalThis.process.exit(1);
  }
}

globalThis.console.log(`Installed current Pronto at ${targetApp}.`);
globalThis.console.log(`Bundle hash: ${installedBundleDigest.slice(0, 12)}`);
globalThis.console.log(`Executable: ${installedExecutableDigest.slice(0, 12)}`);
globalThis.console.log(
  installedAppWasRunning
    ? "Reopened Pronto with the installed replacement."
    : "Pronto was not running; open it when you are ready.",
);
