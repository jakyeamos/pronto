# Task-lane custody

Pronto treats concurrent feature work as task lanes, not as a tree of long-lived
reference branches. One feature owns one `codex/*` branch and one linked Git
worktree. The repository's configured `dev` branch is the integration lane;
`main` or `master` remains the release lane.

The isolated-change workflow owns lane creation, signed custody receipts,
heartbeats, guards, release, and cleanup. Pronto only projects that evidence in
the existing `repo` and composed `route` responses. Quality Runner can validate
the resulting repository state, but it does not assign or transfer custody.

## Custody states

`repo <repository> --json` includes `task_lanes` with schema
`pronto-task-lanes/v1`.

| State         | Meaning                                                                                                                                                |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `active`      | A signed receipt has a lease heartbeat inside the configured grace period.                                                                             |
| `paused`      | The verified owner retained custody but explicitly paused the lane.                                                                                    |
| `stale`       | The lease or workflow state is no longer current, but live Git does not safely match the recorded recoverable lane.                                    |
| `adoptable`   | The signed lease is past its grace period and the recorded worktree and branch still match live Git. Adoption must preserve all commits and dirty WIP. |
| `contested`   | Owner termination or blocked cleanup left recoverable work whose custody needs reconciliation.                                                         |
| `unknown`     | The receipt is unsigned, invalid, unsupported, or otherwise insufficient to establish custody.                                                         |
| `integrating` | The verified owner is folding or reconciling the lane.                                                                                                 |
| `closed`      | The verified workflow reports a terminal lane.                                                                                                         |

An agent process does not own a lane indefinitely. The renewable signed lease
is the custody authority. If an agent stops unexpectedly and does not renew,
the lane first waits through the workflow grace period and then becomes
`adoptable` when its live worktree and branch still match the receipt. Dirty
files do not prevent adoption because adoption is a preservation operation, not
cleanup. A missing or mismatched worktree remains `stale` so another agent does
not claim the wrong filesystem state.

Pronto's adoption flag is coordination evidence, not mutation authority. The
workflow must perform any future ownership transfer atomically and issue a new
signed receipt before the adopting agent mutates the lane.

## Legacy receipt migration

`unknown` is a migration queue, not a permanent ownership state. Existing
unsigned receipts need a bounded one-time reconciliation:

1. Record a migration-review start time and grace deadline for each legacy
   receipt. A returning exact owner may renew and upgrade it to a signed active
   receipt during that window.
2. Inspect the recorded branch and worktree from live Git without changing
   either. Preserve dirty files, commits, and untracked files.
3. After the deadline, assign exactly one disposition:
   - `closed` when the branch is fully integrated or no recoverable work exists;
   - `adoptable` when recoverable work exists and no owner renewed or filed a
     competing signed claim;
   - `contested` when ownership conflicts, Git operations are active, paths do
     not match, or preservation cannot be proven.
4. Claim an adoptable lane atomically by signing a new generation receipt for
   the adopter before permitting mutation. The claim reuses the existing branch
   and worktree when present; if only the branch survives, it creates a linked
   worktree from that branch. It never resets, force-pushes, deletes, or silently
   integrates the legacy work.

The migration needs dry-run inventory plus explicit apply/claim operations in
the isolated-change workflow. Until those mutators exist, Pronto intentionally
keeps legacy receipts `unknown` and reports their count rather than pretending
they are resolved.

## Branch and integration policy

1. Create one task lane from the current configured integration head.
2. Renew its lease while work or review is active.
3. Reuse the same lane through fixes and retries; do not create child feature
   branches merely to continue the same feature.
4. Fold completed work into `dev` through the repository's normal integration
   gates.
5. Promote `dev` to `main` or `master` only through the release workflow.
6. Close and remove the worktree only after preservation and integration are
   proven.

## Incomplete work

Incomplete work may be folded into `dev` only when all of these are true:

- the repository still builds and its required gates pass;
- the incomplete path is inert, disabled, or otherwise unable to activate by
  default;
- negative tests prove the incomplete behavior cannot activate accidentally;
- a machine-readable WIP record names the owner or adopting task, affected
  paths, current state, activation condition, remaining work, and rollback;
- the fold preserves reviewability and does not weaken the release branch.

When those conditions do not hold, preserve the branch and worktree as a
recoverable lane or export a reviewable patch bundle with a ledger pointer.
Never hide incomplete work in an unexplained dirty primary checkout.

## Failure behavior

If the isolated-change status command is missing, times out, fails integrity
checks, or returns malformed data, Pronto reports `source.status: unavailable`,
an empty lane set, and an explicit no-mutation authorization boundary. Missing
custody evidence never becomes an empty-success or takeover signal.
