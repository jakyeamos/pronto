# Repository Telescope

Last reviewed: 2026-08-18.

Telescope is Pronto's read-only, repository-level architecture map. It turns
the active registered worktree into an inspectable spatial model, then layers
Pronto evidence over that same topology. Telescope is a repository feature,
not Pronto's global navigation model, and the source-derived graph remains
canonical when optional workflow adapters are unavailable.

## Projection contract

The Rust core owns `pronto-telescope/v1`. Both
`get_repository_telescope` and `pronto telescope <repository> --json` return
that projection. `refresh_repository_telescope` and
`pronto telescope refresh <repository> --json` force regeneration before
returning the same schema.

Desktop refresh is cooperatively cancellable between directories and source
files. Cancellation never saves a partial projection and leaves the last
matching cached projection intact.

The projection contains:

- `groups`, `nodes`, `edges`, and directional `flows` with deterministic IDs;
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

## Desktop interaction model

The repository detail drawer exposes Telescope alongside Overview. The
workspace uses a compact repository/freshness strip, synchronized group and
entity navigator, low-chrome architecture canvas, and contextual inspector.
The wider Pronto shell uses a grouped global rail and keeps the repository
switcher separate from Telescope's entity navigator.

The canvas supports pan, zoom, fit, reset, keyboard selection, compound group
expansion, and level-of-detail clustering. Selecting a node, group,
relationship, or flow synchronizes the navigator and inspector, highlights
the relevant path, and dims unrelated architecture. The inspector separates
**What it does** from **How it's built**. Directional tokens can be inspected
or paused; reduced-motion mode presents a static token and preserves the same
information.

ELK layout runs in a web worker so graph layout does not block the renderer.
The React Flow canvas uses custom SVG/CSS nodes and edges. Animation updates
remain isolated from topology state to avoid full-canvas rerenders.

## Workflow lenses

Lenses annotate the base graph and can be enabled independently. They never
rewrite base topology:

1. Changes and ownership projects branches, worktrees, dirty state, custody,
   overlaps, and target integration paths.
2. Quality and evidence projects Quality Runner findings, maturity, detector
   coverage, CI, freshness, and installed-runtime parity.
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

## Guarded handoffs

Telescope does not edit source, run remediation, prepare a release, or claim
closure. Source anchors and operational actions hand off to Pronto's existing
editor, remediation, quality, preparation, and release surfaces, where their
normal authority and freshness gates still apply.
