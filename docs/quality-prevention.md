# Quality Runner prevention pilot

Pronto pilots the task-scoped Quality Runner contract without making the
native gates authoritative. The pilot uses an immutable pull-request target
as the baseline and checks the exact pull-request workspace. Existing findings
remain visible and non-blocking; a new behavior-verified `nested-ternary`
occurrence is a policy violation.

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
