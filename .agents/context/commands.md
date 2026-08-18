# Agent command contract

Last reviewed: 2026-08-16.

## Invocation

Run the current checkout's CLI through `pnpm`; do not rely on an unverified
standalone binary:

```sh
PRONTO_ROOT="$(git rev-parse --show-toplevel)"
pnpm --silent --dir "$PRONTO_ROOT" run cli route --json
```

The CLI reads and writes Pronto's local SQLite-backed snapshot. JSON is the
preferred agent interface. `doctor --json` is the read-only freshness and
storage gate; it never refreshes or writes the snapshot. `status --json`
remains the complete legacy snapshot; use a focused projection when one is
available.

`pnpm contract:check` also validates `.pronto/cache-lifecycle.json`. That
contract identifies rebuildable worktree outputs and their rebuild commands;
it is evidence for a separately authorized cleanup, not deletion authority.

`route --json` is the preferred composed entry point for agent orientation. It
performs the same read-only doctor gate for the selected scope and, only when
that gate is ready, includes bounded `next`, repository, quality,
`change_maturity`, `developer_legibility`, `change_surface_hotspots`, and
`fold_preview` projections from the same snapshot boundary. The change summary
and the two maturity gates are advisory and recommend read-only inspection;
they do not create a missing matrix or modify a repository. Older feeds may
omit the new dimensions, in which case their route values remain `unknown`
until the producer audit is rerun. The fold preview uses the
repository's persisted target branch when configured, falls back to the
observed default branch, and remains advisory; use direct `fold preview` when a
one-off explicit target is needed. The composed route leaves `merge_preview`
empty and sets `live_verification_required: true`; direct `fold preview` is the
explicit path for live merge checks. A blocked route
intentionally withholds follow-up projections and exits non-zero; use its
`next_safe_step` before refreshing or repairing evidence.

## Target branch policy

The fleet integration and maturity target is an explicit repository-level
`dev` target. Git's `main` or `master` default branch remains the release and
default branch; configuring Pronto to target `dev` does not rename, replace, or
merge those branches. Set the target only after confirming that the local
repository has the requested branch:

```sh
pnpm --silent --dir "$PRONTO_ROOT" run cli repo set-target <repository> dev --json
pnpm --silent --dir "$PRONTO_ROOT" run cli refresh <repository> --json
```

`repo set-target` persists the target override and validates the local branch;
it does not mutate Git. The subsequent scoped refresh re-evaluates the
repository snapshot. Quality Runner evidence is accepted for the selected
target only when both the scanned branch and scanned commit exactly match the
target branch and current target head. A stale or mismatched scan remains
stale/unknown until it is refreshed from the exact target commit.

## Focused read and preview surfaces

| Need                        | Command                                                                                                                                                           | Contract                                                                                                         |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Agent routing envelope      | `route [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] [--limit <n>] [--fresh] --json`                                                  | `pronto-agent-route/v1`                                                                                          |
| Freshness/storage gate      | `doctor [<repository>] [--product <name> \| --group <name>] [--max-age <minutes>] --json`                                                                         | `pronto-agent-doctor/v1`                                                                                         |
| Daily orientation           | `next [<repository>] [--product <name> \| --group <name>] [--limit <n>] --json`                                                                                   | `pronto-agent-next/v1`                                                                                           |
| Fold preparation            | `fold preview [<repository>] [--target <branch>] [--product <name> \| --group <name>] [--limit <n>] --json`                                                       | `pronto-agent-fold-preview/v1`                                                                                   |
| Fleet orientation           | `summary [--product <name> \| --group <name>] --json`                                                                                                             | `pronto-agent-summary/v1`                                                                                        |
| One repository              | `repo <absolute-repo-path> [--fresh] --json`                                                                                                                      | `pronto-agent-repository/v1`                                                                                     |
| Repository target           | `repo set-target <repository> <branch> --json`                                                                                                                    | persisted target override                                                                                        |
| Quality evidence            | `quality [<repository>] --json`                                                                                                                                   | `pronto-agent-quality/v1`                                                                                        |
| Quality import              | `quality refresh --json`                                                                                                                                          | persisted quality plus accepted Analytics observation                                                            |
| Fleet detector refresh      | `quality detector-refresh [--qr-bin <path>] [--timeout-seconds <seconds>] [--agent-review-mode <off\|auto\|parallel\|required>] --json`                           | `pronto-quality-detector-refresh/v1`                                                                             |
| Custody projection          | `custody [<repository-path>] --json`                                                                                                                              | `pronto-custody-cli/v1`; includes read-only role-based workspace target and canonical/temporary-lane projection  |
| Workspace fleet manifest    | `workspace-manifest --role-map <path\|@json> --json`                                                                                                              | `workspace-fleet-manifest/v1`; exact explicit role coverage plus live temporary-lane counts; read-only           |
| Workspace policy generation | `workspace-policy generate --role-map <path\|@json> [--repository <id\|path\|name>] [--write] [--replace] --json`                                                 | `workspace-policy-generation/v1`; default dry-run, explicit repository-file write only, no Git/provider mutation |
| Parallel local refresh      | `refresh-batch [<repository\|group\|product\|repository-path>] [--parallelism <positive-integer>] --json`                                                         | `pronto-refresh-batch/v1`                                                                                        |
| Behavior assurance          | `behavior [<repository>] [--filter <missing\|legacy\|unprofiled\|partially_verified\|stale\|failed\|blocked\|unknown\|current\|not_applicable>] [--fresh] --json` | `pronto-behavior-assurance-audit/v2`                                                                             |
| Finding adjudication        | `quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] --json`     | repository-owned overlay                                                                                         |
| Skill topology              | `skills [<skill-id>] --json`                                                                                                                                      | `pronto-skills/v4`                                                                                               |
| Analytics evidence          | `analytics [--range-days <days>] --json`                                                                                                                          | `pronto-analytics/v2`                                                                                            |
| Analytics saved views       | `analytics view list\|save --config-json <json\|@file>\|delete <id>\|default <id> --json`                                                                         | `pronto-analytics-view/v1`                                                                                       |
| Papercut corpus             | `papercuts list --json`                                                                                                                                           | `pronto-papercuts/v2`                                                                                            |
| Papercut observation        | `papercuts observe --stdin --json [--dry-run]`                                                                                                                    | idempotent local write                                                                                           |
| Papercut input contract     | `papercuts contract --json`                                                                                                                                       | `pronto-papercuts-observation/v1`                                                                                |
| Papercut weekly digest      | `papercuts digest --week current --json`                                                                                                                          | deterministic sanitized digest                                                                                   |
| Multiplier proposal         | `papercuts propose --stdin --json`                                                                                                                                | draft only                                                                                                       |
| Proposal review             | `papercuts proposal set-status <id> <draft\|accepted\|deferred\|rejected> --json`                                                                                 | human judgment, no implementation                                                                                |
| Papercut capture health     | `papercuts health --json`                                                                                                                                         | local hook and spool state                                                                                       |
| Repository change matrix    | `change-matrix repo <repository> [--operation <add\|change\|remove>] --json`                                                                                      | `pronto-change-matrix/v1`                                                                                        |
| Skill change matrix         | `change-matrix skill <skill-id> [--operation <add\|change\|remove>] --json`                                                                                       | `pronto-change-matrix/v1`                                                                                        |
| Active remediation          | `remediation [<repository>] --json`                                                                                                                               | `pronto-remediation/v3`                                                                                          |
| Remediation execution gate  | `remediation gate <repository> [--workspace <id>] --json`                                                                                                         | `pronto-remediation-execution-gate/v1`                                                                           |
| Work requiring attention    | `attention --json`                                                                                                                                                | `pronto-agent-attention/v1`                                                                                      |
| Recent transitions/audits   | `activity [<repository>] --limit <n> --json`                                                                                                                      | `pronto-agent-activity/v1`                                                                                       |
| Preparation preflight       | `prepare <repository> [--workspace <id>] [--fresh] --json`                                                                                                        | `pronto-agent-preparation/v1`                                                                                    |
| Release preflight           | `release preview <repository> [--workspace <id>] [--fresh] --json`                                                                                                | `pronto-agent-release/v2`                                                                                        |

`workspace-policy generate` is separate from the read-only fleet manifest.
It uses the reviewed exact-coverage role map to plan one
`.agents/workspace-policy.json` per selected registered Git repository. The
default is a no-write plan; `--write` is required to create files and
`--replace --write` is required to replace an existing differing file. The
command validates the selected scope before writing, blocks non-Git roots and
symlinked policy targets, and writes no repository file when a fleet preflight
has a blocked or conflicting target. It never commits, pushes, protects refs,
grants custody, creates worktrees, or deletes branches; generated files enter
each repository's normal review and integration lane.

Resolve a repository with `git rev-parse --show-toplevel` and pass the
absolute path, repository name, ID, or an exact workspace path. Do not pass `.`
and assume it will resolve.

`release preview` v2 exposes every commit since the last published, non-draft,
non-prerelease tag and an explicit advisory `recommendation`. Its disposition
is one of `do_not_release_yet`, `review_required`, `release_patch`,
`release_minor`, or `release_major`; release dispositions include the exact
next SemVer version. Conventional commits provide the bump, but readiness
blockers always force `do_not_release_yet`, unclassified commits force review,
and a change-derived candidate without a passed configured rule remains
`review_required`. The preview never tags, publishes, or otherwise authorizes a
release.

For a full inventory check of one repository standard, Quality Runner owns the
static fleet scope. Run
`qr fleet audit run --all --projects-root /path/to/projects --standard matrix-maintenance --json`
and inspect the resulting private `standard-report.json`. This one-standard
snapshot is not the canonical Pronto maturity feed and must not be published
as one; it reports every audited repository without manufacturing compliance.

Use `--standard cache-design` for the read-only derived-storage audit. Pronto
consumes `quality-runner-cache-design-assessment-v1` only from a replay-valid
complete v2 feed. It preserves maintained, attention, unknown, stale, blocked,
failed, missing, and not-applicable evidence; raw size and an absent projection
are never interpreted as a pass.

`matrix_maintenance` is nevertheless a canonical maturity dimension in a
complete QR fleet audit. The full audit contributes it to repository and fleet
`maturity_score`, `dimension_scores`, `dimension_gaps`, and the Pronto maturity
remediation queue. After using the scoped report to identify gaps, rerun the
complete audit without `--standard`, publish its replay-valid feed, and refresh
Pronto; the diagnostic slice alone does not change the score or queue.

Long-running task review uses the same producer/consumer boundary. Run
`qr fleet audit run --all --projects-root /path/to/projects --standard long-running-tasks --json`
for the diagnostic inventory. QR qualifies tasks from explicit annotations or
concrete timeout declarations, never names alone, and assesses
`long_running_task_observability` plus `long_running_task_optimization`.
Complete QR feeds map both dimensions into Pronto's operability pillar and
create deferred P2 maturity actions. They are triage work, not automatic fixes
or immediate repository blockers.

The canonical maturity feed is `quality-runner-maturity-feed/v2`. Its headline
score is a risk-weighted repository-quality measure over seven pillars, not a
flat mean of however many dimensions happen to exist. Consumers must retain
the pillar vector, explicit `applicable` / `unknown` / `not_applicable` state,
evidence and fresh-evidence coverage, missing capabilities, and critical cap
reasons. Correctness, security, and operability blockers cap the score at 2/4.
Pronto accepts v1 during migration, labels its source score as a legacy
dimension mean, and rebuilds the local pillar view without claiming that the
old feed supplied v2 evidence. Project Compass and product readiness remain
separate projections and never raise or lower repository maturity.

Quality, summary, attention, and remediation projections expose generic
`evidence_contracts` state. Observation freshness and contract freshness are
independent: a recent report whose observed schema differs from the registered
target schema remains readable but reports `audit_required`. At portfolio
scope, any non-current repository means a full fleet audit is required; do not
interpret a legacy report's successful checks as current contract coverage.

Fleet quality presentation uses `quality_outcome_counts` and
`quality_outcome_taxonomy`. Display their bounded labels instead of shortening
everything to `Blocked`: `checks_failing` is observed failing quality evidence;
`verification_blocked` is an execution, setup, timeout, or provenance barrier;
`review_needed` has no known blocker but remains below ideal or unverified;
`evidence_unknown` lacks trustworthy current evidence; and `healthy` is
fresh-passing or safely reused maintained evidence. Repository projections also
carry a bounded `quality_outcome.disposition` that names the affected
dimensions and their evidence state, plus an optional `next_step`. Surface
those details so review-needed and evidence-review rows are actionable: render
`evidence_unknown` as `Evidence review required`, never as the raw machine state
or the old `unknown` label. The legacy QR `quality_status` remains a
compatibility field, not preferred display copy.

Behavior assurance is repository-owned in
`.pronto/behavior-assurance.json` and validated by Quality Runner from immutable
receipts under `.quality-runner/behavior-assurance/receipts/`. The `behavior`
projection keeps required Tier-0 release assurance separate from all-tier edge
coverage and exits non-zero whenever a selected repository is not release-ready.
The optional filter selects missing, legacy v1, unprofiled, stale, failed, or
blocked repositories without mutating their state. Missing, invalid, stale,
failed, blocked, target-mismatched, or insufficiently verified evidence is never
green. A receipt may carry forward from an ancestor commit only when the
contract digest is unchanged and no committed or dirty path matches that
behavior's declared change triggers. `release preview` applies the same Tier-0
gate. See `docs/behavior-assurance.md` for the artifact contract.

The Node adapter normally resolves `cargo` from the documented Homebrew paths
and then `PATH`. Set `PRONTO_CARGO` to an explicit Cargo executable only when a
different verified Rust toolchain is required.

For ordinary app-facing source changes, exercise the live `pnpm dev` window and
run the applicable quality gates. That verifies the current checkout directly
without claiming that `/Applications/Pronto.app` is current. Promote a coherent
checkpoint with `pnpm app:update` when the installed daily-driver app should
change, or when Tauri configuration, bundled assets, native entry points,
installation behavior, or release readiness is in scope. The update builds only
the macOS app bundle, stages and fully replaces `/Applications/Pronto.app`, and
must be followed by `pnpm app:check` plus an installed-app launch. `pnpm build`
and `pnpm app` remain compatibility aliases for `pnpm app:update`;
`pnpm build:release` generates the complete distribution artifact set. If
Pronto was already running, the installer must quit it before replacement and
force a distinct foreground launch afterward. Never use an overlay copy:
obsolete files must not survive an installation. An already loaded
`com.pronto.skill-usage-collector` remains registered across the atomic
replacement and is restarted with `launchctl kickstart -k`; registration is
reserved for a missing or materially changed LaunchAgent plist.

Repository and summary projections include a read-only `project_compass`
summary derived from `.project-compass/contract.json`. `Ready` reports the
current product identity, MVP and complete-product progress, confidence,
target-scoped outcome counts, covered/total pillar counts, blockers, and open
drift. Partial target coverage remains explicit; a progress score never implies
that unmodeled pillars are complete. Open blockers preserve their outcome, kind, and
summary; open drift preserves its kind, summary, and observation time so the UI
and JSON explain the count rather than projecting only a number. `Missing`
means the repository has not established a Compass contract; `Invalid`
preserves the parse or contract error. Pronto never creates or repairs Compass
artifacts during a refresh.

The fleet summary also includes a read-only `showcase` projection with schema
`pronto-showcase/v2`, derived from the single fleet-level
`.pronto/showcase-goal.json` contract. It keeps product readiness,
demo-material readiness, career signal, eligibility, blockers, and missing
materials separate. Public projects also carry a required work disposition and
an active next-step category so product, demo-integration, evidence, content,
and packaging work remain distinct. The readiness ranking covers every assessed repository;
A newly discovered repository enters the normal registered fleet, but
registration and refresh do not mutate the Showcase contract or create a
placeholder row. Showcase additions require an immediate explicit review with
complete fields; until then, registered repositories absent from the reviewed
contract (including legacy gaps) are included as unranked `unknown` entries. If
no valid fleet contract exists, Pronto leaves the contract missing rather than
creating one with fabricated policy or scores. Eligibility is evaluated before
public priority:
`private_client` work can retain private readiness context but never receives
a public priority, counts toward the public goal, or enters the public queue.
`unknown`, `blocked`, and `not_applicable` remain categorical rather than
becoming zero scores.

Repositories may also declare `.pronto/installed-runtime-parity.json` to make
source, packaged-build, installed-artifact, and running-process drift visible.
`repo`, `quality`, `summary`, and composed `route` projections expose the
result; remediation adds a repository-health action when parity is not
`current`. Pronto reads only the declared bounded manifests and validates the
recorded PID against the declared executable. See
`docs/installed-runtime-parity.md` for schemas and state meanings.

`change-matrix` explains an existing repository- or skill-owned contract. A
missing contract returns `status: "missing"`, its maturity impact, observed
topology, and the expected location without synthesizing or writing anything.
`skills [<skill-id>] --json` preserves source paths and hashes, provider state,
parity evidence, and `hosted_in_jakye_agent_setup`. Its `usage.state` is
`observed` only for a structured provider invocation feed. An unavailable feed
reports zeroed compatibility counters with `usage.state: "unavailable"`; agents
must not interpret those counters as observed zero usage. Catalog, prompt, and
transcript text are never invocation evidence. Codex observations come from the
read-only `skill_invocations` table in `~/.codex/state_5.sqlite` when the
instrumented fork is installed. Otherwise Pronto falls back to its
localhost-only `codex.skill.injected` OTLP compatibility feed. The fallback
records coverage start and collector heartbeat; `all_time_count` covers only
events received after activation. Enable, inspect, or reverse that approved
persistent route with `pnpm skills:collector:install`,
`pnpm skills:collector:check`, and `pnpm skills:collector:uninstall`. Enabling it
replaces Codex's default metrics exporter while active, and Codex processes must
restart to load the configuration.

Run `pnpm failure-visibility` after changing fallback, degradation, or evidence
normalization behavior. It exercises the native usage source, renderer
normalization, and collector refusal paths. Quality Runner reports this as the
required `failure_visibility` capability; discovery alone is `configured`, not
a passing result.

`remediation` is an active, goal-aware ranked queue. Its `plans` contain only
actionable repositories and expose the resolved `goal` profile. Repositories
may confirm that profile through `.pronto/remediation-goal.json` using
`pronto-remediation-goal/v1`; absent or invalid contracts remain visibly
inferred and create confirmation work rather than silently becoming truth.
Goal-specific required gates, freshness windows, and closure criteria determine
which actions apply. Every plan also contains a `coverage` ledger for all
repo-level surfaces tracked in the UI: scope, Project Compass, provider, pull
requests, published releases, quality evidence, CI gates, findings, maturity,
workspaces, branches, submodules, conditions, release preparation, agent
permission, and analytics.
Each plan also contains an `explanation` projection that groups only active
(`open`, `in_progress`, or `blocked`) actions into ordered operator phases.
Pronto supplies five default phase definitions—preserve and reconcile
repository work, reconcile product and provider truth, reach quality and
maturity threshold, prove the public distribution boundary, and refresh and
re-evaluate—but that sequence is not a maximum. The public-distribution phase
is active only for `public_release` goals. Pronto reads
`.quality-runner/release-boundary.json` as
`quality-runner-release-boundary/v2`; v1 or unknown schemas require a new audit.
Missing, stale, exact-target-mismatched, matrix-digest-mismatched,
artifact-digest-mismatched, or failed evidence creates a receipt-derived blocked
action and hard-blocks `release preview`. Manual action status cannot bypass the
gate, and Pronto never executes receipt content. The passing receipt proves
`public_core`, `public_adapter`, and `local_only` classification plus artifact,
isolated-install, and sanitized-integration evidence. A repository contract may add phase definitions, assign action
domains to them, and place each addition after an earlier phase. Repository
phases take ownership of their declared domains; active actions in an
unassigned domain remain visible in an explicit `unclassified_remediation`
phase. Every active action must appear exactly once. Every phase exposes linked
action steps and completion criteria; verified history is not presented as
remaining work. The explanation names
`clear` and `verified` coverage surfaces as already healthy and repeats the
goal-specific closure requirements. It is advisory and never authorizes Git,
provider, publication, release, or pruning mutations. The Markdown queue export
includes the ordered remaining phase titles so its human-readable summary stays
aligned with the JSON plan and app detail surface.
Goals that require maturity also expose a structured `maturity_policy`: 3.0/4
is the minimum evidence-backed closure score and 4.0/4 is the ideal. Reaching
the closure score removes blocking maturity work from the active queue, while
the policy remains in retained closure evidence. Agents may continue material,
applicable improvements toward the ideal, but must not create superficial
documentation, configuration, tests, or other artifacts solely to raise the
score. The 4.0/4 ideal additionally requires every configured maturity gate,
including the fresh, commit-matched Mac Control ideal-state gate where
applicable; this is a score condition, not a four-repository count. See
`docs/mac-control-maturity-gate.md` for the report contract and explicit
not-applicable handling.
Only `mac-control-task-manifest/v4` source-grounded dimensions can contribute
to the implementation lane. V1 through v3 declaration counts remain visible
for diagnosis but score zero and must be labeled non-scoring in every consumer.
Validate producer-count consistency, typed semantic claims, provider ownership,
and per-dimension grounding before presenting a Mac Control disposition.
Unresolved coverage must link to action IDs; clear, informational, or
goal-inapplicable surfaces remain explicit without manufacturing work. Terminal evidence-backed outcomes move to the retained
`closures` ledger, including their target state, and may re-enter the queue
after a later refresh. A repository query returns either its active plan, its
retained closures, or both. Ranking preserves status, the earliest unresolved
domain, and action priority before applying explicit fleet leverage for Pronto,
AIOS, and Quality Runner; repository goal and raw action weight are later
tie-breakers. `remediation export` writes the JSON contracts plus
`repository-remediation-order.md`.

The run also exposes `github_only_candidates`. These are authenticated GitHub
repositories present in the provider snapshot without a matching local checkout;
they remain counted as provider evidence rather than being discarded or turned
into synthetic local plans. Their locality label is `GitHub only`, and their
terminal remediation task is `GitHub only`. A locally retained repository whose
goal is explicitly or inferentially labeled `GitHub only` receives the same
disposition as its final verification action.

The cached remediation plan's `status` is queue/closure state. It reports an
explicitly blocked action, such as a missing release receipt, but never claims
that repository mutation is currently impossible. Before executing remediation,
run `pronto remediation gate <repository> --json`. The read-only
`pronto-remediation-execution-gate/v1` projection checks every registered
workspace by default, or one exact `--workspace`, and returns structured
`blockers`, affected `blocked_operations`, a separate `closure_gate`, and the
caller-owned `authorization` boundary. `ready: false` exits non-zero. A
`partially_blocked` repository has at least one safe workspace and at least one
workspace that must remain untouched; it is not blanket permission to mutate
the repository. Explicit active ownership is reported as
`ownership_coordination_required`; failed ownership inspection is separately
reported as `ownership_evidence_unavailable` and never rewritten as an active
owner.

Remediation handoffs use a provider-neutral checkpoint rule: work owned by an
executor must be locally committed on its isolated branch before handoff or
local action verification. `pronto remediation handoff-check <repository> --json`
is a read-only live Git receipt with schema
`pronto-remediation-handoff/v1`; `ready: false` is a hard stop. A dirty or
owner-ambiguous workspace is preserved, not stashed, overwritten, or silently
folded. After a checkpoint commit, run the scoped `pronto refresh` before
rechecking so cached Pronto evidence agrees with live Git. `remediation
set-status` repeats the same gate before marking a local action `verified`.
The `GitHub only` terminal action is the deliberate exception because it does
not require a local checkout. This is bounded Pronto enforcement, not a
fleet-wide auto-commit service; nightly automation may report or recover only
under its separately authorized ownership rules.

`remediation refresh` closes its quality-import checkpoint only when the
canonical QR feed is published and every eligible repository whose goal
requires maturity has a fresh repository-level score. A replay-validated
scoped audit may supply that repository evidence when the repository lives
outside the canonical projects root. Pronto retains that scoped audit
provenance across later local or provider refreshes; the remediation plan,
repository projection, and UI must all read the same imported maturity
snapshot. Missing or stale applicable scores leave the refresh `partial` and
the `quality_import` step `blocked` with the affected repositories named.
Version 2 feeds may additionally publish `measurement_confidence`. Pronto
accepts only `low`, `medium`, or `high`, requires deterministic replay and an
exact population count, and rejects a `high` claim that retains measurement
gaps or limitations. The quality overview displays the imported confidence and
measured population beside the maturity score; older version 1 feeds remain
valid but display confidence as unavailable.
Dynamic audits default to a 120-second per-command timeout. Use
`remediation refresh --dynamic --timeout-seconds <positive-integer>` when a
repository's documented quality command legitimately needs a longer bound;
the same explicit timeout is applied to both the scoped audit and any required
canonical all-projects fallback.

Missing findings evidence is not repaired by the maturity audit. Use the
explicit `quality detector-refresh` lane to run QR full analysis with
deterministic skill packs at every registered repository's exact target commit,
passing Pronto's configured target branches as path-keyed overrides, publishing
normal QR runs, and immediately re-importing them. Pronto performs a local
read-only repository scan before QR starts and again before import so target-ref
provenance cannot remain cached across the detector run. Every QR result marked
`published` is then reconciled against the imported report path, target branch,
target commit, freshness, and findings total. A published result that is not
ingested is returned as `reconciliation[].status = "rejected"`, makes the
overall result `Partial`, and exits nonzero. The command continues past QR's
explicit per-repository `blocked` and `unsupported` outcomes and returns the QR
ledger plus post-import findings coverage. Coverage reports both all tracked
repositories and the detector-applicable denominator; exact QR `unsupported`
results are counted as excluded rather than falsely reported as missing. It
does not execute discovered repository gates. Agent review remains off unless
the operator explicitly sets `--agent-review-mode`.

## Refresh and state boundaries

Use `refresh <repository|group|product|repository-path> --json` when the
persisted snapshot is stale or after a branch/workspace change that needs fresh
evidence. It performs a local read-only Git scan but persists the resulting
snapshot and audit record, so it is state-changing even though it does not
modify a repository. An exact repository path under a registered discovery
root may be scoped-refreshed even when it has not yet been admitted to the
snapshot; the path must be a valid Git repository. An explicit path is allowed
to override that root's automatic discovery exclusion for this refresh without
changing the root configuration.

Run `route --json` before routing across repositories. For repository-local
work, pass the resolved repository path; this prevents unrelated fleet rows
from blocking the task. A non-zero exit or `ready: false` is a hard stop for
the selected scope; refresh or repair only the evidence it identifies, then
rerun route. Use direct `doctor` when a dedicated freshness/storage report is
needed.

Read-only projections (`route`, `status`, `repo`, `summary`, `next`, `fold
preview`, `quality`, `attention`, `activity`, and `remediation`) use the
persisted SQLite snapshot and do not ingest repository quality artifacts while
serializing their response. This keeps follow-up reads observable even when a
repository's quality tree is slow or unavailable. Preparation and release
previews also use the cached snapshot by default. Add `--fresh` to `route`,
`status`, `repo`, `prepare`, or `release preview` only when a bounded fresh
quality projection is required; the projection has a 10-second deadline and
returns an explicit error state on timeout. Release-history inspection is
independently capped at 1,000 commits and 10 seconds; failure is returned as
explicit unavailable evidence rather than an empty history. Use
`quality refresh --json` when the imported quality state should be
persisted for subsequent cached reads. An accepted canonical audit also records
the resulting Analytics observation through the normal fingerprinted sampling
path; an unchanged repeat deduplicates, while rejected or unavailable evidence
does not manufacture history. The local `refresh` and `quality
refresh` commands are single-flight per Pronto database: each writer acquires
the store lock before loading mutable state, so concurrent writers cannot
persist stale snapshots or interleave state. A caller that cannot acquire the
lock within the bounded wait receives a retryable error.

For fleet or multi-repository work, `refresh-batch [<repository|group|product|repository-path>] --parallelism <positive-integer> --json` separates the safe phases: it plans from a read-only snapshot, scans repository Git/filesystem evidence in bounded parallel workers, then acquires the same store lock once and merges results in deterministic repository-id order. The commit phase reloads state after taking the lock and compares a store revision token with the scan baseline; when another writer changed the store during scanning, the batch resamples once and otherwise returns a retryable conflict. Provider calls and Git mutations are not performed by this command. In the bounded case where a scan discovers a new repository and exactly one valid fleet Showcase contract exists, the merge may atomically append that repository's pending `unknown` Showcase goal row; it performs no other repository-file writes. The ordinary `refresh` command remains the compatibility path and keeps its single-flight critical section with the same Showcase onboarding rule.

The following commands can change local Pronto state or touch an external
boundary and require explicit task scope: `root add`, `root exclude`, `refresh`,
`refresh-batch`, `refresh-github`, `clone`, `remediation refresh`, `remediation export`,
`remediation set-status`, and `workspace-policy generate --write`. `quality disposition set` writes the selected
repository's `.pronto/quality-finding-dispositions.json` review ledger and
therefore also requires an exact repository, finding fingerprint, disposition,
reason, and reviewer. These commands do not authorize Git branch cleanup,
merging, pushing, deletion, provider mutation, or release publication.

`remediation gate` and `remediation handoff-check` are read-only live gates
rather than cached projections. The repository-level execution gate also
checks active or ambiguous ownership and unavailable workspace paths, and keeps
closure status separate. The handoff check exits non-zero when the selected
workspace has ambiguous ownership, is dirty, has an interrupted operation, has
stale dirty evidence, or cannot be checked.

Use `refresh-github <repository|group|product> --json` when live provider
evidence is needed for a bounded scope. Omitting the target refreshes the whole
registered fleet and should be reserved for an explicitly fleet-wide task.

`fold preview` is an advisory projection only; it does not clean branches,
fold dev, delete branches, or push. Use it for persisted branch/worktree
evidence, then use the reviewed branch-folding workflow and ordinary Git
commands within their own authorization boundaries.

## Evidence interpretation

The default snapshot freshness window for `route` and `doctor` is 48 hours
(`2,880` minutes). Use `--max-age` for an explicitly different review window;
operations that depend on current Git state still require live verification.

Quality gate and finding evidence uses a separate commit-bound rule: an
in-window observation is `Fresh` only when its scanned commit equals the
current commit. A matching branch name alone is not freshness proof. Without
comparable commit provenance, a matching branch is `Unknown` and a differing
branch is `Stale`; an exact commit match remains authoritative.

- `generated_at` identifies the snapshot time; re-check it after a meaningful
  state change.
- `doctor` reports storage, registered roots, per-repository scan freshness,
  local path availability, and quality warnings without changing local state.
- Workspace `sync_state` is healthy only when it is exactly `Synced`.
- `Ahead by N`, `Behind by N`, divergence, `No upstream`, dirty workspaces,
  active operations, missing/stale quality evidence, `Unknown`, and `Blocked`
  require attention or an explicit explanation.
- A preview or configured ideal is not execution proof. Keep source tests,
  local validation, live browser/device evidence, and release/provider proof
  distinct.
- Quality finding `total` remains the immutable detector count; it never
  includes maturity rows. The explicit detector projection also exposes
  `detector_findings_total`, `detector_actionable_total`, and
  `detector_unreviewed_total`, plus enabled detector/rule counts, producer
  versions and source SHAs, ruleset/configuration fingerprints, QR version,
  target SHA, refresh time, and the delta since the prior comparable scan.
  `actionable_total`, `reviewed_total`, `unreviewed_total`, and
  `disposition_counts` are reconciled from the repository-owned
  `pronto-quality-finding-dispositions/v1` ledger. Missing, invalid, expired,
  absent, or scope-mismatched dispositions never suppress a finding.
- A missing, malformed, failed, or stale detector receipt is blocked evidence,
  never a zero-finding result. Pronto may retain the prior detector count as
  raw evidence while showing `refresh required`; it must not present that
  retained count as current or compute a comparable-scan delta until a fresh
  receipt is imported.
- QR category `debloat` is projected as the conditional
  `Repository debloat review` gate. QR's deterministic findings are structural
  triggers, not proof of architectural bloat or a complete ownership-pressure
  audit. The gate is configured only when the QR report explicitly declares
  category coverage. Unresolved signals block; stale evidence stays stale; and
  a fresh clear scan passes only the candidate-review gate. It does not establish
  repository-wide debloat maturity, deletion readiness, or deletion authority. A reviewed
  false positive or intentional structure can clear its candidate, while
  confirmed, deferred, or accepted-risk bloat remains open for this maturity
  gate. Remediation refresh preserves in-progress state when the historical
  `simplify|simplification and shrink pass` group migrates to the canonical
  `debloat|debloat candidate review` group. When dispositions remove every leaf
  action but the maturity gate remains blocked, Pronto emits one advisory
  debloat-gate review action; it does not duplicate an existing debloat leaf or
  aggregate QR action and does not affect CI-readiness scoring.
- Supported finding dispositions are `confirmed`, `false_positive`,
  `accepted_intentional`, `accepted_risk`, `deferred`, `fixed`, and
  `superseded`. Confirmed and deferred findings remain actionable; false
  positives, accepted intentional behavior, and accepted risk leave the leaf
  remediation queue while their review evidence remains visible. A blocked
  debloat review gate can still retain its single advisory gate-review action.
  Fixed or superseded decisions attached to a current finding are treated as
  stale rather than silently hiding a recurrence.
- Preserve `schema_version`, freshness, evidence, and uncertainty fields when
  handing JSON to another agent.
