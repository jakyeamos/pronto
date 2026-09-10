---
name: pronto
description: Use Pronto as the local-first portfolio, Git workspace, source-derived Repository Telescope, quality-evidence, AI Showcase readiness, and release-preflight command center. Trigger when a task spans repositories, asks how a codebase is structured, asks what needs attention, involves dirty or unsynced workspaces, branch cleanup or folding feature work into dev, demo-readiness prioritization, quality/remediation triage, release readiness, or asks an agent to orient itself from the user's current portfolio state.
---

# Pronto

Use Pronto to establish current, evidence-backed portfolio context before
making cross-repository decisions. Prefer its focused JSON projections over the
full snapshot, preserve uncertainty and freshness, and keep
inventory/preflight separate from Git, provider, and release mutations.

Read [`references/cli-contract.md`](references/cli-contract.md) before using a
less common command or interpreting a projection. When working inside the
Pronto repository, also read
`.agents/context/commands.md in the selected Pronto checkout` for the live
repository contract.

## Route into Pronto

Use this skill when the task includes any of the following:

- portfolio-wide status, repository triage, or “what should I work on next?”
- dirty, unsynced, stale, abandoned, or active agent workspaces
- cleaning branches, consolidating work, folding feature branches, or bringing
  a canonical `dev` line up to date
- quality evidence, remediation queues, release readiness, or preparation
- source-derived repository architecture, module relationships, static flows,
  or Pronto workflow evidence projected onto code structure
- AI Showcase product readiness, demo-material gaps, public eligibility, or
  demo-production priority
- a request to use Pronto as the operating surface for daily repository work

Do not load it for a tiny single-repository edit with no portfolio, workspace,
branch, quality, or release question. Do load it before changing a branch when
the user asks for cleanup or consolidation, even if the eventual Git change is
local.

## Scoped development foundation

Use the selected checkout's CLI and confirm it supports `compass` before using
this interface. The default `route --json` is the bounded v2 orientation;
`route --full --json` explicitly requests the legacy v1 envelope. Preserve the
complete doctor gate and drill into a named projection when needed.

For an enrolled Compass pilot, prepare `compass change-context <workspace>`
from exact paths and a retained base before editing. Save the packet in that
workspace's `.quality-runner/compass/prepared.json`. Compass supplies purpose,
outcomes, inherited constraints, neighboring contracts and required proof;
Telescope supplies observed structure and Quality Runner owns release results.
Use `compass gate <workspace> --json` at completion against the original packet.
Scope expansion, affected intent revisions and missing current proof require
reconciliation. Unrelated legacy gaps remain visible without becoming blanket
blockers. A draft never becomes accepted intent because an import or test passed.

Pronto consumes the installed Python producer. An unavailable runtime or
unsupported projection is an explicit prerequisite failure, not an invitation
to reproduce scoring or validation. Assessment dispositions remain advisory,
reference existing remediation IDs, and create no implementation work. A model
release alone does not justify a migration.

## Operating contract

- Treat Pronto as a local evidence and coordination surface, not an autonomous
  Git operator. Its snapshot describes observed state; it does not grant
  authorization to modify a repository.
- Start read-only. Do not silently clean, reset, delete, merge, rebase, push,
  clone, publish, or change provider state.
- Treat missing, stale, blocked, unknown, ambiguous, or low-confidence evidence
  as unresolved. Never convert it into a passing claim.
- Run `route --json` before cross-repository routing. For repository-local work,
  pass the resolved repository path; for a collection, pass `--product` or
  `--group`. Route includes the doctor gate and a non-zero exit or `ready: false`
  is a hard stop for that scope until its reported storage, freshness, or path
  issue is resolved. Use direct `doctor` only when a dedicated gate projection
  is needed.
- Keep Pronto's local snapshot, source tests, local validation, live provider
  evidence, and release proof distinct in the handoff.
- Treat `pronto-telescope/v2` and `pronto-telescope-city/v2` as coordinated,
  workspace-bound projections. Preserve categorical `map_readiness`, confidence,
  provenance, partial coverage, explicit unknowns, applicability reasons, and
  freshness; never turn readiness into a blended score or treat generated
  descriptions and optional Compass/ICM lenses as confirmed source facts.
- Workspace summaries carry provenance (`canonical`, `linked`, `temporary`, or
  `unknown`), owner/lease, canonical repository, HEAD, preservation evidence,
  and cleanup state. Missing canonical roots block. Missing workspaces with a
  dirty or unknown last state block and are preserved. A missing temporary
  workspace is warning-only only when its last state was clean, its lease was
  completed, its HEAD is reachable, and live Git no longer registers the path.
- `route` is read-only and never removes stale workspace records. Use the exact
  scoped `refresh <repository> --json` named by a warning to reconstruct live
  workspaces and remove safely classified stale records.
- For branch folding, use Pronto to select and refresh evidence, then use the
  `fold preview` projection and the `fold-feature-branches` skill for live
  classification, reviewed integration, and pruning. `fold preview` is
  advisory only; do not invent mutating `pronto clean`, `pronto fold`, or
  `pronto delete` commands.

## Default repository-remediation order

When Pronto exposes several kinds of repository debt at once, use this default
decision order unless repository policy or a concrete higher-severity condition
requires a narrower stop:

1. **Gate and preserve.** Resolve interrupted Git operations, active ownership,
   unavailable paths, and stale or ambiguous evidence. Inventory dirty,
   unpublished, unique, and owner-active work before changing it.
2. **Confirm and reconcile the canonical target.** Verify the repository's
   documented integration branch and fetched/live remote state. Do not assume
   the target is always `dev` or compare every branch to `main`.
3. **Stabilize the target.** Preserve coherent target work atomically, reconcile
   target divergence without discarding either side, and address critical
   security findings, broken required gates, or active incidents that make
   further integration unsafe.
4. **Fold eligible work.** Classify each source against both the local and live
   remote target, then integrate only reviewed, wanted work. Preserve ambiguous,
   dirty, active, target-mismatched, or still-unpublished sources.
5. **Verify the consolidated target.** Run the repository's required focused
   and full quality ladders, including applicable build, test, lint, type,
   security, packaging, and smoke checks.
6. **Publish and verify remote truth.** Use the documented push, PR, protection,
   or merge-queue lane, then read back the live target SHA and required provider
   results.
7. **Prune proven redundancy.** Remove local and remote branches or worktrees
   only after integration or patch equivalence, clean ownership, live target
   publication, and external PR/protection/consumer checks are proven. For a
   temporary worktree, take a fresh clean-status check immediately before
   removal and use ordinary `git worktree remove`; force removal is not routine.
   A completed cleanup must prove path absence, live Git absence, reachable
   HEAD/ref, successful scoped Pronto refresh, and a scoped route that no
   longer reports the workspace. Any failure preserves the exact path, dirty
   files, owner, lease, and preservation action in a blocked receipt.
8. **Refresh and close residuals.** Refresh only the affected Pronto scope,
   compare before/after evidence, and then handle lower-priority configuration,
   maturity, or release-preparation gaps.

Compactly: **preserve -> reconcile -> stabilize -> fold -> verify -> publish ->
prune -> refresh**. Pruning is a proof-backed closing action, not an opening
cleanup step. A critical safety or required-gate failure may interrupt this
sequence, but it does not waive preservation, ownership, or freshness checks.

## Locate and invoke the CLI

Use the current checkout's zero-dependency Node launcher directly. This keeps
Pronto routing independent of whichever `pnpm` shim the calling repository
selects:

```sh
PRONTO_ROOT=/absolute/path/to/pronto
node "$PRONTO_ROOT/bin/pronto.mjs" route --json
node "$PRONTO_ROOT/bin/pronto.mjs" doctor --json
node "$PRONTO_ROOT/bin/pronto.mjs" next --json
node "$PRONTO_ROOT/bin/pronto.mjs" telescope <repository> --json
```

The launcher resolves the checkout from its own path and adds Cargo's quiet
flag for JSON commands. Keep Pronto's exact `packageManager` pin for dependency
installation, development, tests, and builds; do not weaken the pin or use a
warning/ignore mode to recover routing.

Use `git rev-parse --show-toplevel` to resolve the active repository and pass
its absolute path to `doctor`, `repo`, `quality`, `activity`, `prepare`, or
`release preview`. A repository name, ID, or exact workspace path also works.
Do not assume `.` is a valid repository query.

## Default orientation loop

Run the smallest sequence that answers the task:

1. Run `route --json` for the read-only storage, freshness, path, bounded
   orientation, and branch-fold envelope. For repository-local work, run
   `route <resolved-repository> --json`; for a product or group, pass the
   matching flag. It composes doctor, next, repository, and quality evidence
   plus a bounded `fold_preview` from one snapshot boundary. If it exits non-zero or returns `ready: false`,
   follow `next_safe_step` and do not route work from that scope.
2. Run the individual `doctor`, `next`, `repo`, or `quality` projections when
   the route envelope does not answer a narrower question or when a workflow
   needs a dedicated projection contract.
3. Run `summary --json` or `attention --json` when the task needs the complete
   fleet summary or unbounded attention queue. Bound the work to the user's
   named repositories, group, product, or current repository.
4. Run `repo <repository> --json` for each candidate before deciding what it
   means. Use `quality <repository> --json` when quality evidence is part of
   the decision.
   For showcase work, use the `showcase` projection in `summary --json` and
   use the readiness score to compare the complete assessed fleet. Treat
   registered repositories projected as `unknown` as audit gaps, not zeroes.
   When a scoped registration or refresh discovers a new repository while
   exactly one valid fleet Showcase contract exists, Pronto may persist one
   pending `unknown` goal row for that repository; this creates a review target
   but never infers public eligibility, readiness scores, or publication
   authority.
   Apply `public_eligibility` before public-priority or publishing decisions.
   Never move `private_client` work into a public goal or
   materials/publishing queue; its readiness values are private audit context
   only.
5. Run `activity <repository> --limit 24 --json` only when transition history
   or prior action audits explain the current state.
6. If the snapshot is stale or a meaningful branch/workspace change occurred,
   run `refresh <repository> --json`, then repeat the relevant focused query.

Do not refresh the whole portfolio merely to make a report look current when a
scoped refresh answers the question.

When findings are unavailable because repositories have no detector report,
ordinary fleet maturity audits are insufficient. With explicit fleet-refresh
authority, run `quality detector-refresh --json`; it invokes QR full analysis
with deterministic skill packs at each registered exact target, records
published, blocked, and unsupported repositories, refreshes local target-ref
provenance before import, and then re-imports the normal QR runs. Treat every
`published` result as incomplete until `reconciliation[].status` is `ingested`;
a rejected publication makes the command partial and exits nonzero. It does not
execute discovered repository gates. Keep agent review off unless the user
explicitly requests that separate review surface.

Interpret detector coverage against lifecycle applicability: repositories that
QR explicitly classifies as unsupported historical or archival checkouts remain
tracked but are excluded from the applicable denominator. Do not convert that
state into a fresh zero. A full fleet publication may span sibling `inspect`,
`run`, and `verify` directories; Pronto retains the latest verification evidence
while sourcing findings from the highest-priority detector report in that marked
publication group.

Every Quality Runner command that persists a repository run must write a
canonical `.quality-runner/runs/<run-id>/run-manifest.json`. Treat the command
as only partially integrated until `quality <repository> --json` exposes it in
the generic `qr_command_runs` projection; richer command-specific projections
may be additive. The roll-up counts every run and mode, retains malformed
manifests as warnings, and shows a bounded newest-record list without reducing
the total. For test-portfolio evidence, use the dedicated `test_portfolio`
projection as well. Audit candidates and matching removal proofs are advisory:
they neither create a quality gate nor authorize source mutation or deletion.

The imported detector contract keeps code findings separate from maturity
evidence. Repository quality projections expose detector total/actionable/
unreviewed counts, enabled detector and rule counts, producer versions and
source SHAs, ruleset and configuration fingerprints, QR version, target SHA,
refresh time, and the delta since the prior comparable scan. Missing, malformed,
failed, or stale detector receipts are blocked evidence, never zero findings;
Pronto may retain a prior count as raw evidence while displaying `refresh
required`, but must not present it as current or compute a comparable delta.
Ordinary `quality refresh` imports the newest valid QR detector ledger from
`~/.quality-runner/fleet-detector-refresh/<refresh-id>/detector-refresh.json`
and the published `code-quality-scan.json` reports it names. Missing or
malformed ledger/report entries remain blocked, while an explicit scoped
fleet-audit import remains authoritative for that refresh.
Maturity cards separately report assessed versus applicable dimensions and
maturity gaps. Analytics deduplicates unchanged evidence but creates a new
observation when an imported detector or maturity evidence fingerprint changes
materially.

Repository quality also projects `foundation_readiness` using
`pronto-foundation-readiness/v1`. Read its disposition, confidence, freshness,
reasons, and unknowns before proposing substantial additive work. Use that
durable repository evidence to make the feature-specific recommendation in the
current chat: extend the foundation, modernize a bounded slice alongside the
feature, or stabilize the foundation first. Do not create another review or
approval surface. The projection is advisory, grants no rewrite authority, and
does not block urgent containment, correctness, security, or recovery fixes.
Stale, refresh-required, or incomplete evidence requires review or refresh; it
is never `ready_to_extend` by inference.

## Workflow routing

### Daily portfolio triage

Run `route --json` first, followed by scoped `repo` or `quality` projections
only when the bounded envelope is insufficient. Use `summary` and `attention`
when the task needs the complete fleet view. Rank work by explicit user scope
and evidence severity; do not treat the largest count as the highest-priority
task. Report the route/doctor status, snapshot timestamp, freshness window, and
any stale or unavailable sources.

### Current repository work

Resolve the Git root, run `route <root> --json`, and inspect its repository,
quality, and attention evidence for workspace or sync conditions. Refresh only
that repository when evidence is stale. Use the repository's own `AGENTS.md`,
context packet, tests, and quality ladder for implementation details; Pronto
does not replace them.

Before non-trivial mutation, inspect the live checkout rather than treating
Pronto's snapshot as ownership clearance. If the checkout contains pre-existing,
owner-active, mixed, or overlapping work outside the task, invoke
`$isolated-change-workflow` and continue in its exact-base task worktree. Preserve
the primary checkout unchanged. Do not accept `pre-existing`, `unrelated`, or
`overlapping` as a complete commit blocker unless the isolation verifier reports
an exact conflict or named custody constraint.

### Repository Telescope

Use `telescope <repository> --json` when a task needs the repository's
architecture, implementation relationships, static flows, or a common graph
onto which Pronto workflow evidence can be projected. Use `telescope refresh
<repository> --json` when the caller explicitly needs regeneration or when
the branch, commit, staged, unstaged, or untracked workspace content changed.

Consume `groups`, `nodes`, `edges`, `flows`, `actions`, `actors`, `payloads`,
and semantic `scopes` together with `map_readiness`, `blocking_gaps`,
`enhancement_gaps`, `knowledge_tasks`, the readiness receipt, workspace
binding, coverage, warnings, confidence, and provenance. TypeScript,
JavaScript, and Rust receive semantic extraction; generic-language topology is
partial by contract. Derived summaries are interpretation, not source proof.
The local cache may contain topology, summaries, hashes, and
repository-relative anchors, but never source bodies, diffs, runtime values,
credentials, or absolute personal paths.

When `.pronto/telescope-map.json` is present, consume its
`pronto-telescope-map/v2` narrative as an authored layer over the measured
substrate. Districts, city roles, visual archetypes, actors, payload metaphors,
structured explanations, and behavioral stories may be draft or reviewed.
Counts, coverage, geometry inputs, source anchors, and relationship existence
remain extractor-owned. Missing, draft, stale, partial, inferred, and unknown
meaning must remain visibly uncertain; Telescope refresh updates measured
fields and opens targeted drift gaps but never rewrites authored prose, review
state, or layout decisions.

Preserve the categorical readiness states `unavailable`, `measured`,
`needs_information`, `reviewable`, `reviewed`, and `stale`. When publication is
blocked, present the Map Workshop and an explicitly incomplete measured preview
instead of a finished-looking city. Route `telescope_readiness` knowledge tasks
through Pronto's existing remediation and attention handoffs; do not create a
second task store. A task answer updates only draft manifest fields and never
marks the map reviewed. Quality Runner remains the owner of holistic maturity;
Telescope contributes only the developer-legibility architecture-visibility
readiness receipt.

The visual surface is a read-only, isometric living city rather than a generic
box graph. Building counts are adaptive: semantic completeness and readable
composition decide scope, not quotas. Expect distinct silhouettes, typed rails,
source-backed people and crews, labeled payloads, gates, stores, workshops,
ports, district-level clustering, and stable geometry across levels. Buildings
are fixed, Fit frames the active semantic scope, and City, District, and
Building views must be materially different. Source detail is building-local
with progressive evidence; never render the repository-wide file graph there.
Workflow lenses dim and annotate only source-mapped evidence and never redefine
the canonical topology.

The action catalog is a projection over the canonical
`.pronto/behavior-assurance.json` v2 contract and Quality Runner receipts, not
a second behavioral inventory. Authored actions may link a `behaviorId` to
inherit the contract's scenarios; declared behaviors without a visual mapping
appear as review actions; unlinked flow or entrypoint actions remain explicitly
unprofiled. Use `action_coverage` and linked evidence states to distinguish a
helpful visual explanation from verified behavior. Selecting an action opens an
action-specific scope and guided city story, but never executes the operation
or bypasses an existing guarded handoff. Natural-language action routing ranks
the same local projected labels, explanations, evidence, and source anchors;
direct and related matches remain explicit, and the router never calls a model
or creates a new behavior or action.

Telescope is read-only with respect to the repository. An editor link,
remediation action, preparation step, or release question must hand off to the
existing guarded surface. Changes, quality, remediation, delivery, activity,
Compass, and ICM are independent lenses over the source graph; none grants
mutation authority or changes the canonical topology.

### Branch cleanup and folding

1. Resolve the target repository and run `route <root> --json`; stop if it is
   blocked. If the fold spans repositories, run fleet `route --json` as an
   additional gate. Use `summary --json` and `attention --json` when the fold
   needs complete fleet evidence.
2. Inspect the target repository with `repo <root> --json`; refresh it if the
   evidence is stale.
3. Use the route's bounded `fold_preview` first. Run direct
   `fold preview <root> --json` (pass `--target <branch>` when the canonical
   integration target is not the observed default branch) when the route's
   preview is insufficient. Record
   each candidate's decision, target confidence, dirty/activity state, and
   preservation reason. Direct preview is paginated; walk `next_cursor` with
   the same scope and target until `has_more` is false when completeness is
   required. A stale or cross-scope cursor fails closed and requires a new
   preview.
4. Invoke `$fold-feature-branches` for live branch classification and reviewed
   integration. Let that workflow inspect live refs, worktrees, ancestry, patch
   equivalence, and remote truth.
5. Preserve dirty, unique, unpublished, and ambiguous work. Prune only after
   the fold workflow proves integration or patch equivalence and the user has
   authorized pruning.
6. Refresh Pronto after the Git operation and report the before/after evidence.

Pronto inventory is not a substitute for the fold workflow's live Git checks.
The existing target branch, remote, worktree, or agent manifest may have moved
since the snapshot was generated.

### Quality and remediation triage

Run `quality [<repository>] --json`, then `attention --json`. Use `activity`
when the provenance of a finding matters. Read `remediation [<repository>] --json`
only when the user asks about the remediation plan. Treat configured
ideal gates and imported maturity as recommendations/evidence, not fresh
passing execution. Before starting or resuming remediation, run
`remediation gate <repository> --json`; its live execution blockers and
`blocked_operations` are authoritative for repository state, while the plan's
`status` and `closure_gate` describe queue closure. A ready gate does not infer
caller authorization. Use `remediation set-status` only for an explicitly
requested, exact action-status change.

### Preparation and release readiness

Run `prepare <repository> --json` for the combined preflight and
`release preview <repository> --json` for release-specific evidence. A preview
can be blocked, unavailable, or incomplete. It does not create a worktree,
generate files, commit, push, open a pull request, or publish a release.
Keep those actions behind their explicit user and provider boundaries.

Both commands use the persisted Pronto snapshot by default. Add `--fresh` only
when the task requires a bounded live quality import; fresh projection and
release-history inspection each have a 10-second deadline, and release history
is capped at 1,000 commits. Treat timeout or Git failure as explicit unavailable
evidence, never as an empty history.

For web release requirements, inspect the imported
`quality-runner-web-readiness/v1` report, its exact target identity, and its
per-check verification levels. Configure a minimum level from
`source_inferred`, `artifact_inspected`, `browser_rendered`, or
`deployment_verified`, plus `policy: block` or `policy: warn`. Never let a
source-inferred pass satisfy a deployment-verified requirement; warning policy
must remain visible in the trace without becoming a blocker.

## Handoff format

End a Pronto-backed handoff with:

- scope and the `generated_at` timestamp;
- doctor status, freshness window, and any blocking checks;
- repositories/workspaces inspected and the exact focused commands used;
- observed conditions, quality evidence, and freshness/uncertainty;
- the next safe action and the evidence required before it;
- any user authorization still needed for Git, provider, or release mutation.

For Compass orientation use the bounded `compass family WORKSPACE --summary --json`
then affected `change-context` drill-down. Preserve unknown behavior inventory
and bootstrap gaps as uncertainty; linked intent is not verified behavior.
Across task handoffs retain the original base and previous packet through
`change-context --prepared PREVIOUS --continue`; do not reset the effort to hide
unfinished preservation obligations.

Continuation requires an eligible predecessor and retains original base, cumulative paths and active scoped continuity commitments. A failed assessment is not a replacement intent baseline. Reconcile semantic contradictions between commitments and contract text before development; the helper does not infer prose equivalence. Legacy packets lacking commitment/behavior revisions require explicit preparation.
