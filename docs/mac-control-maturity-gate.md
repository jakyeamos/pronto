# Mac Control maturity gate

Pronto's existing maturity score remains a 0–4 score owned by the canonical
Quality Runner maturity feed. The Mac Control ideal-state contract is an
additional maturity gate: a repository may claim the `4.0/4.0` ideal only when
its applicable Mac Control evidence is fresh and passing. `4/4` is not a
four-repository denominator.

## Canonical evidence

Mac Control publishes the report at:

```text
~/.quality-runner/fleet-audit/current/mac-control-ideal-state.json
```

The report must use schema `pronto-mac-control-ideal-state/v1`, producer
`mac-control`, a stable `run_id`, and an RFC 3339 `observed_at`. It must account
for every repository in Pronto's current maturity scope. A report produced by
Quality Runner may declare `scope: "quality_runner_fleet"` and include extra
discovered repositories; Pronto still evaluates only its current
maturity-applicable subset and blocks if any current repository is omitted.
Pronto does not infer a cohort or silently treat an omitted repository as
passing. A repository with no supported Mac Control task surface may instead use
`applicability: "not_applicable"` with a human-readable
`applicability_reason`; that declaration is still required to be current and
commit-matched.

Each `applicable` repository entry contains:

- `repository_id`, `repository_name`, `observed_at`, and `observed_commit`;
- all eight boolean criteria: `stable_identity`, `correct_semantics`,
  `observable_state`, `useful_hierarchy`, `efficient_navigation`,
  `verifiable_outcomes`, `route_flexibility`, and `stable_change_behavior`;
- repository-level `evidence` references; and
- one record for every supported task.

Every supported task must expose a stable target ID, meaningful hierarchy,
direct semantic action, readable observable postcondition, all of
`enabled`, `focused`, `selected`, `expanded`, `visible`, `loading`, and
`completed` states, an efficient navigation strategy, explicit `loading`,
`modal`, `disabled`, and `permission_unavailable` change states, and an
eligible measured route. Eligible routes are `native_api`, `adapter`,
`accessibility`, `keyboard`, `scrolling`, and explicitly approved
`visual_fallback_approved`. The selected route must be eligible, and every
measurement must report successful attempts plus evidence references.

This contract does not require every element to be a tab stop or force every
task through Accessibility. It requires enough semantic and observable
information for Mac Control to choose the fastest eligible route without
guessing and to verify the result independently.

## Evidence lanes

The gate evaluates two distinct evidence lanes:

| Lane                    | What it proves                                                                                                                                                                                                               | What it does not prove                                                           |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Implementation contract | The repository manifest, eight criteria, task shape, semantic routes, state declarations, and static evidence references are present and structurally valid. Source and test references may support this lane when supplied. | It does not prove that the packaged app is running or that a real task succeeds. |
| Live task evidence      | Supported tasks have attempts, readable postconditions, successful receipts, and route evidence from the running app or an approved evidence producer.                                                                       | It does not repair a missing or invalid implementation contract.                 |

The report exposes these lanes as implementation_contract.status and
live_task_evidence.status. A valid static contract with no task attempts is
implementation passed and live review required. The overall repository and
portfolio gate remains review required until both lanes pass. A task that was
attempted and failed is a live failure; a task that has not yet been attempted
is review required. Neither state is mislabeled as a static contract defect.

## Gate semantics

| State              | Meaning                                                                                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Passed` + `Fresh` | Both evidence lanes pass and every observed commit matches the current Pronto snapshot. This satisfies the Mac Control ideal-state gate.                      |
| `Passed` + `Stale` | Both evidence lanes pass, but the evidence is outside the seven-day window or does not represent the current report timestamp. It is not an ideal-state pass. |
| `Review required`  | The implementation contract passes, but one or more live tasks are unattempted or have incomplete live evidence.                                              |
| `Failed`           | A criterion, implementation task contract, live task attempt, or commit check failed.                                                                         |
| `Blocked`          | The report is missing, malformed, out of scope, incomplete, or cannot be compared to a current repository commit.                                             |
| `Not applicable`   | The report explicitly documents that the repository has no supported Mac Control task surface.                                                                |

Pronto keeps the imported Quality Runner score and Mac Control gate in separate
projections. Remediation includes the Mac Control action when a maturity
applicable repository is not `Passed` + `Fresh`, and the action's acceptance
criteria require a fresh commit-matched report. No synthetic Mac Control pass
or score is created when the report is absent.
