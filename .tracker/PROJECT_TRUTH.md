# Project Truth

## Current State

- The first local-first Pronto desktop vertical slice is implemented on `main`.
- Tauri + React/TypeScript provides the desktop shell and portfolio console.
- Native Rust discovers repositories and linked worktrees, scans structured Git state, persists snapshots, transition events, and action audits, and exposes Tauri plus read-only CLI commands.
- The repository-selection Git environment is sanitized before native Git calls so hook variables cannot redirect scans or fixtures.
- A canonical PRD behavior inventory, coverage boundary sheet, delivery plan, and verification matrix are committed at `docs/pronto-behavior-spec.xlsx`; the workbook currently tracks 35 behaviors through implementation snapshot `b13de08` and needs a refresh for the newer provider, agent, handoff, preparation, release-rule, and AI-preview slices.
- The local desktop surface now has truthful navigation boundaries, Activity and Settings views, Cmd/Ctrl-K search focus, Escape drawer dismissal, freshness copy, and distinct filtered-empty states.
- Local durable state now uses a versioned SQLite database with non-destructive import from the legacy JSON registry; renderer and CLI snapshot contracts are unchanged.
- Safe local refresh/inspect preflights now use an explicit allowlist, exact root/repository target IDs, blocked-action records, and durable SQLite audit history; no destructive Git or provider-write action is exposed.
- Local portfolio configuration is now durable and editable: discovery-root ignore/fetch/monitoring settings, explicit product and group membership, product release modes, repository lifecycle confirmation, retention days, and recursive submodule summaries are stored in SQLite schema v3 and surfaced in the desktop UI.
- The companion CLI now shares the local registry and action boundary for product/group status filters, scoped read-only refreshes, structured JSON output, repository/app opening, and an audited blocked clone command.
- A provider-neutral remote snapshot contract now persists GitHub identities, remote repositories, locality matches, freshness, and unavailable states in SQLite schema v4; the desktop Remote catalog is read-only and never stores provider credentials.
- Workspaces now expose provider-neutral agent activity signals from process metadata and optional `.pronto/agent.json` manifests, with explicit `Active`, interrupted, recently active, and `Unknown` states; active evidence blocks integration eligibility without capturing terminal contents or command arguments.
- Workspace detail cards now provide permission-safe external handoff buttons for the exact registered workspace, opening Finder, Terminal, Visual Studio Code, or GitHub Desktop through structured macOS `/usr/bin/open` arguments without staging, committing, or changing registry state.
- Repository detail now offers a read-only preparation preview for pull requests and releases: exact head/base/commit/push/cleanliness evidence, provider PR/check/review uncertainty, published release baselines, deterministic conventional-commit grouping, and a candidate version are shown without creating a branch, worktree, commit, push, PR, or release.
- Repository release preparation now supports locally persisted deterministic rules with normalized AND/OR operators, commit-count/elapsed-time/conventional-type clauses, explicit first-release handling, exact neutral threshold wording, and passed/failed/unknown trace rows; saving a rule remains local-only.
- Repository preparation now exposes a local AI summary payload preview with repository permissions `Disabled`, `Commit metadata only`, and `Committed diff allowed`; payload bytes, categories, committed source references, provider/model trace fields, and an explicit no-request state are visible, and uncommitted content is excluded.

## Current Position

- Branch: `main`
- Implementation and documentation commits: `1817527`, `ef6ea99`, `73c28e5`, `533c2f5`, `885fe1e`, `faf0d99`, `d332573`, `f01c627`, `b13de08`, `5a0b120`, `42e8b54`, `853faa6`, `ed8d05f`, `6fde71e`, `8be4771`, `75889f3`, `3ad1451`, `6ed50bb`
- Verification for `b13de08` and `5a0b120`: 9 Rust tests, hook-mode `pnpm test`, cargo fmt, offline cargo test/check, typecheck, lint, Prettier, CLI JSON smoke, renderer production build, full Tauri app/DMG build, workbook inspect with zero formula errors, three-sheet visual review, `git diff --check`, and the source commit-time Pre-CR gate all passed. The final docs/workbook-only Pre-CR run produced no changed-line result because those paths are configured as ignored surfaces.
- Verification for `42e8b54`: 11 Rust tests, offline cargo check/test, typecheck, lint, Prettier, renderer production build, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `853faa6`: 11 Rust tests, CLI JSON smoke, offline cargo check/test, cargo fmt, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `ed8d05f`: 14 Rust tests, offline cargo check/test, cargo fmt, typecheck, lint, renderer production build, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `6fde71e`: 16 Rust tests, offline cargo check/test, cargo fmt, typecheck, lint, Prettier, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `8be4771`: 17 Rust tests, offline cargo check/test, cargo fmt, typecheck, lint, Prettier, renderer production build, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `75889f3`: 19 Rust tests, offline cargo check/test, cargo fmt, typecheck, lint, Prettier, renderer production build, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `3ad1451`: 19 Rust tests, offline cargo check/test, cargo fmt, typecheck, lint, Prettier, renderer production build, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Verification for `6ed50bb`: 20 Rust tests, offline cargo check/test, cargo fmt, typecheck, lint, Prettier, renderer production build, `git diff --check`, and the source commit-time Pre-CR gate all passed.
- Next deliverable: the release-threshold queue condition, deterministic recipe preflight/preview, and canonical workbook refresh; provider mutations, external AI execution, and publication remain deferred.

## Blockers

- Live GitHub refresh is environment-limited because the existing `gh` credential is invalid; remote-only clone, modifying action execution, provider-backed pull requests/releases, publication, and actual opt-in AI requests remain explicit deferred boundaries from the implementation plan. The local AI payload preview does not contact a model endpoint.

## Risks

- Folder-picker and full Tauri UI interaction still need live desktop verification.
- The SQLite schema is now at version 4 with explicit migrations for action audits, root/product/group configuration, and provider snapshots; future schema migrations still need explicit migration steps rather than silent compatibility assumptions.
- Provider refresh uses the existing GitHub CLI credential path and persists only sanitized identity/repository snapshots; credential repair and live remote evidence still need an authenticated environment.
- GitHub pull-request and release detail refreshes are read-only and may leave checks, reviews, mergeability, or release baselines unknown when provider detail calls are unavailable; the UI preserves that uncertainty instead of inferring readiness.
- Release rules currently cover deterministic numeric and conventional-commit clauses with flat AND/OR composition; repository-specific scripts/predicates and nested grouped conditions still require a separately audited design, and the threshold result is not yet a queue condition.
- AI is currently a local preview/trace surface only; endpoint/provider credential configuration and user-approved request execution remain intentionally unimplemented until the external-permission boundary is explicitly reopened.
- Process inspection is intentionally metadata-only and may report `Activity state uncertain` when OS permissions or platform support prevent working-directory correlation.
- External handoff currently targets macOS and fixed application names; live desktop launch proof and configurable/non-macOS tool mappings remain open.
- Action audit identifiers and transition event identifiers use process-local sequences to avoid same-second collisions; cross-process concurrency is not yet a supported store mode.
- Pre-CR is configured with a zero-threshold no-instrumentation adapter; functional tests, typechecks, and builds are the evidence gates until a real coverage instrumenter is added.

## Recent Progress

| Date       | Change                                                                                                      | Evidence                                        |
| ---------- | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| 2026-07-25 | Implemented local-first Tauri/Rust/React slice and committed it.                                            | `1817527`                                       |
| 2026-07-25 | Fixed inherited `GIT_INDEX_FILE` contamination in native Git calls.                                         | Hook-mode `pnpm test` passed                    |
| 2026-07-25 | Fixed the Pre-CR adapter's strict ESLint global reference.                                                  | `ef6ea99`; lint and hook passed                 |
| 2026-07-25 | Added PRD behavior inventory and verification workbook.                                                     | `533c2f5`; artifact inspect passed              |
| 2026-07-25 | Added truthful local navigation, settings/activity surfaces, and keyboard UX.                               | `885fe1e`; Pre-CR passed                        |
| 2026-07-25 | Refreshed the behavior workbook for the UX slice and accepted delivery decisions.                           | `faf0d99`; 34 rows, zero formula errors         |
| 2026-07-25 | Replaced JSON persistence with versioned SQLite and added legacy import coverage.                           | `d332573`; 6 Rust tests passed                  |
| 2026-07-25 | Refreshed behavior coverage for SQLite and updated the next action gate.                                    | `f01c627`; artifact inspect passed              |
| 2026-07-25 | Verified the full Tauri app/DMG build and hook-mode test path.                                              | Release bundle; 6 Rust tests passed             |
| 2026-07-25 | Added read-only action preflight, durable audit records, schema v2 migration, and collision-safe event IDs. | `b13de08`; 9 Rust tests and commit gates passed |
| 2026-07-25 | Refreshed the canonical behavior workbook and plans for the read-only action boundary.                      | `5a0b120`; 35 rows, zero formula errors         |
| 2026-07-25 | Added durable products/groups, lifecycle confirmation, root policies, retention settings, and submodule summaries. | `42e8b54`; 11 Rust tests, web gates, and Pre-CR passed |
| 2026-07-25 | Aligned the companion CLI with portfolio filters, scoped refresh, desktop opening, JSON, and blocked clone audit. | `853faa6`; 11 Rust tests, CLI JSON smoke, and Pre-CR passed |
| 2026-07-25 | Added provider-neutral GitHub identity/repository snapshots and the read-only Remote catalog surface. | `ed8d05f`; 14 Rust tests, web gates, and Pre-CR passed |
| 2026-07-26 | Added provider-neutral agent/process/manifest evidence and activity-aware integration blocking. | `6fde71e`; 16 Rust tests, web gates, and Pre-CR passed |
| 2026-07-26 | Added exact-workspace external handoff buttons with unsupported-tool and target-lookup tests. | `8be4771`; 17 Rust tests, web gates, and Pre-CR passed |
| 2026-07-26 | Added deterministic read-only PR/release preparation evidence and split the renderer drawers by responsibility. | `75889f3`; 19 Rust tests, web gates, and Pre-CR passed |
| 2026-07-26 | Added locally persisted deterministic release rules, first-release handling, exact threshold wording, and rule trace evidence. | `3ad1451`; 19 Rust tests, web gates, and Pre-CR passed |
| 2026-07-26 | Added repository AI permissions and a local committed-evidence payload preview with source references and no-request traceability. | `6ed50bb`; 20 Rust tests, web gates, and Pre-CR passed |
