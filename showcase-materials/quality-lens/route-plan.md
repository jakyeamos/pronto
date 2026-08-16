# Quality Lens route plan

Status: C0 public Showcase admission is complete; the current `dev` checkout's
headless finding workflow and binary preview are verified, while direct IDE
proof and hosted packaging remain gated.

The [canonical target](../ideal-demo-targets.md#quality-lens) owns the promise,
audience, ownership split, and proof gate. This plan makes the ideal story
specific enough to build without turning a pilot projection into product proof.

## 1. Ideal target

**North star:** a developer opens one Quality Runner finding in the IDE, sees
why it exists, records a human disposition, reruns it, and hands the same
target-bound evidence to remediation without an opaque score.

**Non-negotiable:** the IDE surface must preserve finding identity, source
location, freshness, stale/failed state, and human ownership. A headless receipt
or source-level projection is not a direct Problems-panel acceptance.

## 2. Concept materials

All frames below are **concept frames** until the canonical product path and
task-owned postconditions pass.

| Frame          | Visual                                                       | On-screen line                   | Intended evidence moment                           |
| -------------- | ------------------------------------------------------------ | -------------------------------- | -------------------------------------------------- |
| 1. Finding     | One changed line with a named finding in Problems            | “Attention with a reason”        | The developer sees a concrete problem, not a score |
| 2. Explanation | Source, rule, scope, and originating receipt side by side    | “Every finding keeps its source” | Provenance is inspectable                          |
| 3. Disposition | Human chooses fix, accept, or defer with stale state visible | “The developer decides”          | AI/runtime output is not a merge verdict           |
| 4. Handoff     | Rerun result and remediation reference appear in one receipt | “Carry the evidence forward”     | The next tool receives the same target-bound facts |

**Preview concept.** A crop-safe IDE frame shows one finding selected, its
source and freshness visible, and a small disposition-to-rerun trail. Keep the
finding title and stale badge legible at thumbnail size.

**Narrative spine.** Finding → explanation → human disposition → rerun →
remediation handoff.

## 3. Build-gap specification

Reviewed baseline: the current clean `dev` checkout contains the W1 pilot with
finding normalization, dispositions, rerun reconciliation, a CLI, and a VS
Code projection; five tests, lint, and the package contract pass on
`6a2318ac4f17eea65307d8492375e6016203fd95`. A crop-safe 1600 × 900 PNG is
also visually reviewed. The direct Problems-panel postcondition and hosted
public product path are not yet proven.

Project disposition: `material_build_or_restoration` — restore the smallest
product workflow onto the canonical path before investing in polished release
packaging.

Gap classes: product — QL-0; demo_integration — QL-1; evidence — QL-2;
packaging — QL-3.

| ID   | Gap to close                                  | Observable acceptance condition                                                              | Owner                 | Required proof                                        |
| ---- | --------------------------------------------- | -------------------------------------------------------------------------------------------- | --------------------- | ----------------------------------------------------- |
| QL-0 | Restore the smallest finding workflow         | A user can inspect one finding, its source, freshness, and disposition in the canonical path | Product owner         | Current `dev` implementation, headless checkpoint, and focused tests (verified); IDE readback remains QL-1 |
| QL-1 | Bind the IDE surface to the headless contract | The selected Problems item and receipt share target, finding, and source identity            | IDE/integration owner | Direct Problems-panel readback and structured receipt |
| QL-2 | Prove stale and failed states                 | A stale or failed rerun remains visible and cannot be presented as a pass                    | Verification owner    | Positive, stale, and failure fixtures                 |
| QL-3 | Package the public case                       | Crop-safe preview, short description, no-auth case page, and linked proof agree              | Showcase owner        | Binary preview verified locally; hosted URL/readback receipt remains open |

**Required build order:** QL-0 → QL-1 → QL-2 → QL-3. Video is optional after
the evidence gate.
