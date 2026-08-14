# Debug Trail route plan

Status: C0 public Showcase admission is complete; the current `dev` checkout's
headless continuation workflow and binary preview are verified, while direct
IDE proof and hosted packaging remain gated.

The [canonical target](../ideal-demo-targets.md#debug-trail) owns the durable
promise and proof gate. This route plan keeps the product focused on continuity,
not a generic transcript viewer.

## 1. Ideal target

**North star:** a developer leaves a bounded investigation with its hypothesis,
experiment, evidence, and rerunnable next step so an inheritor can continue
without reconstructing the author's mental state.

**Non-negotiable:** target identity, command scope, redaction, uncertainty, and
continuation state remain explicit. A plausible narrative is not a completed
debugging result.

## 2. Concept materials

All frames below are **concept frames** until the canonical workflow and
target-owned receipts pass.

| Frame           | Visual                                                     | On-screen line                       | Intended evidence moment                |
| --------------- | ---------------------------------------------------------- | ------------------------------------ | --------------------------------------- |
| 1. Hypothesis   | A target-bound investigation card names the question       | “Leave the question behind”          | The inheritor sees the exact problem    |
| 2. Experiment   | One allowlisted command is previewed with scope and expiry | “Run only what was declared”         | Authority is bounded                    |
| 3. Evidence     | Result, redaction, omissions, and uncertainty are visible  | “An inconclusive result still helps” | Failure is not turned into a conclusion |
| 4. Continuation | Receipt opens the next action and target identity          | “Pick up without reconstruction”     | Continuity is the product result        |

**Preview concept.** Show a dark-on-light trail card with hypothesis, one
redacted command, outcome, and next action. Avoid a wall of terminal output.

**Narrative spine.** Hypothesis → bounded experiment → evidence → continuation.

## 3. Build-gap specification

Reviewed baseline: the current clean `dev` checkout contains the W1 pilot with
target-bound trails, declared-command preview, authorization, redacted
evidence, a continuation receipt, and a thin VS Code surface; four tests, lint,
and the package contract pass on
`3756eed8680defe7680e5b27ebb1e790eabc4d70`. A crop-safe 1600 × 900 PNG is
also visually reviewed. Direct IDE continuity and canonical public acceptance
remain unproven.

Project disposition: `material_build_or_restoration` — restore the product
workflow on the canonical branch before public packaging.

Gap classes: product — DT-0; demo_integration — DT-1; evidence — DT-2;
packaging — DT-3.

| ID   | Gap to close                       | Observable acceptance condition                                                         | Owner                 | Required proof                                 |
| ---- | ---------------------------------- | --------------------------------------------------------------------------------------- | --------------------- | ---------------------------------------------- |
| DT-0 | Restore the continuation workflow  | A user can create, inspect, and continue a target-bound trail on the canonical path     | Product owner         | Current `dev` implementation, headless checkpoint, and focused tests (verified); IDE readback remains DT-1 |
| DT-1 | Prove direct IDE continuity        | The IDE opens the same trail and next action without reconstructing state               | IDE/integration owner | Direct IDE readback                            |
| DT-2 | Exercise stale and failed handoffs | Failed, stale, cancelled, and rerunnable experiments remain distinct                    | Verification owner    | Bounded scenario matrix and receipts           |
| DT-3 | Package the public case            | Preview, short description, no-auth case, and linked receipt use the same bounded claim | Showcase owner        | Binary preview verified locally; hosted URL/readback receipt remains open |

**Required build order:** DT-0 → DT-1 → DT-2 → DT-3. Video is optional after
the evidence gate.
