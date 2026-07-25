# Project Truth

## Current State

- The first local-first Pronto desktop vertical slice is implemented on `main`.
- Tauri + React/TypeScript provides the desktop shell and portfolio console.
- Native Rust discovers repositories and linked worktrees, scans structured Git state, persists snapshots and transition events, and exposes Tauri plus read-only CLI commands.
- The repository-selection Git environment is sanitized before native Git calls so hook variables cannot redirect scans or fixtures.

## Current Position

- Branch: `main`
- Implementation commits: `1817527`, `ef6ea99`
- Verification at commit: `GIT_INDEX_FILE=.git/index pnpm test`; five Rust regression tests passed, Vitest had no test files, and the commit-time Pre-CR gate passed.
- Next deliverable: canonical behavior-spec workbook and final release/build verification.

## Blockers

- GitHub identity/provider refresh, remote catalog and clone, action execution/audit, pull requests, releases, process/terminal integration, products/groups, and opt-in AI remain explicit deferred boundaries from the implementation plan.

## Risks

- Folder-picker and full Tauri UI interaction still need live desktop verification.
- Pre-CR is configured with a zero-threshold no-instrumentation adapter; functional tests, typechecks, and builds are the evidence gates until a real coverage instrumenter is added.

## Recent Progress

| Date       | Change                                                              | Evidence                        |
| ---------- | ------------------------------------------------------------------- | ------------------------------- |
| 2026-07-25 | Implemented local-first Tauri/Rust/React slice and committed it.    | `1817527`                       |
| 2026-07-25 | Fixed inherited `GIT_INDEX_FILE` contamination in native Git calls. | Hook-mode `pnpm test` passed    |
| 2026-07-25 | Fixed the Pre-CR adapter's strict ESLint global reference.          | `ef6ea99`; lint and hook passed |
