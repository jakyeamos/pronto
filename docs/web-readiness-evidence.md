# Web-readiness evidence in Pronto

Pronto consumes the stable Quality Runner report at
`.quality-runner/web-readiness.json`. The producer contract is
`quality-runner-web-readiness/v1`; Pronto does not infer web applicability or
run a browser while importing it.

The projection preserves:

- categorical state: `ready`, `warnings`, `blocked`, `unknown`, or
  `not_applicable`;
- repository commit, branch, observation timestamp, and freshness;
- applicability and its repository-owned reason;
- target kind, URL, provider, and deployment ID;
- source, artifact, browser, and deployment verification levels;
- bundle-budget, document, route, console, network, asset, and polish checks.

An applicable report contributes the conditional `web_readiness` quality gate.
`ready` and `warnings` are passing evidence; `blocked` fails; `unknown` remains
blocked; `not_applicable` is not configured. Invalid JSON or a different schema
is imported as explicit blocked unknown evidence.

## Release policy

Release rules choose both a minimum verification level and policy:

- `block` prevents release when evidence is missing, stale, weak, or failing;
- `warn` records the same trace without blocking the release preview.

For example, a public production release can require the `web_readiness` gate
from `quality_runner_web_readiness`, minimum level `deployment_verified`, and
policy `block`. A source-inferred pass does not satisfy that requirement. The
report target must itself be deployment evidence bound by Quality Runner to the
repository's exact `HEAD`; Pronto does not upgrade evidence based on a passing
label.

Preparation and release previews use cached Pronto evidence by default. Add
`--fresh` only when a bounded live import is required. Fresh quality projection
and release-history inspection each have a 10-second deadline, and release
history is capped at 1,000 commits. An unavailable scan remains explicit and
blocks release rather than becoming an empty result.
