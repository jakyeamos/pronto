# Pronto custody ledger projection

Pronto projects isolated-change-workflow custody as a read-only fleet view. The
workflow owns receipt signing, lease renewal, pause/resume, adoption,
integration locking, WIP validation, and cleanup. Pronto never adopts a lane,
merges a branch, deletes a worktree, or pushes a ref from this projection.

Read one repository:

```bash
pnpm --silent --dir "$PRONTO_ROOT" run cli custody /absolute/path/to/repository --json
```

Read the registered fleet:

```bash
pnpm --silent --dir "$PRONTO_ROOT" run cli custody --json
```

The native projection is `pronto-custody-projection/v1`; the CLI envelope is
`pronto-custody-cli/v1`. Both are derived from live Git worktrees, branches,
heads, status, operation markers, open-file evidence, and local receipt files.
Cached Pronto labels do not authorize custody decisions.

## Role-based workspace targets

Custody is projected alongside a separate workspace policy dimension. A policy
classifies the repository's protected canonical workspaces by role:

- `production_product` has two canonical workspaces: `release` on `main` or
  `master`, and `integration` on the repository's `dev` line.
- `supporting_project` has one canonical `working` workspace on its actual
  working branch.
- `role_unresolved` is explicit and does not produce a baseline target.

The fleet baseline is `2P + 1N`, where `P` is the number of production products
and `N` is the number of supporting projects. Active feature, agent,
verification, adoption, and cleanup worktrees are temporary lanes; they do not
count toward that baseline and require isolated-change leases. A deliberate
retention exception is reported separately with its review deadline.

The projection exposes `baseline_target`, `canonical_observed`,
`temporary_observed`, `active_temporary_lanes`, `retained_lane_count`,
`managed_target_total`, and `drift`. Canonical protection and temporary-lane
custody are separate dimensions: a canonical workspace is protected by the
role policy, while every other workspace must be leased. A missing or invalid
policy remains an explicit `policy_missing` or `policy_invalid` condition; it
does not authorize adoption, deletion, or integration.

### Fleet manifest generation

Pronto generates the QR input manifest from the registered fleet and live
temporary-lane projections. It requires an explicit, exact-coverage role map;
it never infers product role from Showcase labels, repository names, or branch
activity:

```bash
pronto workspace-manifest --role-map /path/to/workspace-role-map.json --json
```

The role map uses `workspace-role-map/v1` and contains one entry per registered
repository. Production entries require `release_ref` (`main` or `master`) and
`integration_ref` (`dev`); supporting entries require the actual
`working_ref`; unresolved entries are allowed but keep the fleet baseline
incomplete. Missing or extra repository IDs fail the command. The resulting
`workspace-fleet-manifest/v1` is read-only and can be passed to QR:

```bash
qr fleet workspace-target calculate --manifest /path/to/workspace-fleet.json --json
```

The checked-in `.pronto/workspace-role-map.json` is the reviewed exact-coverage
fleet map: 10 production products and 58 supporting projects. Its production
entries use `main` or `master` plus `dev`; its supporting entries use `dev` as
their working ref. The review explicitly assigns Bballedu's release ref to
`master`. This gives the current fleet a baseline target of `2*10 + 58 = 78`;
future role changes remain separate, reviewable policy decisions.

The manifest generator reports current active temporary lanes from live custody
evidence. It remains read-only: it does not write repository policy files,
protect refs, grant custody, or delete worktrees.

### Repository policy-file generation

Per-repository policy files are a separate, explicit local-write surface. Use
the same reviewed role map to plan or generate `.agents/workspace-policy.json`
in each registered Git repository:

```bash
pronto workspace-policy generate \
  --role-map /path/to/workspace-role-map.json \
  [--repository <id|path|name>] --json
```

Without `--write`, the command emits `workspace-policy-generation/v1` and
per-repository `would_create`/`would_replace` results without changing files.
`--write` creates missing files. An existing differing file is a conflict until
`--replace --write` is explicitly supplied. Fleet generation is plan-first:
any blocked Git root, symlinked `.agents` directory or policy file, malformed
target, or unresolved conflict prevents ready targets from being applied, so a
fleet run does not silently produce a partial policy set. An explicit
`--repository` scopes the plan to one registered repository.

This command writes only the repository-owned policy file. It never commits or
pushes, protects `main`/`master` or `dev`, grants temporary-lane custody, creates
or removes worktrees, or changes provider state. Each generated file must be
reviewed and committed through that repository's normal integration lane.

## State and disposition

Each lane has an independent lifecycle `state`:

`active`, `paused`, `stale`, `adoptable`, `contested`, `integrating`, `closed`,
or `unknown`.

Each lane also has a primary `disposition`, a unique `dispositions` list, and a
`next_action`. This prevents `unknown` from becoming a catch-all. Important
codes include:

- `legacy_unsigned_receipt`: route through the bounded legacy owner-return or adoption review.
- `receipt_malformed`: preserve and repair or replace through the workflow owner.
- `receipt_integrity_invalid`: preserve the known receipt and repair its missing or malformed integrity evidence.
- `receipt_schema_unsupported`: preserve and upgrade the producer before any custody mutation.
- `worktree_not_live`, `branch_binding_mismatch`, or `head_binding_mismatch`: re-establish exact live Git identity.
- `dirty_worktree`, `open_files_observed`, or `git_operation_active`: adoption is blocked by live activity.
- `live_git_evidence_unavailable`: retry; missing evidence is not abandonment.
- `adoption_ready`: recheck the exact generation and head, then use the workflow adoption command.

Legacy and unsupported receipts remain `unknown` with their concrete reason;
they never become `adoptable` because their timestamps are old. An expired
clean signed-shape lane can be `adoptable`, but Pronto reports its receipt
integrity as unverified because it does not own the workflow's HMAC key.

The renderer and agent JSON should show the disposition and next action next to
the lifecycle state. It must not present a raw `unknown` label without the
underlying reason.
