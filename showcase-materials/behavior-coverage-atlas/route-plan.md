# Behavior Coverage Atlas route plan

Status: C0 public Showcase admission is complete; the current behavior fixture
and freshness proof are verified while review integration remains gated.

The [canonical target](../ideal-demo-targets.md#behavior-coverage-atlas) owns
the promise and proof gate. This is behavior evidence, not line-coverage
theater.

## 1. Ideal target

**North star:** a developer maps named product behaviors to tests and sees
strong, weak, stale, missing, and unknown evidence without collapsing the
result into line coverage.

**Non-negotiable:** an assertion, an execution, and an absence of evidence are
different states; duplicate or stale links remain visible.

## 2. Concept materials

The current behavior fixture frame is locally verified; review handoff and the
negative matrix remain gated.

| Frame      | Visual                                                    | On-screen line                      | Intended evidence moment           |
| ---------- | --------------------------------------------------------- | ----------------------------------- | ---------------------------------- |
| 1. Declare | Three named behaviors with owners and predicates          | “Name what must work”               | The contract is explicit           |
| 2. Link    | Tests and receipts attach to each behavior                | “Evidence is not just a line count” | Links carry strength and freshness |
| 3. Compare | Strong, weak, stale, missing, and unknown states contrast | “Know what the test proves”         | Gaps are explainable               |
| 4. Review  | One gap opens its originating receipt and next action     | “Move from behavior to evidence”    | Handoff is useful                  |

**Preview concept.** Show a three-row behavior table with evidence-state badges
and one selected row opening its receipt.

**Narrative spine.** Declare → link → classify evidence → navigate to review.

## 3. Build-gap specification

Reviewed baseline: the current `dev` head passes tests, lint, and packaging and
records explicit manifests and strong, weak, stale, missing, and duplicate-link
states. Review Attention Map integration remains open.

Project disposition: `targeted_gap_closure` — verify the fixture and feed one
evidence state into the review overlay.

Gap classes: evidence — BC-0; demo_integration — BC-1; product — BC-2;
packaging — BC-3.

| ID   | Gap to close                           | Observable acceptance condition                                                      | Owner             | Required proof               |
| ---- | -------------------------------------- | ------------------------------------------------------------------------------------ | ----------------- | ---------------------------- |
| BC-0 | Prove the three-behavior fixture       | Assertion, execution, stale, missing, and unknown states reproduce without inference | Evidence owner    | Fixture receipt matrix       |
| BC-1 | Connect review attention               | One behavior signal reaches Review Attention Map with source and freshness           | Integration owner | Cross-tool receipt           |
| BC-2 | Preserve duplicate and stale semantics | Duplicate links and stale executions remain reviewable, not silently collapsed       | Product owner     | Negative fixture matrix      |
| BC-3 | Package the public case                | Binary preview, short copy, no-auth behavior case, and proof link agree              | Showcase owner    | Material review and readback |

**Required build order:** BC-0 → BC-1 → BC-2 → BC-3. Video is optional after
the evidence gate.
