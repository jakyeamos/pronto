# Pronto repository operating contract

Last reviewed: 2026-08-13.

This is the repository-specific execution contract for contributors and
agents. The canonical branch is `main`; work is developed on an isolated
branch, reviewed through the integration lane, and merged only after required
gates pass on the exact candidate commit. Dirty, unpublished, active, or
ambiguous work is preserved before folding. Branch and worktree removal waits
until integration or patch equivalence and ownership are proven.

For fleet integration and maturity evidence, Pronto uses an explicit `dev`
target when one is configured. This target is separate from Git's release or
default branch (`main` or `master`): configuring `dev` does not rename,
replace, or weaken the release branch. A target-scoped Quality Runner result is
authoritative only when its scanned branch and scanned commit both equal the
configured target branch and its current head. Configure the target with
`pronto repo set-target <repository> dev --json`, then run a scoped
`pronto refresh <repository> --json`; stale or mismatched evidence remains
unavailable until it is regenerated from the exact `dev` commit.

## Architecture and ownership boundaries

- `src-tauri/src/core.rs` owns repository discovery, durable SQLite state, Git
  and provider evidence, focused CLI projections, preflights, and audits.
- Focused Rust modules such as `quality.rs`, `remediation.rs`,
  `change_matrix.rs`, and `showcase.rs` own their domain rules. Add behavior
  there instead of duplicating it in the renderer or Node adapter.
- `src/renderer/src/` renders shared projection contracts. It may format state
  but must not invent freshness, passing evidence, authorization, or closure.
- `bin/pronto.mjs` is a thin launcher for the native CLI. It invokes bare
  `cargo` so Codex storage controls remain in the command path; operators may
  explicitly set `PRONTO_CARGO` outside Codex when needed. The desktop and CLI
  consume the same Rust-owned truth rather than maintaining parallel domain
  implementations.
- `refresh-batch` is the fleet refresh path for bounded parallel Git/filesystem
  scans. It performs one deterministic merge under the normal store lock,
  retries once when the read-only baseline is invalidated, and never calls
  providers. When a scan discovers a new repository and exactly one valid
  fleet Showcase contract exists, that merge may atomically append the
  repository's pending `unknown` Showcase goal row; it performs no other
  repository-file writes. The ordinary `refresh` command remains the
  compatibility path for single-repository and serial workflows and follows
  the same bounded onboarding rule.
- `.agents/context/`, `.pronto/`, `.project-compass/`, and
  `.agents/change-surface-matrix.json` are governed contracts. Update the
  affected projection, consumer, documentation, and machine-readable evidence
  together.

AI Showcase readiness is governed by one fleet-level
`.pronto/showcase-goal.json`; see `docs/showcase-contract.md`. Public
eligibility is a hard gate before public priority and publishing. In
particular, `private_client` work can retain a readiness score as private audit
context but must never count toward the public goal or enter the public
materials/publishing queue. When a new repository is registered while exactly
one valid fleet contract exists, the native merge adds an explicit pending goal
row with unknown eligibility and dimensions before persisting the registry;
this does not create a score, public eligibility, or publication target.
Registered repositories absent from the reviewed contract remain visible as
unranked `unknown` entries, and a missing fleet contract remains missing.

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
Finding categories are an open set: Pronto derives them from the report's
finding records, supplements categories absent from those records with
`summary.findings_by_category`, and renders every non-zero native, `skill:*`,
or future category without a consumer-side allowlist. The finding records win
when a producer summary disagrees, so a stale aggregate cannot hide emitted
findings. Category-level actionable counts apply the same non-actionable
dispositions as the overall finding count.
Fleet finding dimensions remain maturity evidence rather than duplicate code
findings. Pronto preserves every scored dimension as an open set and exposes
the complete score inventory in the maturity disclosure; descriptive gaps are
bounded separately by the feed contract.

The [Mac Control maturity gate](mac-control-maturity-gate.md) contributes
implementation-contract and live-task-evidence dimensions to the consolidated
score for every registered repository unless that repository explicitly proves
it is not applicable. Pronto retains the QR source score separately so local
CI and Mac Control consolidation never obscures provenance.
Project Compass contributes `project_compass.mvp_progress` and
`project_compass.complete_product_progress`, converting its audited 0–100
progress values to 0–4. Missing or invalid contracts contribute zero instead of
disappearing from the fleet denominator; confidence remains evidence metadata
and does not raise maturity by itself.

The [agent usability maturity contract](agent-usability-maturity.md) contributes
four explicit documentation, skill-coverage, behavior, and freshness/portability
lanes plus scored growth health to that aggregate. Repositories own the
tool relationship in `.agents/agent-usability.json`; Pronto imports the QR
projection and never infers passing coverage from file volume.

Network refreshes, Git writes, provider mutations, application installation,
and release publication are separate authority boundaries. Use the narrowest
repository or provider scope and preserve the exact source commit and evidence
timestamp. AI remains disabled by default and must not receive source content,
credentials, or operational authority without a separately designed contract.

## Common failure modes and recovery

| Observed problem                                                | Likely boundary                                                       | Safe recovery                                                                                                                                                 |
| --------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `route` is blocked or the snapshot is stale                     | Pronto evidence, not repository truth                                 | Follow `next_safe_step`, refresh only the named scope with authorization, then rerun the same focused route.                                                  |
| Workspace is ahead, behind, diverged, dirty, or has no upstream | Local branch versus its configured upstream                           | Inspect live Git state, preserve unpublished work, fetch, and classify the branch before any integration or pruning.                                          |
| GitHub evidence is unavailable                                  | `gh` path, version, login, repository identity, or provider freshness | Verify each prerequisite in a fresh shell, run the bounded provider refresh, and confirm the imported commit. Do not substitute SSH success for API evidence. |
| Compass is missing, invalid, blocked, or drifting               | `.project-compass` product evidence                                   | Run Compass validation and scoring, reconcile only observed evidence, then checkpoint. Do not manufacture maturity.                                           |
| Maturity is missing or stale                                    | Stable Quality Runner feed or repository identity                     | Run a protected audit, replay it, publish the selected audit to the stable feed, and refresh Pronto. Keep static and dynamic evidence distinct.               |
| Installed app differs from the promoted checkpoint              | `/Applications/Pronto.app` deployment boundary                        | Run `pnpm app:update` only with installation authority, restart the app, and run `pnpm app:check`.                                                            |

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

`failure_visibility` is a required local capability. `pnpm failure-visibility`
executes representative positive and negative paths across the Rust source of
truth, renderer normalization boundary, and compatibility collector. A
discovered command is only configured coverage; it becomes fresh-passing
evidence only after successful execution. Malformed, stale, unavailable, or
degraded evidence must remain explicit and must never be normalized into a
successful or observed-looking value.

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

`pnpm dev` is the ordinary app-facing development and direct-verification lane;
it runs the current checkout with live reload and does not update
`/Applications/Pronto.app`. `pnpm build:bundle` produces only the macOS app
bundle without installing it, while `pnpm build:release` produces the complete
configured distribution artifact set. `pnpm app:update` builds the app-only
bundle and installs it into `/Applications/Pronto.app`; `pnpm build` and
`pnpm app` remain compatibility aliases for that explicit promotion path.

Installation crosses the application deployment boundary and requires that
exact intent. The installer performs a staged whole-bundle replacement,
verifies exact parity, and restarts Pronto when it was already running; overlay
copies are forbidden because they can retain obsolete files. Installed-app
verification is required for promoted checkpoints, Tauri configuration,
bundled-asset, native-entry-point, or installation-behavior changes, and
release preparation. Those paths are not complete until `pnpm app:check`
passes and the installed version has been launched. Ordinary source changes may
instead complete against the live `pnpm dev` surface plus applicable quality
gates without claiming that the installed app is current.

The optional current-Codex usage bridge is a separate persistent boundary.
`pnpm skills:collector:install` adds a user LaunchAgent and a marked
`~/.codex/config.toml` OTLP exporter; it requires explicit approval and a
currently installed Pronto bundle. `pnpm skills:collector:uninstall` removes
only those marked additions and preserves recorded local counts. A source build
or app installation alone does not prove the collector is loaded; require
`pnpm skills:collector:check`, a fresh Codex process, an observed metric, and a
Skills CLI/UI readback.

Release preparation is not publication. Before a release, require a clean
canonical commit, a confirmed baseline, fresh required gates, a deterministic
`release preview`, exact artifact provenance, and provider-native review. This
repository currently has no verified published-release baseline for the latest
revision; publication remains blocked until that evidence is established.

For every remediation goal resolved as `public_release`, Pronto imports
`.quality-runner/release-boundary.json` using
`quality-runner-release-boundary/v2`. The receipt is read-only evidence: Pronto
never executes commands from it. It must name the exact current branch and
commit, match the current `.agents/change-surface-matrix.json` digest, contain
current wheel and sdist hashes, and show passing classification, tracked-content,
artifact-allowlist, sanitized-adapter-fixture, and isolated-install checks. A
missing, legacy, malformed, stale, policy-mismatched, artifact-mismatched, or
failed receipt creates evidence-derived remediation and hard-blocks release
preview. Manual `verified` or `deferred` action status cannot bypass this gate;
the action disappears only after a fresh passing receipt is imported.

The producer classifies release-relevant surfaces as `public_core`,
`public_adapter`, or `local_only`; rejects unclassified surfaces; scans tracked
source and docs for personal paths or private inventory; inspects built artifacts
against an allowlist; installs and exercises the artifact with an isolated home
and private workspace peers absent; and verifies optional integrations only with
sanitized fixtures or consumer-owned tests. Receipts contain repository-relative
paths rather than personal absolute paths. Non-public and unresolved goals do
not inherit this obligation.

Preparation and release previews read the persisted snapshot by default so a
slow or malformed repository evidence tree cannot hang the command. `--fresh`
opts into a quality projection with a 10-second deadline. Release-history
inspection is separately limited to 1,000 commits and 10 seconds. Timeout or
Git failure is explicit unavailable evidence and blocks release; it is never
normalized into an empty commit set.

Web readiness is imported only from
`.quality-runner/web-readiness.json` with schema
`quality-runner-web-readiness/v1`. Pronto preserves the report's exact commit,
branch, freshness, target URL/provider/deployment identity, categorical status,
per-check verification level, and route evidence. Release requirements select
a minimum evidence level and a `block` or `warn` policy. Source inference cannot
satisfy a deployment-verified requirement, and warning-only requirements stay
visible without becoming release blockers.

For rollback, select a previously verified tag or exact commit in an isolated
worktree, run the full required gates, build its bundle, install it only with
authorization, restart Pronto, and verify `pnpm app:check`. If no previously
verified artifact or source revision exists, stop instead of guessing a
rollback target.
