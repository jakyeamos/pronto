# Evidence Replay route plan

Status: C0 public Showcase admission is complete; the current dev reader and
stale/rerun boundary are verified, while receipt coverage and release proof
remain gated.

The [canonical target](../ideal-demo-targets.md#evidence-replay) owns the
promise and proof gate. Inspection stays safe; execution is always a separate
human action.

## 1. Ideal target

**North star:** a developer opens a local receipt, checks target freshness,
previews a rerun, and preserves unknown or stale evidence without silently
executing it.

**Non-negotiable:** the reader never upgrades stale or unknown evidence to a
pass, and opening a receipt never runs the underlying command.

## 2. Concept materials

All frames below are **concept frames** until the producer matrix passes.

| Frame            | Visual                                                       | On-screen line                 | Intended evidence moment       |
| ---------------- | ------------------------------------------------------------ | ------------------------------ | ------------------------------ |
| 1. Open          | Receipt identity, source, and target are shown               | “Inspect before you act”       | Opening is non-mutating        |
| 2. Freshness     | Current, stale, unknown, and cancelled states are contrasted | “Evidence has a time boundary” | Uncertainty stays visible      |
| 3. Rerun preview | A human sees command, scope, and authority before execution  | “Rerun is explicit”            | No hidden execution            |
| 4. Handoff       | Result links back to producer and next action                | “Keep the trail intact”        | Continuity survives the reader |

**Preview concept.** Use a receipt viewer with a freshness badge, omitted fields,
and a disabled-until-authorized rerun action.

**Narrative spine.** Open → classify freshness → preview rerun → preserve
omissions → hand off.

## 3. Build-gap specification

Reviewed baseline: the W2 reader MVP inspects Debug Trail receipts, compares
target freshness, and previews explicit reruns without executing. The current
dev checkpoint at `780094c` passes tests/lint/package and reproduces the stale
reader flow. Rule Lab coverage and the full stale/unknown matrix remain open.

Project disposition: `targeted_gap_closure` — close the producer matrix and
preserve the inspect-only boundary.

Gap classes: demo_integration — ER-0; evidence — ER-1; product — ER-2;
packaging — ER-3.

| ID   | Gap to close                       | Observable acceptance condition                                                                     | Owner             | Required proof               |
| ---- | ---------------------------------- | --------------------------------------------------------------------------------------------------- | ----------------- | ---------------------------- |
| ER-0 | Close the receipt freshness matrix | Rule Lab and Debug Trail receipts show current, stale, unknown, cancelled, and rerun-preview states | Integration owner | Fixture matrix and receipts  |
| ER-1 | Preserve inspect-only behavior     | Opening or comparing a receipt makes no external or repository mutation                             | Safety owner      | Negative execution probe     |
| ER-2 | Bind producer and target identity  | Source revision, producer, target, and omissions survive replay                                     | Evidence owner    | Schema/parity checks         |
| ER-3 | Package the public case            | Preview, short copy, no-auth reader case, and proof link agree                                      | Showcase owner    | Material review and readback |

**Required build order:** ER-0 → ER-1 → ER-2 → ER-3. Video is optional after
the evidence gate.
