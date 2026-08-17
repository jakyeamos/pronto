# Agent Router route plan

Status: AR-1 through AR-4 are closed for the labeled replay/spec slice. AR-7 is
closed as a local visual package. AR-5 is partially observed but blocked at the
native output contract: the planner exposes conflict labels and
`model_synthesis`, but not the confidence or fallback decision the showcase
acceptance requires. Complete provider and worktree execution remain
incomplete.

The [canonical target](../ideal-demo-targets.md#agent-router) owns the durable
promise and proof gate.

## 1. Ideal target

**North star:** a single complex launch-readiness request becomes a typed task
graph, three credible route candidates, an explainable selected route, parallel
bounded results, and one synthesis receipt that shows cost, quality, evidence,
and fallback decisions.

**Non-negotiable:** the demo cannot fabricate provider comparisons or imply
execution surfaces that remain incomplete. A replay fixture is acceptable when
clearly labeled.

## 2. Concept materials

Frames 1–3 now have a reproducible replay/spec checkpoint. Frames 4–6 remain
**concept** until bounded execution, synthesis, and receipt comparisons pass.

| Frame           | Visual                                                                                | On-screen line                                  | Intended evidence moment         |
| --------------- | ------------------------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------- |
| 1. Complex ask  | Launch-readiness request contains research, code, and risk work                       | “One request can contain several kinds of work” | Decomposition is justified       |
| 2. Task graph   | Typed nodes expose dependencies, outputs, and authority                               | “Route the subtasks—not the sentence”           | Structure becomes inspectable    |
| 3. Alternatives | Three routes compare capability, evidence, cost, latency, and risk                    | “A route should beat a real alternative”        | Choice is explainable            |
| 4. Execution    | Bounded subtasks progress with provider and evidence labels                           | “Keep every result attached to its job”         | Parallel work stays attributable |
| 5. Synthesis    | Conflicts and missing evidence are resolved or retained                               | “Combine results without erasing disagreement”  | Synthesis is honest              |
| 6. Receipt      | Selected route, rejected alternatives, outputs, cost, and fallback share one artifact | “Inspect why this answer exists”                | Routing value is proven          |

**Preview concept.** A typed task graph flowing through a scored route decision
into one evidence receipt. Headline: “Route complex AI work with reasons—not
just round-robin prompts.”

**Narrative spine.** Complex request → typed decomposition → credible options →
bounded execution → conflict-aware synthesis → explanation receipt.

## 3. Build-gap specification

Reviewed baseline: task graphs, provider scoring, replay cases, CLI/MCP, receipts,
synthesis, and learning exist; complete provider and worktree execution do not.

Project disposition: `targeted_gap_closure` — preserve the existing routing
system and close the bounded execution, evidence, and public-inspection gaps.

Gap classes: content — AR-1; product — AR-2, AR-3, AR-4, AR-5; evidence —
AR-6; packaging — AR-7.

| ID   | Gap to close                                      | Observable acceptance condition                                                                                        | Owner                 | Required proof                                                                             |
| ---- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------ |
| AR-1 | Define one representative replay case             | The request produces a stable typed graph with meaningful dependencies and authority constraints                       | Product/routing owner | **Closed:** input fixture, expected graph, and native planner probe                        |
| AR-2 | Establish credible route alternatives             | At least two candidates have evidence-backed capability, cost, quality, and risk inputs rather than placeholder scores | Evaluation owner      | **Closed:** native replay candidate packet with relative usage proxy and explicit unknowns |
| AR-3 | Make selection rationale inspectable              | Every material score and exclusion maps to a constraint or evidence source                                             | Routing owner         | **Closed:** native decision receipt and selection trace                                    |
| AR-4 | Complete or explicitly simulate bounded execution | Selected subtasks return attributable results; replay data is labeled and no incomplete provider is presented as live  | Runtime owner         | **Closed:** labeled bounded replay, worker receipts, and provider-state ledger             |
| AR-5 | Prove conflict-aware synthesis                    | A seeded disagreement or missing result remains visible and changes the final confidence or fallback                   | Synthesis owner       | Boundary fixture and output assertion                                                      |
| AR-6 | Reconcile receipt totals                          | Task outputs, costs, timings, provider identities, and final result agree across the receipt                           | Evidence owner        | Automated reconciliation check                                                             |
| AR-7 | Build a public visual explorer                    | **Closed locally 2026-08-14:** a candidate page, claim ledger, crop-safe 16:9 preview, short copy, and no-auth page source let a viewer inspect the graph, route choice, bounded receipt, and AR-5 boundary without running the CLI | Showcase/design owner | `case-study.json`, `claim-ledger.json`, `assets/preview-16x9.png`, and `public/index.html` |

**Build order:** AR-1 → AR-2/AR-3 → AR-4 → AR-5/AR-6 → AR-7.

## 4. AR-1 closure

`replay-case.json` is the reproducibility appendix for `AR-1`. It is clearly
labeled synthetic because no real provider-backed launch request is authorized
for this packet. The fixed intent carries three authority constraints and an
explicit `repo_edit_with_tests`/high-risk classification override.

`evidence/ar-1-contract-receipt.json` records a fresh native probe at the clean
`dev` checkout. With the repository's pinned `pnpm@10.12.4`, the probe produced
the expected three typed subtasks and two dependency edges:

`plan → execute → verify`

The stable comparison intentionally ignores the planner's random graph ID and
checks the fixed root ID, subtask IDs, types, dependencies, edges, and presence
of the authority constraints. Repository tests, typecheck, and lint also passed.

## 5. AR-1 claim boundary

This closes the replay/spec and native graph-probe portion of AR-1; it does not
claim provider execution, approval enforcement, parallel worktree execution,
cost reconciliation, or a public visual explorer. The three negative cases in
the appendix are acceptance targets for a future replay harness, not claims that
the current native engine already rejects those mutations.

## 6. AR-2/AR-3 closure

The [AR-2/AR-3 routing receipt](evidence/ar-2-3-routing-receipt.json) runs the
same synthetic launch-readiness intent through the native `createReplayCatalog`
and `createTaskPlan` surfaces at the clean `dev` checkout. The plan produces
three subtasks whose candidate ordering and Codex winner are stable across
planning, implementation, and verification.

The packet keeps two different kinds of evidence separate. Codex and Cursor
have task-specific observed quality and relative `usagePerTask` records, so
they are credible alternatives for the replay. Antigravity remains an
eligible-but-weakly-evidenced exploration alternative because it has no direct
`repo_edit_with_tests` observation. Claude, Luna, Shell, and Manual are not
silently dropped: each exclusion is recorded with the native quality-floor,
availability, capability, or native-execution reason.

The selection trace therefore explains both the winner and the rejected paths;
it does not turn the relative usage proxy into currency cost or imply that any
provider actually ran.

## 7. AR-2/AR-3 claim boundary and next gap

AR-2 and AR-3 are closed only for the labeled replay/spec candidate packet.
They prove native candidate scoring, alternatives, and rationale traceability,
not live provider execution, worktree writes, conflict-aware synthesis, receipt
reconciliation, or a public visual explorer. The AR-4 execution appendix now
closes the explicitly simulated branch without changing that boundary.

## 8. AR-4 bounded execution replay

The [AR-4 execution replay](execution-replay.json) binds the three selected
subtasks to three synthetic worker receipts. Each receipt carries a subtask ID,
provider identity, result status, artifact name, and deterministic fixture-test
evidence. The replay matrix also passes five existing routing cases, including
the high-risk migration route.

The receipt deliberately records zero provider invocations, zero worktree
mutations, and unknown usage telemetry. `provider_state` is
`replay_profile_only`, so the appendix demonstrates attributable bounded
results without presenting a fixture as a live provider run.

## 9. AR-4 claim boundary and next gap

AR-4 is closed as an explicitly labeled replay execution, not as live runtime
execution. AR-5 still owns conflict-aware synthesis, and AR-6 still owns
cross-receipt cost, timing, provider identity, and result reconciliation. The
native conflict path is probed below, but the queue moves on because its
required output contract is not present.

## 10. AR-5 partial conflict boundary

The [AR-5 blocker receipt](evidence/ar-5-blocker.json) runs the repository's
existing synthetic `fixtures/worker-receipts.json` through the native
`synthesizeWorkerReceipts` path. The probe detects status, file-set, summary,
weak-evidence, and high-risk conflicts and selects `model_synthesis`; both
fixture receipt IDs and the rationale remain attributable.

AR-5 is **not closed**. `SynthesisDecisionSchema` currently exposes no final
confidence field and no fallback disposition. That means the disagreement is
visible, but the material cannot prove that it changes confidence or fallback,
which is the acceptance condition. No provider was invoked and no worktree was
mutated. Park AR-5 until the owner approves the missing contract, then rerun
the seeded disagreement and missing-result probes before AR-6.
