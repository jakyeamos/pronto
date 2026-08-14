# Review Sandbox: cleanup follows proof

## Target

Review Sandbox runs one repository-declared behavior in a detached worktree,
retains failed or dirty state, and removes a workspace only after a clean
status proof. The primary checkout is outside the disposable boundary.

## Current local checkpoint

- Source: `dev` at `6d657e2fc1efdb06255bef00b7ffdd89c07f58e7`.
- Clean and failed-setup scenarios created and removed clean worktrees;
  failed setup was not run.
- A dirty scenario was retained on the first cleanup and removed after the
  owned dirty artifact was recovered. An occupied path refused overwrite.
- A declared `exit 130` was recorded as failed, making the missing distinct
  cancellation state explicit. The primary fixture checkout stayed unchanged.
- Tests, lint, and package checks passed. The crop-safe 1600×900 PNG/SVG
  preview was visually reviewed.

## Boundaries

Process/port ownership and a dedicated cancellation state are not yet proven.
Hosted no-auth access and GitHub, portfolio, and Handshake readbacks remain
open.
