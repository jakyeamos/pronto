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
inspection; it does not create a missing matrix. The fold preview
uses the observed default branch and remains advisory; use direct `fold
preview` when an explicit target branch is needed. A blocked route
intentionally withholds follow-up projections and exits non-zero; use its
`next_safe_step` before refreshing or repairing evidence.

## Focused read and preview surfaces

| Need                      | Command                                                                                                     | Contract                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------ |
| Agent routing envelope    | `route [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] [--limit <n>] --json`      | `pronto-agent-route/v1`        |
| Freshness/storage gate    | `doctor [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] --json`                   | `pronto-agent-doctor/v1`       |
| Daily orientation         | `next [<repository>] [--product <name> \| --group <name>] [--limit <n>] --json`                             | `pronto-agent-next/v1`         |
| Fold preparation          | `fold preview [<repository>] [--target <branch>] [--product <name> \| --group <name>] [--limit <n>] --json` | `pronto-agent-fold-preview/v1` |
| Fleet orientation         | `summary [--product <name> \| --group <name>] --json`                                                       | `pronto-agent-summary/v1`      |
| One repository            | `repo <absolute-repo-path> --json`                                                                          | `pronto-agent-repository/v1`   |
| Quality evidence          | `quality [<repository>] --json`                                                                             | `pronto-agent-quality/v1`      |
| Skill topology            | `skills [<skill-id>] --json`                                                                                | `pronto-skills/v2`             |
| Repository change matrix  | `change-matrix repo <repository> [--operation <add\|change\|remove>] --json`                                | `pronto-change-matrix/v1`      |
| Skill change matrix       | `change-matrix skill <skill-id> [--operation <add\|change\|remove>] --json`                                 | `pronto-change-matrix/v1`      |
| Active remediation        | `remediation [<repository>] --json`                                                                         | `pronto-remediation/v3`        |
| Work requiring attention  | `attention --json`                                                                                          | `pronto-agent-attention/v1`    |
| Recent transitions/audits | `activity [<repository>] --limit <n> --json`                                                                | `pronto-agent-activity/v1`     |
| Preparation preflight     | `prepare <repository> [--workspace <id>] --json`                                                            | `pronto-agent-preparation/v1`  |
| Release preflight         | `release preview <repository> [--workspace <id>] --json`                                                    | `pronto-agent-release/v1`      |

Resolve a repository with `git rev-parse --show-toplevel` and pass the
absolute path, repository name, ID, or an exact workspace path. Do not pass `.`
and assume it will resolve.

The Node adapter normally resolves `cargo` from the documented Homebrew paths
and then `PATH`. Set `PRONTO_CARGO` to an explicit Cargo executable only when a
different verified Rust toolchain is required.

Repository and summary projections include a read-only `project_compass`
summary derived from `.project-compass/contract.json`. `Ready` reports the
current product identity, MVP and complete-product progress, confidence,
blockers, and open drift. `Missing` means the repository has not established a
Compass contract; `Invalid` preserves the parse or contract error. Pronto never
creates or repairs Compass artifacts during a refresh.

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
`remediation set-status`. They do not authorize Git branch cleanup, merging,
pushing, deletion, provider mutation, or release publication.

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
- Preserve `schema_version`, freshness, evidence, and uncertainty fields when
  handing JSON to another agent.
