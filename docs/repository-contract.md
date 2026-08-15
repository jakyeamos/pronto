# Pronto repository operating contract

Last reviewed: 2026-08-01.

This is the repository-specific execution contract for contributors and
agents. The canonical branch is `main`; work is developed on an isolated
branch, reviewed through the integration lane, and merged only after required
gates pass on the exact candidate commit. Dirty, unpublished, active, or
ambiguous work is preserved before folding. Branch and worktree removal waits
until integration or patch equivalence and ownership are proven.

## Architecture and ownership boundaries

- `src-tauri/src/core.rs` owns repository discovery, durable SQLite state, Git
  and provider evidence, focused CLI projections, preflights, and audits.
- Focused Rust modules such as `quality.rs`, `remediation.rs`, and
  `change_matrix.rs` own their domain rules. Add behavior there instead of
  duplicating it in the renderer or Node adapter.
- `src/renderer/src/` renders shared projection contracts. It may format state
  but must not invent freshness, passing evidence, authorization, or closure.
- `bin/pronto.mjs` is a thin launcher for the native CLI. The desktop and CLI
  consume the same Rust-owned truth rather than maintaining parallel domain
  implementations.
- `.agents/context/`, `.pronto/`, `.project-compass/`, and
  `.agents/change-surface-matrix.json` are governed contracts. Update the
  affected projection, consumer, documentation, and machine-readable evidence
  together.

When a contract changes, trace the full path: source evidence, Rust domain
model, persisted representation, CLI JSON, renderer type and component,
remediation/analytics consumers, tests, and context documentation. Use the
repository change-surface matrix to retain conditional external impacts.

## Coding conventions

- Keep Rust domain types explicit and serializable; preserve schema versions
  and distinguish missing, stale, blocked, failed, unknown, and passing states.
- Keep TypeScript strict. Renderer types must admit every nullable or optional
  value the Rust projection can emit, and components must render unavailable
  evidence without converting it into zero or success.
- Prefer small domain helpers and table-driven mappings over repeated string
  interpretation across components.
- Keep the Node adapter provider-neutral and fail closed when a prerequisite,
  binary, path, or native response is unavailable.
- Run formatting, lint, typecheck, focused tests, and the relevant production
  build. Do not weaken an existing gate to make a candidate pass.

## Security and credential constraints

Pronto is local-first and read-only by default. Never commit credentials,
tokens, raw keychain values, local SQLite databases, or provider response
caches. Do not log secrets or include them in fixtures. `gh` authentication is
provider access evidence, not permission to mutate GitHub.

Quality evidence is fresh only when its observation is inside the configured
freshness window and its scanned commit equals the current repository commit.
A matching branch name is routing context, not proof that the code is unchanged.
When comparable commit provenance is unavailable, a matching branch reports
unknown and a differing branch reports stale; an exact commit match remains
authoritative even if that commit is checked out through another branch name.

For finding counts and the repository review ledger, prefer QR's fingerprinted
`code-quality-scan.json` detector report. Aggregate `quality-audit.json`
findings remain remediation context but must not replace stable detector
identities; use the aggregate only when the fingerprinted report is absent.

The [Mac Control maturity gate](mac-control-maturity-gate.md) is an additional
condition on the existing 4.0/4.0 maturity ideal. It applies to the current
maturity scope and does not define a four-repository denominator or inferred
cohort.

Network refreshes, Git writes, provider mutations, application installation,
and release publication are separate authority boundaries. Use the narrowest
repository or provider scope and preserve the exact source commit and evidence
timestamp. AI remains disabled by default and must not receive source content,
credentials, or operational authority without a separately designed contract.

## Common failure modes and recovery

| Observed problem                                                | Likely boundary                                                       | Safe recovery                                                                                                                                                                                                                                                                                                                                |
| --------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `route` is blocked or the snapshot is stale                     | Pronto evidence, not repository truth                                 | Follow `next_safe_step`, refresh only the named scope with authorization, then rerun the same focused route.                                                                                                                                                                                                                                 |
| Workspace is ahead, behind, diverged, dirty, or has no upstream | Local branch versus its configured upstream                           | Inspect live Git state, preserve unpublished work, fetch, and classify the branch before any integration or pruning.                                                                                                                                                                                                                         |
| GitHub evidence is unavailable                                  | `gh` path, version, login, repository identity, or provider freshness | Verify each prerequisite in a fresh shell, run the bounded provider refresh, and confirm the imported commit. Do not substitute SSH success for API evidence.                                                                                                                                                                                |
| Compass is missing, invalid, blocked, or drifting               | `.project-compass` product evidence                                   | Run Compass validation and scoring, reconcile only observed evidence, then checkpoint. Do not manufacture maturity.                                                                                                                                                                                                                          |
| Maturity is missing or stale                                    | Stable Quality Runner feed or repository identity                     | Run a protected audit, replay it, publish the selected audit to the stable feed, and refresh Pronto. Keep static and dynamic evidence distinct.                                                                                                                                                                                              |
| Findings are unavailable despite fleet audits                   | Repository-local full detector report                                 | Run `pronto quality detector-refresh --json`; inspect every published, blocked, unsupported, ingested, or rejected result. Pronto refreshes target refs before import, excludes exact QR `unsupported` results from the applicable coverage denominator, and exits nonzero if QR says `published` but exact-target evidence is not selected. |
| Installed app differs from the current build                    | `/Applications/Pronto.app` deployment boundary                        | Build a bundle first, install only with authorization, restart the app, and run `pnpm app:check`.                                                                                                                                                                                                                                            |

## Definition of done

A change is done only when all applicable conditions hold:

1. The observed problem, immediate cause, repeatable failure mode, and affected
   consumers are accounted for.
2. Focused tests cover positive, negative, and ambiguous behavior; required
   repository gates pass without bypasses.
3. CLI JSON, renderer behavior, persisted state, remediation coverage,
   analytics, Compass, and external evidence are reconciled when affected.
4. The exact candidate commit passes required pull-request checks, and
   canonical `main` is verified after merge when remote truth matters.
5. Documentation, implementation examples, and the change-surface matrix are
   updated when their contract changes.
6. Every validation failure has a fix or an exact blocker and disposition.
7. Destructive cleanup occurs only after preservation, integration proof,
   publication verification, and separate authorization.

The canonical local gate set is declared in `.quality-runner.toml` and
`package.json`. `pnpm smoke` also runs `pnpm contract:check`, which verifies
this router, linked documentation, canonical branch declaration, and structured
change-surface evidence.

## Approval-gated paths and operations

Read-only inspection is allowed within task scope. Obtain explicit authority
before:

- changing Pronto's persisted local registry or repository-owned disposition
  ledgers;
- installing global/system prerequisites or writing `/Applications/Pronto.app`;
- committing, merging, rebasing, pushing, pruning, deleting, or rewriting Git
  state;
- refreshing provider data, creating or merging pull requests, publishing a
  release, or changing provider permissions;
- changing credentials, security settings, privacy boundaries, or AI data
  access.

Never use a dirty canonical checkout as an integration scratchpad. Never infer
publication authority from authenticated read access.

## Installation, release, and rollback

`pnpm build:bundle` produces the macOS bundle without installing it.
`pnpm build` builds and installs into `/Applications/Pronto.app`, so it crosses
the application deployment boundary and requires that exact intent. The
installer performs a staged whole-bundle replacement, verifies exact parity,
and restarts Pronto when it was already running. It temporarily unloads and
restores the repository-owned `com.pronto.skill-usage-collector` launch agent
so that service's KeepAlive policy cannot race the replacement, then forces a
distinct foreground launch so LaunchServices does not treat that collector as
the desktop window; overlay copies are forbidden because they can retain
obsolete files. An app-facing change is not complete until `pnpm app:check`
passes and the installed version has been launched.

Release preparation is not publication. Before a release, require a clean
canonical commit, a confirmed baseline, fresh required gates, a deterministic
`release preview`, exact artifact provenance, and provider-native review. This
repository currently has no verified published-release baseline for the latest
revision; publication remains blocked until that evidence is established.

For rollback, select a previously verified tag or exact commit in an isolated
worktree, run the full required gates, build its bundle, install it only with
authorization, restart Pronto, and verify `pnpm app:check`. If no previously
verified artifact or source revision exists, stop instead of guessing a
rollback target.
