# Contract Watch route plan

Status: C0 public Showcase admission is complete; the current dev checkpoint
and binary preview are verified, while cross-tool contract evidence and review
integration remain gated.

The [canonical target](../ideal-demo-targets.md#contract-watch) owns the
promise and proof gate. Contract Watch compares declared contracts; it does not
invent certainty for undocumented consumers.

## 1. Ideal target

**North star:** a developer compares two explicit OpenAPI contracts, sees
certainty and policy consequence separately, and records a human disposition
for each change.

**Non-negotiable:** semantic certainty, compatibility policy, and unknown
consumer scope remain separate fields. An undocumented consumer is an explicit
unknown, not a clean result.

## 2. Concept materials

All frames below are **concept frames** until the real change set and source-
linked handoff pass.

| Frame          | Visual                                                        | On-screen line                  | Intended evidence moment   |
| -------------- | ------------------------------------------------------------- | ------------------------------- | -------------------------- |
| 1. Compare     | Baseline and candidate operations align                       | “See the contract change”       | Scope is explicit          |
| 2. Classify    | Certainty and policy consequence appear in separate columns   | “Facts are not decisions”       | Uncertainty is visible     |
| 3. Handoff     | Affected consumers, unknowns, and review attention are linked | “Carry the consequence forward” | Cross-tool value appears   |
| 4. Disposition | Human chooses acknowledge, mitigate, or verify                | “The owner decides”             | Policy remains human-owned |

**Preview concept.** Use a semantic diff with one changed operation, one known
consumer, one undocumented boundary, and a visible disposition.

**Narrative spine.** Compare → classify certainty → expose unknowns → connect
review/deletion → record disposition.

## 3. Build-gap specification

Reviewed baseline: the W3 MVP performs local OpenAPI comparison with
certainty/policy separation and human disposition. The current dev checkpoint
reruns the four-change comparison and disposition flow. Review Attention Map
and Deletion Proof handoffs remain open.

Project disposition: `targeted_gap_closure` — feed one contract change into the
review tools without copying authority.

Gap classes: evidence — CW-0; demo_integration — CW-1; product — CW-2;
packaging — CW-3.

| ID   | Gap to close                         | Observable acceptance condition                                                                | Owner                | Required proof                    |
| ---- | ------------------------------------ | ---------------------------------------------------------------------------------------------- | -------------------- | --------------------------------- |
| CW-0 | Prove semantic handoff               | One OpenAPI change reaches Review Attention Map and Deletion Proof with source identity intact | Integration owner    | Cross-tool receipt                |
| CW-1 | Preserve certainty/policy separation | A semantic change cannot become a merge or release verdict without a human disposition         | Product/policy owner | Negative assertion check          |
| CW-2 | Expose undocumented consumers        | Known and unknown consumer classes remain distinct through the handoff                         | Evidence owner       | Fixture and stale baseline matrix |
| CW-3 | Package the public case              | Binary preview, short copy, no-auth contract case, and proof link agree                        | Showcase owner       | Material review and readback      |

**Required build order:** CW-0 → CW-1 → CW-2 → CW-3. Video is optional after
the evidence gate.
