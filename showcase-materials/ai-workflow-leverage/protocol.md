# AL-1 — Tenure review task-state observability

Status: **closed locally as a protocol package**. The paired manual and
assisted runs are not yet executed, so this artifact does not claim a speed or
quality gain.

## Why this task

The experiment needs a real maintenance task with a bounded surface and a
reviewable result. Tenure's review and capture controls are a good case because
the task has an explicit product outcome: make the UI's current task and action
state machine discoverable to an automation surface without changing the
underlying review policy.

The recorded reference implementation is the real Tenure change from
`a304ce866e1bced294a12f9915cae55ac2b65b13` to
`5dd52328a1a847466db8a7f12c7e6f71b468182e`. It anchors the task and its
acceptance contract; it is **not** a measurement result.
It is a reference contract only, not a third lane.

## Fixed experiment inputs

Both lanes start from the exact protected baseline and receive the same fixed
inputs and brief:

> Expose stable `data-mac-control-id` and `data-task-state` values on the
> review-session, review-workspace, approve, bulk-action, export, and capture
> popup surfaces. Preserve the existing approval, export, and capture
> semantics. Add or update focused tests for the observable contract.

Allowed surfaces, completion criteria, exclusions, and the paired-run fields
are machine-readable in [`case-fixture.json`](case-fixture.json).

The manual lane is completed without an AI assistant. The assisted lane uses a
bounded AI workflow with the same repository, task brief, file scope, command
budget, and human review stop. Neither lane may use provider-backed deployment,
remote database mutation, or an unreviewed commit.

## Shared quality oracle

The result is accepted only when every item below is true in both lanes:

1. The required controls expose stable IDs and state values for the same
   observable states; state changes do not alter approval or capture policy.
2. The focused QueueShell and popup rehydration tests pass from the lane's
   recorded revision.
3. The lane stays inside the allowlisted paths and introduces no migration,
   secret, dependency, deployment, or unrelated UI redesign.
4. The final revision, test receipts, changed-path list, retries, failures, and
   human touches are recorded with the same field definitions.

The oracle intentionally evaluates behavior and scope, not whether a lane's
implementation resembles the recorded reference diff.

## Synthetic appendix

[`synthetic-fixture.json`](synthetic-fixture.json) is a small deterministic
reproduction of the observable-state contract. It exercises one valid state
map, one missing-state negative case, and one invalid-state boundary case. It
does not replace the real Tenure task or supply timing data.

## Claim boundary

AL-1 proves that one comparable task, fixed input set, shared oracle, and
synthetic appendix have been selected. It does not prove that AI is faster,
that the current Tenure implementation is a fresh release, or that either
paired lane has completed. Those claims remain unknown until AL-2 through AL-6
produce prospective evidence.
