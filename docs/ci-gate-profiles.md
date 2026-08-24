# Repository CI gate profiles

Pronto scores CI configuration against a repository-owned contract at
`.pronto/ci-gate-profile.json`. The current schema is
`pronto-ci-gate-profile/v1`.

The nine standard gates are a classification checklist, not a universal
denominator. Every repository contract must classify each standard gate as
`required`, `optional`, or `not_applicable` and explain why. Only `required`
gates count toward the repository's ideal CI configuration and evidence
scores.

Custom gates represent repository-specific invariants such as a restore drill,
schema-compatibility check, policy check, or generated-artifact verification.
They must use a stable `custom:<snake_case_id>`, a human-readable label, a
reason, and either `required` or `optional`. Pronto never turns a discovered CI
job into a requirement by itself.

```json
{
  "schema_version": "pronto-ci-gate-profile/v1",
  "reason": "This service ships a container and owns production data recovery.",
  "gates": [
    {
      "id": "build",
      "classification": "required",
      "reason": "The deployable container must build from the candidate commit."
    },
    {
      "id": "tests",
      "classification": "required",
      "reason": "Unit and integration behavior must pass before merge."
    },
    {
      "id": "runtime_smoke",
      "classification": "required",
      "reason": "The built service must start and answer its health route."
    },
    {
      "id": "lint",
      "classification": "optional",
      "reason": "Lint findings are advisory until the legacy baseline is retired."
    },
    {
      "id": "formatter",
      "classification": "required",
      "reason": "Formatting is deterministic and enforced on changed files."
    },
    {
      "id": "typecheck",
      "classification": "required",
      "reason": "The runtime build does not independently prove all type surfaces."
    },
    {
      "id": "dead_code",
      "classification": "not_applicable",
      "reason": "No supported analyzer produces a trustworthy signal for this stack."
    },
    {
      "id": "secrets_scan",
      "classification": "required",
      "reason": "Repository content must be scanned before it can merge."
    },
    {
      "id": "dependency_audit",
      "classification": "required",
      "reason": "The service publishes third-party packages into production."
    },
    {
      "id": "custom:restore_drill",
      "label": "Restore drill",
      "classification": "required",
      "reason": "A backup is not release-ready until a current restore succeeds."
    },
    {
      "id": "custom:diagnostic_report",
      "label": "Diagnostic report",
      "classification": "optional",
      "reason": "The report helps operators but does not block a merge."
    }
  ]
}
```

## Validation and compatibility

- A repository contract is authoritative when it is valid.
- Omitting a standard gate, duplicating an ID, using an unprefixed custom ID,
  or omitting a required reason makes the contract invalid and leaves the
  repository visibly unscored.
- Required custom gates participate in remediation for public-release,
  deployed-product, and active-maintained goals. Optional gates stay visible
  but do not create remediation work.
- When the repository contract is absent, Pronto may use the static
  [recommendation matrix](quality-gate-recommendation-matrix.md) as an explicit
  compatibility profile. The UI identifies that source; it is not presented
  as repository-owned truth.
- A missing or ambiguous compatibility row remains unscored. Pronto does not
  substitute a six- or nine-gate fallback.

Gate discovery and gate obligation remain separate. CI, local, and Quality
Runner evidence can reveal a check, but only this repository contract or the
legacy compatibility matrix determines whether that check belongs in the ideal
denominator. Passing evidence still requires current target-branch and
candidate-commit provenance.

## Quality Runner semantic audit

Quality Runner owns the read-only semantic audit that proposes repository-specific
gates. Its `quality-runner-ci-gate-candidates/v1` report reaches Pronto through
the validated maturity feed. Pronto rejects reports with mismatched target
branch or commit, unsafe evidence paths, duplicate or invalid custom IDs,
unsupported recommendation states, incomplete admission blockers, or a bad
provenance hash.

Accepted reports remain recommendations. The UI explains each candidate's
invariant, failure mode, evidence paths, suggested trigger and check context,
existing-check discovery, negative controls, and admission blockers. Candidates
do not enter the score denominator, gate matrix, or remediation queue until this
repository-owned profile explicitly accepts them.
