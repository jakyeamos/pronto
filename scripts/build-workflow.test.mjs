import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const packageJson = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
);
const qualityWorkflow = readFileSync(
  join(root, ".github", "workflows", "quality.yml"),
  "utf8",
);
const installer = readFileSync(
  join(root, "scripts", "install-app.mjs"),
  "utf8",
);

test("keeps live development independent of the installed app", () => {
  assert.equal(packageJson.scripts.dev, "tauri dev");
});

test("builds only the macOS app for local checkpoint promotion", () => {
  assert.equal(
    packageJson.scripts["build:bundle"],
    "tauri build --bundles app",
  );
  assert.equal(
    packageJson.scripts["app:update"],
    "pnpm build:bundle && pnpm install:app",
  );
  assert.doesNotMatch(packageJson.scripts["build:bundle"], /dmg|install:app/);
});

test("keeps full distribution packaging explicit", () => {
  assert.equal(packageJson.scripts["build:release"], "tauri build");
  assert.match(qualityWorkflow, /command: pnpm build:release/);
  assert.doesNotMatch(qualityWorkflow, /command: pnpm build:bundle/);
});

test("preserves the existing build and app compatibility aliases", () => {
  assert.equal(packageJson.scripts.build, "pnpm app:update");
  assert.equal(packageJson.scripts.app, "pnpm app:update");
  assert.match(installer, /Run pnpm app:update to promote/);
  assert.doesNotMatch(installer, /Run pnpm app to build/);
});

test("restarts the collector around replacement and force-launches the exact app", () => {
  assert.match(installer, /"bootout", collectorService/);
  assert.match(installer, /"bootstrap",/);
  assert.match(installer, /"kickstart", "-k", collectorService/);
  assert.match(installer, /"-n", targetApp/);
  assert.doesNotMatch(installer, /osascript/);
});

for (const relative of [
  "docs/development.md",
  "docs/repository-contract.md",
  ".agents/context/commands.md",
]) {
  test(`${relative} documents the three build lanes`, () => {
    const contents = readFileSync(join(root, relative), "utf8");

    assert.match(contents, /pnpm dev/);
    assert.match(contents, /pnpm app:update/);
    assert.match(contents, /pnpm build:release/);
  });
}
