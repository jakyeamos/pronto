# Agent usability maturity and growth health

Last reviewed: 2026-08-09.

Pronto consumes a Quality Runner projection that keeps agent usability
diagnostically distinct while rolling applicable evidence into the single
human-and-agent-usability pillar. The projection has four
independent evidence lanes:

1. **Documentation contract** — every declared agent-facing tool has an existing,
   routed usage document.
2. **Tool-to-skill coverage** — every declared tool maps to a known hosted or
   projected skill.
3. **Behavior evidence** — repository-owned evidence exists for the mapped tool
   workflow, with 4/4 reserved for a fresh passing receipt.
4. **Freshness and portability** — the relationship manifest is current and uses
   repository-relative references.

Each applicable lane contributes under a stable `agent_usability.*` dimension
ID. The dedicated projection still exposes the underlying relationship and
behavior state. Those dimensions are averaged within the pillar before its 10%
repository weight is applied, preventing a large agent manifest from
outweighing security, correctness, or operability. A 4/4 repository therefore
requires maintained evidence in every applicable pillar and no unresolved
applicability or critical blocker.

## Repository contract

Repositories declare the relationship in `.agents/agent-usability.json` using
the `agent-usability/v1` schema. Each tool lists its documentation, associated
skill IDs, and behavior evidence. Each skill declares its family and whether it
is hosted by the repository or projected from a canonical external source.

Missing manifests remain visible as `untracked`; Quality Runner does not infer
coverage from similarly named files. A projected skill is coverage evidence,
not proof that the skill behaves correctly in every provider.

Every repository in the registered fleet is expected to own this decision. A
repository with no agent-facing tool or hosted skill surface declares
`applicability: not_applicable`, supplies a concrete reason, and leaves `tools`
and `skills` empty. This prevents ordinary product repositories from receiving
artificially low agent-usability scores while keeping the applicability review
explicit and refreshable as the repository evolves.

The fleet bootstrap is only a baseline. Empty tool-to-skill mappings and empty
behavior evidence remain visible gaps; the contract does not turn existing
documentation or a test filename into behavior proof.

## Growth health

Growth health reports:

- total and agent-facing document counts, routed and unrouted agent documents,
  oversized documents, and bounded-inventory truncation;
- skill count, family count, largest family size, unclassified skills, and
  oversized skill contracts;
- tool count and documentation, skill, declared-behavior, and verified-behavior
  coverage.

Its blocked, attention, and healthy states score 0, 2, and 4 respectively under
`agent_usability.growth_health` and contribute to repository and fleet maturity.
Adding more prose or more skills cannot improve the score by itself. Unrouted documents,
unclassified skills, oversized required surfaces, unresolved references, or a
bounded inventory limit create explicit pressure instead.

## Evidence interpretation

Static validation can establish documentation, mapping, and portability at
3/4. A behavior lane reaches 4/4 only when every declared tool has a fresh,
passing repository-owned receipt. Source files and tests may be declared as
candidate evidence, but they do not become execution proof without a passing
status and observation date.

Explicit `not_applicable` lanes and growth health carry no score and remain
outside the human-and-agent-usability calculation. When the whole repository
declares agent usability `not_applicable`, the pillar is excluded from the
repository denominator. With no declaration and no agent evidence, the pillar
is `unknown`, not zero and not implicitly inapplicable. Pronto renders the
holistic score, pillar status, evidence coverage, four lane scores, and
growth-health counts. It does not infer coverage or turn missing evidence into
zero or a passing state.
