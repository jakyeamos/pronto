# Cache-design maturity gate

Pronto reads cache lifecycle evidence from Quality Runner's canonical,
replay-validated `quality-runner-maturity-feed/v2`. The repository projection
is `quality-runner-cache-design-assessment-v1`; a standard-only audit is a
private diagnostic and is not a publishable feed.

The assessment measures lifecycle quality rather than penalizing repository
size. It keeps logical and allocated bytes, file and shared-file counts,
category totals, policy risks, and two-snapshot growth evidence. Published
data is category-level and repository-scoped: absolute local paths are never
part of the Pronto contract.

Pronto preserves the producer state instead of treating missing evidence as a
pass. `maintained`, `validated`, and `discoverable` retain their score;
`unknown`, `blocked`, `failed`, `missing`, and `absent` remain explicit review
states; `not_applicable` means QR found no derived-storage surface. Feed
freshness can independently mark otherwise maintained evidence stale.

`cache_design` contributes to the conditional `cache_lifecycle` capability in
the governance and sustainability pillar during the pilot. It may influence
maturity, but does not cap maturity and does not block release. Remediation is
advisory: separate durable state, define invalidation, add bounds and pruning,
remove avoidable duplication, then collect two bounded snapshots and cold/warm
functional-equivalence evidence. Cleanup remains a separate authorized action.
