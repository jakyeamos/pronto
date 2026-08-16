# Terrace TR-1 case: preserve the active workflow on a blocked autonomous run

This case comes from Terrace's own product history. It reproduces the regression
fixed by commit `cd44e3b0c634eb2c25820dcd2f2b6857c82aa0cb`, rather than inventing a
showcase-only failure.

## Representative specification

When an autonomous Terrace run encounters a gate-complete active senior-cycle
feature that is not a roadmap phase, it must:

1. keep that feature as the active workflow;
2. return a structured blocked handoff with code
   `ACTIVE_FEATURE_NOT_ROADMAP_PHASE`;
3. name `terrace workbench status --feature gpt56-modernization` as the safe
   next command; and
4. leave `.terrace/state.json` and the unrelated roadmap plan unchanged.

The fourth outcome is the proof boundary. A safe stop is not enough if the
router has already replanned unrelated work.

## Real failure

At parent commit `77f1f5e37160264e913b2df9386e42da4fa51cc4`, the focused regression
test exits 1. Terrace ignores the active `gpt56-modernization` feature, reports
`blocked: false`, and routes to `terrace phase show phase-11-notifications`.
The failure belongs to the workflow-routing validation stage; no later
implementation, review, or completion stage should run.

## Bounded correction

Commit `cd44e3b0c634eb2c25820dcd2f2b6857c82aa0cb` adds an explicit
active-feature handoff and makes `autonomousWorkflow` stop before phase
planning. The same focused regression test then exits 0.

## Reproduction boundary

The fixture uses the fixed commit's repository and regression test, then swaps
only `packages/terrace-core/src/workflow.cjs` to the parent revision for the
failing run. This isolates the owning stage and avoids depending on unrelated
historical package state. `case-fixture.json` records the exact revisions and
`expected-failure.json` records the observed mismatch.

TR-1 proves the case and failure. It does not yet prove durable stage state,
restart-safe replay, the final stop-packet contract, or bypass resistance;
those remain TR-2 through TR-5.
