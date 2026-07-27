# Agent command contract

## Invocation

Run the current checkout's CLI through `pnpm`; do not rely on an unverified
standalone binary:

```sh
PRONTO_ROOT=/Users/jakyeamos/Documents/pronto
pnpm --dir "$PRONTO_ROOT" run cli next --json
```

The CLI reads and writes Pronto's local SQLite-backed snapshot. JSON is the
preferred agent interface. `status --json` remains the complete legacy
snapshot; use a focused projection when one is available.

## Focused read and preview surfaces

| Need                      | Command                                                  | Contract                      |
| ------------------------- | -------------------------------------------------------- | ----------------------------- |
| Daily orientation         | `next [<repository>] [--limit <n>] --json`               | `pronto-agent-next/v1`        |
| Fleet orientation         | `summary --json`                                         | `pronto-agent-summary/v1`     |
| One repository            | `repo <absolute-repo-path> --json`                       | `pronto-agent-repository/v1`  |
| Quality evidence          | `quality [<repository>] --json`                          | `pronto-agent-quality/v1`     |
| Work requiring attention  | `attention --json`                                       | `pronto-agent-attention/v1`   |
| Recent transitions/audits | `activity [<repository>] --limit <n> --json`             | `pronto-agent-activity/v1`    |
| Preparation preflight     | `prepare <repository> [--workspace <id>] --json`         | `pronto-agent-preparation/v1` |
| Release preflight         | `release preview <repository> [--workspace <id>] --json` | `pronto-agent-release/v1`     |

Resolve a repository with `git rev-parse --show-toplevel` and pass the
absolute path, repository name, ID, or an exact workspace path. Do not pass `.`
and assume it will resolve.

## Refresh and state boundaries

Use `refresh <repository> --json` when the persisted snapshot is stale or after
a branch/workspace change that needs fresh evidence. It performs a local
read-only Git scan but persists the resulting snapshot and audit record, so it
is state-changing even though it does not modify a repository.

The following commands can change local Pronto state or touch an external
boundary and require explicit task scope: `root add`, `root exclude`, `refresh`,
`refresh-github`, `clone`, `remediation refresh`, `remediation export`, and
`remediation set-status`. They do not authorize Git branch cleanup, merging,
pushing, deletion, provider mutation, or release publication.

There is no Pronto command that means “clean branches,” “fold dev,” “delete
branch,” or “push.” Use Pronto for inventory and preflight, then use the
reviewed branch-folding workflow and ordinary Git commands within their own
authorization boundaries.

## Evidence interpretation

- `generated_at` identifies the snapshot time; re-check it after a meaningful
  state change.
- Workspace `sync_state` is healthy only when it is exactly `Synced`.
- `Ahead by N`, `Behind by N`, divergence, `No upstream`, dirty workspaces,
  active operations, missing/stale quality evidence, `Unknown`, and `Blocked`
  require attention or an explicit explanation.
- A preview or configured ideal is not execution proof. Keep source tests,
  local validation, live browser/device evidence, and release/provider proof
  distinct.
- Preserve `schema_version`, freshness, evidence, and uncertainty fields when
  handing JSON to another agent.
