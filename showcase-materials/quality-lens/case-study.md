# Quality Lens: one finding without an opaque score

Quality Lens turns a Quality Runner report into a human-complete inspection and
action path. The smallest useful unit is not a score. It is one finding that a
developer can locate, understand, disposition, and revisit.

## The local W1 case

The fixture report contains one warning, `QL-FIX-001`, at
`fixtures/example.js:4`. The normalized model preserves the rule identity,
source location, explanation, recommended fix, and the requirement for human
confirmation.

The report is deliberately older than the inspected checkout. Quality Lens
keeps that fact visible as **stale** instead of presenting the finding as a
fresh pass. A human then records `accepted` with the note that this is a
fixture-only review and no source change is claimed. Rerun reconciliation
carries that disposition into the next model for the same finding.

That is the product behavior worth showing: evidence stays attached to its
source, freshness is not hidden, and a human decision is explicit.

## Evidence boundary

The local packet proves the W1 normalizer, stale-state handling, disposition,
and rerun reconciliation. The current clean `dev` checkout passes the five
tests, lint, and package contract on revision
`6a2318ac4f17eea65307d8492375e6016203fd95`. The packet also includes a
visually reviewed 1600 × 900 binary preview.

Direct VS Code Problems-panel interaction, a hosted no-auth case page, and
external publication are still open. This packet does not claim any of those
gates are complete.
