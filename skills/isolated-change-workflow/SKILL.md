---
name: isolated-change-workflow
description: Create and verify task-owned Git worktrees so repository changes remain attributable and reviewable when a primary checkout is dirty, owner-active, or contains overlapping pre-existing work. Use before non-trivial repository implementation when the current checkout is not a clean task-owned lane; when an agent cites pre-existing, unrelated, mixed, or overlapping changes as a commit blocker; when preparing parallel agent work; or when handing a completed task into an integration queue. Provides exact-base receipts, a mutation guard, optional pre-commit enforcement, conflict-aware readiness checks, and a clean handoff to the repository's existing integration workflow.
---

# Isolated Change Workflow

Turn dirty or owner-active primary checkouts into routing conditions, not blanket
completion blockers. Keep each task on a real branch in a dedicated Git worktree,
then integrate its ordinary commits through the repository's documented lane.

## Hard rule

Before modifying a non-trivial repository, prove that the current directory is a
task-owned isolated worktree. If it is not, create one with the bundled tool and
continue there. Do not edit first and attempt retrospective attribution later.

Never stash, reset, clean, overwrite, stage, or commit the primary checkout merely
to make it usable. Preserve its staged, unstaged, untracked, and unpublished state.

## Verify prerequisites

Resolve and report Git and Python before using the tool:

```sh
command -v git && git --version
command -v python3 && python3 --version
```

Set `SKILL_DIR` to this skill directory and use:

```sh
python3 "$SKILL_DIR/scripts/isolated_change.py" <command>
```

## Start the task lane

From any path inside the repository:

```sh
python3 "$SKILL_DIR/scripts/isolated_change.py" start \
  --repo "$(git rev-parse --show-toplevel)" \
  --task <short-task-name> \
  --base HEAD \
  --json
```

The command records the immutable base commit, creates a `codex/<task>` branch in
a locked linked worktree, and writes an untracked custody receipt under the common
Git directory. Existing primary-checkout edits are neither copied nor changed.

The default worktree root is `~/.codex/worktrees`. If sandbox permissions block
that location, request the required scoped permission or pass `--worktree-root`
with an approved external scratch root such as `/private/tmp/<task>-worktrees`.
The resolved root must be outside the source checkout; the tool rejects nested
roots before creating a directory or branch. Never place `.codex-task-worktrees`
or another linked-worktree container inside the primary checkout.

Fresh-agent fixtures must commit repository-required hook and test configuration,
including `.pre-cr.json` when applicable, before owner-active dirty state is
introduced. Force-add intentionally ignored required configuration and verify it
exists in the exact-base task worktree. Ignored or untracked primary-only files
are deliberately not copied into the task lane.

Use the returned `worktree` as the working directory for every edit, test, stage,
and commit in the task. Run the guard before the first mutation:

```sh
python3 "$SKILL_DIR/scripts/isolated_change.py" guard --repo "$PWD" --json
```

A failed guard is a hard stop for mutation in that directory. Move to the returned
task worktree or create a new lane; do not downgrade the failure to a warning.

## Commit coherent changes

Create ordinary, reviewable Git commits on the task branch. Stage exact paths or
hunks. Keep generated/mechanical changes separate when that improves review.
Never use the receipt as commit authorization for unrelated paths.

If requirements change substantially, start another task lane or record the scope
change explicitly before proceeding. Same-line product conflicts still require an
explicit resolution; isolation preserves provenance but cannot infer intent.

## Verify integration readiness

After tests pass and the task worktree is clean:

```sh
python3 "$SKILL_DIR/scripts/isolated_change.py" verify \
  --repo "$PWD" \
  --target <canonical-integration-ref> \
  --json
```

Require `ready_for_integration: true`. The verifier checks the receipt, branch,
base ancestry, clean state, task commits, changed paths, target movement, and a
non-mutating merge-tree conflict probe. File overlap is reported for review; an
actual merge conflict blocks readiness.

List all receipts for a repository with:

```sh
python3 "$SKILL_DIR/scripts/isolated_change.py" status \
  --repo "$(git rev-parse --show-toplevel)" \
  --json
```

Route a ready branch into the repository's existing PR, merge queue, or
`fold-feature-branches` workflow. Do not add a second integration mechanism here.

## Optional repository enforcement

Install the managed pre-commit guard only with explicit authority for that
repository:

```sh
python3 "$SKILL_DIR/scripts/isolated_change.py" install-hook \
  --repo "$(git rev-parse --show-toplevel)" \
  --apply \
  --json
```

The installer refuses to overwrite an existing unmanaged pre-commit hook. The
managed hook blocks commits from any worktree without a matching active receipt.
It does not prevent `--no-verify`; repository policy must continue to prohibit
hook bypasses.

The installer also refuses a `core.hooksPath` outside the repository's common Git
directory. Treat a shared hook manager as a separate owner and integrate through
its documented extension point instead of overwriting it.

Do not silently install the hook fleet-wide. Persistent enforcement across
multiple repositories requires explicit scope and a repository-by-repository
compatibility check.

## Completion contract

Do not report “overlapping pre-existing work” as a complete blocker unless all of
these are true:

1. A clean exact-base task lane was attempted or is technically impossible.
2. The conflict occurs in exact lines or inseparable generated state, not merely
   the same file.
3. The owner or protected process and exact affected paths are identified.
4. A patch, commit, or receipt-backed handoff cannot preserve the task safely.
5. The handoff names the concrete human decision or authority required.

Otherwise, isolate the task, finish its commits, and report primary-checkout work
as preserved rather than as an excuse for an uncommitted feature.
