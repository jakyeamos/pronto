# Quality Runner × Tenure: values-driven burndown

The headline case is Tenure’s July 17–28 Quality Runner reconciliation. It
starts with **4,022 raw code-quality findings** and ends with **537 raw rows but
0 open actionable findings**. That distinction is the product story: Quality
Runner does not win by hiding detector output. It wins by making a repository’s
definition of quality inspectable and every resulting decision defensible.

## How your values enter execution

The same standards that guide agents can also inspect what those agents
produce. A Skill, guide, or agent rule first describes what good output and
execution look like. Reviewed ingest makes the implied negative concrete—such
as generic copy, missing interface states, or unsafe cross-layer imports—and
expresses it as either a low-noise deterministic rule or a bounded agent-review
rubric.

Quality Runner then turns each observed violation into a finding that keeps its
skill, rule, file, line, evidence, severity, scope, and coverage. That gives the
owner a debt ledger organized around their values, plus bounded remediation
slices that can be verified against the same standard.

This is reviewed compilation, not arbitrary execution. Quality Runner does not
run Skill code or silently invent rules during a scan: an ingest agent proposes
the pack, a human reviews it, and Quality Runner validates and records the exact
non-executable definition before it produces findings.

## What drove the 4,022 findings

The July 17 run did not apply one generic standard. In `relevant` selection
mode, Quality Runner inferred repository signals and selected eight of twelve
eligible packs:

- copy specificity
- data integrity
- developer experience
- PR risk
- release readiness
- test strategy
- UI foundations
- UI specificity

Those packs produced 50 coverage entries. The scan recorded, among other
categories, 1,394 hardening findings, 931 UI-structural findings, 768
simplification findings, and 339 integration findings. The active Quality
Skills themselves contributed 192 UI-foundations, 179 UI-specificity, one
release-readiness, and one developer-experience finding.

Owners are not locked to the automatic selection. They can change selection
mode, active packs, local skills, rule and path scope, thresholds, and scan
exclusions. The same engine can therefore represent different codebase values
without losing provenance or coverage.

## How the burndown worked

Quality Runner classified the baseline as broad debt that needed a roadmap, not
a one-shot cleanup. Work proceeded in bounded slices with changed-surface scans.
A full scan re-established the baseline after roughly 100 net findings and on
any material QR, corpus, pack, policy, configuration, or scope change.

Selected historical checkpoints:

| Checkpoint                |   Raw code-quality findings |
| ------------------------- | --------------------------: |
| July 17 baseline          |                       4,022 |
| Phase 111                 |                       2,422 |
| Phase 118                 |                       2,265 |
| July 27 scheduled refresh |                       1,060 |
| Refresh 11                |                         694 |
| Refresh 22                |                         566 |
| Refresh 32                |                         538 |
| Refresh 35                | 537 raw / 0 open actionable |

Rows closed through either a source change with focused proof or an exact,
source-evidenced disposition. At the terminal checkpoint, source review retained
raw rows representing intentional test-harness control flow, file-local fixture
helpers, UI primitive wrappers, and documented environment accessors. The
correct claim is **zero open actionable findings**, never “zero findings.”

## Evidence boundary

The baseline was recorded from a dirty `dev` checkout at
`0702be6ce94d845dc281a19c881f9a742846a33d`, so it is historical evidence, not a
clean-release receipt. The terminal tracked record is commit `8b395b2d`; the
burndown archive commit is `f4d9d1d1`. The final run directory is not retained in
the current checkout, and QR gate execution remained blocked because execution
consent was not supplied.

The later August branch reconciliation remains a supporting vignette: a
684-candidate comparison rehearsed a 127-file shortcut, produced 15 integration
or type errors, and rejected the unsafe route before merge. It proves why the
review layer matters, but it is no longer the headline case.

The synthetic `partial-js` fixture remains a small reproducibility appendix.
