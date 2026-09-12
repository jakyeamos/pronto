# Branch-sensitive quality verification

Read this packet whenever a UI control selects a branch or commit and the same
surface displays quality, tenure, remediation, release, or other branch-sensitive
statistics.

## Semantic contract

- Name the three states separately: the live workspace branch, the configured
  target/comparison branch, and the branch plus commit that produced the
  displayed evidence.
- A persisted target-branch round trip proves only that the setting was saved
  and reloaded. It does not prove that displayed statistics were recomputed for
  that target.
- Treat evidence as target-specific only when the evidence branch matches the
  selected target and the evidence commit exactly matches the target branch
  head. A matching branch name without comparable commits is not confirmation.
- Never render a zero or empty result as a target-branch result when the source
  branch, source commit, or target commit is missing or mismatched. Show the
  raw scan scope and an explicit `not verified`, `stale`, or `unavailable`
  state instead.
- Keep detector findings, maturity observations, gate evidence, and aggregate
  audit rows as distinct evidence types. A newer aggregate feed must not replace
  a stable detector report merely because its timestamp is newer.
- A dirty workspace must not be silently checked out, reset, or scanned on a
  different branch to satisfy this contract. If a targeted scan is unavailable,
  preserve the existing workspace and show that the target has not been
  verified.

## Required verification ladder

For every branch-sensitive change, verify all three paths:

1. Positive: select a target whose evidence branch and commit match; confirm the
   UI says the evidence is verified for that target.
2. Negative: select a different target or provide evidence from another commit;
   confirm the UI exposes the mismatch and does not label the number as the
   target's statistic.
3. Ambiguous: remove branch or commit provenance; confirm the UI says the target
   is unverified rather than inferring a match.

Then prove the persistence path, source/typecheck/focused tests, production
build, installed-app identity, and a semantic UI readback. Report configured,
persisted, loaded, evidence-matching, and visibly exercised as separate states.

## Numeric score availability

`QualityScoreGuards.ts` consumes the Rust importer's freshness/status decision;
it does not recompute an audit clock. Fleet maturity requires an imported audit
identity and timestamp, a matched population, a numeric score, and `Ready` or
`Ready with blockers`. Repository maturity requires its own fresh audit identity
and observation timestamp as well as the caller's exact target check. Cached
numbers are withheld from maturity, pillar, cache, and agent-usability surfaces
when those prerequisites are absent; source identifiers and explicit diagnostic
states remain available.

Mac Control numeric display additionally requires a fresh, explicitly assessed lane
(`Passed`, `Failed`, or `Review required`) and an evidence denominator. Missing or
unrecognized statuses remain unavailable. Live scores require at least one measured task. A
measured failed zero remains a valid numeric observation; an unmeasured `0/N`
report is unavailable. These display guards do not turn failing observations
into passing behavior, refresh source evidence, or establish installed-app parity.
The guard boundary tests and rendered fixture suites are local evidence;
independent live surface verification remains a separate step in the ladder above.
