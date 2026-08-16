# Papercuts

Papercuts is Pronto's local interaction-quality corpus. It keeps evidence,
recurrence claims, and proposed interventions separate:

- **Observations** are individual explicit corrections, dissatisfaction,
  failure reports, failed verification, repeated failure, agent suggestions,
  capability gaps, or manual handoffs.
- **Patterns** are deterministic recurrence projections. Two matching
  observations in one scope create a local pattern. Three matching observations
  across at least two scopes create a cross-scope candidate. A verified urgent
  security, data-loss, destructive, or task-blocking failure creates an
  immediate local pattern but never implies cross-scope recurrence.
- **Multiplier proposals** are reviewable causal hypotheses. AI may draft them
  from the sanitized weekly digest, but it cannot merge evidence, accept a
  proposal, modify a repository, or start implementation. Acceptance records a
  human judgment; implementation is always a separate task.

## Capture and privacy boundary

Manual capture remains available in the built-in **Papercuts** skill. Codex can
also send explicit signals through a fail-open local hook and semantic agent
route. The passive route accepts direct user corrections, dissatisfaction, and
reported failures plus structural tool failures. It rejects quoted criticism,
hypothetical examples, third-party sentiment, ordinary negative subject matter,
and unsupported agent speculation.

Capture stores a structured summary, a pseudonymous provider/task/turn event
key, and at most 240 Unicode characters of sanitized evidence. Secrets and
absolute paths are redacted before persistence. Excerpt text is deleted after
90 days; its hash and structured observation remain. No transcript or full
assistant response is persisted.

The primary write is Pronto's SQLite database. A failed write is atomically
spooled under `~/Library/Application Support/Pronto/papercuts-hook` for at most
seven days and 10,000 events. If a task began before that path was granted, or
its sandbox otherwise denies Application Support, the hook falls back to a
private `0700` directory under that user's `$TMPDIR` and writes `0600` event
files. A later permitted hook invocation migrates emergency events to the
primary spool and flushes both tiers through the idempotent event-key contract.

One failure is silent. Three consecutive flush failures, or inability to write
either spool, produces one concise task warning. The hook always exits
successfully and never blocks or changes the answer. The legacy
`~/.codex/papercuts/health.json` path remains read-compatible during migration.

The repository source of truth is `scripts/papercuts-capture.py`. Run
`pnpm papercuts:hook:test`, install it atomically with
`pnpm papercuts:hook:install`, and verify the deployed copy with
`pnpm papercuts:hook:check`. Build the hook's narrow standalone CLI with
`pnpm papercuts:cli:build`, install it atomically with
`pnpm papercuts:cli:install`, and verify exact binary parity with
`pnpm papercuts:cli:check`.

Repository scopes use an opaque derivative of the current stable Pronto
repository identity because legacy repository IDs can contain absolute paths.
Unregistered repositories use an opaque Codex project identity; non-project
work falls back to `global-agent`.

## Machine-readable interface

The corpus schema is `pronto-papercuts/v2`. JSON is the supported agent
interface:

```sh
pronto papercuts observe --stdin --json [--dry-run]
pronto papercuts list --json
pronto papercuts digest --week current --json
pronto papercuts propose --stdin --json
pronto papercuts proposal set-status <id> <draft|accepted|deferred|rejected> --json
pronto papercuts health --json
```

Observation input accepts `signal_kind`, `target_kind`, `summary`,
`phenomenon_key`, `failure_mode`, scope fields, evidence references, priority,
and verified/urgent flags. Event keys are unique and ingestion is idempotent.
Fingerprints are versioned and deterministically combine normalized phenomenon,
target class, and failure mode.

The compatibility `Papercut` projection remains available during the migration
release. Existing v1 rows migrate transactionally into local patterns with one
`legacy_manual` observation each; IDs, statuses, evidence, and timestamps are
preserved.

## Weekly digest

Pronto generates the deterministic sanitized JSON digest first. The Sunday
Codex automation may then draft hypotheses from that JSON and write only draft
proposals through the CLI. The app's **Weekly digest** view exposes counts,
leading patterns, capture health, and proposal review controls.

The automation is `configured` after installation. It is not `live-passing`
until a real scheduled Sunday execution has been observed with successful
digest read, proposal write-back, and notification delivery.

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
