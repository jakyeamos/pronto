# RemodelVision route plan

Status: RV-1 is blocked at collaborator approval; RV-2 is closed with a
synthetic rights-safe fixture; RV-3 is blocked at runtime prerequisites and
direct-surface access. Current runnable behavior and contribution boundaries
remain unverified.

The [canonical target](../ideal-demo-targets.md#remodelvision) owns the durable
promise and proof gate.

## 1. Ideal target

**North star:** a homeowner uploads a plain room, sets a real budget and one
non-negotiable constraint, and receives three visually distinct concepts with
traceable choices, editable tradeoffs, and an estimate-oriented next-step plan.

**Non-negotiable:** generated visuals are concepts, not construction guarantees.
Personal contribution and collaborator ownership must be reviewed before any
public attribution.

## 2. Concept materials

All frames are **concept** until current flow, rights, and attribution pass.

| Frame                  | Visual                                                                  | On-screen line                              | Intended evidence moment                    |
| ---------------------- | ----------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------- |
| 1. Real constraint     | Owned room photo beside “Keep the floor; budget $12k”                   | “Design starts with what cannot change”     | Input is more than an image prompt          |
| 2. Scene understanding | Floor, windows, fixtures, and preserved elements are outlined           | “Understand the room before redesigning it” | Visual reasoning is inspectable             |
| 3. Three directions    | Warm minimal, color-forward, and storage-first concepts appear          | “Explore real tradeoffs”                    | Variety serves decision-making              |
| 4. Constraint proof    | Preserved floor and budget-sensitive choices are highlighted            | “Show how the brief shaped the result”      | Output follows a meaningful constraint      |
| 5. Human edit          | User swaps one finish and reprioritizes storage                         | “The homeowner remains the designer”        | Human judgment changes the concept          |
| 6. Next-step artifact  | Selected concept becomes a scoped estimate and contractor-question list | “From inspiration to a better conversation” | Output supports action without overclaiming |

**Preview concept.** Before/after room split with the preserved floor subtly
outlined and a small budget/constraint card. Headline: “Renovation concepts that
remember the room you actually have.”

**Narrative spine.** Owned room → explicit constraints → scene understanding →
meaningful options → human edit → estimate-oriented handoff.

## 3. Build-gap specification

Reviewed baseline: a collaborative photo-to-estimate story exists, but current
runnable status and personal contribution evidence are uncertain.

Project disposition: `material_build_or_restoration` — ownership must be
resolved and the current core flow may need restoration before its product
promise can be demonstrated safely.

Gap classes: evidence — RV-1, RV-4, RV-5; content — RV-2; product — RV-3;
packaging — RV-6.

| ID   | Gap to close                                  | Observable acceptance condition                                                                                             | Owner                       | Required proof                               |
| ---- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------- | -------------------------------------------- |
| RV-1 | Resolve ownership and contribution boundaries | Collaborator-owned, personal, third-party, and model-generated work are explicitly attributed and approved for showcase use | Project owner/collaborators | Attribution ledger and approval record       |
| RV-2 | Establish a rights-safe room fixture          | Input image and all displayed assets have documented rights and contain no private location data                            | Content/privacy owner       | Asset manifest and metadata review           |
| RV-3 | Verify or rebuild the current core flow       | A clean environment completes upload, constraints, concepts, edit, and handoff without undocumented setup                   | Implementation owner        | Setup receipt and direct-surface recording   |
| RV-4 | Make constraint adherence visible             | At least one preserved element and one budget-driven tradeoff can be traced from brief to output                            | Product/AI owner            | Input-output comparison and evaluation notes |
| RV-5 | Bound estimate claims                         | Cost outputs expose assumptions, ranges, date/location limits, and no-guarantee language                                    | Domain/product owner        | Calculation review and claim ledger          |
| RV-6 | Build the public case-study path              | A no-auth viewer reaches the concept, constraint proof, contribution statement, and limitation in one click                 | Showcase owner              | Public-link and responsive-layout check      |

**Build order:** RV-1 → RV-2 → RV-3 → RV-4/RV-5 → RV-6. If RV-1 or RV-3
cannot close, retain the concept internally and do not produce public materials.

## 4. RV-1 blocker

The repository and presentation identify RemodelVision as a four-person senior
project, and the inspected checkout is clean. The contribution ledger also
records the observed Git identities and AI-pipeline history. That evidence does
not assign personal roles or grant permission to publish a team case study,
however. No collaborator approval or provider/asset-rights record was found.

The exact stop is recorded in
[`evidence/rv-1-blocker.json`](evidence/rv-1-blocker.json), and the non-public
ledger is [`contribution-ledger.json`](contribution-ledger.json). Do not infer a
personal role from commit counts or publish repository/provider assets until the
owner and collaborators approve the attribution boundary.

## 5. RV-2 closure

RV-2 is closed for the material track with
[`rights-safe-fixture.svg`](rights-safe-fixture.svg) and its
[`asset-manifest.json`](asset-manifest.json). The fixture is an invented SVG
scene with no people, address, geolocation, photographed property, provider
output, or third-party asset. The manifest keeps the synthetic label and
claim-limit rules attached to the asset. This clears rights-safe material
preparation only; it does not clear RV-1 or prove the live RemodelVision flow.

## 6. RV-3 blocker

The target checkout passes its 26-test suite, TypeScript check, and production
build, but the documented runtime requires nine service variables that are not
present in this environment. The existing lint command also reports 41 errors
and 29 warnings. A bounded direct-surface probe could not bind the local server
because the host returned `listen EPERM` for `127.0.0.1:3000`.

The exact evidence and claim boundary are recorded in
[`evidence/rv-3-blocker.json`](evidence/rv-3-blocker.json). Static checks are
useful implementation evidence, not proof of upload, generation, analysis,
estimation, persistence, or handoff. RV-4 and RV-5 remain downstream of this
stop; do not create a synthetic product result to fill the gap.
