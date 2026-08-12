# Development workflow

Use the live desktop development command while changing code:

```sh
pnpm dev
```

That launches the current checkout with Tauri's live reload. It does not use
or update the copy in `/Applications`. Use this live window for direct behavior
verification while iterating; frontend changes reload automatically and native
changes rebuild through Tauri's development process.

When a coherent checkpoint is ready to become the installed daily-driver app,
promote it explicitly with:

```sh
pnpm app:update
```

This builds only the macOS app bundle, stages and atomically replaces
`/Applications/Pronto.app`, and verifies that the entire installed app bundle
and native executable match the build. Obsolete files from an earlier build
cannot survive the replacement. If Pronto is running, the installer quits it
before the swap and reopens the newly installed version afterward. `pnpm build`
and `pnpm app` remain compatibility aliases for this explicit update lane.

To build only the macOS app bundle without installing it, use:

```sh
pnpm build:bundle
```

This skips distribution-only artifacts such as the DMG. Generate the complete
configured release artifact set only when preparing a distribution:

```sh
pnpm build:release
```

To check for install drift without rebuilding:

```sh
pnpm app:check
```

These build/install commands do not modify the local Pronto database.

An ordinary app-facing source change may be completed against the live
`pnpm dev` window plus its applicable quality gates without replacing the
installed app. Installed-app verification remains required when promoting a
checkpoint, changing Tauri configuration, bundled assets, native entry points,
or installation behavior, and when preparing a release. Those paths finish
with `pnpm app:update`, `pnpm app:check`, and a launch of the installed app when
it was not already running.

## Rust toolchain contract

Pronto's minimum supported Rust version is `1.88.0`, matching the current
locked dependency graph. Verify the minimum explicitly with:

```sh
cargo +1.88.0 check --manifest-path src-tauri/Cargo.toml --locked
```

The `MSRV` GitHub Actions job runs this command independently from the stable
Rust build so a dependency update cannot silently raise the minimum.

## Remediation queue contract

For sequential agent execution, use the
[remediation handoff protocol](./remediation-sequential-handoff.md). It keeps
the Pronto projection authoritative while recording one active Sol-to-Luna
dispatch, the returned evidence, independent verification, and the next queue
transition.

`pronto remediation --json` exposes `pronto-remediation/v3`. The `plans`
array is the ranked active queue rather than an all-repository inventory.
Freshly verified or explicitly deferred action sets leave that array and are
retained in the compatibility field `closures` as resolved-action history; a
later evidence refresh can create a new active plan for the same repository.
This history records queue transitions, not permanent repository closure.
Queue order preserves status, remediation domain, and priority before giving
the Pronto and AIOS control planes and the Quality Runner evidence provider
explicit fleet leverage. Repository goal and raw action weight are used only
after those safety and leverage decisions.

The run also exposes `github_only_candidates`: authenticated GitHub repositories
present in the provider snapshot without a matching local checkout. They remain
counted as provider evidence, carry the `GitHub only` locality label, and record
`GitHub only` as their terminal remediation task without creating a synthetic
local plan.

Active plans retain actions that disappear from a refreshed projection as
verified history, so completed work continues to contribute to weighted
progress while other actions remain. If the same stable action key appears
again, it reopens instead of inheriting that historical verification. The
derived `integration_only_remaining` flag is true only when every active,
non-verification action is an unblocked branch-integration action; repository
surfaces use that field before presenting integration as the sole remaining
state.

Each plan carries a `coverage` ledger for every repo-level surface rendered by
Pronto: scope, Project Compass, provider evidence, pull requests, published
releases, quality evidence, CI gates, quality findings, maturity, workspaces,
branches, submodules, conditions, release preparation, agent permission, and
analytics. Every surface is
classified as `clear`, `attention`, `blocked`, `deferred`, `verified`, or
`not_applicable`. Attention and blocked entries link to concrete action IDs;
informational and goal-inapplicable surfaces remain visible without creating
fake work. This parity invariant prevents a UI warning such as “Compass
missing” from disappearing from fleet remediation plans.

The plan's `explanation` is the standard human- and machine-readable operating
narrative over those actions. It groups active work into ordered phases,
preserves each action's title, summary, priority, status, and acceptance
criteria as concrete steps, lists already-healthy coverage separately, and
states what must become true for the current queue refresh. The underlying
actions remain the authority: the explanation must never hide active work, present verified
history as remaining work, or imply authorization for Git, provider,
publication, release, or pruning mutations. The active-queue Markdown export
projects the same ordered phase titles as its `Remaining path` column.

The five built-in phases are defaults, not a phase-count limit. A repository
may add as many ordered phases as its real workflow requires through the
optional `remediation_phases` array in its goal contract. Each phase owns one
or more action domains and may name an earlier built-in or repository phase in
`after_phase_id`. Declared ownership moves those domains out of the built-in
phase that previously held them. IDs and domain ownership must be unique, and
an ordering reference must point backward so the contract cannot contain an
ambiguous or cyclic phase graph. If an active action reaches the planner with
an unassigned domain, Pronto exposes it in `unclassified_remediation` instead
of omitting it. The explanation projection must contain every active action
exactly once.

Plans whose resolved target is `public_release` include the built-in
`public_distribution_boundary` phase whenever the imported
`quality-runner-release-boundary/v2` receipt is missing, legacy, invalid, stale,
target-mismatched, policy-mismatched, artifact-mismatched, or failed. That action
is derived from the receipt's blocking check IDs and remains blocked regardless
of manual action status. A fresh passing receipt removes it. The receipt proves
that release-relevant surfaces are classified as `public_core`, `public_adapter`,
or `local_only`; tracked source and docs are free of personal paths and private
inventory; artifacts match their allowlist and hashes; installation works with
an isolated temporary home and private workspace peers absent; and integration
proof uses sanitized fixtures or consumer-owned tests. `release preview` applies
the same hard gate, and Pronto never executes receipt content. Other goal targets
do not inherit this work, and an inferred non-release goal retains only the
existing goal-confirmation action.

Each plan also carries a goal profile. A repository can confirm that profile in
`.pronto/remediation-goal.json`:

```json
{
  "schema_version": "pronto-remediation-goal/v1",
  "target_state": "public_release",
  "reason": "This package is distributed as a supported public release.",
  "additional_required_gate_ids": [],
  "optional_gate_ids": [],
  "evidence_max_age_days": 7,
  "remediation_phases": [
    {
      "id": "deployment_validation",
      "title": "Validate deployment behavior",
      "summary": "Verify the repository-specific deployment path.",
      "domains": ["deployment_validation"],
      "completion_criterion": "Fresh deployment evidence satisfies the repository contract.",
      "after_phase_id": "quality_and_maturity"
    }
  ]
}
```

Supported targets are `public_release`, `deployed_product`,
`active_maintained`, `clean_only`, `prototype`, and `archived`. Pronto infers a
candidate when the contract is missing or invalid, but marks the source as
`inferred` and keeps a goal-confirmation action active. Target defaults select
the applicable quality gates, evidence freshness window, ranking priority, and
closure criteria. Repository additions may strengthen but never remove the
target's required gates.

Use `pronto remediation export [output-dir] --json` to write the JSON manifest,
active plan files, retained goal-stamped resolved-action history, and a generated
`repository-remediation-order.md`. Exporting does not authorize repository,
Git, provider, publication, or pruning changes.
