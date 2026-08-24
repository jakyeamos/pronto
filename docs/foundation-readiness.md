# Foundation readiness maturity gate

Pronto projects one repository-level modernization maturity gate at
`quality.foundation_readiness`. Its schema is
`pronto-foundation-readiness/v1`; the user-facing label is **Modernization
readiness**.

The gate answers a narrower question than feature design: does current durable
repository evidence support building on the existing foundation, or should the
next feature carry or follow foundation work? The actual feature-specific
extend-versus-modernize decision stays in the agent chat. Pronto does not add a
review queue, inbox, or approval screen for it.

## Dispositions

- `ready_to_extend`: current evidence coverage is sufficient and no unresolved
  foundation signal was found.
- `modernize_alongside`: the repository can be extended, but the feature should
  include a bounded modernization or evidence-closure slice.
- `modernize_first`: current evidence contains a critical or severely degraded
  foundation signal that should be stabilized before ordinary feature work.
- `review_required`: evidence exists but is stale, conflicted,
  refresh-required, or below the minimum coverage needed for a recommendation.
- `unknown`: no trustworthy repository foundation measurement is available.
- `not_applicable`: the imported maturity contract explicitly has no applicable
  repository pillars.

## Evidence and derivation

Pronto derives the gate after building the risk-weighted repository maturity
model. The derivation uses:

- maturity score, freshness, evidence coverage, fresh-evidence coverage, pillar
  states, missing maintainability capabilities, and critical cap reasons;
- actionable structural findings after repository-owned dispositions;
- applicable behavior-assurance gaps; and
- applicable installed-runtime drift.

Current correctness, security, or operability critical caps produce
`modernize_first`. A severely degraded maintainability/change-safety pillar can
also produce `modernize_first`; ordinary structural, behavior-assurance,
runtime, or maintainability gaps produce `modernize_alongside`. Coverage below
0.60 produces `review_required`, as does stale or refresh-required evidence.
Missing measurement produces `unknown`.

Pronto intentionally does not derive modernization readiness from file age,
file size, dependency age alone, or the existence of legacy-named code. Those
are possible investigation inputs, not proof that a foundation should be
replaced. Raw structural findings likewise do not bypass reviewed dispositions:
only actionable counts contribute to the gate.

## Agent consumption boundary

Agents read the disposition, confidence, freshness, reasons, unknowns, and next
step before proposing substantial additive work. They then make the
task-specific recommendation in chat, comparing:

1. extending the current foundation;
2. coupling the feature to a bounded modernization slice; and
3. stabilizing or replacing a foundation component first.

The projection is always advisory. `execution_authority` is false, so it never
authorizes a rewrite or other scope expansion. `blocks_urgent_fixes` is false,
so a containment, correctness, security, or recovery fix may proceed while the
agent explains how to avoid deepening the debt. Any larger modernization still
requires the normal user authorization and repository ownership checks.

## Projection surfaces

The existing `quality`, `repo`, and `route` JSON outputs carry the full typed
gate because they already project `QualitySnapshot`. No new command, human
review surface, stored decision record, remediation item, or renderer surface
is created.
