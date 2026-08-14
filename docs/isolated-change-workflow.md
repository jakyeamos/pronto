# Isolated change workflow

Pronto treats a dirty or owner-active primary checkout as a reason to route new
work into a clean task worktree. It is not, by itself, a reason to leave an
otherwise completed feature uncommitted.

The canonical workflow is distributed as the `isolated-change-workflow` skill.
Its bundled command records an exact base, creates a locked `codex/*` worktree,
guards mutations and commits against a task receipt, and verifies the resulting
branch against the current integration target with a non-mutating merge-tree
probe.

## Responsibility split

- Git worktrees provide file and index isolation.
- Ordinary commits provide GitHub-native authorship, review, CI, revert, and
  blame history.
- Untracked receipts under the common Git directory preserve task/base/worktree
  custody without adding product files to commits.
- The existing Pronto fold preview and `fold-feature-branches` workflow remain
  the integration queue.
- An optional managed pre-commit hook rejects commits outside receipt-backed
  task worktrees. Installation is repository-scoped and explicit; an external
  shared `core.hooksPath` is preserved and reported as an ownership blocker.

The workflow does not infer ownership after same-line edits have already been
combined. An actual merge-tree conflict remains a review blocker and must name
the affected paths and incompatible intent.

## Worktree placement

Task worktrees live outside the source checkout. The default root is
`~/.codex/worktrees`; when that location is unavailable, use an approved external
scratch root such as `/private/tmp/<task>-worktrees`. The command rejects a
resolved `--worktree-root` equal to or nested beneath the source checkout before
creating directories, branches, or receipts. This prevents a linked-worktree
container from becoming new untracked state in an already dirty primary checkout.

Fresh-agent verification fixtures must commit every repository-required hook and
test configuration file before introducing simulated owner-active changes.
Intentionally ignored configuration such as `.pre-cr.json` must be force-added to
the fixture baseline, and fixture setup must assert that the exact-base task lane
contains it. Primary-only ignored or untracked files are not task prerequisites.

## Agent completion rule

An agent encountering overlapping pre-existing work must attempt the isolated
lane before reporting a commit blocker. A valid blocker identifies the exact
conflicting paths or lines, owner or protected process, failed custody or merge
check, and the decision needed to proceed. Labels such as `pre-existing`,
`unrelated`, `mixed`, or `overlapping` are not sufficient dispositions.

## Validation

Run the skill's structural validator and its fixture suite:

```sh
python3 ~/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
  skills/isolated-change-workflow
python3 -m unittest discover \
  -s skills/isolated-change-workflow/scripts \
  -p 'test_*.py'
```

The fixture suite covers dirty-primary preservation, external-root acceptance,
nested-root rejection without mutation, committed hook-configuration transfer,
primary-checkout rejection, receipt-backed commits, target same-line conflicts,
and refusal to replace an existing unmanaged hook.
