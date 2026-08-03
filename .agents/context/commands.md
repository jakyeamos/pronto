# Agent command contract

Last reviewed: 2026-07-29.

## Invocation

Run the current checkout's CLI through `pnpm`; do not rely on an unverified
standalone binary:

```sh
PRONTO_ROOT="$(git rev-parse --show-toplevel)"
pnpm --silent --dir "$PRONTO_ROOT" run cli route --json
```

The CLI reads and writes Pronto's local SQLite-backed snapshot. JSON is the
preferred agent interface. `doctor --json` is the read-only freshness and
storage gate; it never refreshes or writes the snapshot. `status --json`
remains the complete legacy snapshot; use a focused projection when one is
available.

`route --json` is the preferred composed entry point for agent orientation. It
performs the same read-only doctor gate for the selected scope and, only when
that gate is ready, includes bounded `next`, repository, quality,
`change_maturity`, and `fold_preview` projections from the same snapshot
boundary. The change summary is advisory and recommends a read-only matrix
inspection; it does not create a missing matrix. The fold preview uses the
repository's persisted target branch when configured, falls back to the
observed default branch, and remains advisory; use direct `fold preview` when a
one-off explicit target is needed. A blocked route
intentionally withholds follow-up projections and exits non-zero; use its
`next_safe_step` before refreshing or repairing evidence.

## Focused read and preview surfaces

| Need                      | Command                                                                                                                                                       | Contract                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| Agent routing envelope    | `route [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] [--limit <n>] --json`                                                        | `pronto-agent-route/v1`        |
| Freshness/storage gate    | `doctor [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] --json`                                                                     | `pronto-agent-doctor/v1`       |
| Daily orientation         | `next [<repository>] [--product <name> \| --group <name>] [--limit <n>] --json`                                                                               | `pronto-agent-next/v1`         |
| Fold preparation          | `fold preview [<repository>] [--target <branch>] [--product <name> \| --group <name>] [--limit <n>] --json`                                                   | `pronto-agent-fold-preview/v1` |
| Fleet orientation         | `summary [--product <name> \| --group <name>] --json`                                                                                                         | `pronto-agent-summary/v1`      |
| One repository            | `repo <absolute-repo-path> --json`                                                                                                                            | `pronto-agent-repository/v1`   |
| Quality evidence          | `quality [<repository>] --json`                                                                                                                               | `pronto-agent-quality/v1`      |
| Finding adjudication      | `quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] --json` | repository-owned overlay       |
| Skill topology            | `skills [<skill-id>] --json`                                                                                                                                  | `pronto-skills/v2`             |
| Repository change matrix  | `change-matrix repo <repository> [--operation <add\|change\|remove>] --json`                                                                                  | `pronto-change-matrix/v1`      |
| Skill change matrix       | `change-matrix skill <skill-id> [--operation <add\|change\|remove>] --json`                                                                                   | `pronto-change-matrix/v1`      |
| Active remediation        | `remediation [<repository>] --json`                                                                                                                           | `pronto-remediation/v3`        |
| Work requiring attention  | `attention --json`                                                                                                                                            | `pronto-agent-attention/v1`    |
| Recent transitions/audits | `activity [<repository>] --limit <n> --json`                                                                                                                  | `pronto-agent-activity/v1`     |
| Preparation preflight     | `prepare <repository> [--workspace <id>] --json`                                                                                                              | `pronto-agent-preparation/v1`  |
| Release preflight         | `release preview <repository> [--workspace <id>] --json`                                                                                                      | `pronto-agent-release/v1`      |

Resolve a repository with `git rev-parse --show-toplevel` and pass the
absolute path, repository name, ID, or an exact workspace path. Do not pass `.`
and assume it will resolve.

The Node adapter normally resolves `cargo` from the documented Homebrew paths
and then `PATH`. Set `PRONTO_CARGO` to an explicit Cargo executable only when a
different verified Rust toolchain is required.

Repository and summary projections include a read-only `project_compass`
summary derived from `.project-compass/contract.json`. `Ready` reports the
current product identity, MVP and complete-product progress, confidence,
blockers, and open drift. Open blockers preserve their outcome, kind, and
summary; open drift preserves its kind, summary, and observation time so the UI
and JSON explain the count rather than projecting only a number. `Missing`
means the repository has not established a Compass contract; `Invalid`
preserves the parse or contract error. Pronto never creates or repairs Compass
artifacts during a refresh.

`change-matrix` explains an existing repository- or skill-owned contract. A
missing contract returns `status: "missing"`, its maturity impact, observed
topology, and the expected location without synthesizing or writing anything.
`skills [<skill-id>] --json` preserves source paths and hashes, provider state,
parity evidence, and `hosted_in_jakye_agent_setup`.

`remediation` is an active, goal-aware ranked queue. Its `plans` contain only
actionable repositories and expose the resolved `goal` profile. Repositories
may confirm that profile through `.pronto/remediation-goal.json` using
`pronto-remediation-goal/v1`; absent or invalid contracts remain visibly
inferred and create confirmation work rather than silently becoming truth.
Goal-specific required gates, freshness windows, and closure criteria determine
which actions apply. Every plan also contains a `coverage` ledger for all
repo-level surfaces tracked in the UI: scope, Project Compass, provider, pull
requests, published releases, quality evidence, CI gates, findings, maturity,
workspaces, branches, submodules, conditions, release preparation, agent
permission, and analytics.
Each plan also contains an `explanation` projection that groups only active
(`open`, `in_progress`, or `blocked`) actions into ordered operator phases.
Pronto supplies four default phase definitions—preserve and reconcile
repository work, reconcile product and provider truth, reach quality and
maturity closure, and refresh, verify, and close—but that sequence is not a
maximum. A repository contract may add phase definitions, assign action
domains to them, and place each addition after an earlier phase. Repository
phases take ownership of their declared domains; active actions in an
unassigned domain remain visible in an explicit `unclassified_remediation`
phase. Every active action must appear exactly once. Every phase exposes linked
action steps and completion criteria; verified history is not presented as
remaining work. The explanation names
`clear` and `verified` coverage surfaces as already healthy and repeats the
goal-specific closure requirements. It is advisory and never authorizes Git,
provider, publication, release, or pruning mutations. The Markdown queue export
includes the ordered remaining phase titles so its human-readable summary stays
aligned with the JSON plan and app detail surface.
Goals that require maturity also expose a structured `maturity_policy`: 3.0/4
is the minimum evidence-backed closure score and 4.0/4 is the ideal. Reaching
the closure score removes blocking maturity work from the active queue, while
the policy remains in retained closure evidence. Agents may continue material,
applicable improvements toward the ideal, but must not create superficial
documentation, configuration, tests, or other artifacts solely to raise the
score.
Unresolved coverage must link to action IDs; clear, informational, or
goal-inapplicable surfaces remain explicit without manufacturing work. Terminal evidence-backed outcomes move to the retained
`closures` ledger, including their target state, and may re-enter the queue
after a later refresh. A repository query returns either its active plan, its
retained closures, or both. Ranking preserves status, the earliest unresolved
domain, and action priority before applying explicit fleet leverage for Pronto,
AIOS, and Quality Runner; repository goal and raw action weight are later
tie-breakers. `remediation export` writes the JSON contracts plus
`repository-remediation-order.md`.

`remediation refresh` closes its quality-import checkpoint only when the
canonical QR feed is published and every eligible repository whose goal
requires maturity has a fresh repository-level score. A replay-validated
scoped audit may supply that repository evidence when the repository lives
outside the canonical projects root. Pronto retains that scoped audit
provenance across later local or provider refreshes; the remediation plan,
repository projection, and UI must all read the same imported maturity
snapshot. Missing or stale applicable scores leave the refresh `partial` and
the `quality_import` step `blocked` with the affected repositories named.
Dynamic audits default to a 120-second per-command timeout. Use
`remediation refresh --dynamic --timeout-seconds <positive-integer>` when a
repository's documented quality command legitimately needs a longer bound;
the same explicit timeout is applied to both the scoped audit and any required
canonical all-projects fallback.

## Refresh and state boundaries

Use `refresh <repository> --json` when the persisted snapshot is stale or after
a branch/workspace change that needs fresh evidence. It performs a local
read-only Git scan but persists the resulting snapshot and audit record, so it
is state-changing even though it does not modify a repository.

Run `route --json` before routing across repositories. For repository-local
work, pass the resolved repository path; this prevents unrelated fleet rows
from blocking the task. A non-zero exit or `ready: false` is a hard stop for
the selected scope; refresh or repair only the evidence it identifies, then
rerun route. Use direct `doctor` when a dedicated freshness/storage report is
needed.

The following commands can change local Pronto state or touch an external
boundary and require explicit task scope: `root add`, `root exclude`, `refresh`,
`refresh-github`, `clone`, `remediation refresh`, `remediation export`, and
`remediation set-status`. `quality disposition set` writes the selected
repository's `.pronto/quality-finding-dispositions.json` review ledger and
therefore also requires an exact repository, finding fingerprint, disposition,
reason, and reviewer. These commands do not authorize Git branch cleanup,
merging, pushing, deletion, provider mutation, or release publication.

Use `refresh-github <repository|group|product> --json` when live provider
evidence is needed for a bounded scope. Omitting the target refreshes the whole
registered fleet and should be reserved for an explicitly fleet-wide task.

`fold preview` is an advisory projection only; it does not clean branches,
fold dev, delete branches, or push. Use it for persisted branch/worktree
evidence, then use the reviewed branch-folding workflow and ordinary Git
commands within their own authorization boundaries.

## Evidence interpretation

The default snapshot freshness window for `route` and `doctor` is 48 hours
(`2,880` minutes). Use `--max-age` for an explicitly different review window;
operations that depend on current Git state still require live verification.

Quality gate and finding evidence uses a separate commit-bound rule: an
in-window observation is `Fresh` only when its scanned commit equals the
current commit. A matching branch name alone is not freshness proof. Without
comparable commit provenance, a matching branch is `Unknown` and a differing
branch is `Stale`; an exact commit match remains authoritative.

- `generated_at` identifies the snapshot time; re-check it after a meaningful
  state change.
- `doctor` reports storage, registered roots, per-repository scan freshness,
  local path availability, and quality warnings without changing local state.
- Workspace `sync_state` is healthy only when it is exactly `Synced`.
- `Ahead by N`, `Behind by N`, divergence, `No upstream`, dirty workspaces,
  active operations, missing/stale quality evidence, `Unknown`, and `Blocked`
  require attention or an explicit explanation.
- A preview or configured ideal is not execution proof. Keep source tests,
  local validation, live browser/device evidence, and release/provider proof
  distinct.
- Quality finding `total` remains the immutable detector count.
  `actionable_total`, `reviewed_total`, `unreviewed_total`, and
  `disposition_counts` are reconciled from the repository-owned
  `pronto-quality-finding-dispositions/v1` ledger. Missing, invalid, expired,
  absent, or scope-mismatched dispositions never suppress a finding.
- Supported finding dispositions are `confirmed`, `false_positive`,
  `accepted_intentional`, `accepted_risk`, `deferred`, `fixed`, and
  `superseded`. Confirmed and deferred findings remain actionable; false
  positives, accepted intentional behavior, and accepted risk leave the active
  remediation queue while their review evidence remains visible. Fixed or
  superseded decisions attached to a current finding are treated as stale
  rather than silently hiding a recurrence.
- Preserve `schema_version`, freshness, evidence, and uncertainty fields when
  handing JSON to another agent.
