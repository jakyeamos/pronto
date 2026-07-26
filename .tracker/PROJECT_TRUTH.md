# Project Truth

## Current State

- The first local-first Pronto desktop vertical slice is implemented on `main`.
- Tauri + React/TypeScript provides the desktop shell and portfolio console.
- Native Rust discovers repositories and linked worktrees, scans structured Git state, persists snapshots, transition events, and action audits, and exposes Tauri plus read-only CLI commands.
- The repository-selection Git environment is sanitized before native Git calls so hook variables cannot redirect scans or fixtures.
- A canonical PRD behavior inventory, coverage boundary sheet, delivery plan, and verification matrix are committed at `docs/pronto-behavior-spec.xlsx`; the workbook now tracks 34 behaviors through implementation snapshot `d332573`.
- The local desktop surface now has truthful navigation boundaries, Activity and Settings views, Cmd/Ctrl-K search focus, Escape drawer dismissal, freshness copy, and distinct filtered-empty states.
- Local durable state now uses a versioned SQLite database with non-destructive import from the legacy JSON registry; renderer and CLI snapshot contracts are unchanged.
- Safe local refresh/inspect preflights now use an explicit allowlist, exact root/repository target IDs, blocked-action records, and durable SQLite audit history; no destructive Git or provider-write action is exposed.

## Current Position

- Branch: `main`
- Implementation and documentation commits: `1817527`, `ef6ea99`, `73c28e5`, `533c2f5`, `885fe1e`, `faf0d99`, `d332573`, `f01c627`, `b13de08`
- Verification for `b13de08`: 9 Rust tests, hook-mode `pnpm test`, cargo fmt, offline cargo check, typecheck, lint, Prettier, renderer production build, `git diff --check`, and commit-time source-delta Pre-CR all passed. The full Tauri app/DMG build and refreshed workbook remain final handoff gates.
- Next deliverable: read-only provider context after the local action boundary is reviewed; modifying Git actions remain deferred.

## Blockers

- GitHub identity/provider refresh, remote catalog and clone, modifying action execution, pull requests, releases, process/terminal integration, products/groups, and opt-in AI remain explicit deferred boundaries from the implementation plan.

## Risks

- Folder-picker and full Tauri UI interaction still need live desktop verification.
- The SQLite schema is now at version 2 with an explicit v1 migration for action audits and rejects unknown versions; future schema migrations still need explicit migration steps rather than silent compatibility assumptions.
- Action audit identifiers and transition event identifiers use process-local sequences to avoid same-second collisions; cross-process concurrency is not yet a supported store mode.
- Pre-CR is configured with a zero-threshold no-instrumentation adapter; functional tests, typechecks, and builds are the evidence gates until a real coverage instrumenter is added.

## Recent Progress

| Date       | Change                                                                            | Evidence                                |
| ---------- | --------------------------------------------------------------------------------- | --------------------------------------- |
| 2026-07-25 | Implemented local-first Tauri/Rust/React slice and committed it.                  | `1817527`                               |
| 2026-07-25 | Fixed inherited `GIT_INDEX_FILE` contamination in native Git calls.               | Hook-mode `pnpm test` passed            |
| 2026-07-25 | Fixed the Pre-CR adapter's strict ESLint global reference.                        | `ef6ea99`; lint and hook passed         |
| 2026-07-25 | Added PRD behavior inventory and verification workbook.                           | `533c2f5`; artifact inspect passed      |
| 2026-07-25 | Added truthful local navigation, settings/activity surfaces, and keyboard UX.     | `885fe1e`; Pre-CR passed                |
| 2026-07-25 | Refreshed the behavior workbook for the UX slice and accepted delivery decisions. | `faf0d99`; 34 rows, zero formula errors |
| 2026-07-25 | Replaced JSON persistence with versioned SQLite and added legacy import coverage. | `d332573`; 6 Rust tests passed          |
| 2026-07-25 | Refreshed behavior coverage for SQLite and updated the next action gate.          | `f01c627`; artifact inspect passed      |
| 2026-07-25 | Verified the full Tauri app/DMG build and hook-mode test path.                    | Release bundle; 6 Rust tests passed     |
| 2026-07-25 | Added read-only action preflight, durable audit records, schema v2 migration, and collision-safe event IDs. | `b13de08`; 9 Rust tests and commit gates passed |
