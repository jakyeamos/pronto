# Release showcase material inventory

Reviewed: 2026-08-14. This is the joined planning view for the public release
route. It combines:

- `.pronto/showcase-goal.json` for the current material gaps and next closure;
- `public-release-targets.json` for destination-specific artifacts and status;
- the project packages under this directory for local evidence and production
  assets; and
- the Handshake packet ledger for the small set of externally recorded actions.

This is not a publication record. A target marked `planned`, `in_progress`, or
`gated` still needs a real external-state readback before it can be called
posted.

## Inventory totals

- **34** active `public_showcase` repositories are in the release matrix,
  including the 18 DevOps tooling sprint repositories classified on
  2026-08-13.
- **128** current material/gap entries are open across the 34 active targets
  (**43** distinct labels). The 18 new targets are explicitly gated and remain
  non-publishable until their local proof and material packages are complete.
- **161** destination rows exist: **102 required**, **38 recommended**, and
  **21 conditional**.
- Every project has the same three required release slots: **GitHub canonical
  home, no-auth portfolio case, and Handshake package** (102 required rows).
- The local Handshake ledger records repo-link posts for **Pre-CR Suite,
  Quality Runner, and Terrace**. Context Compiler Contract is profile-only with
  its showcase upload blocked. No other external posting is inferred here.

## Material set

Every row must account for these materials, even when a product/evidence gate
means packaging cannot start yet:

| Slot                | What it contains                                                               | Required?                 |
| ------------------- | ------------------------------------------------------------------------------ | ------------------------- |
| Story route         | Ideal target, storyboard, narrative, and limitation                            | Yes, internally           |
| Evidence package    | Real or explicitly synthetic inputs, outputs, receipts, and claim boundary     | Yes                       |
| Public case         | No-auth demo or structured case-study page, with the core proof one click away | Yes                       |
| Preview             | Crop-safe 16:9 image or an explicitly labeled project thumbnail                | Yes for release packaging |
| Short copy          | Verified Handshake description, 500 characters or fewer                        | Yes for Handshake         |
| Role/review         | AI-versus-human role, attribution, rights, privacy, and editorial approval     | Yes where applicable      |
| Walkthrough         | Captioned recording, shot list, or equivalent guided walkthrough               | Optional enhancement      |
| Target copy         | Audience-specific GitHub, technical, community, LinkedIn, and Handshake copy   | Per matrix row            |
| Publication receipt | External URL/status and fresh readback                                         | Required to claim posted  |

`open` below means the current ledger names a gap. `gated` means the material
cannot honestly be treated as release-ready until the active product or
evidence gate closes. `optional` is not a publication blocker.

## Project-by-project grid

The three status cells are the required baseline destinations. The extra-target
cell lists the non-baseline rows from the machine-readable release matrix; it
is a plan, not a posting claim.

| Project                    | Release state | GitHub / Portfolio / Handshake          | Material inventory still open                                                                                                                                                                  | Extra target rows                                               | Next closure                                                                                       |
| -------------------------- | ------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Mac Control                | Gated         | planned / planned / planned             | Preview; no-auth case page; redacted authorization and outcome receipt                                                                                                                         | Product Hunt gated; Reddit planned; LinkedIn planned            | MC-2 installed document-open/layout proof, safe partial failure, and structured receipt            |
| Quality Runner             | In progress   | in_progress / in_progress / in_progress | Crop-safe preview; <=500-character description; no-auth Tenure case; owner-approved copy and linked proof                                                                                      | DEV.to, daily.dev, Hacker News, LinkedIn planned                | QR-8 refresh the corrected Tenure page, preview, short copy, linked proof, and hosted verification |
| Chiron's Forge             | Blocked       | blocked / blocked / blocked             | Backing-repository and deployed-revision provenance; real before/after case; judge/refinement trace; public-safe output; privacy/deletion evidence; baseline package remains gated behind CF-0 | Product Hunt blocked; LinkedIn planned                          | CF-0 owner-visible backing repository and exact deployed revision                                  |
| Research Domain Writing    | Gated         | gated / gated / gated                   | Release-source attestation or rebuild; editorial approval; thumbnail; no-auth hosted verification                                                                                              | DEV.to, daily.dev, LinkedIn planned                             | RDW-0 bind the installed artifact to the intended source revision, then complete editorial review  |
| Pre-CR Suite               | In progress   | in_progress / in_progress / in_progress | Local standalone package is reviewed; owner copy/marketplace review remains optional; hosted case and video are optional follow-ons                                                            | DEV.to, daily.dev, Hacker News, LinkedIn planned                | Keep PCR-4 package; perform owner copy review before any external claim                            |
| Terrace                    | In progress   | in_progress / in_progress / in_progress | Post-integration TR-6 static preview capture; the posted packet still points at the repo until a hosted case is verified                                                                       | DEV.to, daily.dev, Hacker News, LinkedIn planned                | Refresh TR-6 capture on a permitted browser surface                                                |
| Context Compiler Contract  | In progress   | in_progress / in_progress / in_progress | Browser-rendered thumbnail is optional; Handshake upload is blocked; hosted comparison page is separate                                                                                        | DEV.to, daily.dev, Hacker News, LinkedIn planned                | CC-6 permitted browser capture, then independently host/verify the comparison if desired           |
| Portable Agentic Workbench | Gated         | gated / gated / gated                   | Permitted host-runtime equivalence proof; browser-rendered thumbnail optional; target package remains gated                                                                                    | DEV.to, daily.dev, Hacker News, Reddit, LinkedIn planned        | PW-4 resolve the host-runtime/provider boundary                                                    |
| AI Workflow Leverage       | Deferred      | planned / deferred / deferred           | Credible measured outcome; walkthrough; thumbnail; all release packaging waits on the measurement contract                                                                                     | DEV.to, daily.dev, Product Hunt, LinkedIn deferred              | AL-2 approved append-only event contract owned by `agent-eval-runtime`                             |
| Marketing Autoresearch     | Deferred      | planned / deferred / deferred           | Traceable shadow run; report; publication-block proof; walkthrough; thumbnail; release packaging waits on the brief-aware receipt contract                                                     | DEV.to, daily.dev, Product Hunt, LinkedIn deferred              | MA-2 brief-aware source-to-claim receipt contract, then rerun the public brief                     |
| RemodelVision              | Gated         | gated / gated / gated                   | Contribution approval; current runnable proof; walkthrough; thumbnail                                                                                                                          | Product Hunt, Reddit, LinkedIn planned                          | RV-1 explicit owner/collaborator approval; RV-3 current runtime proof remains separate             |
| Dsci-proj                  | Gated         | gated / gated / gated                   | Second backlog adapter; scenario proof; current runnable proof; interactive walkthrough; thumbnail                                                                                             | DEV.to, daily.dev, Hacker News, LinkedIn planned                | DS-1 map two backlog schemas into the decision contract                                            |
| Book                       | Gated         | gated / gated / gated                   | Approved chapter and asset rights/attribution ledger; rights-cleared preview; <=500-character description; no-auth reader case; AI/human role statement                                        | LinkedIn planned; no broad launch until rights clear            | BK-1 complete the representative chapter rights and attribution ledger                             |
| Agent Router               | Gated         | gated / gated / gated                   | Routing-decision preview; <=500-character description; public route-and-receipt case; verified comparison outcome                                                                              | DEV.to, daily.dev, Hacker News, LinkedIn planned                | AR-5 approve confidence/fallback output contract while preserving native conflict receipt          |
| Codex Browser Control      | Gated         | gated / gated / gated                   | Side-panel preview; <=500-character description; public synthetic case; redacted plan and verification receipt                                                                                 | DEV.to, daily.dev, Hacker News, Reddit, LinkedIn planned        | CB-2/CB-3 reload the matching extension/native-host pair and capture round trip                    |
| Participant Deduplication  | Gated         | gated / gated / gated                   | Authenticated live-sheet UAT receipt; hosted no-auth case URL and responsive readback; local synthetic case/proofs are not live-provider proof                                                 | DEV.to, daily.dev, Product Hunt gated, Reddit, LinkedIn planned | PD-6 complete live-sheet UAT and hosted case without exposing participant data                     |

## Dev-tooling public-target grid (18 newly classified)

The following repositories are the 18 independently versioned products in
[`dev-ops-tooling-sprint/IMPLEMENTATION.md`](../../../projects/dev-ops-tooling-sprint/IMPLEMENTATION.md).
They are real local MVPs or approved product definitions with repository-owned
PRDs, Compass contracts, and local quality evidence. They now have an explicit
`public_showcase` disposition and GitHub/portfolio/Handshake target rows, but
all three destinations remain gated and no external publication is inferred.
`dev-ops-tooling-sprint` is the coordination plan, not a nineteenth repository.

| Repository                   | Current implementation state                                                                                                                                                                                                                                                                                      | Showcase/release state                                                                                                                                     | Next step                                                                                                                                          |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quality Lens                 | W1 pilot MVP: QR finding normalization, dispositions, rerun reconciliation, CLI, and VS Code projection; current clean `dev` tests/lint/package pass. Candidate packet now includes current headless checkpoint, normalized stale-finding evidence, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; direct Problems-panel postcondition, hosted no-auth URL, and destination readback remain partial. | Close QL-1 with exact task-owned VS Code Problems-panel proof, then promote the packet to QL-3 hosting/readback. |
| Debug Trail                  | W1 pilot MVP: target-bound trails, declared-command preview, authorization, redacted evidence, continuation receipt, and thin VS Code surface; current clean `dev` tests/lint/package pass. Candidate packet now includes current headless checkpoint, target-bound receipt, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; direct IDE continuity, broader scenario matrix, hosted no-auth URL, and destination readback remain partial. | Close DT-1 with exact task-owned VS Code continuity proof, add the remaining scenario matrix, then promote the packet to DT-3 hosting/readback. |
| Quality Setup                | W1 enabler MVP: preview, explicit apply, conflict refusal, verify, and rollback receipt; current clean `dev` tests/lint/package pass. Candidate local packet now includes a current-dev checkpoint, target-bound scenario matrix, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; real quality-command smoke, hosting, and destination readback remain partial. | Close the real quality-command smoke boundary, then promote the packet to QS-3 hosting/readback. |
| Rule Lab                     | W2 producer MVP: human rule edit, positive/negative fixture comparison, target-bound receipt, and read-only VS Code projection; current clean `dev` pytest/Ruff/extension tests pass while pyright remains open. Candidate local packet now includes a current-dev checkpoint, fixture comparison, fresh receipt, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; cross-producer handoff, direct IDE parity, pyright, hosting, and destination readback remain partial. | Close RL-0/RL-1 and the typecheck disposition, then promote the packet to RL-3. |
| Evidence Replay              | W2 reader MVP: inspect Debug Trail receipts, compare target freshness, and preview explicit reruns without executing. Candidate local packet now includes the historical stale-target matrix, current-dev checkpoint, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; producer/state matrix, hosting, and destination readback remain partial. | Close ER-0/ER-1 with current producer receipts and inspect-only negative proof, then promote the packet to ER-3. |
| Workflow Gateboard           | W2 execution MVP: repository-declared non-mutating gates, prerequisites, freshness, and bounded receipts. Candidate local packet now includes the historical declared-gate fixture, current-dev checkpoint, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; prerequisite-state persistence, trace handoff, state matrix, hosting, and destination readback remain partial. | Close WG-0/WG-1 with Flight Recorder handoff, state persistence, and no-mutation proof, then promote the packet to WG-3. |
| Failure Capsule              | W2 evidence MVP: bounded redacted failure capture with target/tool/artifact identity and inspect-only opening. Candidate local packet now includes historical/current-dev receipts, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source.                                   | Gated public target; Replay handoff, cancellation/recovery breadth, hosting, and destination readback remain partial.                                    | Close FC-0/FC-2 with a Debug Trail-to-Replay handoff and cancellation/recovery receipts, then promote to FC-3.                                  |
| Change Radius                | W2 change MVP: bounded static consumer/test graph with provider names and explicit unknown classes. Candidate local packet now includes historical/current-dev TypeScript receipts, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source.                         | Gated public target; downstream parity, negative fixture breadth, hosting, and destination readback remain partial.                                      | Close CR-1/CR-2 with Deletion Proof/Contract Watch parity and broader negative coverage, then promote to CR-3.                                  |
| Behavior Coverage Atlas      | W2 behavior MVP: declared behavior-to-test mapping with strong, weak, stale, missing, and duplicate-link states. Candidate local packet now includes historical/current-dev matrices, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source.                    | Gated public target; Review Attention Map handoff, negative fixture breadth, hosting, and destination readback remain partial.                             | Close BC-1/BC-2 with the source/freshness handoff and parameterized/deleted-test matrix, then promote to BC-3.                                   |
| Automation Flight Recorder   | W3 execution MVP: bounded parent/child local traces with timing, redaction, hashes, omissions, and rerun eligibility. Candidate local packet now includes historical/current-dev pass, failure, cancellation, and inspect traces, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; Gateboard handoff, cancellation/recovery breadth, hosting, and destination readback remain partial. | Close AFR-0/AFR-2 with Gateboard identity, failure/recovery matrix, and rerun-authority proof, then promote to AFR-3. |
| Remediation Canvas           | W3 composition MVP: Quality Lens finding references, human intent/dispositions, and stale refresh behavior. Candidate local packet now includes historical/current-dev fresh/stale handoff receipts, preserved partial dispositions, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; source-authority parity, downstream review handoff, hosting, and destination readback remain partial. | Close RC-0/RC-2 with one verification/authority handoff, downstream review connection, and stale partial-work preservation, then promote to RC-3. |
| Contract Watch               | W3 contract MVP: local OpenAPI diff, certainty/policy separation, and human disposition. Candidate local packet now includes historical/current-dev four-change semantic-diff receipts, claim ledger, crop-safe 1600×900 PNG/SVG preview, short copy, and no-auth case source. | Gated public target; downstream review handoff, unknown-consumer/stale coverage, hosting, and destination readback remain partial. | Close CW-0/CW-2 with downstream source-identity handoff, certainty/policy preservation, and unknown-consumer/stale coverage, then promote to CW-3. |
| Review Attention Map         | W3 review MVP: explicit contract/behavior signals over a diff with source evidence and stale/unmatched visibility. Candidate local packet now includes a two-source overlay receipt, claim ledger, preview source, short copy, and no-auth case source.                                                           | Gated public target; current producer/source-freshness readback, negative-state breadth, binary preview, hosting, and destination readback remain partial. | Close RAM-0/RAM-2 with current producer binding, direct navigation, and stale/unmatched/reviewed matrix, then promote to RAM-3.                    |
| Review Sandbox               | W4 disposable-workspace MVP: preview/create/cleanup for repository-declared scenarios with dirty-state retention. Candidate local packet now includes clean/conflict/failed/dirty/cancellation-shaped scenario receipts, claim ledger, preview source, short copy, and no-auth case source.                       | Gated public target; distinct cancellation/process proof, primary-checkout safety, binary preview, hosting, and destination readback remain partial.       | Close RS-0/RS-2 with distinct cancellation/process proof, primary-checkout safety, and cleanup authority, then promote to RS-3.                    |
| Change Integration Simulator | W4 integration MVP: immutable source/target resolution and merge-tree receipts without ref mutation. Candidate local packet now includes clean/conflict simulation and gate-boundary receipts, claim ledger, preview source, short copy, and no-auth case source.                                                 | Gated public target; Gateboard handoff, stale/cancellation/dirty breadth, binary preview, hosting, and destination readback remain partial.                | Close CIS-0/CIS-2 with a real Gateboard receipt, negative matrix, and ref-safety proof, then promote to CIS-3.                                     |
| Deletion Proof Workbench     | W4 proof MVP: bounded symbol inspection, unknown consumer classes, explicit apply, and recovery reference. Candidate local packet now includes bounded deletion/refusal and unknown-class receipts, claim ledger, preview source, short copy, and no-auth case source.                                            | Gated public target; stale/recovery breadth, downstream parity, binary preview, hosting, and destination readback remain partial.                          | Close DPW-0/DPW-2 with stale/refusal/recovery fixtures and Change Radius/Contract Watch parity, then promote to DPW-3.                             |
| Readiness Inspector          | W5 portfolio MVP: goal-specific checks with owner, predicate, outcome, evidence, and next action; no aggregate score. Candidate local packet now includes passed/failed/blocked/unsupported state evidence, claim ledger, preview source, short copy, and no-auth case source.                                    | Gated public target; upstream projection, full state matrix, binary preview, hosting, and destination readback remain partial.                             | Close RI-0/RI-2 with Quality Setup/Evidence Replay projection, state breadth, and native follow-up, then promote to RI-3.                          |
| Fleet Radar                  | W5–W6 portfolio MVP: explicit registry, per-repository freshness/blockers, receipt-contract classification, native-link metadata, broad failure states, and read-only snapshot writing. Candidate local packet now includes fresh/stale refresh evidence, W6 contract/failure evidence, a visually inspected 1600×900 PNG, a validated owner-only Sites version, claim ledger, preview source, short copy, and no-auth case source. | Gated public target; real producer readback and direct native navigation remain product/evidence gates; public access, deployment, credentialless URL readback, and destination readbacks remain packaging gates. | Close FR-0/FR-1 with two real producer receipts and direct native navigation, then confirm public access, deploy the saved case, and capture no-auth/external readbacks for FR-3. |

The shared **C0 disposition** gate is complete for all 18. The active gate is
now each repository's route-plan ID and its local product, evidence, or
packaging proof. Public classification does not imply product readiness or
publication: every destination remains `gated` until the material package,
authority, and external readback are all present.

## Exclusions and supporting routes

- **BidCamp** is client work and does not receive Showcase materials.
- **Soundscape** is excluded by owner decision.
- **TMCP** is deferred until the always-on atomic-node product direction is
  specified and proven.
- **ESLint Anti-Slop** is retained as a standalone package but its showcase
  material is the supporting layer of the AI Code Quality Stack, not a separate
  release card.

## Linear execution rule

The next execution pass starts with the 18 route-plan gates in dependency order:
Quality Lens and Debug Trail; Quality Setup and Rule Lab; Evidence Replay and
Workflow Gateboard; Failure Capsule, Change Radius, and Behavior Coverage Atlas;
Automation Flight Recorder, Remediation Canvas, and Contract Watch; then the
review, integration, deletion, readiness, and Fleet Radar surfaces. After each
local gate closes, work from the project's **Material inventory still open**
column and update the matching target rows. A project can advance one target
without advancing every other target, but no target becomes `posted` until its
material, authority, and external readback are all recorded.
