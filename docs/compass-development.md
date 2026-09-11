# Compass development foundation

Pronto consumes `compass-family/v1` from the installed canonical Python Compass
helper. It keeps existing root summary fields while projecting subsystem draft
status, invalid children, handoffs, provenance and path coverage. The importer
no longer independently validates or scores Compass contracts. `Ready` means a
valid root contract; declared maturity never proves current implementation.
Old cached summaries have an unavailable family projection until refreshed.

## Focused commands

```sh
node bin/pronto.mjs compass family <workspace> --json
node bin/pronto.mjs compass change-context <workspace> --path <path> --base <revision> --json
node bin/pronto.mjs compass change-context <workspace> --base <revision> --prepared <packet.json> --completion --json
node bin/pronto.mjs compass assess <workspace> --proposal <proposal.json> --json
```

These commands are live and require an explicit workspace. They are read-only
unless `change-context --packet-output` explicitly requests a task artifact. They do
not read the portfolio cache, refresh Pronto, execute repository commands, or
create remediation work. Missing Python, a missing helper, unsupported schema,
oversized output or a timeout is explicitly unavailable. `PRONTO_COMPASS_HELPER`
and `PRONTO_PYTHON` are explicit operator overrides for candidate verification;
repository metadata cannot select an executable. The bridge allows 30 seconds
and 1 MiB of output. Change-context packets are limited by the producer to
32 KiB. Broad reconciled scopes may use the explicit complete-artifact route below.

`compass change-context WORKSPACE --base ORIGINAL_SHA --path PATH --packet-output
.quality-runner/compass/reconciled.json --json` writes a complete eligible entry
packet of at most 1 MiB. The producer refuses existing files, unsafe paths and
ineligible packets. Its `compass-packet-artifact/v1` descriptor is not a prepared
packet: inspect the complete file and preserve the previous packet before explicit
adoption. Gate validation always uses the complete packet and actual diff. An
oversized result returns `compass-gate-summary/v1` with all blockers, identities,
proof counts and full-result digest; if that summary exceeds 32 KiB, it fails
explicitly. Neither descriptor nor summary can serve as an intent baseline.
Gate failure retains a nonzero exit. Older consumers report these schemas as
unsupported. This CLI-only addition has no new UI or execution authority.

At task entry, retain the packet in the ignored task evidence directory specified below. At completion compare
it against the actual diff, preserving scope expansion, renames, removals and
concurrent intent edits. The Python helper owns evidence freshness and scoped
eligibility. Quality Runner remains the independent release gate.
The pilot `compass-development` Quality Runner gate runs `compass gate . --json`.
Save the entry packet at `.quality-runner/compass/prepared.json` in the task-owned
worktree; preserve its original base. Missing packets, actual scope expansion,
changed affected decisions/relationships and stale required proof fail the gate.
Reviewed reconciliation creates a new packet; do not silently rebaseline.
This enrolls only the two named pilots.

## Canonical references and authority

`.project-compass/development.json` links exact paths, outcome IDs, canonical
specification references, optional behavior IDs, constraints and proof to stable
subsystem IDs. The approved foundation plan establishes the new pilot subsystem;
it does not ratify the three historical drafts or resolve their two handoffs.
Drafts, missing inventory, broken references and ambiguous ownership remain
visible. Existing behavior assurance and Telescope documents remain canonical.

The Compass detail provides expandable subsystem navigation, observed revision
and status, source opening, handoffs and measured path coverage. It uses the
same family payload as the CLI. Subsystem decisions join the existing grouped
Project Compass remediation item; they do not introduce another queue.

Modernization proposals carry the existing remediation ID, concrete problem,
expected benefit, applicability, preserved outcomes/constraints, evidence and
rollback approach. `compass assess` returns a disposition and creates no work.
Model releases can prompt reassessment but cannot justify a rewrite by themselves.

## Bounded orientation

`route --json` defaults to `pronto-agent-route/v2`: the complete doctor gate,
three concise next actions, repository identity and maturity summaries, with
explicit drill-down commands. `route --full --json` preserves the legacy v1
payload for existing integrations. Large doctor evidence returns a bounded
blocked envelope that requires narrowing the repository scope. No safety check
is silently removed to fit the packet.

## Acceptance

The shared producer fixture includes a valid legacy root, a draft child and an
invalid child. Rust verifies compatibility, workspace/schema mismatch and draft
visibility. Python acceptance checks shared consumers, missing and renamed paths,
selective evidence invalidation, scope expansion, revision reconciliation,
unrelated legacy isolation and inapplicable modernization. Renderer tests cover
subsystem disclosure, handoffs and source state. Native quality checks and the
exact-current Quality Runner release check remain independently required.

An optional `.project-compass/modernization.json` proposal is assessed during
Compass refresh and joined into existing foundation readiness. The detail shows
its disposition and existing remediation ID. An assessment never authorizes or
creates implementation work. Telescope drill-down uses the same exact workspace
and selected source paths; its observed structure does not replace intended truth.

## Validation scope and gate repairs

The Compass pilot accepts only the new foundation subsystem under the approved
plan. The existing three draft subsystems and two unresolved handoffs retain
their original authority. Other implementation paths remain visibly unmapped.

The required full test suite exposed two clock-dependent branch-lifecycle
fixtures. Production still uses the real clock; tests inject their historical
observation date. The required dead-code gate identified private renderer
exports without consumers. Local helpers remain; unused lookup functions and
exports were removed. These changes are separate from the Compass feature:
`src-tauri/src/branch_lifecycle.rs`, `src-tauri/src/branch_lifecycle_tests.rs`,
`src/renderer/src/components/ConsolePrimitives.tsx`,
`src/renderer/src/components/QualityComponents.test-support.ts`,
`src/renderer/src/components/telescopeSceneModel.ts`, and
`src/renderer/src/components/telescopeSceneBuckets.ts`.

Change-surface review: no public interface, command, product intent, persistence,
provider contract or renderer behavior changes from these repairs. Their exact
paths are excluded from Compass foundation coverage with this reason; the
repository's full tests, typecheck, dead-code and release gates still apply.
This is an explicit pilot enrollment exclusion, not a claim that the paths are
covered or permission for future intent changes.

`node scripts/verify-compass.mjs` is the reviewed bounded proof command: it
exercises Rust producer consumption and route projection, then the actual
renderer family component. Quality Runner independently runs the complete
repository checks.

The formatter also normalized these existing paths without semantic changes:

- `src/renderer/src/components/AppRouteSurface.tsx`
- `src/renderer/src/components/PortfolioNavigationSurfaces.test.tsx`
- `src/renderer/src/components/TelescopeGraphElements.tsx`
- `src/renderer/src/components/TelescopeSurface.test.tsx`
- `src/renderer/src/components/TelescopeWorkspaceView.tsx`
- `src/renderer/src/components/useTelescopeWorkspaceModel.ts`
- `src/renderer/src/styles.css`
- `src/renderer/src/types/telescope.ts`

The gate is a required native `qualityCommands` entry in `.pre-cr.json` and is
registered in Quality Runner's repository gate inventory. This does not claim
QR prevention certification: certification requires separate CI evidence and
isolated-snapshot support for task-local packets. Run the native completion
gate and the independent exact-current QR release check; neither substitutes
for the other.

The dependency audit repair resolves the existing nanoid 3.3.18 override in the lockfile. It is an explicitly excluded validation repair, checked by the full dependency audit and build.

## Intent documentation and continuous preservation

The canonical helper supplies `bootstrap` (`compass-bootstrap/v1`) alongside the
family. Pronto displays subsystem linkage gaps and independent behavior inventory
status. Missing bootstrap support in older snapshots is unknown, not mature.
These assessments join purpose, parent outcomes, exclusions, canonical behavior
specifications, handoffs, and behavior-specific proof. They do not replace Quality
Runner documentation evidence or certify semantic correctness. The attention
surface includes incomplete subsystem intent documentation.

Start with `pronto compass family WORKSPACE --summary --json` for bounded counts
and subsystem gap summaries. Full family remains explicit drill-down. The summary
has its own `compass-family-summary/v1` schema, so count fields cannot be mistaken
for complete path arrays.

For multi-task efforts, preserve the original base and previous packet in the
existing task/remediation handoff. `pronto compass change-context WORKSPACE
--base ORIGINAL_SHA --prepared PREVIOUS_PACKET --continue --path NEXT_PATH --json`
adds scope without dropping prior subsystems or accepted-intent revisions.
Completion recomputes the cumulative actual diff and proof. Selective enforcement
is owned by the producer's `compass-preservation/v1` enrollment; Pronto never
ratifies inferred intent or upgrades legacy contracts automatically.

Continuation requires an eligible predecessor and retains original base, cumulative paths and active scoped continuity commitments. A failed assessment is not a replacement intent baseline. Reconcile semantic contradictions between commitments and contract text before development; the helper does not infer prose equivalence. Legacy packets lacking commitment/behavior revisions require explicit preparation.

### Reviewed modernization alternatives

Compass proposal/v2 compares delete, simplify and retain choices against affected
outcomes. The Python producer owns review completeness and evidence freshness.
Pronto preserves its additive assessment/v1 fields: `proposal_complete`, `review`,
and `benefit_observation`. A legacy missing field is unknown. A current declared
review is not authenticated approval, execution authority, preservation proof, or
measured benefit. Retention requires no remediation ticket. Benefit follow-up
compares the original promise with separately reviewed observations; never infer
realized benefit from a passing test or a complete proposal. Use `$project-compass` → `references/assessment.md` in the installed Compass
skill directory for proposal/v2 and review/observation fields.
