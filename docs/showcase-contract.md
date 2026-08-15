# AI Showcase readiness contract

Pronto reads one fleet-level `.pronto/showcase-goal.json` contract with schema
`pronto-showcase-goal/v2`. The contract is an explicit, reviewable portfolio
judgment; Pronto does not infer publication authority from repository quality,
README visibility, or a high score.

## Independent dimensions

Each project records product readiness, demo-material readiness, and career
signal on a 0–5 scale. A dimension can instead be `unknown`, `blocked`, or
`not_applicable`; those states omit a score and remain categorical in the
projection.

The combined showcase score is:

```text
0.60 × product readiness + 0.40 × demo materials
```

Work priority is separate:

```text
0.50 × career signal + 0.30 × product readiness + 0.20 × materials gap
```

The materials gap is `5 - demo materials`. The combined readiness score ranks
every assessed repository, including private audit context. Public priority is
calculated only for eligible public work; neither score grants eligibility.

## Eligibility and lanes

`public_eligibility` is evaluated before public priority and publishing:

- `public_showcase` may enter `publish_ready`, `create_materials`,
  `product_first`, `blocked`, or `unknown`.
- `private_client` is always projected as `private_client`, never receives a
  public priority score, never counts toward the goal, and never enters the
  public queue. Private readiness scores are audit context only.
- `blocked` requires a disposition change before public work can be queued.
- `not_applicable` stays outside both the numerator and denominator.

A public project is publishable only when product readiness and demo materials
meet the configured minimums, `missing_materials` is empty, and no blocker is
present. The public materials queue includes every `create_materials` project
and is sorted by work priority; the goal gap remains a target, not a display
cap.

Every registered Pronto repository appears in the projection. A newly discovered
repository is added to the normal Pronto fleet, but registration and refresh do
not mutate the Showcase contract and never create a placeholder row. If the
repository should enter Showcase, the owner must review it immediately and add
a complete contract entry with an explicit eligibility, disposition, evidence,
and next step. Until that review is complete, the repository is synthesized as
an unranked `unknown` audit gap. This exposes coverage gaps without
manufacturing scores, eligibility, or publication authority, and keeps Showcase
additions attributable to a real review rather than an automated guess.

## Material production route

Every `public_showcase` project follows the same aspirational-first route. The
ideal story is allowed to lead the current implementation; today's proven flow
must not quietly become the creative ceiling.

| Stage                      | Durable stage ID    | Required output                                                                                                  | Exit condition                                                                                                         |
| -------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 1. Ideal demo target       | `ideal_target`      | The strongest useful promise, audience, interaction, proof moment, ownership split, and limitation               | The target is specific enough to storyboard without treating current implementation gaps as creative constraints       |
| 2. Concept materials       | `concept_materials` | Aspirational storyboard, key frames or mock preview, narrative, and desired evidence moment                      | The concept communicates the intended experience and every unbuilt element is labeled as a concept                     |
| 3. Build-gap specification | `build_gap`         | A bounded list of product, data, safety, attribution, and evidence gaps between the concept and current behavior | Each gap has an observable acceptance condition and an owner; the team can decide whether the target is worth building |
| 4. Gap closure             | `gap_closure`       | Only the product, demo-integration, evidence, content, or packaging work required by the accepted target         | Every accepted gap is closed with its required proof; already-capable product behavior is not rebuilt                  |
| 5. Evidence capture        | `evidence`          | Sanitized inputs, outputs, receipts, limitations, and current-surface proof                                      | The target's proof gate passes; concept-only claims are either proven, revised, or removed                             |
| 6. Final production        | `final_materials`   | Public demo or case study, crop-safe preview, short description, and linked proof                                | All required public materials make the same evidence-backed promise and satisfy the material quality bar               |
| 7. Review and readiness    | `reviewed`          | Editorial, factual, privacy, attribution, accessibility, and link review                                         | Artifacts exist, review passes, and only then is `.pronto/showcase-goal.json` updated                                  |

The order is directional, not a one-way gate: discovery during build, evidence,
final production, or review can send a project back to an earlier stage.
Concept materials may depict the
intended product before it exists, but must remain labeled and must not raise
demo-material readiness or be published as current behavior. Final materials
may simplify the presentation; they may not exceed the verified claim boundary.

`video_enhancement` is an optional production branch after evidence capture. A
project may use a timed rehearsal, screen recording, captions, or narration when
motion materially improves comprehension or uniquely proves behavior. This
branch never blocks publication when the required package can make and prove the
same claim. Human voice recording is not a fleet-wide requirement.

The stage IDs are stable documentation vocabulary. The current showcase schema
does not persist stage state; until it does, each project's `next_step` names
the active action and its package holds the stage artifacts. The package
convention and step 1–3 completion rule live in `showcase-materials/README.md`.

Every public project also carries a structured work disposition. The
disposition distinguishes a largely product-ready project from targeted gap
closure, material build or restoration, and a conditional gate. Its active
next step is separately typed as `product`, `demo_integration`, `evidence`,
`content`, or `packaging`. These fields prevent unfinished presentation work
from being reported as unfinished product work.

## Handshake quality gates and material set

The reviewed story target for each eligible public project lives in
`showcase-materials/ideal-demo-targets.md`. Project packages should link to that
target rather than restating it: the package owns capture logistics and final
copy, while the target document owns the intended demo story.

The contract records the exact missing artifacts rather than treating “make a
demo” as one opaque task. The quality bar is the six-gate standard documented
in Career Ops research: five-second comprehension, immediate product proof,
clear AI and human roles, credible evidence, public low-friction access, and
consistent Handshake packaging.

A complete package contains three coordinated materials:

1. A crop-safe 16:9 Handshake preview image that remains legible around 400px.
2. A verified Handshake description of 500 characters or fewer.
3. A public, no-auth demo or structured case-study page with core proof no
   more than one intentional click away.

An optional 45–90 second captioned recording or shot list may supplement those
materials. Human narration is not required: an accurate silent capture,
on-screen captions, synthetic narration, or a live presentation can be used
when appropriately disclosed. If a static page cannot prove a material
interaction, the project must still provide direct behavioral evidence; that is
an evidence requirement, not a narrated-video requirement.

The page or demo must distinguish creator-owned decisions, AI runtime work,
AI-assisted implementation, human review, limitations, synthetic data, and
claim sources. Preview, description, and page must make the same factual
promise. Any optional recording must match that promise too. Projects may
specialize the proof artifacts without manufacturing a video deliverable.

## Public release targets

Distribution planning is separate from readiness and publication authority.
The durable channel grid for eligible projects lives in
[`showcase-materials/public-release-targets.md`](../showcase-materials/public-release-targets.md),
with a machine-readable companion at
[`showcase-materials/public-release-targets.json`](../showcase-materials/public-release-targets.json).
The grid assigns each project a canonical home, a technical or product
discovery route, an optional community feedback/launch route, and a career
distribution route. It does not mark a project as externally posted.

GitHub and a no-auth portfolio case are the canonical release targets. DEV.to
and daily.dev are used for substantive human-written technical stories; their
links do not replace the canonical case. Hacker News and Product Hunt are
conditional targets with their own tryability and product-readiness gates.
Indie Hackers, Reddit, LinkedIn, Handshake, and X/Bluesky remain audience-
specific distribution layers. A project can be ready for one target while
remaining gated for another.

### Public-release target eligibility invariant

`public_showcase` is not a label that can be added independently of release
planning. Before an existing or new project receives that value in
`.pronto/showcase-goal.json`, the release-target matrix must contain an exact
`project_targets` record for the same repository. That record must name, at
minimum, a GitHub home, a no-auth portfolio case page, and a Handshake
destination, plus a status, artifact, and active gate for each destination.
The status may be `planned`, `in_progress`, `gated`, `deferred`, or `blocked`;
the requirement is to know where the materials belong, not to pretend they
have already been posted. The Showcase materials test enforces this exact
coverage and fails when a new public label lacks a target record.

Pronto exposes the derived `pronto-showcase/v2` projection in the dedicated AI
Showcase desktop tab and `summary --json`. The renderer orders the complete
fleet by the projected readiness score and does not invent scores or reinterpret
private/client dispositions.
