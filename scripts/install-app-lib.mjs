import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  renameSync,
  rmSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";

export function digestFile(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function digestBundle(bundlePath) {
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

export function replaceAppBundle(sourceApp, targetApp) {
  const expectedDigest = digestBundle(sourceApp);
  const stagingRoot = mkdtempSync(
    join(dirname(targetApp), `.${basename(targetApp)}.install-`),
  );
  const stagedApp = join(stagingRoot, basename(targetApp));
  const previousApp = join(stagingRoot, `${basename(targetApp)}.previous`);
  let previousMoved = false;
  let replacementInstalled = false;

  try {
    cpSync(sourceApp, stagedApp, { recursive: true, force: true });
    if (digestBundle(stagedApp) !== expectedDigest) {
      throw new Error("The staged app bundle does not match the local build.");
    }

    if (existsSync(targetApp)) {
      renameSync(targetApp, previousApp);
      previousMoved = true;
    }
    renameSync(stagedApp, targetApp);
    replacementInstalled = true;

    if (digestBundle(targetApp) !== expectedDigest) {
      throw new Error(
        "The installed app bundle does not match the local build.",
      );
    }

    if (previousMoved) {
      rmSync(previousApp, { recursive: true, force: true });
      previousMoved = false;
    }
  } catch (error) {
    if (replacementInstalled && existsSync(targetApp)) {
      rmSync(targetApp, { recursive: true, force: true });
    }
    if (previousMoved && existsSync(previousApp)) {
      renameSync(previousApp, targetApp);
    }
    throw error;
  } finally {
    rmSync(stagingRoot, { recursive: true, force: true });
  }
}
