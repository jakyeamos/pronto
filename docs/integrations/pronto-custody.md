# Pronto custody ledger projection

Pronto projects isolated-change-workflow custody as a read-only fleet view. The
workflow owns receipt signing, lease renewal, pause/resume, adoption,
integration locking, WIP validation, and cleanup. Pronto never adopts a lane,
merges a branch, deletes a worktree, or pushes a ref from this projection.

Read one repository:

```bash
pnpm --silent --dir "$PRONTO_ROOT" run cli custody /absolute/path/to/repository --json
```

Read the registered fleet:

```bash
pnpm --silent --dir "$PRONTO_ROOT" run cli custody --json
```

The native projection is `pronto-custody-projection/v1`; the CLI envelope is
`pronto-custody-cli/v1`. Both are derived from live Git worktrees, branches,
heads, status, operation markers, open-file evidence, and local receipt files.
Cached Pronto labels do not authorize custody decisions.

## State and disposition

Each lane has an independent lifecycle `state`:

`active`, `paused`, `stale`, `adoptable`, `contested`, `integrating`, `closed`,
or `unknown`.

Each lane also has a primary `disposition`, a unique `dispositions` list, and a
`next_action`. This prevents `unknown` from becoming a catch-all. Important
codes include:

- `legacy_unsigned_receipt`: route through the bounded legacy owner-return or adoption review.
- `receipt_malformed`: preserve and repair or replace through the workflow owner.
- `receipt_integrity_invalid`: preserve the known receipt and repair its missing or malformed integrity evidence.
- `receipt_schema_unsupported`: preserve and upgrade the producer before any custody mutation.
- `worktree_not_live`, `branch_binding_mismatch`, or `head_binding_mismatch`: re-establish exact live Git identity.
- `dirty_worktree`, `open_files_observed`, or `git_operation_active`: adoption is blocked by live activity.
- `live_git_evidence_unavailable`: retry; missing evidence is not abandonment.
- `adoption_ready`: recheck the exact generation and head, then use the workflow adoption command.

Legacy and unsupported receipts remain `unknown` with their concrete reason;
they never become `adoptable` because their timestamps are old. An expired
clean signed-shape lane can be `adoptable`, but Pronto reports its receipt
integrity as unverified because it does not own the workflow's HMAC key.

The renderer and agent JSON should show the disposition and next action next to
the lifecycle state. It must not present a raw `unknown` label without the
underlying reason.
