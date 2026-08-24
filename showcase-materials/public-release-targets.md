# Public release targets

This is the distribution plan for the eligible AI Showcase projects. It answers
**where each project should go once its proof package is ready**; it is not a
publication record and does not authorize posting to an external service.

The machine-readable matrix is
[`public-release-targets.json`](public-release-targets.json). Readiness and
eligibility remain owned by [`.pronto/showcase-goal.json`](../.pronto/showcase-goal.json).

## Eligibility invariant

Any project labeled `public_showcase` in `.pronto/showcase-goal.json` must have
an exact row in the JSON matrix. The row must name the posting destination for
the demo materials and carry its status, artifact, and active gate. The current
baseline is three destinations for every public target:

- GitHub for the canonical source, release, and provenance home.
- Portfolio for the no-auth demo or case-study page.
- Handshake for the career-facing Showcase package.

This is a planning requirement, not proof of publication. A new project must
be added to this grid before it is labeled `public_showcase`; the automated
Showcase materials test enforces exact coverage.

## Durable release order

1. **Canonical home** — GitHub source/release page and a no-auth portfolio case
   page.
2. **Technical or product discovery** — a human-written DEV.to article and
   daily.dev direct post for technical stories; Product Hunt only for a live,
   user-facing product; Hacker News only when the project is genuinely
   inspectable or tryable.
3. **Community launch or feedback** — Indie Hackers for build-in-public/product
   lessons, or one carefully selected Reddit community for domain feedback.
4. **Career distribution** — LinkedIn result post and the Handshake project/
   AI Showcase entry.

The same case study can feed several channels, but the copy should be adapted
to the audience. The canonical page remains the durable archive.

## Channel rules

| Channel             | Role                 | Use it when                                                          | Do not treat it as                                      |
| ------------------- | -------------------- | -------------------------------------------------------------------- | ------------------------------------------------------- |
| GitHub              | Canonical            | The repository, release boundary, and evidence are coherent          | Proof that a hosted product exists                      |
| Portfolio           | Canonical            | A no-auth recruiter-facing case or demo is ready                     | A substitute for repository provenance                  |
| DEV.to / `#showdev` | Technical discovery  | The post teaches a concrete build lesson or shows a project          | A bare promotional link                                 |
| daily.dev           | Technical discovery  | A substantive, human-authored developer post is ready                | A place for generic AI-generated launch copy            |
| Hacker News         | Technical discussion | A non-trivial project can be inspected or tried with little friction | A landing-page or coordinated-upvote channel            |
| Product Hunt        | Product launch       | A polished, live, user-facing product is explorable                  | The default destination for libraries or internal tools |
| Indie Hackers       | Build-in-public      | The story is about product decisions, progress, or lessons           | A duplicate launch blast                                |
| Reddit              | Targeted feedback    | One community has a specific reason to care and its rules allow it   | A cross-posting queue                                   |
| LinkedIn            | Career distribution  | The result maps to a role or capability                              | The canonical project archive                           |
| Handshake           | Career distribution  | The preview, description, proof link, and role boundary pass review  | Evidence that a public release already happened         |
| X / Bluesky         | Amplifier            | A short thread can point people to the canonical case                | A durable release record                                |

Current platform guidance: [daily.dev feature paths](https://docs.daily.dev/how-to-get-featured/),
[daily.dev content guidelines](https://docs.daily.dev/content-guidelines/),
[DEV Show DEV](https://dev.to/t/showdev/),
[Hacker News Show HN](https://news.ycombinator.com/showhn.html),
[Product Hunt launch guidance](https://www.producthunt.com/launch), and
[Reddit rules](https://redditinc.com/policies/reddit-rules).

## Project target grid

`Required` means part of the durable release package. `Recommended` is a good
career or discovery layer for the story. `Conditional` requires the channel's
own entry condition. `Gated` and `Deferred` describe the project, not a promise
that anything has been posted.

| Project                    | Release posture | Primary path                                                               | Conditional or optional reach                                     | Active gate                                                             |
| -------------------------- | --------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Mac Control                | Gated           | GitHub → portfolio → LinkedIn → Handshake                                  | Product Hunt, targeted Reddit                                     | MC-2 installed bounded document-open/layout proof                       |
| Quality Runner             | In progress     | GitHub → Tenure case → DEV.to → daily.dev → LinkedIn → Handshake           | Hacker News after a low-friction public path                      | QR-8 hosted case, preview, copy, and linked proof                       |
| Chiron's Forge             | Blocked         | GitHub → portfolio → LinkedIn → Handshake                                  | Product Hunt after provenance and authenticated proof             | CF-0 backing repository and deployed revision                           |
| Research Domain Writing    | Gated           | GitHub → source-to-claim case → DEV.to → daily.dev → LinkedIn → Handshake  | Hacker News only as a regular technical submission if appropriate | RDW-5 no-auth hosting and destination readbacks                         |
| Pre-CR Suite               | In progress     | GitHub → IDE case → DEV.to → daily.dev → LinkedIn → Handshake              | Hacker News if the extension is easy to try                       | PCR-4 is locally closed; owner copy/marketplace review remains optional |
| Terrace                    | In progress     | GitHub → workflow case → DEV.to → daily.dev → LinkedIn → Handshake         | Hacker News with a frictionless runnable path                     | Post-integration TR-6 visual refresh                                    |
| Context Compiler Contract  | In progress     | GitHub → validator case → DEV.to → daily.dev → LinkedIn → Handshake        | Hacker News if the validator is easy to inspect                   | CC-6 browser capture is optional; hosted proof remains separate         |
| Portable Agentic Workbench | Gated           | GitHub → safety case → DEV.to → daily.dev → LinkedIn → Handshake           | Hacker News, targeted Reddit                                      | PW-4 host-runtime equivalence boundary                                  |
| AI Workflow Leverage       | Excluded        | Supporting packet retained locally; not in current queue                    | None                                                            | Owner decision: excluded from current AI Showcase queue                  |
| Marketing Autoresearch     | Excluded        | Supporting packet retained locally; not in current queue                    | None                                                            | Owner decision: excluded from current AI Showcase queue                  |
| RemodelVision              | Gated           | GitHub → visual case → LinkedIn → Handshake                                | Product Hunt, targeted Reddit                                     | RV-3 current runtime proof; scoped RV-1 approval is closed               |
| Dsci-proj                  | Gated           | GitHub → contract case → DEV.to → daily.dev → LinkedIn → Handshake         | Hacker News after DS-1                                            | Two backlog schemas mapped into the decision contract                   |
| Book                       | Gated           | GitHub → synthetic interactive-media case → LinkedIn → Handshake           | Real Book assets remain excluded; no external posting until direct proof and hosting | BK-2 transformation arc and direct-reader proof                 |
| Agent Router               | Gated           | GitHub → routing case → DEV.to → daily.dev → LinkedIn → Handshake          | Hacker News after AR-5                                            | Confidence/fallback output contract                                     |
| Codex Browser Control      | Gated           | GitHub → browser-control case → DEV.to → daily.dev → LinkedIn → Handshake  | Hacker News, targeted Reddit                                      | CB-2/CB-3 installed extension/native-host round trip                    |
| Participant Deduplication  | Gated           | GitHub → synthetic safety case → DEV.to → daily.dev → LinkedIn → Handshake | Product Hunt, targeted Reddit                                     | PD-6 authenticated UAT and hosted no-auth proof                         |

This grid deliberately keeps the technical projects together around a shared
case-study route while reserving product-launch channels for projects that can
support a real, low-friction product experience. Client work, excluded
projects, TMCP's deferred ideal-state work, and supporting repositories do not
enter this matrix.

## DevOps tooling sprint target grid

The following 18 repositories are now explicit public Showcase targets in the
same planning matrix. Every row is **Gated**: the repository, no-auth case, and
Handshake package are planned destinations, not evidence that anything has been
posted. Their active product/evidence gates are recorded in the machine-readable
matrix and the Pronto contract.

| Project                      | Release posture | Primary path                   | Active gate                                                       |
| ---------------------------- | --------------- | ------------------------------ | ----------------------------------------------------------------- |
| Quality Lens                 | Gated           | GitHub → portfolio → Handshake | Implement and directly exercise the smallest IDE finding workflow |
| Debug Trail                  | Gated           | GitHub → portfolio → Handshake | Implement the target-bound debugging continuity workflow          |
| Quality Setup                | Gated           | GitHub → portfolio → Handshake | Demonstrate one supported-ecosystem setup case with receipts      |
| Rule Lab                     | Gated           | GitHub → portfolio → Handshake | Produce a second receipt and prove the cross-tool handoff         |
| Evidence Replay              | Gated           | GitHub → portfolio → Handshake | Consume a Rule Lab receipt and close the stale/rerun matrix       |
| Workflow Gateboard           | Gated           | GitHub → portfolio → Handshake | Connect one declared gate to Flight Recorder                      |
| Failure Capsule              | Gated           | GitHub → portfolio → Handshake | Verify redaction, cancellation, and recovery through Replay       |
| Change Radius                | Gated           | GitHub → portfolio → Handshake | Run the TypeScript fixture and connect unknown boundaries         |
| Behavior Coverage Atlas      | Gated           | GitHub → portfolio → Handshake | Verify the behavior fixture and feed the review overlay           |
| Automation Flight Recorder   | Gated           | GitHub → portfolio → Handshake | Prove parent/child causality and safe rerun                       |
| Remediation Canvas           | Gated           | GitHub → portfolio → Handshake | Complete one disposition-and-refresh handoff                      |
| Contract Watch               | Gated           | GitHub → portfolio → Handshake | Feed one contract change into the review tools                    |
| Review Attention Map         | Gated           | GitHub → portfolio → Handshake | Prove two-source overlay and evidence navigation                  |
| Review Sandbox               | Gated           | GitHub → portfolio → Handshake | Exercise failure, cancellation, and retained-dirty scenarios      |
| Change Integration Simulator | Gated           | GitHub → portfolio → Handshake | Run clean/conflict simulations plus one declared gate             |
| Deletion Proof Workbench     | Gated           | GitHub → portfolio → Handshake | Prove one bounded deletion with recovery evidence                 |
| Readiness Inspector          | Gated           | GitHub → portfolio → Handshake | Consume upstream receipts and close individual checks             |
| Fleet Radar                  | Gated           | GitHub → portfolio → Handshake | Inspect two repositories and refresh one read-only status         |
