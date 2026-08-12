import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const packageJson = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
);

for (const scriptName of ["build:bundle", "test"]) {
  test(`${scriptName} preserves the caller's guarded cargo routing`, () => {
    const script = packageJson.scripts[scriptName];

    assert.equal(typeof script, "string");
    assert.doesNotMatch(script, /PATH\s*=/);
    assert.doesNotMatch(script, /\/opt\/homebrew\/bin\/cargo/);
  });
}
