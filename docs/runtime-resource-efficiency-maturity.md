# Runtime resource efficiency maturity

`runtime_resource_efficiency` is a canonical maturity-v2 dimension in the
`user_facing_quality` pillar. It measures shipped outcomes and the strength of
their evidence. It does not award points merely for using route-level lazy
loading, tree shaking, compression, or any other implementation technique.

## Applicable outcomes

Quality Runner should evaluate the subset that applies to the repository's
distributed artifact and runtime:

- initial JavaScript bytes and the initial-to-total JavaScript ratio;
- packaged or installed artifact size and growth from the accepted baseline;
- startup latency and peak steady-state memory for a representative journey;
- duplicate or unexpectedly heavy production dependencies;
- whether optional features remain outside the initial-load artifact;
- whether production artifacts exclude unintended debug payloads and source
  maps.

Every observation must carry its unit, scope, aggregation, time window,
artifact or commit identity, timestamp, and declared budget or accepted
baseline. Measurements with incompatible units or windows stay separate.
Remote observations must remain timestamped and source-labelled rather than
being silently mixed with local evidence.

## Applicability and evidence

The producer emits one of these applicability states:

- `applicable`: the repository ships or runs an artifact whose resource use can
  be measured;
- `not_applicable`: no such artifact exists, with a concrete reason;
- `unknown`: applicability or trustworthy evidence has not been established.

Applicable evidence advances through these levels:

1. `artifact_inspected`: reproducible artifact measurements exist.
2. `runtime_verified`: representative startup and memory behavior is measured
   in addition to the artifact.
3. `regression_guarded`: applicable budgets are enforced by a deterministic
   release or CI gate.

Unavailable, stale, conflicted, or incomparable evidence stays explicit and
does not become a zero that looks measured. `not_applicable` removes the
dimension from the applicable denominator; it is never treated as passing.

## Scoring

The producer supplies the governed 0–4 dimension score:

| Score | Required outcome and evidence                                                                                |
| ----- | ------------------------------------------------------------------------------------------------------------ |
| 0     | A verified material budget breach or demonstrated startup, memory, or storage harm blocks the artifact.      |
| 1     | Artifact evidence exists, but a material outcome exceeds its accepted budget or lacks a defensible baseline. |
| 2     | Applicable artifact outcomes pass declared budgets with `artifact_inspected` evidence.                       |
| 3     | Artifact outcomes pass and representative startup and memory outcomes have `runtime_verified` evidence.      |
| 4     | Outcomes pass and applicable budgets are `regression_guarded`.                                               |

An oversized chunk is therefore a finding, not automatic proof of user harm and
not a critical maturity cap by itself. It becomes a stronger penalty when it
breaches a declared budget, regresses against the accepted baseline, or is tied
to measured startup, memory, or storage harm. Technique-level facts such as
"uses lazy loading" may explain the evidence, but cannot determine the score.

Pronto consumes the score as an open-set `dimension_scores` entry, exposes the
dimension in the user-facing-quality pillar, and reports the capability as
missing when an applicable repository has no current evidence. Quality Runner
remains the authority that measures outcomes and publishes replay-valid feed
evidence.
