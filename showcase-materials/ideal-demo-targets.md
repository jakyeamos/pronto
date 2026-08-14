# AI Showcase ideal demo targets

Last reviewed: 2026-08-13.

This is the canonical creative target for the 34 projects currently marked
`public_showcase` in `.pronto/showcase-goal.json`. The 18 new developer
tooling repositories are independently versioned public release targets and are
included here with gated proof states. BidCamp is retained as an excluded
inventory record because it is client work, Soundscape is excluded by owner
decision, TMCP is deferred behind its product-direction gate, and ESLint
Anti-Slop is retained as a supporting package inside the AI Code Quality Stack.
The durable inventory is now 38 projects: 34 active public targets plus the
four excluded/supporting records.
It defines the story that the preview, description, case-study page, and proof
artifacts should all tell. An optional recording should tell the same story. It
does not grant publication authority or replace the readiness and
eligibility judgments in the Pronto contract.

All projects move through the canonical
[material production route](../docs/showcase-contract.md#material-production-route).
These targets are intentionally aspirational: concept materials can lead the
implementation as long as unbuilt behavior stays labeled until its proof gate
passes.

## Shared quality bar

An ideal demo is the smallest honest end-to-end story that lets a new viewer:

1. understand the problem and consequence in five seconds;
2. see a real interaction or a clearly labeled synthetic case;
3. distinguish creator decisions, AI runtime work, and human review;
4. inspect one decisive result or evidence receipt;
5. understand the material limitation or safety boundary; and
6. reach the core proof without authentication and within one intentional click.

Video is an optional enhancement when motion materially improves understanding
or uniquely proves behavior. It is not a publication gate, and human narration
is not required. Prefer a compact case-study page when video would mostly show
terminal output or static text. A visible refusal, failed gate, or bounded
unknown can be stronger proof than a manufactured clean outcome.

## Mac Control

[Steps 1–3 route plan](mac-control/route-plan.md)

**Promise.** An agent performs a bounded macOS action without hiding who
authorized it or whether it worked.

**Opening state and audience.** Show AI product, agent-platform, and systems
teams a safe, reversible task pending in the control surface, with the target
application and requested effect visible. Use a clean demo account and redact
private paths, tokens, and unrelated application state.

**Demo beats.** State the intent; show the proposed action and scope; obtain
human approval; execute once; verify the postcondition independently; finish on
the redacted receipt. Add a brief denied or stale request if it does not break
the main story.

**Proof moment.** The native UI reaches the requested state and the receipt
reports a verified outcome tied to the approved action.

**Ownership.** The creator designed the control, authorization, and verification
model. The agent plans and executes within it. The human owns approval and
consequential judgment.

**Proof gate.** One redacted end-to-end run passes on the current demo build;
receipt and visible result agree; the action is reversible; and no installed or
live behavior is inferred from source tests.

## BidCamp

[Steps 1–3 route plan](bidcamp/route-plan.md)

**Showcase status: excluded.** This is client work. Do not create, publish, or
derive showcase materials from its repository or data. This inventory tombstone
exists only to prevent the project from being routed back into showcase work.

## Quality Runner

[Steps 1–3 route plan](quality-runner/route-plan.md)

**Promise.** A repository's own quality values become scoped, explainable
findings, then a compact remediation decision and machine-readable proof on the
exact target revision.

**Opening state and audience.** Show developer-tools and engineering-productivity
teams how a repository opts into local standards packs, scopes where each value
applies, and retains the originating skill and rule on every finding. Then show
the real Tenure reconciliation: a large provenance-bound candidate set,
meaningful branch risk, and no safe reason to treat scanner output as a cleanup
checklist. Keep controlled examples as reproducibility evidence, not substitutes
for the real case.

**Demo beats.** Start by showing the customer that the same Skills and agent
rules that guide output can be compiled through reviewed ingest into prevented
outcomes, deterministic checks or agent-review rubrics, concrete findings, and
a remediation plan. Then show the actual Tenure repository signals and eight
selected Quality Skills; trace pack, rule, scope, finding, and coverage into the
4,022-row baseline; show the bounded slice and full-refresh cadence; then land
on the final exact comparison of 537 raw rows and 0 open actionable findings.
Use the later rejected bulk-reconciliation shortcut as a supporting vignette.

**Proof moment.** The audience can see why a finding exists and which codebase
value selected it. The terminal receipt then keeps `537 raw` beside `0 open`,
proving that Quality Runner reconciles evidence instead of gaming warning counts.

**Ownership.** The creator or team owns what quality means, which standards are
active, where they apply, and the repair policy. Quality Runner evaluates and
preserves provenance. The human owns semantic disposition, repair, and release.

**Proof gate.** The actual Tenure skill selection, scoped findings, coverage,
checkpoint counts, and exact historical revisions agree; raw rows remain
distinct from open actionable findings; the dirty baseline and missing final
run directory remain explicit; and neither historical inspect evidence nor a
tracked branch record is presented as fresh gate or deployment proof.

## Chiron's Forge

[Steps 1–3 route plan](chirons-forge/route-plan.md)

**Promise.** A raw domain need becomes a portable expert artifact through
parallel research, independent judgment, and visible refinement.

**Opening state and audience.** Show applied-AI, agent-infrastructure, and
research-product teams a real, consequential but public-safe request plus an
optional source or data packet. State the desired output format and success
criteria before the build begins.

**Demo beats.** Scope the request; add grounding material; show multiple
research models investigating in parallel; expose synthesis and gap checking;
show a different model judging the candidate and triggering refinement; then
download the finished SKILL.md, research report, or Cursor rule.

**Proof moment.** The downloaded artifact is tied to its sources, judge scores,
and refinement history, and the final version visibly resolves a weakness in
the first candidate.

**Ownership.** The creator owns the orchestration, evaluation rubric, privacy
boundary, and product experience. Models research, draft, judge, and refine.
The human owns the scope, uploaded material, publication, and downstream use.

**Proof gate.** An authenticated run completes on the current build; the judge
is a different model from the candidate generator; scores and iterations are
inspectable; the downloaded artifact opens or runs; applicable redaction and
deletion behavior has a receipt or direct readback; and the deployed revision
is known.

## Research Domain Writing

[Steps 1–3 route plan](research-domain-writing/route-plan.md)

**Promise.** Research becomes useful prose while unsupported claims are traced,
qualified, or stopped before publication.

**Opening state and audience.** Show applied-AI, research, and trust teams a
compact source packet containing supported facts plus one plausible but
unsupported claim that a weaker workflow might accept.

**Demo beats.** Ingest the packet; map sources to claims; draft with claim
labels; show QA rejecting or narrowing the unsafe claim; finish on the corrected
draft and claim ledger.

**Proof moment.** The tempting unsupported claim does not survive unchanged,
and accepted factual claims link back to evidence.

**Ownership.** The creator owns the research and claim-safety method. AI assists
synthesis and drafting. The human owns source choice, judgment, voice, and
publication.

**Proof gate.** Positive and refusal paths reproduce from the same packet,
citations resolve, and the final prose contains no unsupported factual claim.

## Soundscape

[Steps 1–3 route plan](soundscape/route-plan.md)

**Showcase status: excluded by owner decision.** Do not create or route AI
Showcase materials for this project. This inventory tombstone preserves the
decision without changing the project's separate product or quality status.

## TMCP

[Steps 1–3 route plan](tmcp/route-plan.md)

**Showcase status: deferred by product direction.** The prior bounded
release-readiness packet story is not the target: it materially undershoots the
intended always-on atomic-node system. Do not create or rehearse Showcase
materials until the owner-approved atomic-node behavior is specified and a
representative flow is verified. At that point, redefine the ideal demo around
the proven system rather than reviving the narrower compiler storyboard.

## ESLint Anti-Slop

[Steps 1–3 route plan](eslint-anti-slop/route-plan.md)

**Showcase status: supporting component.** Retain the independent ESLint
package, but do not build or rehearse a standalone card. Anti-Slop owns precise,
offline AST detection and line-level JS/TS diagnostics inside the combined
[AI Code Quality Stack](#ai-code-quality-stack). Its reviewed
[Pronto case](eslint-anti-slop/case-study.md) remains integration evidence and
a rule-quality input.

## Pre-CR Suite

[Steps 1–3 route plan](pre-cr-suite/route-plan.md)

**Promise.** Pre-CR is an IDE-native pre-review workspace: it restores the
developer's branch context, brings useful commands to the current change, makes
coverage and review state visible, and proves readiness before review.

**Opening state and audience.** Show engineering-productivity and code-quality
teams a developer returning to a real branch after an interruption. Several
files are open and the change is unfinished.

**Demo beats.** Invoke **Where Was I?**; show the saved branch summary and jump
back to the relevant file; restore the saved editor state; open Pre-CR Quick
Actions; inspect changed-line coverage and the PR checklist; run Pre-CR Check;
show an uncovered changed line blocking readiness; add the missing focused test;
rerun; and finish with the editor state and readiness evidence aligned.

**Proof moment.** A developer returns cold, recovers the exact working context,
acts through the editor, and reaches a passing pre-review receipt. The same
workspace supplies the context, commands, diagnostics, and visible proof.

**Ownership.** Pre-CR owns developer continuity, editor interaction,
changed-line coverage, review assistance, changed-file execution, and
pre-review policy. The human owns acceptance and merge decisions.

**Proof gate.** Where Was I?, Save Snapshot, Restore Snapshot, Quick Actions,
changed-line coverage, and Pre-CR Check are exercised in the installed VS Code
extension on one safe fixture; context restoration is accurate; the uncovered
line and repair reproduce locally; and no merge, push, or remote review action
is implied.

## AI Code Quality Stack

[Steps 1–3 route plan](ai-code-quality-stack/route-plan.md)

**Showcase status: combined portfolio story.** This does not replace the
standalone Pre-CR or Quality Runner demos. It shows the narrow seam where three
independent products cooperate.

**Promise.** One generated-code problem moves through a coherent local quality
stack: Anti-Slop detects it, Pre-CR enforces it against the changed-file set,
and Quality Runner adds repository context and remediation order without
duplicating the finding.

**Demo beats.** Run Anti-Slop on one reviewed real case; invoke the same adapter
through Pre-CR; show the changed-file block and attributable receipt; import the
receipt into Quality Runner; repair the line; rerun all three layers; show
advisory fallback and dual-source deduplication.

**Proof gate.** Ownership, fallback, failure, and deduplication behavior pass;
the exact-revision gap and repair reproduce locally; provenance crosses all
three layers; and the stack claims no ownership over Pre-CR's IDE suite or
Quality Runner's broader audit product.

## Terrace

[Steps 1–3 route plan](terrace/route-plan.md)

**Promise.** A spec-driven workflow stops at a real failed quality gate, supports
a bounded correction, and resumes without bypassing evidence.

**Opening state and audience.** Show agent-workflow and software-delivery teams
a small specification and fixture with one intentional, understandable failure.

**Demo beats.** Start the workflow; show its stages; reach the failing gate;
display the exact failure and stop; make a local correction; resume from the safe
checkpoint; finish on passing evidence.

**Proof moment.** The workflow refuses to advance while the gate fails and
resumes only after the owning validation passes.

**Ownership.** The creator owns orchestration and gate semantics. AI plans and
performs bounded work. The human owns corrections, exceptions, and release.

**Proof gate.** Stop and resume are deterministic, the failure is not merely
mocked for recording, and no hook or permission is bypassed.

## Context Compiler Contract

[Steps 1–3 route plan](context-compiler-contract/route-plan.md)

**Promise.** A real AIOS context result remains portable only when its source
reason and route boundary survive validation; two small mutations fail with
exact contract reasons and the corrected result passes.

**Opening state and audience.** Show agent-infrastructure and protocol teams a
compact projection of a real AIOS compile result, then remove one source
reason and flip route compatibility for the two invalid states.

**Demo beats.** Show the real baseline; apply the labelled mutations; show each
validator failure and path; restore the fields; revalidate; compare invalid and
valid artifacts.

**Proof moment.** The corrected artifact passes for the right reason and makes
its selection reasons, route boundary, and runtime ownership inspectable.

**Ownership.** The creator owns the contract and validation semantics. The
validator enforces them. The human or caller owns source choice and remediation.

**Proof gate.** The real baseline plus two minimal mutations are reproducible,
errors point to actionable fields, and runtime behavior owned elsewhere is not
attributed to this package. Browser capture and video are optional.

## Portable Agentic Workbench

[Steps 1–3 route plan](portable-agentic-workbench/route-plan.md)

**Promise.** The public `safe-tool-guards` workflow can be previewed and
projected into generic and Codex target roots without erasing the manual host
registration boundary.

**Opening state and audience.** Show agent-platform and developer-experience
teams two clean target roots and one small public workflow pack with explicit
public/private boundaries.

**Demo beats.** Inspect the manifest; dry-run `safe-tool-guards` in generic;
apply only the two allowlisted files; repeat in Codex; compare statuses and
hashes; show the overwrite refusal; preview and apply receipt-scoped recovery
with an unrelated sentinel; then show the manual-review note.

**Proof moment.** The two native portable projections have identical files and
relative destinations while host registration remains explicitly manual. The
receipt-scoped uninstall returns unchanged manifest files to baseline,
preserves an unrelated file, and blocks a modified file. The attempted
host-runtime probe is shown as a declared blocker rather than provider parity.

**Ownership.** The creator owns portability contracts, manifests, and validation.
The installer projects allowlisted files. The host and human own persistent
registration, credentials, provider access, and environment changes.

**Proof gate.** Both clean target previews are non-mutating, apply receipts,
overwrite refusal, and receipt-scoped recovery are saved, hashes show
install-contract parity, and the runtime probe records the exact blocked and
unavailable statuses. PW-4 remains a blocked follow-up owned by the runtime or
provider surface.

## AI Workflow Leverage

[Steps 1–3 route plan](ai-workflow-leverage/route-plan.md)

**Promise.** A bounded AI workflow improves a real task in a way that survives
fair before/after measurement rather than a productivity anecdote.

**Opening state and audience.** Show AI operations, product, and analytics teams
one repeated task, a fixed input set, a shared quality oracle, and a baseline
measuring the same scope as the assisted run.

**Demo beats.** Show the baseline; run the assisted workflow; inspect quality
and recovery; compare paired time, effort, and outcome measures; name confounders
and what the sample cannot establish.

**Proof moment.** A paired result shows a meaningful gain without quality loss,
with raw observations available behind the summary.

**Ownership.** The creator owns experiment design and interpretation. AI
performs the assisted workflow. The human owns the oracle and result claims.

**Proof gate.** At least one comparable before/after run is complete, scope
and uncertainty are explicit, and missing telemetry remains unknown rather than
estimated. Measurement comes before production materials.

## Marketing Autoresearch

[Steps 1–3 route plan](marketing-autoresearch/route-plan.md)

**Promise.** A campaign question produces a sourced report while a fail-closed
boundary keeps unreviewed material from publication.

**Opening state and audience.** Show marketing-operations and AI-safety teams a
sanitized brief, approved public sources, and a shadow or report-only environment
with no live publishing authority.

**Demo beats.** Start the research run; inspect evidence; generate the report;
surface one uncertain claim; reach the publication gate; show that it remains
closed pending human review.

**Proof moment.** The report is useful and traceable, but no post, email, or
campaign mutation occurs without approval.

**Ownership.** The creator owns research policy and publication controls. AI
researches and drafts. The human owns sources, facts, brand, and publishing.

**Proof gate.** One shadow run completes, sources resolve, the blocked
publication state is independently visible, and the demo account has no
unneeded external mutation authority.

## RemodelVision

[Steps 1–3 route plan](remodelvision/route-plan.md)

**Promise.** A room image and renovation constraints become a visually useful
concept and estimate-oriented artifact that a person can critique.

**Opening state and audience.** Show applied computer-vision and consumer-product
teams an owned or synthetic room image, a few constraints, and a verified current
public or local flow.

**Demo beats.** Upload the room; specify the change and constraints; generate
the concept; compare before and after; inspect or adjust the estimate or
recommendation.

**Proof moment.** The output respects a meaningful input constraint and supports
a concrete next decision instead of presenting an unexplained image.

**Ownership.** State the creator's exact contribution and collaborator ownership.
Separate model generation from product design, implementation, and human judgment.

**Proof gate.** The current flow runs end to end, attribution is reviewed,
input rights are clear, and generated concepts are not presented as construction
or pricing guarantees.

## Dsci-proj

[Steps 1–3 route plan](dsci-proj/route-plan.md)

**Promise.** A crowded backlog becomes an explainable priority queue whose
criteria, weights, uncertainty, and source evidence remain visible and editable.

**Opening state and audience.** Give a product, engineering, research, or OSS
owner a rights-safe backlog with competing work and no agreed ordering. The
original research backlog is the first use case, not the product boundary.

**Demo beats.** Import and normalize the backlog; select or define evaluation
criteria; inspect the ranked queue; open one item's score explanation; adjust a
weight or assumption; compare the changed ordering; export the chosen next-work
queue with its decision provenance.

**Proof moment.** The same engine prioritizes two meaningfully different
backlogs without code changes, and a reviewer can explain why an item moved.

**Ownership.** The user owns criteria, weights, constraints, and the final
decision. The system calculates and explains a recommendation; it does not
silently define what matters or claim objective priority.

**Proof gate.** Configurable criteria and schema mapping work across at least
two backlog types, every rank is traceable to inputs and assumptions, and
weight changes produce deterministic, reviewable results.

## Book

[Steps 1–3 route plan](book/route-plan.md)

**Promise.** A written chapter becomes an immersive reading scene where motion
and layered audio support the text instead of competing with it.

**Opening state and audience.** Show creative-tool, interactive-media, and design
engineering teams one creator-owned chapter with a strong transition, loaded in
a public reader or no-auth case study.

**Demo beats.** Begin in the clean reading view; move through the passage;
reveal motion and layered audio; show mute, reduced motion, or navigation;
briefly reveal the authoring model behind the scene.

**Proof moment.** Text, motion, and audio resolve into one coherent authored
moment while the reader retains control.

**Ownership.** State who wrote the text and directed the work. Separate
AI-assisted implementation or assets from authorship, editing, and composition.

**Proof gate.** The chapter loads reliably, synchronization survives a fresh
session, accessibility controls work, and text and assets have clear rights.

## Agent Router

[Steps 1–3 route plan](agent-router/route-plan.md)

**Promise.** A complex request is decomposed and routed with inspectable
alternatives, then returned as a bounded result with an explanation receipt.

**Opening state and audience.** Show multi-agent and AI-systems teams one
realistic multi-part task, a small capability catalog, and explicit cost,
quality, authority, or latency constraints.

**Demo beats.** Submit the task; show its typed task graph; inspect candidate
routes and scores; execute the selected route; synthesize the result; open the
receipt and fallback rationale.

**Proof moment.** The receipt explains why the chosen route beat a credible
alternative and ties each result to its assigned subtask.

**Ownership.** The creator owns routing policy, contracts, and evidence design.
Providers perform assigned work. The human owns authority and acceptance.

**Proof gate.** One replayable case completes through the supported surface,
comparison evidence is real, and incomplete provider or worktree coverage stays
visibly incomplete.

## Codex Browser Control

[Steps 1–3 route plan](codex-browser-control/route-plan.md)

**Promise.** An agent performs one bounded browser action only after human
approval, then refuses when page state is stale.

**Opening state and audience.** Show browser-agent, security, and trust teams a
synthetic local site or disposable demo account, the side panel, and a harmless
reversible action with a clear precondition.

**Demo beats.** Observe the page; inspect the target; present the exact plan;
obtain approval; apply once; refresh and verify; replay an old plan against
changed state and show refusal.

**Proof moment.** The approved action succeeds and verifies while the stale plan
is rejected before mutation.

**Ownership.** The creator owns browser architecture, approval, stale-state
defense, privacy, and verification. The agent acts inside those bounds. The
human owns approval and external-account consequences.

**Proof gate.** The installed Chrome round trip passes on synthetic data,
redacted receipts agree with the page, and no real account action or private
browser state enters the materials.

## Participant Deduplication

[Steps 1–3 route plan](participant-deduplication/route-plan.md)

**Promise.** Possible duplicate participants become an explainable review queue,
and approved cleanup occurs only on a recoverable copy with an audit trail.

**Opening state and audience.** Show operations, data-quality, and human-in-the-loop
teams a synthetic workbook with exact matches, fuzzy matches, and one ambiguous
non-duplicate. Preserve the original.

**Demo beats.** Scan the sheet; open candidate reasons and confidence; reject
the ambiguous pair; approve true duplicates; apply cleanup to a copy; inspect
audit history and recovery.

**Proof moment.** Only reviewer-approved rows change in the copy, the original
remains untouched, and every action is recorded.

**Ownership.** The creator owns matching, review, stale-state, and recovery
design. The system proposes and applies approved operations. The human owns
identity judgment and deletion approval.

**Proof gate.** Authenticated live-sheet UAT or a clearly labeled synthetic
equivalent passes, ambiguous matches remain reviewable, stale-state refusal
works, and copy-only recovery is shown.

+## Automation Flight Recorder

[Steps 1–3 route plan](automation-flight-recorder/route-plan.md)

**Promise.** A developer can inspect one bounded automation run as a causal, redacted receipt and safely rerun only the declared step.

**Opening state and audience.** Show developer-tools and reliability teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** State the workflow; show parent and child actions; expose timing, redaction, omissions, and hashes; inspect a failed step; preview a safe rerun.

**Proof moment.** The receipt preserves causality, omissions, and rerun eligibility without a telemetry backend.

**Ownership.** The creator owns instrumentation and privacy; the recorder records declared local actions; the human owns rerun authority.

**Proof gate.** A Gateboard trace completes on the current build, failure and cancellation remain distinct, and the receipt is target-bound.

## Behavior Coverage Atlas

[Steps 1–3 route plan](behavior-coverage-atlas/route-plan.md)

**Promise.** A developer maps named product behaviors to tests and sees strong, weak, stale, missing, and unknown evidence without collapsing the result into line coverage.

**Opening state and audience.** Show engineering productivity and quality teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Declare three behaviors; link tests; run one bounded command; compare assertion strength and freshness; navigate one gap into review evidence.

**Proof moment.** A behavior with no assertion is visibly different from a behavior with a passing assertion.

**Ownership.** The creator owns the behavior contract; the test runner supplies execution evidence; the human owns the meaning of coverage.

**Proof gate.** The three-behavior fixture reproduces every claimed evidence class and one signal reaches the originating receipt.

## Change Integration Simulator

[Steps 1–3 route plan](change-integration-simulator/route-plan.md)

**Promise.** A reviewer previews a source-to-target integration, inspects conflicts and one declared gate, and leaves both branches untouched.

**Opening state and audience.** Show engineering productivity and release-safety teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Resolve exact source and target commits; preview a disposable merge; show clean and conflict outcomes; run one gate; retain uncertain state.

**Proof moment.** Textual mergeability and behavioral verification remain separate receipts.

**Ownership.** The creator owns the simulation boundary; Git supplies merge-tree facts; the human owns integration and publication.

**Proof gate.** Clean, conflict, stale, cancellation, and retained-dirty cases pass without ref mutation.

## Change Radius

[Steps 1–3 route plan](change-radius/route-plan.md)

**Promise.** A reviewer follows one changed TypeScript symbol to known consumers and tests while seeing dynamic, generated, external, and reflective unknowns explicitly.

**Opening state and audience.** Show reviewers and maintainers a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Select a changed symbol; render provider-backed edges; inspect known consumers and tests; open unknown boundaries; save a receipt.

**Proof moment.** The map is useful because it refuses to call partial evidence a complete blast radius.

**Ownership.** The creator owns graph presentation; providers supply bounded edges; the human owns scope and follow-up.

**Proof gate.** A real TypeScript fixture produces a target-bound graph with explicit unknown classes and downstream links.

## Contract Watch

[Steps 1–3 route plan](contract-watch/route-plan.md)

**Promise.** A developer compares two explicit OpenAPI contracts, sees certainty and policy consequence separately, and records a human disposition for each change.

**Opening state and audience.** Show API and platform teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Load a baseline; compare operations; classify certainty; inspect affected consumers and unknowns; record acknowledge, mitigate, or verify.

**Proof moment.** The human disposition is visible beside the semantic contract change.

**Ownership.** The creator owns compatibility policy; the declared contract is authoritative; the human owns consequence decisions.

**Proof gate.** A real change set, stale baseline, and undocumented-consumer boundary reproduce with source-linked receipts.

## Debug Trail

[Steps 1–3 route plan](debug-trail/route-plan.md)

**Promise.** A developer leaves a bounded investigation with its hypothesis, experiment, evidence, and rerunnable next step so an inheritor can continue without reconstructing mental state.

**Opening state and audience.** Show debugging, incident, and developer-experience teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Record a hypothesis; preview one allowlisted command; classify the result; preserve redacted evidence; open the continuation receipt.

**Proof moment.** An inconclusive or failed experiment remains useful instead of becoming a false conclusion.

**Ownership.** The creator owns continuity and redaction; the command runner supplies bounded evidence; the human owns interpretation and rerun authority.

**Proof gate.** The canonical branch exposes a human-complete success, stale, failed, and rerun path with target identity.

## Deletion Proof Workbench

[Steps 1–3 route plan](deletion-proof-workbench/route-plan.md)

**Promise.** A developer previews and verifies one bounded deletion while preserving unknown consumers, explicit authorization, and recovery evidence.

**Opening state and audience.** Show maintainers and code-safety teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Select an exported symbol; inspect references and unknowns; preview deletion; authorize apply; run verification; open recovery.

**Proof moment.** The safest result may be “unknown consumers remain,” not a forced approval.

**Ownership.** The creator owns the mutation boundary; static tools supply bounded references; the human owns deletion approval.

**Proof gate.** One real export deletion passes with unknown consumer classes and a recoverable prior state.

## Evidence Replay

[Steps 1–3 route plan](evidence-replay/route-plan.md)

**Promise.** A developer opens a local receipt, checks target freshness, previews a rerun, and preserves unknown or stale evidence without silently executing it.

**Opening state and audience.** Show developer-tools and inheritor workflows a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Open receipts from two producers; inspect source and target; classify freshness; preview an explicit rerun; preserve omissions.

**Proof moment.** Inspection is always safe and execution is always a separate human action.

**Ownership.** The creator owns the reader; producers own their evidence; the human owns rerun authority.

**Proof gate.** Rule Lab and Debug Trail receipts open through the same reader with stale, unknown, cancelled, and rerun cases intact.

## Failure Capsule

[Steps 1–3 route plan](failure-capsule/route-plan.md)

**Promise.** A failed declared command becomes a portable, redacted capsule that an inheritor can inspect without receiving the entire machine state.

**Opening state and audience.** Show developer-tools and incident handoff teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Capture one failure; redact and mark omissions; inspect the capsule; link it to replay; leave a next action.

**Proof moment.** The capsule explains what failed and what was intentionally not captured.

**Ownership.** The creator owns capture and privacy; the command producer owns the failure; the human owns any rerun.

**Proof gate.** A Debug Trail failure reaches Replay with redaction, cancellation, and recovery evidence.

## Fleet Radar

[Steps 1–3 route plan](fleet-radar/route-plan.md)

**Promise.** A maintainer sees two repositories' freshness, blockers, and next actions and refreshes one read-only status without inventing an aggregate score.

**Opening state and audience.** Show portfolio maintainers and platform teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Load two registered repositories; inspect per-check evidence; follow native links; refresh one status; compare before and after.

**Proof moment.** Individual evidence stays explainable; no single readiness score hides missing data.

**Ownership.** The creator owns the projection; Pronto and repositories own evidence; the human owns follow-up decisions.

**Proof gate.** Two real repositories and one read-only refresh preserve passing, stale, blocked, missing, and unavailable states.

## Quality Lens

[Steps 1–3 route plan](quality-lens/route-plan.md)

**Promise.** A developer understands one Quality Runner finding in the IDE, sees its source and disposition, reruns it, and hands evidence to remediation without an opaque score.

**Opening state and audience.** Show engineering productivity and code-quality teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Open a finding; navigate to source; inspect provenance; record a disposition; rerun; hand off to remediation.

**Proof moment.** The IDE shows the same facts as the headless contract and keeps stale or failed evidence visible.

**Ownership.** The creator owns the human-facing lens; Quality Runner owns finding semantics; the developer owns repair and disposition.

**Proof gate.** The canonical product path proves one direct Problems-panel finding flow with exact task-owned postconditions.

## Quality Setup

[Steps 1–3 route plan](quality-setup/route-plan.md)

**Promise.** A developer previews a supported Quality Runner setup, applies it explicitly, verifies the result, and can recover through a rollback receipt.

**Opening state and audience.** Show developer-tools and repository enablement teams a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Inspect ecosystem support; preview setup; apply with authority; verify; show conflict refusal; roll back.

**Proof moment.** Installation is an explicit, recoverable action rather than an invisible prerequisite.

**Ownership.** The creator owns setup policy; Quality Runner owns the installed capability; the human owns mutation authority.

**Proof gate.** One supported ecosystem completes preview, apply, conflict refusal, verification, and rollback.

## Readiness Inspector

[Steps 1–3 route plan](readiness-inspector/route-plan.md)

**Promise.** A maintainer answers whether one repository is ready for a declared goal through individually explainable checks, owners, predicates, evidence, and next actions.

**Opening state and audience.** Show maintainers and inheritors a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Choose a goal; inspect prerequisites; run one safe check; distinguish missing, stale, blocked, and not-applicable; save a receipt.

**Proof moment.** The product explains readiness without an aggregate score.

**Ownership.** The creator owns check composition; repositories own evidence; the human owns the declared goal.

**Proof gate.** C5 evidence from upstream tools renders as individually inspectable checks with native links.

## Remediation Canvas

[Steps 1–3 route plan](remediation-canvas/route-plan.md)

**Promise.** A developer groups related findings, states intent, verifies once, and leaves a partial or completed remediation handoff without a second issue tracker.

**Opening state and audience.** Show developers resolving related findings a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Import findings; group a cause; write intent; run one verification; mark dispositions; leave a partial handoff.

**Proof moment.** A changed finding set makes the canvas stale rather than silently preserving a false plan.

**Ownership.** The creator owns organization and handoff; Quality Lens owns findings; the human owns remediation and mutation.

**Proof gate.** One real finding set preserves intent, partial work, stale refresh, and optional Debug Trail references.

## Review Attention Map

[Steps 1–3 route plan](review-attention-map/route-plan.md)

**Promise.** A reviewer follows concrete contract and behavior signals onto a diff, sees why an area deserves attention, and navigates to originating evidence.

**Opening state and audience.** Show reviewers and authors a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Load a diff; overlay two signals; inspect reasons; navigate to source evidence; record review dispositions.

**Proof moment.** Attention is a suggestion grounded in evidence, never an opaque risk score or merge verdict.

**Ownership.** The creator owns the review surface; producer tools own signals; the human owns judgment.

**Proof gate.** Two real signals overlay one diff with source, freshness, unmatched, and reviewed states intact.

## Review Sandbox

[Steps 1–3 route plan](review-sandbox/route-plan.md)

**Promise.** A reviewer exercises one repository-declared behavior in disposable state, retains trustworthy evidence, and cleans up only after proving inactivity.

**Opening state and audience.** Show reviewers unfamiliar with a repository a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Preview setup; create an isolated worktree; run a meaningful scenario; inspect evidence; clean up only after inactivity proof.

**Proof moment.** Dirty or uncertain sandboxes remain retained and visible.

**Ownership.** The creator owns isolation and cleanup; the repository declares the scenario; the reviewer owns execution and evidence.

**Proof gate.** Clean, conflict, failed-gate, cancellation, retained-dirty, and safe-cleanup cases pass on the current surface.

## Rule Lab

[Steps 1–3 route plan](rule-lab/route-plan.md)

**Promise.** A developer inspects a quality finding, edits an isolated rule, tests positive and negative fixtures, compares the result, and saves a target-bound receipt.

**Opening state and audience.** Show rule authors, finding recipients, and inheritors a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Open a finding; inspect provenance; edit a draft; run positive and negative fixtures; compare gained/lost matches; save a receipt.

**Proof moment.** A rule edit is not accepted when it merely removes a finding and breaks a counterexample.

**Ownership.** The creator owns the workbench; Quality Runner owns the engine; the human owns rule approval.

**Proof gate.** Two real receipts, C2 replay, and headless/IDE parity prove a false-positive-safe edit.

## Workflow Gateboard

[Steps 1–3 route plan](workflow-gateboard/route-plan.md)

**Promise.** A developer loads repository-declared gates, previews prerequisites and mutation class, runs one non-mutating gate, and sees a continuation-ready receipt.

**Opening state and audience.** Show developers and inheritors a public-safe repository or clearly labeled fixture at the moment the problem becomes consequential.

**Demo beats.** Load three gates; inspect dependencies; preview one command; run it; inspect the receipt; refresh the board.

**Proof moment.** Inspection never executes and local passes never masquerade as hosted CI or release proof.

**Ownership.** The creator owns the board; the repository declares policy; the human owns execution authority.

**Proof gate.** One Gateboard action preserves stale, blocked, failed, and not-run states and reaches Flight Recorder.

## Applying the route

This document owns the ideal story and project-specific proof gate. The
[showcase contract](../docs/showcase-contract.md#material-production-route) owns
the shared stage order. Each project package owns its concept frames, build-gap
specification, evidence, rehearsal notes, and final materials.

If implementation discoveries materially change the promise, update this
target before final production. Do not weaken the target merely to match the
first working flow; make the tradeoff explicit and preserve the stronger idea
as a future build gap when it remains worthwhile.
