# Development workflow

Use the live desktop development command while changing code:

```sh
pnpm dev
```

That launches the current checkout with Tauri's live reload. It does not use
the copy in `/Applications`.

Every production build now installs the resulting macOS app into
`/Applications/Pronto.app`. Use:

```sh
pnpm build
```

This builds the current checkout, copies the resulting `Pronto.app` to
`/Applications/Pronto.app`, and verifies that the entire installed app bundle
and native executable match the build. `pnpm app` is retained as an alias.
Quit and reopen Pronto after installing if it was already running.

To build only the repository release bundle without installing it, use:

```sh
pnpm build:bundle
```

To check for install drift without rebuilding:

```sh
pnpm app:check
```

These build/install commands do not modify the local Pronto database.

## Rust toolchain contract

Pronto's minimum supported Rust version is `1.88.0`, matching the current
locked dependency graph. Verify the minimum explicitly with:

```sh
cargo +1.88.0 check --manifest-path src-tauri/Cargo.toml --locked
```

The `MSRV` GitHub Actions job runs this command independently from the stable
Rust build so a dependency update cannot silently raise the minimum.

## Remediation queue contract

`pronto remediation --json` exposes `pronto-remediation/v3`. The `plans`
array is the ranked active queue rather than an all-repository inventory.
Freshly verified or explicitly deferred terminal plans leave that array and
are retained in `closures`; a later evidence refresh can create a new active
plan for the same repository. Queue order preserves status, remediation domain,
and priority before giving the Pronto and AIOS control planes and the Quality
Runner evidence provider explicit fleet leverage. Repository goal and raw
action weight are used only after those safety and leverage decisions.

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

Each plan also carries a goal profile. A repository can confirm that profile in
`.pronto/remediation-goal.json`:

```json
{
  "schema_version": "pronto-remediation-goal/v1",
  "target_state": "public_release",
  "reason": "This package is distributed as a supported public release.",
  "additional_required_gate_ids": [],
  "optional_gate_ids": [],
  "evidence_max_age_days": 7
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
active plan files, retained goal-stamped closure data, and a generated
`repository-remediation-order.md`. Exporting does not authorize repository,
Git, provider, publication, or pruning changes.
