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
