# Quality Runner prevention pilot

Pronto pilots the task-scoped Quality Runner contract without making the
native gates authoritative. The pilot uses an immutable pull-request target
as the baseline and checks the exact pull-request workspace. Existing findings
remain visible and non-blocking; a new behavior-verified `nested-ternary`
occurrence is a policy violation.

## Implementation contract

Pronto uses a hybrid feedback loop:

1. Agent guidance requires a task baseline before source edits.
2. Repository-native checks provide fast feedback where they apply.
3. `qr task check` is the independent completion and pull-request evidence
   checkpoint.
4. Full QR scans remain nightly or rule-pack-change audit boundaries.

The task check is not required on every save. Run it before declaring the
implementation complete and again after correcting a violation or blocker.
`task-check.json` is authoritative; its `next_action` states the required
response, and `task-check.md` is the derived human projection. A QR `pass` does
not waive Pronto's other required repository checks.

Static agent rules explain this workflow but do not replace QR's baseline,
coverage, deterministic matching, or policy evidence. Advisory findings are not
automatically copied into agent prohibitions. A repeatedly trusted
deterministic rule should graduate through behavior verification and, when
practical, into a faster native checker with separately certified maturity.

## Current evidence

The clean pilot worktree was reconciled onto `origin/main` at
`bd259121c827f63a6ef85a2524200e75d40930b2`. Dependencies were installed with
`pnpm install --frozen-lockfile` using pnpm 11.9.0 and Node 22.14.0.

The following commands passed twice on the unchanged local worktree:

- `pnpm lint` with ESLint 10.8.0.
- `pnpm format:check` with Prettier 3.9.6.
- `pnpm typecheck` with TypeScript 5.9.3.
- `pnpm test` with Cargo 1.97.1 and Vitest 4.1.10.
- `pnpm smoke`, which exercises the read-only CLI help path.

That is repeatability evidence, not certification. None of these checks has a
checked-in intentional-failure fixture or execution evidence from the pilot CI
environment yet, so each remains `candidate` and advisory.

Candidate state means QR does not execute these commands during `task check`.
They remain independently required wherever Pronto's ordinary repository and
CI contracts require them; QR certification is an additional preventative
claim, not a replacement for those contracts.

The warmed implementation-loop speed trial measured 14.944 seconds for those
five native commands and 20.608 seconds for the same commands followed by
`qr task check`, an added 5.664 seconds (37.9%). The separate one-time
`qr task start` baseline capture took 7.706 seconds. QR's analysis phase took
0.594 seconds with 124 cache hits and zero misses; snapshot and artifact work
accounted for the remainder of the 5.468-second task check. An initial cold
native run took 36.462 seconds because `pnpm test` spent 27.019 seconds warming
the Rust build cache, so it is not used as the steady-state comparison.

The first post-pilot `format:check` also showed that Prettier traversed
`.quality-runner` evidence artifacts. The pilot now excludes that generated
directory in both `.gitignore` and `.prettierignore`; the check passes again
without formatting or mutating QR evidence.

## Rule promotion

`nested-ternary` is the only promoted Pronto rule. Quality Runner owns its
positive, negative, and ambiguous TypeScript fixtures in
`tests/test_prevention_policy.py`. Pronto scopes enforcement to TypeScript
under `src/`; every other Quality Runner finding remains advisory.

## CI activation boundary

Do not make the task check authoritative or mark a native gate `certified`
until all of the following exist on integrated branches:

1. A published Quality Runner revision containing `qr task`.
2. An isolated locked bootstrap for the Pronto checkout.
3. A repeatable intentional-failure fixture for the exact gate command.
4. Passing local and CI evidence with the resolved command paths and versions.

At that point, PR CI should run:

```sh
qr task start . --task-id "pr-${PR_NUMBER}" --baseline-ref "${BASE_SHA}"
qr task check . --task-id "pr-${PR_NUMBER}"
```

Build, dead-code, secret, and dependency checks remain in their existing
quality lanes and are outside this pilot.
