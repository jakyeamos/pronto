# Pre-CR Suite route plan

Status: steps 1–3 describe Pre-CR as a standalone IDE product. PCR-0 through
PCR-4 are closed: continuity, the useful command path, a native changed-line
repair, and the standalone material package are proven. Its separate
enforcement role in the combined quality story is owned by the
[AI Code Quality Stack plan](../ai-code-quality-stack/route-plan.md).

The [canonical target](../ideal-demo-targets.md#pre-cr-suite) owns this durable
promise and proof gate.

## 1. Ideal target

**North star:** a developer returns to an interrupted branch, asks **Where Was
I?**, restores the exact files and cursor location, uses Pre-CR's editor commands
to understand the change, sees changed-line coverage and check state, and
finishes with a passing pre-review receipt.

**Non-negotiable:** this demo must stand on Pre-CR's own IDE capabilities. It
must not require Anti-Slop or Quality Runner to explain why Pre-CR is useful,
and it must not imply a merge decision.

## 2. Concept materials

The standalone target frames are implemented in `concept/index.html`. They
remain **concept** until the installed extension passes the proof gate.

| Frame          | Visual                                                             | On-screen line                               | Intended evidence moment               |
| -------------- | ------------------------------------------------------------------ | -------------------------------------------- | -------------------------------------- |
| 1. Return cold | A real branch with several unfinished files                        | “Come back without reconstructing the work.” | The interruption cost is recognizable  |
| 2. Recall      | **Where Was I?** shows the saved summary and relevant next actions | “The branch remembers.”                      | Context is useful, not decorative      |
| 3. Restore     | Saved files and cursor positions reopen                            | “Back to the exact working state.”           | Restoration accuracy is visible        |
| 4. Act         | Quick Actions exposes coverage, setup, and Pre-CR Check            | “The next useful action is already here.”    | The IDE integration reduces navigation |
| 5. Inspect     | Coverage overlays identify the next concrete gap                   | “See what blocks readiness in the code.”     | The product makes check state legible  |
| 6. Prove       | A focused test closes the changed-line gap and Pre-CR Check passes | “Ready before review.”                       | Pre-CR closes its own workflow         |

**Preview concept.** A VS Code-shaped continuity card leads with **Where Was
I?**, followed by Return → Recall → Act → Prove.

**Narrative spine.** Interrupted work → branch memory → restored editor state →
focused actions → visible readiness gap → focused repair → passing editor
receipt.

## 3. Build-gap specification

Reviewed baseline: the VS Code extension registers 37 Pre-CR commands across
coverage, checklist, security, docs, review, context, and debug categories. It
also exposes scoped Quick Actions, recent actions, **Where Was I?**, Save
Snapshot, Restore Snapshot, branch-switch restoration, a dashboard, and editor
views. PCR-0 promoted context continuity as a **source-verified candidate**:
snapshots now persist in VS Code workspace storage, reload after a language
server restart, validate workspace paths, clamp stale cursors, and report
partial restoration. The supported standalone subset is Save Snapshot, Where
Was I?, Restore Snapshot, Quick Actions, Refresh Coverage, Run Pre-CR Check,
and Fix Setup. Checklist, docs, review, and debug remain experimental.
Source tests and builds are implementation evidence, not installed behavior
proof.

Project disposition: `targeted_gap_closure` — retain and prove Pre-CR as a
standalone developer-workflow product. Promote the smallest high-value IDE
subset before expanding the supported surface.

Gap classes: product — PCR-0 (closed); demo_integration — PCR-1 and PCR-2 (closed);
evidence — PCR-3 (closed); packaging — PCR-4 (closed).

| ID    | Gap to close                     | Observable acceptance condition                                                                                                                                                                                                                                                                                                                  | Owner                 | Required proof                                                                                         |
| ----- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------- | ------------------------------------------------------------------------------------------------------ |
| PCR-0 | Define the dependable IDE subset | **Closed 2026-08-12.** Context continuity persists across server restarts; empty, stale-file, stale-cursor, outside-workspace, partial-restore, and branch-switch outcomes are explicit. Checklist, docs, review, and debug remain outside the promoted subset                                                                                   | Pre-CR owner          | `docs/IDE_WORKFLOW.md`; core and extension continuity tests; typecheck, lint, and production builds    |
| PCR-1 | Prove context continuity         | **Closed 2026-08-12.** An installed VSIX captured three open text and text-diff tabs, survived an extension-host restart, restored all three files and the active cursor at line 11, and visibly degraded to two restored plus one skipped after a fixture file was removed                                                                      | Pre-CR owner          | `evidence/pcr-1-behavior-receipt.json` and paired installed VS Code captures                           |
| PCR-2 | Prove the useful command flow    | **Closed 2026-08-12.** From restored context, the installed Quick Actions workspace scope exposed Run Pre-CR Check, Refresh Coverage, and Fix Setup; each command produced a visible state without opening VS Code's command palette. The main check surfaced the fixture's raw-TypeScript runtime error, which PCR-3 owns rather than hiding it | Pre-CR owner          | `evidence/pcr-2-command-state-receipt.json` and four installed VS Code captures                        |
| PCR-3 | Close one native readiness gap   | **Closed 2026-08-12.** The installed extension marked one uncovered changed line in `src/session.js`, the focused behavior test executed it, the diagnostic cleared, and the same Pre-CR Check reached 100% changed-line coverage                                                                                                                | Pre-CR owner          | `evidence/pcr-3-readiness-receipt.json`, before/after fixture files, and paired installed captures     |
| PCR-4 | Package the standalone story     | **Closed 2026-08-12.** The IDE concept, 16:9 preview, command legend, and claim ledger remain legible as a standalone story; external Chrome rendered the 1600 × 900 target without overflow or console errors, and the 800 × 450 downscale preserved the core message                                                                           | Showcase/design owner | `concept/index.html`, `preview.html`, `preview-16x9.png`, `claim-ledger.json`, and `surface-review.md` |

**Build order:** PCR-0 → PCR-1/PCR-2 → PCR-3 → PCR-4.

**Current closure:** PCR-4. The crop-safe 16:9 preview and evidence-bound claim
ledger package Return → Recall → Act → Prove as a standalone IDE story. No
required material gap remains. Owner copy review, marketplace distribution
proof, and video are optional follow-ons rather than publication prerequisites.
The current headless CLI reports the PCR-3 failing state as `ok: false` with
`gateDecision: warn` and exit code 0, so the package claims detection and repair
but not CLI blocking.
