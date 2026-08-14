# Readiness Inspector route plan

Status: C0 public Showcase admission is complete; a local individual-check
candidate packet now exists, while upstream receipt projection and continuity
proof remain gated.

The [canonical target](../ideal-demo-targets.md#readiness-inspector) owns the
promise and proof gate. Readiness is a declared-goal question, not an aggregate
score.

## 1. Ideal target

**North star:** a maintainer answers whether one repository is ready for a
declared goal through individually explainable checks, owners, predicates,
evidence, and next actions.

**Non-negotiable:** missing, stale, blocked, and not-applicable states remain
separate; no score hides an unproven check.

## 2. Concept materials

The candidate frames now have a local W5 fixture behind them. Upstream receipts
and native follow-up are still concept-to-proof gates.

| Frame      | Visual                                                                           | On-screen line                    | Intended evidence moment    |
| ---------- | -------------------------------------------------------------------------------- | --------------------------------- | --------------------------- |
| 1. Goal    | One declared release or product goal is selected                                 | “Ready for what?”                 | The question has a boundary |
| 2. Checks  | Owner, predicate, evidence, and next action appear per check                     | “Every answer has a reason”       | Readiness is explainable    |
| 3. States  | Passing, failed, blocked, unsupported, stale, and not-applicable checks contrast | “Do not average uncertainty away” | Unknowns stay visible       |
| 4. Handoff | Native receipt links open the upstream evidence                                  | “Follow the next action”          | Continuity is durable       |

**Preview concept.** Show a goal header with four individual checks and one
selected upstream receipt. Do not display a single readiness number.

**Narrative spine.** Declare goal → inspect checks → classify state → follow
evidence → choose next action.

## 3. Build-gap specification

Reviewed baseline: the W5 portfolio MVP records goal-specific checks with owner,
predicate, outcome, evidence, and next action without an aggregate score. The
local candidate packet proves individual state separation and target binding;
Quality Setup/Evidence Replay projection and C5 continuity proof remain open.

Project disposition: `targeted_gap_closure` — consume upstream receipts and
close the individual-check projection.

Gap classes: demo_integration — RI-0; evidence — RI-1; product — RI-2;
packaging — RI-3.

| ID   | Gap to close              | Observable acceptance condition                                                            | Owner             | Required proof               |
| ---- | ------------------------- | ------------------------------------------------------------------------------------------ | ----------------- | ---------------------------- |
| RI-0 | Project upstream receipts | Quality Setup and Evidence Replay receipts appear as owner-bound checks                    | Integration owner | Projection/parity receipt    |
| RI-1 | Preserve individual state | Passing, stale, blocked, missing, contradictory, and not-applicable checks remain distinct | Evidence owner    | State matrix                 |
| RI-2 | Prove native follow-up    | Each failed check opens its native evidence and next action                                | Product owner     | Direct navigation readback   |
| RI-3 | Package the public case   | Preview, short copy, no-auth readiness case, and proof link agree                          | Showcase owner    | Material review and readback |

**Required build order:** RI-0 → RI-1 → RI-2 → RI-3. Video is optional after
the evidence gate.
