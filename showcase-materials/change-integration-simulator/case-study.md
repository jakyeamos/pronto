# Change Integration Simulator: mergeability is not verification

## Target

Change Integration Simulator pins exact source and target commits, reports a
bounded clean or conflict result, runs any declared gate as a separate probe,
and leaves both refs untouched.

## Current local checkpoint

- Source: `dev` at `5b07ed441e1e662dc7c08c948a444f6f04fac0cc`.
- A clean fixture resolved source `106b668` into target `8216268` with merge
  base and tree identity; a conflict fixture named `file.txt` and produced no
  merge tree.
- `git diff-tree --check -r target` passed as a separately labeled local gate
  probe. Source and target refs were unchanged after both simulations, and a
  missing-ref input was rejected as unsupported.
- Tests, lint, and package checks passed. The crop-safe 1600×900 PNG/SVG
  preview was visually reviewed.

## Boundaries

Workflow Gateboard handoff, stale/cancellation/retained-workspace breadth,
remote CI, hosted no-auth access, and GitHub, portfolio, and Handshake
readbacks remain open.
