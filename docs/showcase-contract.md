# AI Showcase readiness contract

Pronto reads one fleet-level `.pronto/showcase-goal.json` contract with schema
`pronto-showcase-goal/v1`. The contract is an explicit, reviewable portfolio
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

Every registered Pronto repository appears in the projection. Repositories not
yet represented in the reviewed contract are synthesized as `unknown`, remain
unranked, and carry an explicit audit next step. This exposes coverage gaps
without manufacturing scores or publication authority.

## Handshake quality gates and material set

The contract records the exact missing artifacts rather than treating “make a
demo” as one opaque task. The quality bar is the six-gate standard documented
in Career Ops research: five-second comprehension, immediate product proof,
clear AI and human roles, credible evidence, public low-friction access, and
consistent Handshake packaging.

A complete package contains four coordinated materials:

1. A crop-safe 16:9 Handshake preview image that remains legible around 400px.
2. A verified Handshake description of 500 characters or fewer.
3. A public, no-auth demo or structured case-study page with core proof no
   more than one intentional click away.
4. A 45–90 second captioned demo recording and shot list. A reviewed live
   interaction or excellent case study can satisfy the proof need when video
   adds no value, but the contract must record that disposition explicitly.

The page or demo must distinguish creator-owned decisions, AI runtime work,
AI-assisted implementation, human review, limitations, synthetic data, and
claim sources. Preview, description, page, and recording must make the same
factual promise. Projects may specialize the artifact list, but removing an
item requires review evidence in the contract.

Pronto exposes the derived `pronto-showcase/v1` projection in the dedicated AI
Showcase desktop tab and `summary --json`. The renderer orders the complete
fleet by the projected readiness score and does not invent scores or reinterpret
private/client dispositions.
