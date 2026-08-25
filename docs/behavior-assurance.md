# Behavior assurance, edge durability, and systemic resilience

Pronto presents two related but separate claims:

- Release assurance asks whether every required Tier-0 behavior has current,
  trusted evidence for the selected target.
- Edge durability asks how much of the full Tier 0-2 scenario inventory is
  profiled and verified under careless, hostile, and unexpected use.
- Systemic resilience asks whether scenarios preserve plural failure
  trajectories, defenses, degraded modes, near misses, operator adaptations,
  and change risks without reducing them to a single root cause.

Completing more inventory never silently widens the release gate.

Each repository projection also carries one explicit onboarding state: missing
contract, legacy v1, unprofiled, partially verified, stale, failed, blocked,
unknown, current, or not_applicable. The state is a routing summary; the
separate applicability, contract, profile, result, freshness, release, and
coverage fields remain authoritative for their individual claims.

## Owned artifacts

The repository-owned contract is .pronto/behavior-assurance.json. Quality
Runner owns immutable receipts under
.quality-runner/behavior-assurance/receipts. Pronto imports the replay-verified
fleet projection; it does not execute receipt contents or synthesize passing
evidence.

Quality Runner accepts pronto-behavior-assurance/v1 as compatible legacy input.
It remains valid for Tier-0 release assessment but is visibly edge-unprofiled.
Version 2 requires every behavior to declare non-empty invariants. A scenario
may declare:

    "edge_profile": {
      "categories": ["state_and_ordering"],
      "risk": "routine",
      "side_effects": "reversible"
    }

The canonical categories are input_and_encoding, state_and_ordering,
repetition_and_idempotency, timing_and_concurrency,
interruption_and_recovery, resource_pressure, authorization_and_session, and
environment_and_cross_surface.

Risk is routine or hostile. Side effects are none, reversible, or destructive.
A missing profile remains an auditable gap instead of making the contract
invalid, allowing deliberate incremental adoption.

A v2 scenario may also declare a systemic-resilience profile:

    "resilience_profile": {
      "failure_trajectories": ["A stale feed and target mismatch can combine."],
      "defenses": ["Typed fail-closed fields preserve uncertainty."],
      "degraded_modes": ["Cached evidence remains visibly stale."],
      "near_misses": ["A missing field defaulted to unknown instead of current."],
      "operator_adaptations": ["Operators refresh evidence before acting."],
      "change_risks": ["Combining profile coverage with release readiness would widen the gate."]
    }

When present, all six dimensions must contain at least one observation. Quality
Runner projects only dimension names and bounded item counts; the descriptive
contract text stays repository-owned. A missing profile is visible as missing,
unprofiled, or partially profiled, but never changes Tier-0 release readiness,
score, or receipt trust. The adjacent scenario receipt status distinguishes a
documented resilience model from verified behavior.

## Projection semantics

Tier-0 required and passed counts, gaps, and release_ready retain their original
meaning. The separate coverage object evaluates every declared Tier 0-2
scenario and reports total, profiled, verified, stale, failed, blocked, and
unknown counts, with per-tier, per-category, per-resilience-dimension, and
bounded per-scenario records.

Contract, profile, result, freshness, verification level, and automation state
remain independent. Target mismatch removes release readiness and converts
previously current edge verification into stale coverage. Security-sensitive
traces expose only sanitized reproduction information and hashes through
repository receipts.

## Recording evidence

Source and automated evidence is produced through the bounded Quality Runner
command:

    qr behavior verify --behavior-id BEHAVIOR_ID \
      --scenario-id SCENARIO_ID \
      --timeout-seconds 120 \
      /path/to/repository \
      -- COMMAND ARG...

Direct edge evidence uses:

    qr behavior record-edge \
      --repo /path/to/repository \
      --behavior BEHAVIOR_ID \
      --scenario SCENARIO_ID \
      --environment local \
      --surface cli \
      --status passed \
      --trace /tmp/edge-trace.json \
      --json

record-edge only accepts local, test, preview, or staging. It refuses production,
destructive profiles, independent claims, uncommitted contracts, dirty trigger
paths, unbounded traces, and failed traces without the required replay and
minimization evidence. Quality Runner writes sensitive raw traces only to its
private local evidence store.

## Audit surfaces

The CLI and installed app show release and edge assurance separately:

    pnpm --silent run cli behavior --json
    pnpm --silent run cli behavior --filter missing --json
    pnpm --silent run cli behavior --filter legacy --json
    pnpm --silent run cli behavior --filter unprofiled --json
    pnpm --silent run cli behavior --filter resilience_unprofiled --json
    pnpm --silent run cli behavior --filter partially_verified --json
    pnpm --silent run cli behavior --filter stale --json
    pnpm --silent run cli behavior --filter failed --json
    pnpm --silent run cli behavior --filter blocked --json
    pnpm --silent run cli behavior --filter unknown --json
    pnpm --silent run cli behavior --filter current --json
    pnpm --silent run cli behavior --filter not_applicable --json

The CLI JSON and text output expose resilience profile status and bounded
dimension counts beside release and edge evidence. The CLI exits
non-zero when selected repositories contain Tier-0 release gaps. Filters select
repositories; they never mutate or reinterpret evidence.

A current assessment requires the ordinary Quality Runner fleet sequence:

    qr fleet audit run --all --projects-root /path/to/projects --json
    qr fleet audit replay --audit-id AUDIT_ID --json
    qr fleet audit feed --audit-id AUDIT_ID --json
    pnpm --silent run cli quality refresh --json

The first v2 fleet audit is expected to show missing, legacy, unprofiled,
partially verified, stale, failed, blocked, or current repositories. Do not
bulk-edit contracts or synthesize receipts to improve the baseline.
