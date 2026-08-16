# Terrace route plan

Status: steps 1–3 complete. TR-1 through TR-6 are closed on the local evidence
route. TR-2 through TR-5 are committed and fast-forwarded into local `dev` at
`9ac360b0f246aa91c85a15183cf9de10c94330b0`; the source branch remains local-only
because no remote push was authorized. The TR-6 visual capture is a dated
pre-integration artifact and needs a browser-permitted refresh before it can
show the integrated state.

The [canonical target](../ideal-demo-targets.md#terrace) owns the durable promise
and proof gate.

## 1. Ideal target

**North star:** a small feature specification moves through a visible workflow,
hits a genuine quality failure, stops with an exact recovery path, accepts a
bounded correction, and resumes from the safe checkpoint to passing evidence.

**Non-negotiable:** the demo cannot hide or bypass the failing gate. Recovery
must preserve what passed and rerun what owns the failure.

## 2. Concept materials

The frames remain **concept art direction**; stop/resume behavior is proven by
the integrated TR-2 through TR-5 receipts, while the static capture itself
predates that fold.

| Frame              | Visual                                                        | On-screen line                                 | Intended evidence moment            |
| ------------------ | ------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------- |
| 1. The spec        | A three-outcome feature brief enters a staged workflow        | “Turn the spec into observable work”           | Success is defined before execution |
| 2. Progress        | Plan, implementation, and validation stages advance           | “Know what the agent is doing now”             | State is inspectable                |
| 3. Hard stop       | A real type or behavior gate fails and later stages lock      | “Failure stops the workflow”                   | Enforcement is visible              |
| 4. Recovery packet | Exact error, owner, affected step, and safe correction appear | “A stop should tell you how to recover”        | Failure remains actionable          |
| 5. Resume          | The correction passes; workflow resumes from the checkpoint   | “Continue without pretending nothing happened” | Recovery preserves history          |
| 6. Evidence        | Final receipt links spec outcomes to passing checks           | “Done means the outcomes passed”               | Completion is evidence-backed       |

**Preview concept.** A horizontal workflow interrupted by one red gate, then
resuming into a compact green evidence receipt. Headline: “Agent workflows that
stop safely—and know how to continue.”

**Narrative spine.** Spec → visible stages → required failure → actionable stop
→ bounded correction → evidence-backed resume.

## 3. Build-gap specification

Reviewed baseline: the public CLI and staged workflow are integrated and
verified; the remaining local follow-up is a post-integration visual capture.

Project disposition: `targeted_gap_closure` — preserve the public CLI and close
the durable stage-state, failure-packet, replay-proof, and visual workflow gaps.

Gap classes: content — TR-1 (closed); product — TR-2, TR-3; evidence — TR-4, TR-5;
packaging — TR-6.

| ID   | Gap to close                               | Observable acceptance condition                                                                                                                                                                                                                                        | Owner                  | Required proof                                                                                                          |
| ---- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| TR-1 | Define the representative spec and failure | **Closed 2026-08-12.** A historical Terrace regression proves that autonomous routing selected an unrelated roadmap phase instead of preserving the active `gpt56-modernization` workflow; the focused test fails on the parent revision and passes on the bounded fix | Workflow/product owner | `case-spec.md`, `case-fixture.json`, `expected-failure.json`, and `evidence/tr-1-regression-receipt.json`               |
| TR-2 | Expose durable stage state                 | **Integrated and verified 2026-08-13.** Pending, active, passed, failed, and blocked stages persist in a versioned snapshot; a second CLI process returns the same run and states, and replay repairs a deliberately stale snapshot                                    | Runtime owner          | `evidence/tr-2-stage-state-receipt.json`; `evidence/integration-receipt.json`                                           |
| TR-3 | Produce an actionable stop packet          | **Integrated and verified 2026-08-13.** The blocked CLI result and a separate-process resume name the command, evidence, owner, safe next step, and forbidden bypass; later stages remain locked                                                                       | Evidence owner         | `evidence/tr-3-stop-packet-receipt.json`; `evidence/integration-receipt.json`                                           |
| TR-4 | Resume from the correct checkpoint         | **Integrated and verified 2026-08-13.** A supported evidence-bearing correction resumes the same run, retries only the blocked owning stage, preserves the passed plan and its evidence, and completes each required successor once                                    | Runtime owner          | `evidence/tr-4-bounded-resume-receipt.json`; `evidence/integration-receipt.json`                                        |
| TR-5 | Prove bypass resistance                    | **Integrated and verified 2026-08-13.** A forged passing snapshot is repaired from the blocked event history, and an injected terminal event that skips predecessor gates is rejected                                                                                  | Safety/quality owner   | `evidence/tr-5-bypass-resistance-receipt.json`; `evidence/integration-receipt.json`                                     |
| TR-6 | Create the visual workflow surface         | **Closed 2026-08-12 for the local static artifact.** Stages, stop, correction, preserved work, and final evidence pass rendered desktop/mobile comprehension review without raw-log narration                                                                          | Showcase/design owner  | `workflow-preview.html`, responsive captures, `comprehension-review.md`, and `evidence/tr-6-visual-review-receipt.json` |

**Build order:** TR-1 → TR-2/TR-3 → TR-4/TR-5 → TR-6.

**Current closures:** TR-1 through TR-6. **Integrated commit:**
`9ac360b0f246aa91c85a15183cf9de10c94330b0` on local `dev`; the source branch is
preserved as a clean local-only candidate because no remote push was authorized.
The
representative case is Terrace's own
active-feature handoff regression, fixed by commit
`cd44e3b0c634eb2c25820dcd2f2b6857c82aa0cb`. On the parent revision, the
router reports `blocked: false` and selects `phase-11-notifications`; with the
bounded fix, it returns `ACTIVE_FEATURE_NOT_ROADMAP_PHASE`, points to the
feature workbench, and does not create the unrelated plan. TR-2's isolated
candidate adds an ordered `plan → execute → validate → review → complete`
ledger, atomic snapshots, append-only transition events, replay repair, and a
durable `terrace-stop-packet/v1`. A blocked run and later `terrace resume`
return the stopped command, source evidence, responsible owner, safe next
step, and explicit forbidden bypass while later stages stay pending. TR-4 adds
supported blocker discovery and evidence-bearing resolution, then proves a
separate-process retry preserves the passed plan at attempt one, retries the
blocked execute stage at attempt two, and runs each required successor once.
TR-5 proves that replay repairs a forged terminal snapshot to the authoritative
blocked history and refuses an injected terminal event that skips predecessor
gates. TR-6 packages the same bounded-resume trace into a locally reviewed,
responsive workflow surface while keeping the synthetic appendix and
integration boundaries visible. The local `dev` revision now contains the
durable behavior and passes the full CI/CLI proof; the static TR-6 capture was
rendered before that fold and still says “Source candidate” / “Integration
pending” by design. A permitted browser refresh is the only remaining local
packaging follow-up; hosted publication and video remain separate, optional
delivery tracks.
