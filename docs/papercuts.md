# Papercuts

Papercuts are the durable backlog surface for the design-audit family.

The boundary is intentional:

- `design-friction-audit` is a per-turn sensor. It can point out a repeated
  small hurt while answering a request, but it does not write project state.
- **Papercuts** is a built-in skill in Pronto's **Skills** surface. Selecting
  the skill opens the explicit capture point, where a person records the
  symptom, surface, evidence references, impact, priority, and next validation
  step. The same skill is addressable through `pronto skills papercuts`.
- The backlog is stored locally in Pronto under the
  `pronto-papercuts/v1` schema. Items can be `open`, `in_progress`,
  `deferred`, or `resolved`; resolved items remain visible as audit history.
- The **Promotion inbox** remains a separate AWL → JAS handoff. Papercuts are
  not promotion candidates and are not automatically sent to another system.

## Agent detection policy

Agents should surface pipeline optimization as a Papercut, even when the task
succeeds, when current-run evidence shows both a reproducible cost and a
plausible local, reversible code fix. Repeated deterministic work, needless
serialization or polling, and independent stages that can be parallelized are
examples. A long run by itself is not enough: external queueing, provider
latency, rate limits, authentication/setup delays, and one-off unexplained
slowdowns stay out of this candidate class.

The UI does not ingest prompt transcripts automatically. Repeated friction
becomes durable only through an explicit capture in the Papercuts skill detail
surface.

## Codex capture runtime

The local Codex capture hook writes primary observations through
`pronto-papercuts` and keeps its fail-open spool and health state under
`~/Library/Application Support/Pronto`. Workspace-write sessions must grant
that Pronto-owned application-support directory as an additional writable root.
If the CLI and spool are both unavailable, the hook returns a specific
fail-open warning and exits successfully; it must not replace that warning with
an internal-error response caused by health bookkeeping.

The hook preserves sanitized diagnostics at the delivery boundary. Stable
codes identify the remediation path without exposing exception text or user
content:

- `PAPERCUTS-E4001`: the Pronto capture process timed out.
- `PAPERCUTS-E4002`: the Pronto capture process failed.
- `PAPERCUTS-E4003`: the Pronto capture executable was unavailable.
- `PAPERCUTS-E4004`: the Pronto capture process returned invalid JSON.
- `PAPERCUTS-E5002`: a queued observation failed contract validation.
- `PAPERCUTS-E6002`: a queued observation could not be read.
- `PAPERCUTS-E6003`: a flushed spool file could not be removed.

Health state keeps the latest diagnostic in `last_error` with its `stage`,
`operation` (`capture` or `drain`), and observation time. The consecutive
failure counter and threshold warning remain separate so an outage can be
counted even when its diagnostic changes. The hook's JSON flush response also
includes the diagnostic when a drain stops, while hook exit behavior stays
fail-open.
