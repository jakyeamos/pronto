# Pronto implementation review and plan

## Review summary

The PRD is a strong product definition for a local-first portfolio command center. Its most important design constraints are consistent throughout the document:

- portfolio state is presented before repository detail;
- facts, freshness, and evidence are visible before derived labels;
- local scanning, fetching, provider refresh, and actions are separate operations;
- expected state is not the same as resolved state;
- agent workspaces are operational objects, not just branches;
- modifying actions require a fresh preflight and a local audit record;
- destructive and history-rewriting Git operations are prohibited;
- AI is disabled by default and cannot make operational decisions.

The scope is also deliberately broad: 117 functional requirements, 33 acceptance criteria, a cross-platform desktop shell, a local database, GitHub identity management, process inspection, a CLI, pull requests, release preparation, and optional AI. This repository is currently empty, so implementing all of V1 in one unvalidated pass would create a large amount of unproven surface area.

The first build therefore establishes a runnable vertical slice around the most differentiating local behavior: discovery, Git evidence, explainable portfolio conditions, expected-state handling, and safe read-only handoff. Provider actions and release automation are represented as explicit unavailable states until their permissions, persistence, and preflight contracts can be implemented and tested.

## Architecture decision

The PRD recommends Tauri with a React/TypeScript interface and Rust backend. Rust is now installed locally, so this implementation uses that architecture:

- Tauri/Rust core: filesystem access, structured Git subprocesses, persistence, discovery, scanning, and command handlers.
- Tauri invoke boundary: a narrow typed API; the renderer does not receive Node access.
- React renderer: portfolio command center and evidence surfaces.
- Shared JSON contracts: Rust serializes the same snapshot shape consumed by the renderer and CLI.
- Companion CLI: invokes the same Rust core and reads the same local registry.

The provider, release, process, and AI boundaries remain explicit deferred states while the local evidence slice is validated.

## Accepted product decisions

The next slices follow [`product-decisions.md`](./product-decisions.md): local desktop UX comes first; SQLite precedes provider data; GitHub starts read-only; local actions are non-destructive; macOS is the first live shell target; AI remains disabled; and products/groups are manually configured before any inference is considered.

## Phase plan

### Phase 1 — Local evidence foundation (this implementation)

1. Scaffold a strict TypeScript React renderer and Tauri/Rust application with reproducible scripts.
2. Add versioned local persistence under the platform user-data directory; the initial JSON store is migrated to SQLite before provider data is introduced.
3. Register one or more repository roots and recursively discover Git repositories.
4. Canonicalize paths, deduplicate repositories, and attach linked worktrees to their parent repository.
5. Scan local Git state with structured arguments only:
   - current branch and upstream;
   - ahead/behind state;
   - dirty/clean state with aggregate `+N / −N` line totals;
   - untracked text totals where safely measurable;
   - interrupted merge/rebase/cherry-pick/revert/bisect operations;
   - remote URL and local freshness boundary;
   - branch role, target, confidence, and local integration state.
6. Derive grouped conditions with deterministic priorities and evidence.
7. Persist transition-only events and expected-condition fingerprints.
8. Persist explicit read-only action preflights and audit outcomes, including rejected unsupported actions and exact local target IDs; keep modifying Git/provider actions absent.
9. Build the command center, repository detail drawer, evidence panel, onboarding, and unavailable provider/release states.
10. Expose `pronto status` and JSON output through the same core modules.

### Phase 2 — Provider and durable state expansion

- Polish local navigation, freshness, settings, keyboard access, and empty states before expanding the data boundary.
- Continue the versioned SQLite contract while preserving the domain contracts and CLI/renderer snapshot shapes; the initial JSON store has already been imported non-destructively.
- Add GitHub identities, explicit per-repository mapping, refresh freshness, remote-only catalog, and clone preflight.
- Add process/terminal metadata adapters and structured agent manifests.
- Extend the read-only action contract into authorized bounded workspace actions only after permission, target, failure, and review semantics are explicit; destructive/history-rewriting Git operations remain prohibited.

### Phase 3 — Pull requests and releases

- Add provider-backed pull-request snapshots and permission-aware merge eligibility.
- Add deterministic release rules, candidate-version traceability, and release-note generation.
- Add isolated release worktrees, exact-diff review, validation gates, resumable recipe state, and draft-release handoff.
- Add products, groups, and coordinated release configuration after single-repository release behavior is stable.

## Verification strategy

- Unit tests cover Git parsing, dirty-line aggregation, operation detection, condition ordering, expected fingerprints, and prohibited-operation guards.
- Typecheck and production build validate renderer/main/preload boundaries.
- CLI smoke checks validate human-readable and JSON status output against a temporary fixture repository.
- Agent-facing CLI projections preserve the full `pronto status --json` snapshot while adding focused, versioned read-only envelopes:
  - `pronto next [<repository>] [--limit <n>] --json` for bounded daily orientation, ranked attention, and safe inspection follow-ups;
  - `pronto fold preview [<repository>] [--target <branch>] --json` for advisory branch/worktree candidates and preservation reasons before the reviewed fold workflow;
  - `pronto summary --json` for fleet counts and repository summaries;
  - `pronto repo <repository> --json` for one repository plus its product/group memberships;
  - `pronto quality [<repository>] --json` for fleet or repository quality evidence;
  - `pronto attention --json` for conditions, dirty workspaces, synchronization gaps, and quality gaps;
  - `pronto activity [<repository>] --limit <n> --json` for bounded events and action audits;
  - `pronto prepare <repository> --json` for pull-request, release, and recipe evidence;
  - `pronto release preview <repository> --json` for the release-specific evidence and review boundary.
- These projections are derived from the same SQLite-backed snapshot consumed by the renderer. They are local/private outputs and are not public-export contracts.
- The agent operating route is provider-neutral: the global home contract routes portfolio, workspace, branch, quality, and release triage to the `$pronto` skill; this repository's `.agents/context/` packet supplies the live CLI contract.
- `$pronto` is an evidence and preflight surface, not an autonomous Git operator. `fold preview` supplies persisted branch/worktree candidates, while the reviewed `fold-feature-branches` workflow owns live ref classification, integration, and pruning authorization.
- The behavior inventory in `docs/pronto-behavior-spec.xlsx` tracks each implemented feature, source function, test method, evidence, and remaining open question.
- Tauri interaction verification is performed when the local desktop runtime can launch; provider and release rows remain explicitly blocked rather than being marked verified from static code.

## Complexity and safety gate

Every change must keep Git execution structured (`spawnFile` with argument arrays), keep Node access behind preload, avoid exposing filenames or uncommitted diff content in the UI, and refuse prohibited operations. No feature may silently turn a missing provider permission or stale remote comparison into a passing state. The first implementation favors small domain modules over a large state-management abstraction and keeps external actions out of the initial slice.

## PRD coverage at the end of this slice

Implemented or substantially exercised: FR-001–004, FR-006, FR-016–022, FR-027–039, FR-040–043, FR-048–055 (local evidence subset), FR-064–070 (read-only action subset), FR-102/104/106/107 (privacy boundary), FR-109–112, and FR-113/115/116 (read-only CLI subset).

Explicitly deferred or environment-blocked: GitHub accounts and provider refresh, remote-only import/clone, pull requests, merge actions, release preparation, process inspection, coordinated products/releases, credential-store integration, and packaging/signing.
