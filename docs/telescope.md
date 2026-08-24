# Repository Telescope

Last reviewed: 2026-08-18.

Telescope is Pronto's read-only, repository-level architecture map. It turns
the active registered worktree into an inspectable spatial model, then layers
Pronto evidence over that same topology. Telescope is a repository feature,
not Pronto's global navigation model, and the source-derived graph remains
canonical when optional workflow adapters are unavailable.

## Projection contract

The Rust core owns `pronto-telescope/v2` and its coordinated visual projection
`pronto-telescope-city/v2`. Both
`get_repository_telescope` and `pronto telescope <repository> --json` return
that projection. `refresh_repository_telescope` and
`pronto telescope refresh <repository> --json` force regeneration before
returning the same schema.

Desktop refresh is cooperatively cancellable between directories and source
files. Cancellation never saves a partial projection and leaves the last
matching cached projection intact.

The projection contains:

- `groups`, `nodes`, `edges`, and directional `flows` with deterministic IDs;
- `actions` and `action_coverage`, which make useful read-only operations
  searchable and map each operation to city nodes, rails, flows, source
  anchors, and (when available) a behavior-assurance ID;
- categorical `map_readiness`, semantic blocking and enhancement gaps, and
  dependency-ordered `knowledge_tasks` projected into Pronto's existing
  remediation/attention workflow under the `telescope_readiness` domain;
- actors, payloads, overview/district/building/action scopes, structured
  explanations, and a developer-legibility readiness receipt;
- semantic and implementation summaries, repository-relative source anchors,
  symbols, technologies, and statically derived data shapes;
- relationship direction, kind, confidence, and provenance;
- branch, commit, dirty-state and workspace fingerprints, generation time,
  extraction coverage, freshness, and warnings; and
- enrichment metadata that states whether AI enrichment is enabled and which
  model or provider produced an inferred summary.

Repository and workspace identifiers are projected as opaque stable IDs when
the registry identity contains a filesystem path. Absolute paths are never
part of the returned or cached contract.

Descriptions derived from manifests, source declarations, imports, routes,
types, or documentation are labeled `derived`, not confirmed. Unsupported
languages receive generic directory and module topology and remain visibly
partial. Dynamic or unresolved relationships retain uncertain provenance and
must never be rendered as confirmed source facts.

## Extraction and freshness

The first adapters cover TypeScript, JavaScript, and Rust. They identify
modules, declarations, import relationships, entry points, routes, and bounded
data-shape evidence without storing source text. Other tracked source files
participate through the generic partial adapter.

Every request resolves a registered repository to its active worktree and
binds the result to the current branch, commit, dirty-state fingerprint,
workspace fingerprint, schema version, and generation time. A normal `get`
reuses a cache row only when the schema and workspace fingerprint match.
`refresh` always regenerates. Changing staged, unstaged, or untracked content
changes the dirty fingerprint and invalidates the cache even when the set of
dirty paths is unchanged.

The SQLite cache contains topology, summaries, hashes, and repository-relative
references only. It must not contain source bodies, uncommitted diffs, runtime
payload values, credentials, or absolute personal paths. AI enrichment is
disabled by default and, when introduced, remains explicit and
repository-scoped.

## Authored map and visual model

Telescope separates what the machine can measure from what a human chooses to
explain. A repository may add `.pronto/telescope-map.json` using
`pronto-telescope-map/v2`. The manifest is a small, repository-local narrative
layer: its groups are neighborhoods, its nodes are meaningful buildings, its
edges name the story rails, and its flows describe the user or system journeys
worth following.

The boundary is deliberate:

- authored groups, labels, `whatItDoes`, `howItsBuilt`, visual archetypes,
  representative files, and flow meaning are `draft` or `reviewed` prose;
- measured files, counts, lines, extracted symbols, source anchors,
  relationships, coverage, freshness, and geometry inputs come from the active
  worktree; and
- generated summaries and optional AI enrichment stay inferred until a person
  reviews them. Refresh updates measured data, reports narrative drift, and
  never overwrites the manifest's explanations or layout decisions.

District and building counts are adaptive. The overview includes every
district and landmark needed to explain the repository and clusters routine
implementation structures whenever showing them separately would make the
explanation less readable. It never treats an arbitrary node quota as
architectural completeness. `pathPrefixes` create conceptual districts from
source-backed files; a node's `groupId` and `files` can make a more specific
building assignment. Every authored edge must identify source-relative files
that the extractor can resolve, and every flow must reference valid authored
node and edge IDs. Unmapped source files remain visible as measured topology,
and missing, partial, unsupported, stale, or inferred relationships remain
explicit in the projection rather than becoming confirmed claims.

The renderer presents this narrative as a visual explanation, not a box graph:
it uses a deterministic 2:1 dimetric scene, neighborhood plates, recognizable
`fin-row`, `tower`, `slab-stack`, `cube`, and `low-slab` silhouettes, typed data,
control, event, and import rails, and plain-language moving payload tokens.
Solid rails carry confirmed extracted relationships; muted, dashed, or hatched
rails signal inferred or partial evidence. Overview, subsystem, and source
levels progressively reveal detail while preserving stable positions and a
deterministic primary story flow. Pronto lenses tint or annotate these
districts and rails but never redefine the source-derived city.

Actors and activity are evidence-bound. People appear only when a reviewed or
draft actor maps to the facility or action; payloads become labeled parcels,
documents, cargo, or service traffic only when a real flow supports them.
Routes become gates, persistence becomes archives or vaults, workers become
yards or utility crews, queues become depots, and external boundaries become
ports or bridges. The technical role and source evidence remain visible beside
the metaphor. Telescope never invents pedestrians or traffic for atmosphere.

## Map readiness and workshop

Telescope publishes one categorical readiness state rather than a blended
score:

- `unavailable`: extraction or workspace binding failed;
- `measured`: source topology exists but repository meaning is not established;
- `needs_information`: a consequential purpose, boundary, action, movement,
  constraint, metaphor, or evidence question remains;
- `reviewable`: the candidate city is complete enough for explicit review;
- `reviewed`: high-impact meaning is approved against the current narrative
  and workspace fingerprint; or
- `stale`: a reviewed explanation may no longer match the active worktree.

Applicability is repository-specific. `not_applicable` is accepted only with a
reason, while `unknown` remains unknown. A gap blocks publication when it
prevents an unfamiliar person from understanding purpose, a major boundary, a
primary action or state transition, or the evidence supporting the story.
Lower-impact omissions can remain enhancement gaps.

Incomplete repositories open a clearly labeled Map Workshop and may show a
measured preview; they do not present the preview as a finished city. Workshop
tasks ask one consequential question at a time, explain why source could not
answer it, show candidate answers and source anchors, name the exact city
element unlocked, and allow confirm, choose, edit, not-applicable, unknown, or
point-to-evidence responses. Stable gap keys deduplicate tasks and dependencies
order identity before actors, boundaries, actions, movement, metaphor, and
final review.

There is no Telescope task database. `knowledge_tasks` are a read-only
`telescope_readiness` projection for Pronto's remediation and attention
systems. “Answer next question” invokes the existing guarded repository-task
handoff, which may prepare a draft manifest change in an isolated worktree.
Completing a task never marks the city reviewed. Refresh updates measured
evidence, opens only affected drift questions, and never rewrites authored
meaning.

The `readiness_receipt` is narrowly scoped to Quality Runner's existing
developer-legibility architecture-visibility lane. Quality Runner remains the
owner of holistic maturity and must preserve Telescope's applicability,
freshness, unknowns, and blocking gap keys instead of converting them to a
generic score.

## Action catalog and behavior assurance

The action catalog is a projection layer over Pronto's existing behavioral
analysis. `.pronto/behavior-assurance.json` (`pronto-behavior-assurance/v2`)
remains the canonical source for declared behaviors, invariants, scenarios,
and change triggers. Quality Runner receipts remain authoritative for current,
stale, failed, blocked, and unknown evidence. Telescope does not introduce a
second behavior contract or infer verification from an action's label, source
mapping, or authored prose.

Repository authors may add `actions` to `.pronto/telescope-map.json` to explain
useful operations in plain language and map them to buildings, rails, and
flows. A `behaviorId` links that visual explanation to the canonical behavior
contract; it does not copy or replace the contract. Declared behaviors without
an authored action are projected automatically as `Review` actions so the
catalog exposes behavioral coverage gaps instead of hiding them. Actions with
no behavior link are explicitly `unprofiled` exploration aids, not behavioral
proof. The projection reports authored, inferred, mapped, behavior-backed, and
unprofiled counts separately.

The desktop surface provides one action search across labels, explanations,
behavior IDs, and source paths. It accepts conversational questions such as
“how does search work?”: the renderer removes conversational filler, ranks the
same projected action metadata, and presents direct and related matches. It
does not invent a behavior or transmit the query. Pressing Enter or selecting
a result focuses the top action's simplified city neighborhood, starts a
guided camera story through mapped facilities, opens the explanation in the
same **What it does** / **How it's built** inspector, and
shows linked behavior state and scenario evidence without displaying runtime
payload values. The selection is read-only; source opening, remediation,
preparation, and release actions still hand off to their existing guarded
surfaces.

## Desktop interaction model

The repository detail drawer exposes Telescope alongside Overview. The
workspace uses a compact repository/freshness strip, synchronized group and
entity navigator, low-chrome architecture canvas, and contextual inspector.
The wider Pronto shell uses a grouped global rail and keeps the repository
switcher separate from Telescope's entity navigator.

The canvas supports pan, zoom, fit, reset, keyboard selection, compound group
expansion, and level-of-detail clustering. Buildings are never draggable; Fit
is the default after load, refresh, and scope changes, and it frames the
semantic scope rather than an unfiltered source graph. Selecting a node, group,
relationship, flow, or catalog action synchronizes the navigator and inspector,
highlights the relevant path, and dims unrelated architecture. The inspector separates
**What it does** from **How it's built**. Directional tokens can be inspected
or paused; reduced-motion mode presents a static token and preserves the same
information.

Scene layout is deterministic and kept separate from the React Flow rendering
path so interaction remains responsive as the measured graph grows. The React
Flow canvas uses custom SVG/CSS nodes and typed rails. Animation updates remain
isolated from topology state to avoid full-canvas rerenders.

The exploration levels are distinct products over stable geometry:

- **Overview** shows the complete explanatory city, its districts, landmarks,
  and one primary story while hiding routine imports.
- **Subsystems** enters the selected district and reveals internal services,
  routes, stores, workers, integrations, and boundary crossings while keeping
  only useful neighboring landmarks.
- **Source detail** enters one building. It renders that building and immediate
  handoffs beside progressively expanded files, symbols, behavioral steps,
  relationships, and line anchors. It must never materialize the repository's
  global file graph; this local scope is both the readability contract and the
  recurrence prevention for the prior source-detail resource exhaustion.

## Workflow lenses

Lenses annotate the base graph and can be enabled independently. They never
rewrite base topology:

1. Changes and ownership projects source-mapped construction, branches,
   worktrees, dirty state, custody, overlaps, and target integration paths.
2. Quality and evidence projects source-mapped inspections, Quality Runner
   findings, maturity, detector coverage, CI, freshness, and installed-runtime
   parity.
3. Remediation filters and annotates architecture connected to open actions,
   dependencies, blockers, and closure evidence.
4. Delivery projects pull requests, checks, release rules, versions, recipes,
   and blockers.
5. Activity and automation accepts verified Pronto events, custody, skill
   usage, promotion candidates, and papercut evidence. Prompt text, filenames,
   and catalog presence are not proof of activity.
6. Intent and conceptual architecture accepts optional Project Compass and ICM
   adapters while keeping them separate from source-derived topology.

The repository projection is the current fidelity boundary. A future fleet
view may connect repository projections through explicit cross-repository
evidence, but it must not weaken repository binding, privacy, or uncertainty.

A lens dims unaffected architecture, activates relevant stories, states the
evidence used, and reports unmapped records. Aggregate evidence with no
repository-relative anchor produces no decorative tint, crew, checkpoint, or
traffic.

## Guarded handoffs

Telescope does not edit source, run remediation, prepare a release, or claim
closure. Source anchors and operational actions hand off to Pronto's existing
editor, remediation, quality, preparation, and release surfaces, where their
normal authority and freshness gates still apply.
