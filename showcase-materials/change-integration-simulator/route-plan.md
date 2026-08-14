# Change Integration Simulator route plan

Status: C0 public Showcase admission is complete; current-dev clean/conflict
simulation, separate gate probe, ref readback, and binary preview are verified.
Gate handoff, negative breadth, hosting, and readbacks remain gated.

The [canonical target](../ideal-demo-targets.md#change-integration-simulator)
owns the promise and proof gate. The simulator is read-only and separates
textual mergeability from behavioral verification.

## 1. Ideal target

**North star:** a reviewer previews a source-to-target integration, inspects
conflicts and one declared gate, and leaves both branches untouched.

**Non-negotiable:** exact source and target commits are recorded; no ref or
primary-checkout mutation occurs during simulation.

## 2. Concept materials

The frame set is now a reviewed local material package. Integrated Gateboard and
negative-state breadth remain future gates.

| Frame      | Visual                                                        | On-screen line                     | Intended evidence moment  |
| ---------- | ------------------------------------------------------------- | ---------------------------------- | ------------------------- |
| 1. Resolve | Source and target commit identities are pinned                | “Simulate the exact inputs”        | Provenance is explicit    |
| 2. Preview | Disposable merge tree shows clean or conflict outcome         | “Mergeability is not verification” | Textual result is bounded |
| 3. Gate    | One repository-declared gate runs against the simulated state | “Check the behavior separately”    | Verification is distinct  |
| 4. Receipt | Refs, checkout, conflict, and gate results are recorded       | “Leave the branches untouched”     | Safety is inspectable     |

**Preview concept.** A two-column source/target view feeds a clean/conflict
simulation, then a separate gate receipt. Keep the no-ref-mutation badge visible.

**Narrative spine.** Resolve commits → simulate merge → show conflict/clean →
run declared gate → prove no mutation.

## 3. Build-gap specification

Reviewed baseline: the current-dev integration MVP records immutable
source/target resolution, clean/conflict merge-tree receipts, a separate local
gate probe, and unchanged source/target refs. Gateboard handoff and
stale/cancellation/retained-workspace breadth remain open.

Project disposition: `targeted_gap_closure` — run clean and conflict simulations
plus one declared gate.

Gap classes: demo_integration — CIS-0; evidence — CIS-1; product — CIS-2;
packaging — CIS-3.

| ID    | Gap to close                              | Observable acceptance condition                                                  | Owner             | Required proof                    |
| ----- | ----------------------------------------- | -------------------------------------------------------------------------------- | ----------------- | --------------------------------- |
| CIS-0 | Prove clean/conflict simulation           | Current-dev clean and conflict source/target cases produce bounded merge-tree receipts       | Integration owner | Current-dev simulation matrix                 |
| CIS-1 | Bind one gate to the simulated state      | A declared gate runs against the exact simulated target without ref mutation     | Evidence owner    | Gate receipt with commit identity |
| CIS-2 | Preserve stale/cancelled/dirty boundaries | Stale inputs, cancellation, and retained dirty state do not become clean results | Safety owner      | Negative scenario matrix          |
| CIS-3 | Package the public case                   | Reviewed 1600×900 PNG/SVG preview, short copy, no-auth integration case, and proof link agree              | Showcase owner    | Material review and hosted/readback      |

**Required build order:** CIS-0 → CIS-1 → CIS-2 → CIS-3. Video is optional
after the evidence gate.
