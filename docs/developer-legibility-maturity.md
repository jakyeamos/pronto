# Developer legibility maturity

Last reviewed: 2026-08-16.

Pronto projects Quality Runner's `developer_legibility` dimension in the
repository route report. The gate answers a practical question: can a developer
new to this repository understand it quickly enough to make a safe bounded
change?

The standard evaluates eight lanes:

1. orientation and quick start;
2. documentation navigation and source traceability;
3. architecture visibility;
4. semantic naming;
5. public contracts, docstrings, and doc comments;
6. rationale and invariant comments;
7. executable examples and verification;
8. ownership and freshness.

The shared maturity scale is `0 unknown`, `1 ad hoc`, `2 defined`, `3 enforced`,
and `4 newcomer verified`. Missing quick-start instructions or weak public
contract coverage caps the result at level 2. Static analysis is capped at
level 3. Level 4 requires commit-bound evidence that a newcomer could orient,
run the project, locate behavior, explain an invariant, and complete a bounded
change.

The policy is intentionally not a comment-density rule. Code states what
happens; names carry domain meaning; comments explain why; docstrings define
the caller contract. Suppressions and debt markers need a reason, issue, or
removal condition. Vague public names are review findings, but semantic naming
still requires developer judgment.

Inspect the route projection with:

```bash
pnpm cli route "$(git rev-parse --show-toplevel)" --json
```

Run the producer's scoped standard directly with:

```bash
qr fleet audit run --repo-path "$(git rev-parse --show-toplevel)" \
  --standard developer-legibility --json
```

`change_surface_hotspots` remains a separate signal. It identifies multi-signal
change amplification; it does not change the developer-legibility score or
automatically prescribe a refactor.

## Telescope architecture-visibility receipt

Telescope v2 contributes a narrow readiness receipt to the existing
`developer_legibility` architecture-visibility lane. The receipt reports the
active workspace fingerprint, categorical `map_readiness`, blocking and
enhancement gap keys, extraction coverage, authored-review state, and freshness.
It is evidence for this one lane; Telescope does not calculate or replace the
holistic developer-legibility maturity level.

Quality Runner consumers must preserve `unavailable`, `measured`,
`needs_information`, `reviewable`, `reviewed`, and `stale` as distinct states.
They must also preserve explicit unknowns and reason-bearing `not_applicable`
decisions. Only a fresh `reviewed` receipt can support reviewed
architecture-visibility evidence. Counts, extracted coverage, or an attractive
measured preview cannot be promoted into that claim.

Unresolved receipt gaps project through Pronto's existing remediation and
attention systems with the `telescope_readiness` domain. They remain supporting
evidence and work items, not a second maturity score or a separate task store.
