# Quality Runner prevention pilot

Pronto pilots the task-scoped Quality Runner contract without making the
native gates authoritative. The pilot uses an immutable pull-request target
as the baseline and checks the exact pull-request workspace. Existing findings
remain visible and non-blocking; a new behavior-verified `nested-ternary`
occurrence is a policy violation.

## Current evidence

The clean pilot worktree was created from `origin/main` at
`65e26a270f26a056972c09bb32679dbe862b8c4a`. Dependencies were installed with
`pnpm install --frozen-lockfile` using pnpm 11.9.0 and Node 22.14.0.

The following commands passed twice on the unchanged local worktree:

- `pnpm lint` with ESLint 10.8.0.
- `pnpm format:check` with Prettier 3.9.6.
- `pnpm typecheck` with TypeScript 5.9.3.
- `pnpm test` with Cargo 1.97.1 and Vitest 4.1.10; each run passed 54 Rust
  tests and 14 renderer tests.

That is repeatability evidence, not certification. None of these checks has a
checked-in intentional-failure fixture or execution evidence from the pilot CI
environment yet, so each remains `candidate` and advisory.

`pnpm smoke` is unavailable on the canonical branch because `package.json`
does not define a `smoke` script. It must not be certified from evidence in a
different or dirty checkout.

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
