import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixtureFiles = [
  ".agents/agent-usability.json",
  ".agents/change-surface-matrix.json",
  ".agents/context/README.md",
  ".agents/context/branch-sensitive-quality-verification.md",
  ".agents/context/commands.md",
  ".agents/environment-legibility.json",
  ".gitignore",
  ".github/workflows/quality.yml",
  "package.json",
  "docs/implementation-examples.md",
  "docs/implementation-plan.md",
  "docs/agent-usability-maturity.md",
  "docs/cache-design-maturity-gate.md",
  "docs/mac-control-maturity-gate.md",
  "docs/remediation-sequential-handoff.md",
  "docs/repository-contract.md",
  ".pronto/cache-lifecycle.json",
  "scripts/validate-repository-contract.mjs",
];

function isoDate(offsetDays = 0) {
  return new Date(Date.now() + offsetDays * 24 * 60 * 60 * 1000)
    .toISOString()
    .slice(0, 10);
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function createFixture(t) {
  const fixture = fs.mkdtempSync(
    path.join(os.tmpdir(), "pronto-contract-fixture-"),
  );
  t.after(() => fs.rmSync(fixture, { recursive: true, force: true }));

  for (const relative of fixtureFiles) {
    const destination = path.join(fixture, relative);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(root, relative), destination);
  }

  for (const relative of [
    ".agents/change-surface-matrix.json",
    ".agents/environment-legibility.json",
  ]) {
    const absolute = path.join(fixture, relative);
    const contract = JSON.parse(fs.readFileSync(absolute, "utf8"));
    contract.last_reviewed = isoDate();
    writeJson(absolute, contract);
  }

  return fixture;
}

function mutateEvidenceContract(fixture, mutate) {
  const contractPath = path.join(
    fixture,
    ".agents/environment-legibility.json",
  );
  const contract = JSON.parse(fs.readFileSync(contractPath, "utf8"));
  mutate(contract);
  writeJson(contractPath, contract);
}

function mutateCacheLifecycle(fixture, mutate) {
  const contractPath = path.join(fixture, ".pronto/cache-lifecycle.json");
  const contract = JSON.parse(fs.readFileSync(contractPath, "utf8"));
  mutate(contract);
  writeJson(contractPath, contract);
}

function runValidator(fixture) {
  return spawnSync(
    process.execPath,
    [path.join(fixture, "scripts/validate-repository-contract.mjs")],
    { cwd: fixture, encoding: "utf8" },
  );
}

function assertRejected(result, expected) {
  assert.equal(result.status, 1, result.stderr || result.stdout);
  assert.match(result.stderr, expected);
  assert.doesNotMatch(result.stdout, /Repository contract valid/);
}

test("accepts the repository-owned maturity evidence fixture", (t) => {
  const result = runValidator(createFixture(t));

  assert.equal(result.status, 0, result.stderr);
  assert.match(
    result.stdout,
    /9 maturity dimensions, 4 rebuildable cache artifacts/,
  );
});

test("rejects an unsafe cache artifact path", (t) => {
  const fixture = createFixture(t);
  mutateCacheLifecycle(fixture, (contract) => {
    contract.artifacts[0].path = "../node_modules";
  });

  assertRejected(runValidator(fixture), /artifact path is missing or unsafe/);
});

test("rejects shared mutable Cargo intermediates", (t) => {
  const fixture = createFixture(t);
  mutateCacheLifecycle(fixture, (contract) => {
    contract.central_cargo_intermediates.sharing = "shared_across_worktrees";
  });

  assertRejected(
    runValidator(fixture),
    /central Cargo ownership, isolation, and authorization boundaries are required/,
  );
});

test("rejects cache artifacts without matching ignore evidence", (t) => {
  const fixture = createFixture(t);
  mutateCacheLifecycle(fixture, (contract) => {
    contract.artifacts[0].ignore_evidence.contains = "not-an-ignore-rule";
  });

  assertRejected(runValidator(fixture), /ignore evidence did not match/);
});

test("rejects a cache artifact with a stale rebuild script", (t) => {
  const fixture = createFixture(t);
  mutateCacheLifecycle(fixture, (contract) => {
    contract.artifacts[1].rebuild_command = "pnpm removed-script";
  });

  assertRejected(
    runValidator(fixture),
    /rebuild command is not a current pnpm script/,
  );
});

test("rejects an unresolved tool-to-skill relationship", (t) => {
  const fixture = createFixture(t);
  const contractPath = path.join(fixture, ".agents/agent-usability.json");
  const contract = JSON.parse(fs.readFileSync(contractPath, "utf8"));
  contract.tools[0].skills = ["missing-skill"];
  writeJson(contractPath, contract);

  assertRejected(
    runValidator(fixture),
    /pronto-cli: unknown skill reference: missing-skill/,
  );
});

test("rejects an unresolved tool-to-skill relationship", (t) => {
  const fixture = createFixture(t);
  const contractPath = path.join(fixture, ".agents/agent-usability.json");
  const contract = JSON.parse(fs.readFileSync(contractPath, "utf8"));
  contract.tools[0].skills = ["missing-skill"];
  writeJson(contractPath, contract);

  assertRejected(
    runValidator(fixture),
    /pronto-cli: unknown skill reference: missing-skill/,
  );
});

for (const [label, unsafePath] of [
  ["parent traversal", "../outside.md"],
  ["absolute path", "/tmp/outside.md"],
]) {
  test(`rejects ${label} evidence`, (t) => {
    const fixture = createFixture(t);
    mutateEvidenceContract(fixture, (contract) => {
      contract.dimensions.architecture_boundaries.evidence = [unsafePath];
    });

    assertRejected(
      runValidator(fixture),
      /architecture_boundaries\.evidence: path is outside the repository/,
    );
  });
}

test("rejects symlinked evidence even when its target is repository-owned", (t) => {
  const fixture = createFixture(t);
  const symlink = path.join(fixture, "docs/symlinked-evidence.md");
  fs.symlinkSync("repository-contract.md", symlink);
  mutateEvidenceContract(fixture, (contract) => {
    contract.dimensions.architecture_boundaries.evidence = [
      "docs/symlinked-evidence.md",
    ];
  });

  assertRejected(
    runValidator(fixture),
    /architecture_boundaries\.evidence: symlinked paths are not allowed/,
  );
});

for (const [label, reviewDate] of [
  ["stale", isoDate(-91)],
  ["future", isoDate(1)],
]) {
  test(`rejects a ${label} maturity review date`, (t) => {
    const fixture = createFixture(t);
    mutateEvidenceContract(fixture, (contract) => {
      contract.last_reviewed = reviewDate;
    });

    assertRejected(
      runValidator(fixture),
      /last_reviewed is missing, future-dated, or stale/,
    );
  });
}

for (const assertionKind of ["validation", "automation"]) {
  test(`rejects an unmatched ${assertionKind} assertion`, (t) => {
    const fixture = createFixture(t);
    mutateEvidenceContract(fixture, (contract) => {
      contract.dimensions.architecture_boundaries[assertionKind][0].contains = [
        "adversarial term absent from the asserted surface",
      ];
    });

    assertRejected(
      runValidator(fixture),
      new RegExp(
        `architecture_boundaries\\.${assertionKind} assertion did not match`,
      ),
    );
  });
}
