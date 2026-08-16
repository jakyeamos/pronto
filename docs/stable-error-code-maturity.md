# Stable error-code maturity

Quality Runner owns the `diagnosability.stable_error_codes` maturity
dimension. Pronto consumes it from the private maturity feed and displays it as
repository maturity evidence and, when below 4/4, a remediation gap.

The dimension is intentionally separate from fresh test readiness. Quality
Runner uses bounded static evidence to assess four signals:

- a discoverable declaration of machine-readable error identities;
- propagation of the identity through a response, result, envelope, payload,
  serialization, or JSON boundary;
- documentation describing the stable error contract; and
- regression-test evidence that names or asserts the contract.

The maintained 4/4 level additionally requires at least two distinct code
values. A repository with no supported runtime source is `not_applicable`; a
source repository with no stable error-code contract scores 0. Unknown,
blocked, stale, and not-applicable states remain evidence states and must not be
rendered as passed tests.

The producer contract is documented in
`quality-runner/docs/integrations/pronto-maturity-feed.md`. Pronto's renderer
must preserve the feed's dimension ID for machine-readable access while using
the friendly label “Stable error codes” for people.
