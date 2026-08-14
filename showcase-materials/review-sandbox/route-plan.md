# Review Sandbox route plan

Status: C0 public Showcase admission is complete; failure, cancellation, and
retained-dirty proof remain gated.

The [canonical target](../ideal-demo-targets.md#review-sandbox) owns the
promise and proof gate. Disposable state is useful only when cleanup is itself
provable.

## 1. Ideal target

**North star:** a reviewer exercises one repository-declared behavior in
disposable state, retains trustworthy evidence, and cleans up only after proving
inactivity.

**Non-negotiable:** the primary checkout and unrelated dirty state remain safe;
failed, cancelled, and uncertain sandboxes are retained and visible.

## 2. Concept materials

All frames below are **concept frames** until the scenario and cleanup matrix
passes.

| Frame           | Visual                                                 | On-screen line                    | Intended evidence moment |
| --------------- | ------------------------------------------------------ | --------------------------------- | ------------------------ |
| 1. Preview      | Declared scenario, source revision, and cleanup policy | “Know the disposable boundary”    | Scope is clear           |
| 2. Create       | Isolated worktree appears with target identity         | “Try it without touching main”    | Isolation is visible     |
| 3. Exercise     | Meaningful behavior runs with evidence and gate state  | “A sandbox still tells the truth” | Review result is bounded |
| 4. Retain/clean | Clean case removes state; dirty or failed case remains | “Cleanup follows proof”           | Safety is the result     |

**Preview concept.** Show a scenario card with clean, conflict, failed-gate,
cancellation, and retained-dirty outcomes branching from one preview.

**Narrative spine.** Declare → preview → isolate → exercise → prove inactivity
→ clean or retain.

## 3. Build-gap specification

Reviewed baseline: the W4 disposable-workspace MVP supports preview/create/
cleanup for repository-declared scenarios with dirty-state retention. Failure,
cancellation, and full cleanup proof remain open.

Project disposition: `targeted_gap_closure` — exercise the complete scenario
matrix without mutating the primary checkout.

Gap classes: demo_integration — RS-0; product — RS-1; evidence — RS-2;
packaging — RS-3.

| ID   | Gap to close                     | Observable acceptance condition                                                                    | Owner              | Required proof               |
| ---- | -------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------- |
| RS-0 | Prove the scenario matrix        | Clean, conflict, failed-gate, cancellation, and retained-dirty cases complete with explicit states | Verification owner | Scenario receipts            |
| RS-1 | Preserve primary checkout safety | Creation, exercise, and cleanup leave refs and unrelated dirty files unchanged                     | Safety owner       | Before/after repository diff |
| RS-2 | Prove cleanup authority          | Clean removal occurs only after inactivity; uncertain state is retained                            | Product owner      | Cleanup/refusal receipts     |
| RS-3 | Package the public case          | Preview, short copy, no-auth sandbox case, and proof link agree                                    | Showcase owner     | Material review and readback |

**Required build order:** RS-0 → RS-1 → RS-2 → RS-3. Video is optional after
the evidence gate.
