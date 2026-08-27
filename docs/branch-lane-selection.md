# Branch lane selection

Last reviewed: 2026-08-27.

Pronto uses three development lanes: `direct_dev`, `short_lived`, and
`long_lived`. Select the lane before the first repository mutation. The choice
is about custody, review, and integration boundaries; it is not inferred from
diff size, line count, or an informal "non-trivial" label.

The documented integration branch is the default target (`dev` when the
repository configures it). `main` or `master` remains the release branch.

## Decision record

Every task that changes a repository records these fields in its handoff or
receipt:

```text
lane: direct_dev | short_lived | long_lived
reason: <the selected predicate or explicit designation>
target: <integration or designated long-lived branch>
owner: <task or human owner>
verification: <focused checks required before integration>
exit_condition: <fold, release, or explicit continuation condition>
review_by: <date or not_applicable>
```

The record is evidence of a decision, not mutation authority. Git hooks,
repository protections, custody receipts, and required quality gates still
control what can be committed or integrated.

## Selection matrix

| Lane | Select when | Execution and close condition |
| --- | --- | --- |
| `direct_dev` | The change is bounded, reversible, single-repository, atomically committable, and has no required review/protection boundary, parallel owner, exact path conflict, interrupted Git operation, or separate release track. | Work on the documented integration branch. Commit the coherent unit, run focused verification, and leave the target current and clean. |
| `short_lived` | Isolation is required for review/protection, parallel or owner-active work, an exact conflict, a risky or irreversible change, a migration/schema/security boundary, a multi-repository change, or a bounded experiment that cannot safely land directly. | Create a receipt-backed isolated `codex/*` lane. Name the target and fold condition before starting. Fold into `dev` or the explicitly named long-lived target promptly after focused verification, then close the lane only after preservation and integration are proven. |
| `long_lived` | The user or repository contract explicitly designates a sustained product, release, compatibility, or experiment track. | Keep the named line's owner, purpose, target, sync cadence, review-by date, and exit/integration condition current. It is a target/lifecycle designation, not a permanent branch for every task. |

A short-lived lane may target a designated long-lived line when that target is
explicitly recorded. Otherwise it targets the documented integration branch.
Routine work on a designated long-lived line may use that line only under its
own contract; a change that needs isolation uses a short-lived child lane and
folds into the named line.

## Precedence and dirty targets

Apply these rules in order:

1. An explicit long-lived designation determines the sustained target, but it
   does not waive review, ownership, custody, or verification requirements.
2. Any hard isolation predicate selects `short_lived` execution. If the task
   targets a designated long-lived line, that line is the fold target.
3. Otherwise select `direct_dev` for the documented integration branch.

A dirty integration branch alone never selects `short_lived`. Inspect and
classify the existing paths first. Path-disjoint, safely stageable work may
remain on the integration branch with explicit-path staging. Active ownership,
mixed intent, an exact conflict, or an interrupted operation requires
preservation and an isolated lane. Do not overwrite, stash, or silently fold
unowned work.

Long-lived status is never inferred from age, estimated duration, branch name,
or diff size. Until the designation is recorded in the repository contract or
an accepted retention record, treat the branch as short-lived for admission,
lease, and retirement purposes. A designation also never authorizes deletion;
pruning still requires live ancestry or patch-equivalence, ownership, remote,
and consumer evidence.

## Lifecycle expectation

Short-lived lanes are for one bounded change and should fold in the next
available verification/review cycle. Reuse the same lane through fixes and
retries; do not create child branches for the same task.

Long-lived lines are reviewed on their recorded cadence and integrated into
`dev` according to their contract. They must not become a holding area for
unfinished short-lived work or an excuse to defer a clear integration decision.
