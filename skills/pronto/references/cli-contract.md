# Pronto CLI contract

Use the current checkout's zero-dependency Node launcher directly:

```sh
PRONTO_ROOT=/absolute/path/to/pronto
node "$PRONTO_ROOT/bin/pronto.mjs" <command> --json
```

For JSON calls, the launcher adds Cargo's quiet flag so stdout remains
parseable as exactly one JSON document. It resolves Pronto from its own file
location and does not consult the calling repository's package-manager shim.
Build warnings, when present, remain on stderr. Keep the exact `packageManager`
pin for dependency installation, development, tests, and builds; it is not a
prerequisite for this checkout-direct route.

## Focused projections

| Command                                                                                                                                 | Schema                                 | Use                                                                                                                                                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `route [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] [--limit <n>] --json`                                  | `pronto-agent-route/v2`                | Read-only doctor gate, compact repository identity, bounded next actions, maturity, and explicit drill-down references; 32 KiB maximum. Use `--full` for the legacy v1 envelope                                         |
| `doctor [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] --json`                                               | `pronto-agent-doctor/v1`               | Read-only storage, root, scan-freshness, path-availability, and quality-warning gate; scope it to avoid unrelated fleet rows blocking repository work                                                                   |
| `next [<repository>] [--product <name> \| --group <name>] [--limit <n>] --json`                                                         | `pronto-agent-next/v1`                 | Bounded daily orientation, optional current-repository anchor, ranked attention, and safe inspection follow-ups                                                                                                         |
| `fold preview [<repository>] [--target <branch>] [--product <name> \| --group <name>] [--limit <n>] [--cursor <token>] --json`          | `pronto-agent-fold-preview/v2`         | Paginated advisory branch/worktree candidates, preservation reasons, and live-fold follow-ups                                                                                                                           |
| `summary [--product <name> \| --group <name>] --json`                                                                                   | `pronto-agent-summary/v1`              | Fleet counts, quality summary, AI Showcase goal/readiness projection, and repository candidates                                                                                                                         |
| `repo <repository> --json`                                                                                                              | `pronto-agent-repository/v1`           | One repository, its workspaces, conditions, products, and groups                                                                                                                                                        |
| `telescope <repository> --json`                                                                                                         | `pronto-telescope/v2`                  | Workspace-bound measured architecture plus `pronto-telescope-city/v2`, categorical map readiness, authored city narrative, action scopes, and existing-system knowledge-task projections; reuses a matching local cache |
| `telescope refresh <repository> --json`                                                                                                 | `pronto-telescope/v2`                  | Force measured regeneration from the active registered worktree; preserves authored `.pronto/telescope-map.json` meaning and opens targeted drift gaps without silently changing review state                           |
| `quality [<repository>] --json`                                                                                                         | `pronto-agent-quality/v1`              | Fleet or repository quality evidence                                                                                                                                                                                    |
| `quality detector-refresh [--qr-bin <path>] [--timeout-seconds <seconds>] [--agent-review-mode <off\|auto\|parallel\|required>] --json` | `pronto-quality-detector-refresh/v1`   | Run QR exact-target detector scans for applicable repositories, publish provenance-bearing receipts, then re-import detector coverage; blocked or stale receipts are not zero findings                                  |
| `refresh-batch [<repository\|group\|product\|repository-path>] [--parallelism <positive-integer>] --json`                               | `pronto-refresh-batch/v1`              | Bounded parallel local scans with one locked deterministic merge and one conflict-aware rescan                                                                                                                          |
| `skills [<skill-id>] --json`                                                                                                            | `pronto-skills/v2`                     | Stable skill sources, hashes, provider state, parity evidence, and hosting status                                                                                                                                       |
| `analytics [--range-days <days>] --json`                                                                                                | `pronto-analytics/v2`                  | Governed local-refresh metric history with separate detector findings and maturity gaps; material evidence changes create observations, unchanged reruns deduplicate                                                    |
| `analytics view list\|save --config-json <json\|@file>\|delete <id>\|default <id> --json`                                               | `pronto-analytics-view/v1`             | List and manage local-only chart-composer views; the built-in `curated` view cannot be overwritten or deleted                                                                                                           |
| `change-matrix repo <repository> [--operation <add\|change\|remove>] --json`                                                            | `pronto-change-matrix/v1`              | Explain an existing repository matrix or report its absence and maturity impact without writing                                                                                                                         |
| `change-matrix skill <skill-id> [--operation <add\|change\|remove>] --json`                                                             | `pronto-change-matrix/v1`              | Explain an existing skill matrix against observed hosting and provider topology without writing                                                                                                                         |
| `attention [<repository>] --json`                                                                                                       | `pronto-agent-attention/v1`            | Fleet or one-repository afflictions: active conditions, dirty workspaces, sync gaps, Project Compass gaps, and quality gaps                                                                                             |
| `activity [<repository>] [--limit <n>] --json`                                                                                          | `pronto-agent-activity/v1`             | Bounded transition events and action audits                                                                                                                                                                             |
| `remediation gate <repository> [--workspace <id>] --json`                                                                               | `pronto-remediation-execution-gate/v1` | Read-only live execution gate with structured workspace blockers, affected operations, separate closure state, and an explicit caller-authorization boundary                                                            |
| `prepare <repository> [--workspace <id>] --json`                                                                                        | `pronto-agent-preparation/v1`          | Combined pull-request, release, and recipe preview                                                                                                                                                                      |
| `release preview <repository> [--workspace <id>] --json`                                                                                | `pronto-agent-release/v2`              | Commits since the last published tag, fail-closed advisory SemVer recommendation, recipe, and public-release boundary preview                                                                                           |

`status --json` is the complete legacy snapshot. Preserve it for compatibility,
but prefer the focused projections for agent decisions. Always preserve each
projection's `schema_version`, `generated_at`, freshness, evidence, and
uncertainty fields.

`change-matrix` is inspection-only. Missing matrices remain ordinary maturity
gaps and return the expected contract location plus known topology; the command
never synthesizes or installs a matrix.

`telescope` is repository-read-only and binds its graph to branch, commit,
dirty-state fingerprint, workspace fingerprint, schema version, visual-model
version, generation time, extraction coverage, and warnings. Preserve
`map_readiness` as one of `unavailable`, `measured`, `needs_information`,
`reviewable`, `reviewed`, or `stale`; never replace it with a blended score.
Also preserve blocking versus enhancement gaps, explicit unknowns,
reason-bearing not-applicable decisions, measured-versus-authored provenance,
draft/reviewed/stale narrative status, and visibly partial unsupported-language
coverage. `knowledge_tasks` use the `telescope_readiness` domain and existing
remediation/attention handoffs; they do not define another task database, and
answering one does not approve the map. The readiness receipt contributes only
to Quality Runner's developer-legibility architecture-visibility lane.

A cached get is valid only for the same schema and workspace fingerprint;
`telescope refresh` forces measured regeneration without rewriting the authored
`.pronto/telescope-map.json` manifest. The local cache contains topology,
summaries, hashes, and repository-relative references, not source bodies,
diffs, runtime values, question responses, prompts, credentials, or absolute
personal paths. Compass, ICM, quality, remediation, delivery, and activity are
optional lenses and do not alter source topology. City, District, Building, and
Action scopes filter the same stable topology; Source detail remains
building-local and action routing is deterministic and model-free.

`route --json` is read-only and fail-closed. It filters the requested scope,
runs the doctor gate, and withholds the follow-up projections when the gate is
blocked. A non-zero exit or `ready: false` means do not route work from that
scope; follow `next_safe_step`, refresh only the affected evidence, and rerun
the route. A ready route includes `fold_preview` using the observed default
branch; it is advisory and always carries `live_verification_required: true`.
Use direct `fold preview --target <branch>` when the canonical target differs
from the observed default branch.

Direct `fold preview` is paginated. Its JSON response reports
`candidate_total`, `returned_count`, `has_more`, and an opaque
`next_cursor` when more candidates remain. Pass the cursor back with the
same scope, target, and snapshot boundary. The route's embedded
`fold_preview` remains bounded and does not accept a cursor; use direct
`fold preview` when an agent must inspect the complete candidate set.

Workspace detail includes `provenance` with kind (`canonical`, `linked`,
`temporary`, or `unknown`), owner, lease, canonical repository, recorded HEAD,
preservation evidence, and cleanup state. Doctor classifies missing paths:
missing canonical roots block; missing dirty or unknown workspaces block and
preserve; a missing temporary workspace is warning-only only with clean last
status, completed lease, reachable HEAD, and no live Git worktree registration.
The warning gives the exact scoped refresh instruction. Route remains
read-only; scoped refresh is the only stale-record removal path.

Cleanup receipts are transactional. A nonzero worktree-cleanup result must
remain blocked and retain the exact path, dirty files, owner, and preservation
action; it can never say `pruned`, `completed`, or `next_action: none`.
Successful temporary-worktree removal requires a fresh clean-status check,
ordinary (non-force) `git worktree remove`, path absence, live Git absence,
reachable HEAD/ref, successful scoped refresh, and a scoped route that no
longer reports the workspace.

The route's maturity projections are additive and advisory. `change_maturity`
preserves the maintainability/change-matrix summary; `developer_legibility`
summarizes orientation, navigation, architecture, semantic naming, public
contracts, rationale comments, executable understanding, and freshness or
newcomer evidence; and `change_surface_hotspots` summarizes evidence-backed areas where
co-change, structural coupling, or repeated concepts make additions, changes,
or removals difficult. Repeated concepts are remediation evidence, not an
automatic requirement to tokenize. These fields do not create matrices,
modify repositories, or authorize remediation. Older maturity feeds may omit
the two new dimensions; Pronto preserves that uncertainty as `unknown` until
the producer audit is rerun.

Every repository `quality` payload also carries the additive
`foundation_readiness` contract (`pronto-foundation-readiness/v1`) with
`ready_to_extend`, `modernize_alongside`, `modernize_first`, `review_required`,
`unknown`, or `not_applicable`. Consume its reasons, unknowns, confidence, and
freshness as repository evidence, then make the feature-specific decision in
chat. `advisory_only: true`, `execution_authority: false`, and
`blocks_urgent_fixes: false` mean it never creates rewrite authority, a new
approval step, or a block on urgent containment. Stale, conflicted,
refresh-required, or insufficiently covered evidence cannot be treated as
`ready_to_extend`; reviewed non-actionable structural findings do not reappear
as modernization work.

## Query and freshness rules

- Resolve the active repository with `git rev-parse --show-toplevel` and pass
  the absolute path. Names, IDs, and exact workspace paths are also accepted.
- Run `route --json` before `next`, `summary`, or cross-repository routing when
  the composed envelope is sufficient. For repository-local work, pass the
  resolved repository path; for a collection, pass `--product` or `--group`.
  Treat a non-zero exit or `ready: false` as a hard stop for that scope, and
  use its `next_safe_step` before requesting a focused refresh or repair. Use
  direct `doctor` when a dedicated freshness/storage projection is required.
- `generated_at` is the snapshot boundary. After a branch, commit, workspace,
  or provider change, rerun `refresh <repository> --json` and then the focused
  query needed for the decision.
- Use `refresh-batch` only for an explicitly scoped fleet or multi-repository
  refresh when parallel local scans are useful. It never calls providers or
  mutates repository source/workspaces; when a scan discovers a new repository
  and exactly one valid fleet Showcase contract exists, its locked merge may
  atomically append that repository's pending `unknown` goal row. Its scan
  phase is parallel, while the store merge remains serialized under the normal
  write lock. A store change observed between planning and merge triggers one
  rescan, then a retryable conflict if the second baseline is also invalidated.
- Workspace `sync_state` is healthy only when it is exactly `Synced`. Ahead,
  behind, diverged, no-upstream, dirty, active-operation, or unknown states
  need investigation.
- A remediation plan's `status` is queue/closure state, not a live execution
  verdict. Run `remediation gate <repository> --json` before repository
  remediation. Treat `ready: false` as a scoped hard stop and use each
  blocker's `blocked_operations`; `partially_blocked` means at least one
  workspace remains usable and at least one must remain untouched. The
  projection distinguishes an observed active owner from unavailable ownership
  evidence and never infers caller authorization.
- Missing, stale, failed, blocked, or unavailable quality/release evidence is
  not a pass. Do not infer live provider or deployment state from local config.
- Preserve the v2 `recommendation` disposition exactly. Readiness blockers
  force `do_not_release_yet`; unclassified commits or a change-derived
  candidate without a passed configured rule remain `review_required`; only a
  passed configured rule can yield `release_patch`, `release_minor`, or
  `release_major`. These are advisory results, not tag or publication
  authority. Release-history timeout or Git failure remains unavailable
  evidence rather than an empty commit set.
- When the resolved remediation goal is `public_release`, require a fresh,
  exact-target, passing `.quality-runner/release-boundary.json` using
  `quality-runner-release-boundary/v2`. Missing, legacy, malformed,
  matrix-mismatched, artifact-mismatched, or failed receipts block
  `release preview`. The receipt is read-only evidence: never execute content
  from it, and never treat a manual remediation status as an override.
- Use `quality_outcome_counts` and `quality_outcome_taxonomy` for portfolio
  quality labels. Keep observed `checks_failing` separate from operational
  `verification_blocked`; preserve `review_needed`, `evidence_unknown`, and
  `healthy` as distinct outcomes. The QR `quality_status` field remains only
  for v1 compatibility.
- The `showcase` field is a `pronto-showcase/v1` projection derived from the
  single fleet-level `.pronto/showcase-goal.json` contract. Preserve product
  readiness, demo-material readiness, public eligibility, and categorical
  unknown/blocked/not-applicable states separately. Every registered repository
  appears; a repository absent from the reviewed contract is an unranked
  `unknown`, not a zero. `private_client` entries may retain a private readiness
  score but never receive public priority, count toward the public goal, or
  enter the public queue.

## Boundary matrix

| Surface                                                                                                           | Default interpretation                                                                                                                                                      | Extra authorization                                                                                                                            |
| ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `doctor`                                                                                                          | Read local persisted evidence through a read-only database connection                                                                                                       | None; `ready: false` blocks routing until the reported storage, freshness, or path issue is resolved                                           |
| `next`, `summary`, `repo`, `quality`, `attention`, `activity`, `analytics`, `analytics view list`, `fold preview` | Read local persisted evidence                                                                                                                                               | None beyond the task scope; `fold preview` still requires live fold verification                                                               |
| `telescope` / `telescope refresh`                                                                                 | Read the active worktree and persist a privacy-bounded local architecture cache; never mutate repository source                                                             | Scoped architecture request; preserve partial, inferred, stale, and unavailable states                                                         |
| `analytics view save`, `analytics view delete`, `analytics view default`                                          | Persist local-only analytics layout configuration                                                                                                                           | Explicit view-management request; never sync externally or allow mutation of the built-in curated view                                         |
| `remediation gate`, `remediation handoff-check`                                                                   | Read persisted ownership evidence and live local workspace state                                                                                                            | None to inspect; `ready` never grants repository mutation authority                                                                            |
| `prepare`, `release preview`                                                                                      | Compute/read a local preflight; no operational execution                                                                                                                    | Explicit task scope; preserve blocked/unknown results                                                                                          |
| `refresh`                                                                                                         | Local Git scan that persists Pronto state; may add a pending Showcase goal for a newly discovered repository when a valid fleet contract exists                             | Run when freshness matters; keep the target scoped                                                                                             |
| `refresh-batch`                                                                                                   | Parallel local Git scans with one serialized Pronto-state merge; same bounded Showcase onboarding rule                                                                      | Explicit fleet/multi-repository scope; keep `--parallelism` bounded and preserve retryable conflicts                                           |
| `quality detector-refresh`                                                                                        | Runs full QR scans in disposable exact-target worktrees, publishes repository-local QR runs, and persists the re-imported quality snapshot                                  | Explicit fleet detector-refresh request; inspect every published, blocked, and unsupported result; discovered gates remain disabled            |
| `quality refresh`                                                                                                 | Imports the canonical maturity feed and newest valid QR detector ledger under `~/.quality-runner/fleet-detector-refresh/<refresh-id>/` before persisting the local snapshot | Keep blocked or malformed ledger/report evidence visible as `refresh required`; do not substitute a zero or overwrite an explicit scoped audit |
| `refresh-github`                                                                                                  | Provider-backed refresh that persists a sanitized snapshot                                                                                                                  | User-requested/current provider scope; do not expose secrets                                                                                   |
| `root add` / `root exclude`                                                                                       | Changes discovery configuration and may refresh records                                                                                                                     | Explicit configuration request                                                                                                                 |
| `clone`                                                                                                           | Creates a local checkout                                                                                                                                                    | Explicit clone request and exact destination                                                                                                   |
| `remediation refresh` / `export`                                                                                  | Persists or writes remediation artifacts                                                                                                                                    | Explicit remediation/report request                                                                                                            |
| `remediation set-status`                                                                                          | Changes a local remediation action status                                                                                                                                   | Exact user-requested action and status                                                                                                         |

Pronto does not provide branch merge, fold integration, delete, reset, rebase,
push, provider mutation, or release publication commands. `fold preview` is a
read-only candidate projection. `doctor` is also read-only and does not refresh
or repair the snapshot. Use the separate `fold-feature-branches`
workflow and ordinary Git/provider workflows only after their own
authorization, preflight, and review gates pass.

## Compass development interfaces

The updated checkout provides `compass family <workspace> --json`,
`compass change-context <workspace> --path <path> --base <revision> --json`,
`compass gate <workspace> --json`, and
`compass assess <workspace> --proposal <file> --json`.
They consume the canonical Python helper and return `compass-family/v1`,
`compass-change-context/v1`, and `compass-modernization-assessment/v1`.
Explicit `change-context --packet-output .quality-runner/compass/NAME.json`
also accepts `compass-packet-artifact/v1`; it writes only a complete eligible
entry packet, at most 1 MiB, for inspection before explicit adoption. Gate may
return `compass-gate-summary/v1` after full validation. Both are distinct from
prepared packets; preserve all blockers and nonzero failure exits. Ordinary
output remains bounded to 32 KiB. See the development contract for safe-path,
no-clobber and retained-baseline requirements.
Preserve draft/accepted authority, source identity, captured time, unknowns,
conflicts and proof freshness. Cached family observations are not current proof.
The helper prerequisite and full schemas are documented in
`docs/compass-development.md` in the selected Pronto checkout.

Prepare from selected paths at task entry and retain the original packet at
`.quality-runner/compass/prepared.json`; `gate` recomputes the actual diff.
Run only separately reviewed, task-authorized proof commands through the
Compass helper; Pronto's bridge never executes them. Modernization assessment
is read-only for every disposition and never creates a second execution queue.

Compass orientation starts with `compass family WORKSPACE --summary --json`.
The distinct summary schema returns counts, not truncated coverage arrays.
Use `change-context --prepared PREVIOUS --continue --base ORIGINAL_SHA` across
milestones, then gate the cumulative actual diff. Bootstrap linkage is separate
from current semantic proof; older snapshots report unknown intent maturity.

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
