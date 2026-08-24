# AI Workflow Leverage route plan

Status: AL-1 is closed in a protocol package grounded in a real Tenure
maintenance task. AL-2 is parked as a genuine cross-product blocker: the
current runtime has aggregate run fields but no shared append-only event
contract, and paired measurement belongs to `agent-eval-runtime`. The paired
manual and assisted runs remain unexecuted; AL-2 is still the next required
closure after that owner boundary is resolved.

The [canonical target](../ideal-demo-targets.md#ai-workflow-leverage) owns the
durable promise and proof gate.

## 1. Ideal target

**North star:** compare the same difficult maintenance task performed manually
and with a bounded AI workflow, then show where time was saved, where review
moved, whether quality held, and what the sample cannot establish.

**Non-negotiable:** the result must be paired and evidence-bound. Missing timing,
quality, or recovery data remains unknown; it is never filled with estimates.

## 2. Concept materials

All frames are **concept** until the paired experiment and quality oracle pass.

| Frame             | Visual                                                        | On-screen line                                | Intended evidence moment                |
| ----------------- | ------------------------------------------------------------- | --------------------------------------------- | --------------------------------------- |
| 1. Same task      | One fixed task card splits into manual and assisted lanes     | “Compare the same work”                       | Scope equivalence is visible            |
| 2. Baseline       | Manual lane records active time, handoffs, checks, and result | “Measure the work before improving it”        | Baseline is not retrospective guesswork |
| 3. Assisted run   | AI lane separates automation, waiting, and human review       | “Move effort—do not hide it”                  | Labor displacement is inspectable       |
| 4. Quality oracle | Both outcomes face the same behavioral and review checks      | “Speed only counts if quality holds”          | Comparison uses one standard            |
| 5. Result         | Paired deltas show time, touch count, quality, and recovery   | “Here is the gain we actually observed”       | Raw evidence supports summary           |
| 6. Limits         | Sample size and confounders sit beside the conclusion         | “One result is evidence, not a universal law” | Claim strength is bounded               |

**Preview concept.** Two clean lanes converging on the same quality gate, with a
paired delta card below. Headline: “Measure AI leverage without grading it on a
curve.”

**Narrative spine.** Fixed task → observed baseline → assisted workflow → same
oracle → paired result → explicit limits.

## 3. Build-gap specification

Reviewed baseline: an evidence-bounded measurement concept exists, but public
materials and trustworthy outcome proof do not.

Project disposition: `material_build_or_restoration` — the measurement system
and prospective paired run must exist before the intended claim can be proven.

Gap classes: content — AL-1; product — AL-2; evidence — AL-3, AL-4, AL-5,
AL-6.

| ID   | Gap to close                                       | Observable acceptance condition                                                                                       | Owner                   | Required proof                          |
| ---- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ----------------------- | --------------------------------------- |
| AL-1 | Select a comparable task and oracle                | Manual and assisted lanes use identical inputs, completion criteria, exclusions, and behavioral checks                | Experiment owner        | Protocol and fixed fixture              |
| AL-2 | Instrument both lanes                              | Active work, wait time, retries, human touches, failures, and outcome evidence are captured with the same definitions | Measurement owner       | Raw event records and field dictionary  |
| AL-3 | Run the manual baseline prospectively              | The baseline completes without reconstruction or substituted telemetry                                                | Human operator          | Timestamped baseline receipt and output |
| AL-4 | Run the bounded assisted workflow                  | The assisted lane completes under recorded model, tool, authority, and retry constraints                              | Workflow owner          | Assisted-run receipt and output         |
| AL-5 | Adjudicate outcome quality blindly where practical | Both outputs receive the same oracle and any human scoring criteria are applied without lane favoritism               | Review owner            | Quality results and adjudication notes  |
| AL-6 | Publish only supported deltas                      | Summary metrics reconcile to raw observations; unknowns and confounders remain visible                                | Analysis/showcase owner | Reconciliation check and claim ledger   |

**Build order:** AL-1 → AL-2 → AL-3 → AL-4 → AL-5/AL-6.

## 4. AL-1 closure

AL-1 selects the real Tenure task **Expose Tenure review and capture
task-state contracts**. Both future lanes start from protected revision
`a304ce866e1bced294a12f9915cae55ac2b65b13` and use the same brief, allowlisted
paths, exclusions, and oracle. The recorded implementation at
`5dd52328a1a847466db8a7f12c7e6f71b468182e` is a reference contract only; it is
not a third lane and does not prove AI leverage.

- [`protocol.md`](protocol.md) explains the comparable task and lane rules.
- [`case-fixture.json`](case-fixture.json) fixes the real inputs, scope, and
  shared quality oracle.
- [`synthetic-fixture.json`](synthetic-fixture.json) is the short,
  deterministic reproducibility appendix; it does not replace Tenure evidence.
- [`evidence/protocol-receipt.json`](evidence/protocol-receipt.json) records
  the passed AL-1 checks and keeps both paired results explicitly `not_run`.

AL-2 must instrument active work, wait time, retries, human touches, failures,
and outcome evidence before either lane is run. Missing telemetry remains
unknown rather than estimated.

## 5. AL-2 blocker

The current `ai-workflow-leverage` surface records aggregate run fields and a
typed intervention total, but it does not record the raw event stream required
for a defensible paired comparison. Its own product topology assigns paired
comparisons, eval runs, harnesses, and benchmarks to `agent-eval-runtime`.

- [`evidence/al-2-blocker.json`](evidence/al-2-blocker.json) records the live
  boundary, the observed fields, and the exact missing contract.
- No second measurement engine is being added to this repository as a showcase
  shortcut.
- AL-3 and AL-4 stay parked until the owner approves and implements the shared
  event contract; no timing, retry, human-touch, failure, or outcome values are
  inferred in the meantime.

## 6. Local showcase package

The agent-owned material layer is now assembled locally: the real Tenure task
case, paired-protocol explanation, claim ledger, no-auth page source, and
reviewed 1600×900 preview are present in this directory. The preview shows the
manual and assisted lanes converging on one oracle, then stops at AL-2 rather
than manufacturing a result delta.

This closes local case/visual packaging only. The shared event contract,
prospective paired runs, outcome reconciliation, hosted no-auth verification,
and external destination readbacks remain open; no AI-leverage claim is made.
