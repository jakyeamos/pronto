# Pronto implementation examples

Last reviewed: 2026-08-01.

These are references to maintained implementation patterns in the repository,
not copy-and-paste templates. Read the focused files and their tests before
extending a pattern.

## Add or change an evidence projection

Workspace synchronization detail is a reference implementation for preserving
source evidence and freshness through the agent API:

- `src-tauri/src/core.rs` owns `WorkspaceSyncDetail`, derives it only for an
  unsynchronized workspace, and carries observation and expiry timestamps.
- The focused repository projection exposes the detail without mutating Git.
- `workspace_sync_detail_exposes_expiry_reason_and_scoped_refresh` covers the
  reason, upstream comparison, exact expiry, and bounded refresh instruction.
- Renderer types and components consume the optional detail and keep a synced
  workspace visually distinct from missing evidence.

When adding a field, update the Rust-owned type and derivation first, then every
serialized consumer and focused test. Do not compute a second interpretation
in React.

## Add or change a quality or remediation rule

Quality finding dispositions show the intended evidence overlay pattern:

- `.pronto/quality-finding-dispositions.json` stores repository-owned review
  decisions by stable fingerprint with reviewer and timestamp provenance.
- `src-tauri/src/quality.rs` reconciles the overlay against current findings
  and treats missing, expired, invalid, or mismatched decisions as non-suppressing.
- `src-tauri/src/remediation.rs` uses the reconciled actionable count while
  retaining raw, reviewed, and disposition totals.
- `src/renderer/src/components/QualityComponents.tsx` presents configured gates
  separately from execution evidence.

A new rule must retain raw evidence, define stale and ambiguous behavior, keep
the quality and remediation projections consistent, and test both suppression
and fail-open recurrence.

## Add or change a repository surface

The remediation coverage ledger is the reference for cross-surface parity:

- `src-tauri/src/remediation.rs` defines one coverage entry for every
  repository-level UI surface and links attention or blocked entries to action
  IDs.
- Renderer coverage consumes that machine-readable ledger rather than keeping
  a separate warning inventory.
- `docs/development.md` documents the same coverage invariant for operators.

When a repository card, warning, or evidence source is added, changed, or
removed, update the producer, renderer, remediation ledger, tests, and
`.agents/change-surface-matrix.json` in the same change.

## Preserve before folding

`fold_preview_preserves_unpublished_branch_and_requires_live_verification` in
`src-tauri/src/core.rs` is the safety reference for branch remediation. It
shows that an integration-eligible diff is not sufficient when unpublished or
active-work evidence exists. The correct sequence is preserve, reconcile the
canonical branch, fold the wanted change, verify and publish, then prune only
after equivalence and ownership checks.

## Keep fail-open capture durable across sandbox generations

`scripts/papercuts-capture.py` is the reference for a bounded two-tier local
handoff. It writes Pronto-owned Application Support first, falls back to a
private `$TMPDIR` spool when an already-running task lacks that grant, and
migrates the emergency tier on the next permitted invocation. Stable event
keys make retry, migration, and flush idempotent. The paired test covers denied
primary storage, secure permissions, deduplication, migration, and recovery.

Keep the hook in repository source, deploy it atomically through
`pnpm papercuts:hook:install`, and verify the live copy separately from source
tests with `pnpm papercuts:hook:check`.
