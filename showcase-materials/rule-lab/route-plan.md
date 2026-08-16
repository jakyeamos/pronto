# Rule Lab route plan

Status: C0 public Showcase admission is complete; current headless proof and
local visual packaging are verified. Cross-producer, direct IDE, typecheck, and
hosted acceptance remain gated.

The [canonical target](../ideal-demo-targets.md#rule-lab) owns the promise and
proof gate. This route keeps rule editing safe by making counterexamples part of
the story.

## 1. Ideal target

**North star:** a developer inspects a quality finding, edits an isolated rule,
tests positive and negative fixtures, compares gained and lost matches, and
saves a target-bound receipt.

**Non-negotiable:** removing a finding is not success if the edit breaks a
counterexample or loses target identity. Rule approval remains human-owned.

## 2. Concept materials

The local frame below is now a verified binary preview. Two current headless
fixture results pass; direct IDE parity and downstream receipt parity remain
open.

| Frame      | Visual                                                  | On-screen line                       | Intended evidence moment |
| ---------- | ------------------------------------------------------- | ------------------------------------ | ------------------------ |
| 1. Finding | A finding points to the rule and fixture                | “Start from the behavior”            | The edit has a reason    |
| 2. Draft   | Isolated rule change with predicate and target shown    | “Change one thing”                   | Scope is bounded         |
| 3. Compare | Positive and negative fixtures show gained/lost matches | “A fix must keep its counterexample” | Safety is measurable     |
| 4. Receipt | Headless and IDE views agree on the target-bound result | “Save the reasoning”                 | Handoff is durable       |

**Preview concept.** Split the frame between a small rule diff and a positive /
negative fixture comparison; keep the target and receipt state visible.

**Narrative spine.** Finding → isolated edit → fixture comparison → parity →
receipt.

## 3. Build-gap specification

Reviewed baseline: the W2 producer MVP has human rule edit, positive/negative
fixture comparison, target-bound receipts, and a read-only VS Code projection.
The current `dev` revision is clean; pytest, Ruff, and extension tests pass, but
pyright reports eight type errors. Direct IDE acceptance and downstream parity
remain open.

Project disposition: `targeted_gap_closure` — close the cross-producer proof
before producing a release story.

Gap classes: demo_integration — RL-0; product — RL-1; evidence — RL-2;
packaging — RL-3.

| ID   | Gap to close                   | Observable acceptance condition                                                        | Owner              | Required proof                        |
| ---- | ------------------------------ | -------------------------------------------------------------------------------------- | ------------------ | ------------------------------------- |
| RL-0 | Prove the producer handoff     | Receipt seam is documented; **live Quality Lens/Evidence Replay handoff remains open** | Integration owner  | Cross-producer receipt                |
| RL-1 | Prove direct IDE parity        | IDE projection tests pass; **human-visible VS Code parity remains open**               | IDE owner          | Direct readback and parity comparison |
| RL-2 | Preserve false-positive safety | **Verified locally for current positive/negative suite; stale/malformed captures remain open** | Verification owner | Current checkpoint + scenario matrix |
| RL-3 | Package the public case        | **Local partial:** preview, copy, and claim boundary agree; no-auth/readbacks remain open | Showcase owner     | PNG/SVG + hosted/readback evidence    |

**Required build order:** RL-0 → RL-1 → RL-2 → RL-3. RL-2 is partially closed
for the current suite. RL-3 is locally packaged but not release complete.
Video is optional after the evidence gate.
