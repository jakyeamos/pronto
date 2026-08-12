# Mac Control maturity gate

Pronto imports the canonical Quality Runner maturity feed as its source score,
then consolidates locally audited operational dimensions into the displayed
0–4 fleet score. The Mac Control ideal-state contract contributes two of those
dimensions: `mac_control.implementation_contract` and
`mac_control.live_task_evidence`. A repository may claim the `4.0/4.0` ideal
only when both applicable lanes are fresh and passing. `4/4` is not a
four-repository denominator.

## Canonical evidence

Mac Control publishes the report at:

```text
~/.quality-runner/fleet-audit/current/mac-control-ideal-state.json
```

The report must use schema `pronto-mac-control-ideal-state/v1`, producer
`mac-control`, a stable `run_id`, and an RFC 3339 `observed_at`. It must account
for every repository registered in Pronto. A report produced by
Quality Runner may declare `scope: "quality_runner_fleet"` and include extra
discovered repositories; Pronto evaluates its registered fleet and blocks if
any current repository is omitted.
Pronto does not infer a cohort or silently treat an omitted repository as
passing. A repository with no supported Mac Control task surface may instead use
`applicability: "not_applicable"` with a human-readable
`applicability_reason`; that declaration is still required to be current and
commit-matched.

Each `applicable` repository entry contains:

- `repository_id`, `repository_name`, `observed_at`, and `observed_commit`;
- all eight producer-derived semantic results: `stable_identity`, `correct_semantics`,
  `observable_state`, `useful_hierarchy`, `efficient_navigation`,
  `verifiable_outcomes`, `route_flexibility`, and `stable_change_behavior`;
- `implementation_contract.evidence_level`, per-dimension states, grounding
  errors, and any retained non-scoring legacy declaration count;
- repository-level `evidence` references; and
- one record for every supported task.

Each repository entry carries its `manifest_schema`. The current
`mac-control-task-manifest/v4` contract forbids self-scoring `criteria`
booleans. It separates stable target identity, foreground focus policy,
semantic action, independent verification oracle, and
provider/method/interaction-mode route candidates. Every task declares a
surface kind plus typed, criterion-specific semantic claims with
repository-relative source anchors and evidence tokens. Quality Runner derives
the eight results only after those references resolve to implementation files
and the tokens occur near one unique anchor. Docs, tests, fixtures, snapshots,
symlinks, and path traversal cannot score. Navigation, direct entry point,
verification expectation, readback provider, distinct secondary provider,
fallback policy, selector type, and failure behavior must agree with the task's
other fields. Pronto rejects a producer count that disagrees with the derived
results.

Surface claims must respect provider ownership: native app UI needs a native
semantic candidate, web content needs a browser connector, and a hybrid
transition needs both. Accessibility candidates need a stable identifier;
visual, pointer, and drag candidates need a fresh-state handoff. A task also
declares the observable and change states that apply and gives a reason for
every standard state it exempts. It accounts for shortcut acceleration through a verified
built-in binding, a verified customization surface, or a reasoned exemption.
Shortcut-capable tasks retain stable command identity, contextual availability,
conflict handling, and reversible custom assignment while reusing the normal
independent task oracle. V1 through v3 remain readable during migration, but
their true booleans are exposed only as legacy declarations. They project
`audit_required`, contribute `0/8` implementation points, cannot
satisfy the ideal state, and trigger the generic full-fleet-audit workflow in
`docs/evidence-contract-freshness.md`.

V2 through v4 repositories do not select a route. `selected_route` appears only in live
task evidence after an eligible candidate has been measured for the current
context. Missing runtime selection is a live review requirement, not
an implementation defect. Pronto does not reward universal Accessibility
scrolling, Command-K, a blanket ban on sequential keyboard navigation, or a
visual fallback added solely for scoring.

Human accessibility metadata remains distinct from agent task-operability
fields, and both remain distinct from runtime performance evidence. The current
v1 report envelope carries the static dimensions in one implementation lane,
but Pronto remediation must not use one as proof of another. The repository
supplies enough semantic and observable information for Mac Control to choose
and verify a route without guessing; route ranking and latency belong to Mac
Control and Quality Runner, not to Pronto remediation.

Quality Runner also publishes `source_provenance` for each repository. A v4
entry scores only when the manifest plus every referenced implementation path
are clean and digest-bound to the observed commit. Dirty, missing, or
unverifiable provenance is shown as a specific review reason; Pronto does not
attribute working-tree source to `HEAD` or silently accept a producer pass.

## Evidence lanes

The gate evaluates two distinct evidence lanes:

| Lane                    | What it proves                                                                                                                                         | What it does not prove                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Implementation contract | The v4 task contract is structurally valid and all eight semantic dimensions are grounded in repository source for every supported task.               | It does not prove runtime route selection, task success, speed, or that a packaged app is running. |
| Live task evidence      | Supported tasks have attempts, readable postconditions, successful receipts, and route evidence from the running app or an approved evidence producer. | It does not repair a missing or invalid implementation contract.                                   |

The current live sidecar is `mac-control-task-evidence/v2`. It carries
structured attempts with producer identity, the audited source digest, an
eligible selected route, a digest-bound receipt, and an independent
postcondition readback. Quality Runner derives attempts and successes and sets
`measurement_valid`; Pronto requires that verdict for v4. Caller-supplied
aggregate counts or free-form evidence strings cannot satisfy the live lane.

The report exposes these lanes as implementation_contract.status and
live_task_evidence.status. A v4 contract may be structurally valid but still
require implementation review when a source anchor or typed semantic claim is
missing. A fully source-grounded contract with no task attempts is implementation
passed and live review required. The overall repository and portfolio gate
remains review required until both lanes pass. A task that was attempted and
failed is a live failure; a task that has not yet been attempted is review
required. Neither state is mislabeled as a static contract defect.

The implementation score is the fraction of the eight required semantic
dimensions that Quality Runner source-grounded in a current v4 manifest. A
readable v1, v2, or v3 manifest contributes zero while retaining the
eight-dimension denominator and its declaration count for explanation. Pronto
labels the two values separately as `Semantic source evidence` and `Legacy
declarations · non-scoring`; it never presents a legacy `8/8` as readiness. The live score
is the fraction of supported tasks with a measured, successful route. Blocked
or missing evidence scores zero; evidence outside the fresh window cannot score
above 3.0. Explicit `not_applicable` entries remain audited but are excluded
from both score denominators.

## Gate semantics

| State              | Meaning                                                                                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Passed` + `Fresh` | Both evidence lanes pass and every observed commit matches the current Pronto snapshot. This satisfies the Mac Control ideal-state gate.                      |
| `Passed` + `Stale` | Both evidence lanes pass, but the evidence is outside the seven-day window or does not represent the current report timestamp. It is not an ideal-state pass. |
| `Review required`  | One or more semantic dimensions lack source grounding, or live tasks are unattempted or have incomplete evidence.                                             |
| `Failed`           | The manifest or producer projection is structurally inconsistent, a live task attempt failed, or commit evidence does not match.                              |
| `Blocked`          | The report is missing, malformed, out of scope, incomplete, or cannot be compared to a current repository commit.                                             |
| `Not applicable`   | The report explicitly documents that the repository has no supported Mac Control task surface.                                                                |

Pronto preserves the imported Quality Runner score as `source_maturity_score`
for provenance and publishes the consolidated score as `maturity_score`.
Remediation includes the Mac Control action when a maturity applicable
repository is not `Passed` + `Fresh`, and the action's acceptance criteria
require a fresh commit-matched report. Missing evidence remains a scored zero,
never a synthetic pass.
