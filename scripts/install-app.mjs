#!/usr/bin/env node

import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  lstatSync,
  readFileSync,
  readdirSync,
  readlinkSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

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

function digestFile(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function digestBundle(bundlePath) {
  const entries = [];

  function collect(currentPath, relativePath) {
    for (const entry of readdirSync(currentPath, { withFileTypes: true }).sort(
      (left, right) => left.name.localeCompare(right.name),
    )) {
      const entryPath = join(currentPath, entry.name);
      const entryRelativePath = join(relativePath, entry.name);
      if (entry.isDirectory()) {
        collect(entryPath, entryRelativePath);
      } else if (entry.isFile()) {
        entries.push({
          kind: "file",
          path: entryRelativePath,
          value: readFileSync(entryPath),
        });
      } else if (entry.isSymbolicLink()) {
        entries.push({
          kind: "symlink",
          path: entryRelativePath,
          value: readlinkSync(entryPath),
        });
      }
    }
  }

  collect(bundlePath, "");
  const hash = createHash("sha256");
  for (const entry of entries) {
    hash.update(`${entry.kind}:${entry.path}\0`);
    hash.update(entry.value);
  }
  return hash.digest("hex");
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

try {
  cpSync(sourceApp, targetApp, { recursive: true, force: true });
} catch (error) {
  fail(
    `Could not copy the local release bundle to ${targetApp}: ${
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

globalThis.console.log(`Installed current Pronto at ${targetApp}.`);
globalThis.console.log(`Bundle hash: ${installedBundleDigest.slice(0, 12)}`);
globalThis.console.log(`Executable: ${installedExecutableDigest.slice(0, 12)}`);
globalThis.console.log("Quit and reopen Pronto if it was already running.");
