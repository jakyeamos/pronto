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

Each spooled observation is drained independently. A malformed record, or one
rejected by the installed CLI's observation contract, is moved to a private
`quarantine` directory instead of blocking later records. Health reports the
isolated count as `quarantined_events`; quarantined content remains local for
bounded diagnosis and is never retried automatically.

One failure is silent. Three consecutive flush failures, or inability to write
either spool, produces one actionable task warning with the stable error code,
plain-language cause, failed operation, attempt count, queued-observation count,
and recovery command. Process failures also expose a bounded timeout or exit
code when available. The same sanitized detail is retained in `last_error` in
the health JSON and shown in Pronto's Papercuts surface. The hook always exits
successfully and never blocks or changes the answer. The legacy
`~/.codex/papercuts/health.json` path remains read-compatible during migration.

The repository source of truth is the thin `scripts/papercuts-capture.py`
entrypoint and the responsibility-split `scripts/papercuts_capture/` runtime
package. Run `pnpm papercuts:hook:test`, install the complete runtime with
`pnpm papercuts:hook:install`, and verify the deployed copy with
`pnpm papercuts:hook:check`. Build the hook's narrow standalone CLI with
`pnpm papercuts:cli:build`, install it atomically with
`pnpm papercuts:cli:install`, and verify exact binary parity with
`pnpm papercuts:cli:check`. Installation writes private runtime modules before
atomically replacing the entrypoint. Installation and check compare every
deployed source file and the hook's public
observation contract with the standalone CLI contract as well as comparing
binary bytes and permissions, so producer/consumer enum drift fails at
deployment rather than during capture.

Repository scopes use an opaque derivative of the current stable Pronto
repository identity because legacy repository IDs can contain absolute paths.
Unregistered repositories use an opaque Codex project identity; non-project
work falls back to `global-agent`.

## Machine-readable interface

The corpus schema is `pronto-papercuts/v2`. JSON is the supported agent
interface:

```sh
pronto papercuts observe --stdin --json [--dry-run]
pronto papercuts contract --json
pronto papercuts list --json
pronto papercuts digest --week current --json
pronto papercuts propose --stdin --json
pronto papercuts proposal set-status <id> <draft|accepted|deferred|rejected> --json
pronto papercuts health --json
```

Capture-health failures expose `error_code`, `failure_kind`, `stage`, `message`,
`operation`, `attempt`, `observed_at`, `retryable`, and `recovery_command`.
Process diagnostics may also include `timeout_seconds` or `exit_code`. Run
`pronto-papercuts papercuts health --json` to inspect the standalone collector
without depending on the full app command surface.
`papercuts contract --json` returns `pronto-papercuts-observation/v1`, including
the accepted signal and target enums and a minimal sanitized input. It is a
read-only deployment and diagnostic surface, not an ingestion command.

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
