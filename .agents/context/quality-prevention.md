# Pronto implementation quality prevention

Last reviewed: 2026-08-01.

Load this packet for Pronto source or configuration changes. Quality Runner is
the independent task-relative evidence check; agent instructions guide the
implementation but do not replace QR evidence.

## Checkpoint contract

Before source edits, capture the exact starting workspace:

```sh
PRONTO_ROOT="$(git rev-parse --show-toplevel)"
qr task start "$PRONTO_ROOT" --task-id <stable-task-id> --json
```

During editing, use the repository-required native checks for fast feedback as
appropriate to the touched scope. The current prevention policy lists
`pnpm lint`, `pnpm format:check`, `pnpm typecheck`, `pnpm test`, and `pnpm smoke`
as candidates. They remain required where Pronto's ordinary repository policy
requires them, but candidate state does not authorize QR to execute them or
prove that they are certified preventative gates.

Before declaring implementation complete, run:

```sh
qr task check "$PRONTO_ROOT" --task-id <stable-task-id> --json
```

Use `task-check.json` as the canonical result and `task-check.md` as its human
projection:

- `pass`: proceed only after every other repository-required check passes.
- `violation`: fix or explicitly disposition every new enforced finding and
  failed certified gate, then rerun the check.
- `blocked`: resolve the missing or ambiguous evidence; never treat it as pass.
- `invalid`: correct the invocation or prevention configuration and rerun.

If `qr task` is unavailable, report the prerequisite as blocked. Do not
substitute a full scan, an agent assertion, or `HEAD` for the task baseline.
Use `qr task rebaseline --reason ...` only when the policy or evidence contract
actually changed; preserve lineage.

This is a baseline and completion/PR checkpoint, not a continuous-save or
editor-hook loop. Advisory findings may guide implementation, but must not be
copied wholesale into static prohibitions. Promote a stable deterministic
finding only through the documented behavior-verification process, preferably
into a faster native checker when that checker can be certified.

See `docs/quality-prevention.md` for current pilot evidence and certification
boundaries.
