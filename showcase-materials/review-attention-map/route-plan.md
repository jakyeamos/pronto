# Review Attention Map route plan

Status: C0 public Showcase admission is complete; the current-dev overlay,
unmatched-state, review-disposition, and binary preview checkpoint are
verified. Producer handoff, broader negative coverage, hosting, and readbacks
remain gated.

The [canonical target](../ideal-demo-targets.md#review-attention-map) owns the
promise and proof gate. Attention is an evidence-grounded suggestion, never an
opaque risk score or merge verdict.

## 1. Ideal target

**North star:** a reviewer follows concrete contract and behavior signals onto
a diff, sees why an area deserves attention, and navigates to originating
evidence.

**Non-negotiable:** every signal keeps source, freshness, unmatched state, and
review disposition visible. The map does not decide whether to merge.

## 2. Concept materials

The frame set is now a reviewed local material package. The integrated producer
handoff and hosted case remain future gates.

| Frame          | Visual                                                        | On-screen line              | Intended evidence moment    |
| -------------- | ------------------------------------------------------------- | --------------------------- | --------------------------- |
| 1. Diff        | One changed area is selected                                  | “Start where the change is” | Review scope is concrete    |
| 2. Overlay     | Contract and behavior signals align on the same area          | “Attention has sources”     | Evidence is grounded        |
| 3. Inspect     | Source, freshness, and unmatched state open beside the signal | “Follow the reason”         | Uncertainty remains visible |
| 4. Disposition | Reviewer records a follow-up without a hidden verdict         | “Judgment stays human”      | The surface supports review |

**Preview concept.** A diff pane with two labeled signals, an evidence drawer,
and a visible stale/unmatched badge. Do not use a numerical risk score.

**Narrative spine.** Diff → overlay → inspect source → expose uncertainty →
record review disposition.

## 3. Build-gap specification

Reviewed baseline: the current-dev review MVP overlays explicit contract and
behavior signals with source evidence, records a human disposition, and keeps
an outside-diff signal unmatched. Producer freshness handoff and broader
negative coverage remain open.

Project disposition: `targeted_gap_closure` — prove one two-source overlay and
direct navigation to originating evidence.

Gap classes: evidence — RAM-0; demo_integration — RAM-1; product — RAM-2;
packaging — RAM-3.

| ID    | Gap to close                           | Observable acceptance condition                                                | Owner             | Required proof               |
| ----- | -------------------------------------- | ------------------------------------------------------------------------------ | ----------------- | ---------------------------- |
| RAM-0 | Prove the two-source overlay           | Current producer contract and behavior signals align on a diff with source and freshness intact | Evidence owner    | Current-dev overlay receipt              |
| RAM-1 | Prove direct navigation                | Each signal opens its originating receipt or source location                   | Integration owner | Current-surface readback     |
| RAM-2 | Preserve unmatched and reviewed states | Missing, stale, unmatched, and human-reviewed signals remain distinct          | Product owner     | Negative scenario matrix     |
| RAM-3 | Package the public case                | Reviewed 1600×900 PNG/SVG preview, short copy, no-auth review case, and proof link agree                 | Showcase owner    | Material review and hosted/readback |

**Required build order:** RAM-0 → RAM-1 → RAM-2 → RAM-3. Video is optional
after the evidence gate.
