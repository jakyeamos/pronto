# Pronto UI Refresh Plan

## Purpose

Pronto's interface should feel calm, open, and easy to move through even when
the underlying evidence is dense. The refresh is a product-wide information
architecture and interaction change, not a reskin and not a consequence of the
Telescope feature.

Telescope will use the refreshed shell when it is implemented, but it remains
one repository-level workspace. It must not become the organizing model for the
rest of Pronto.

## Desired experience

The refreshed app should provide:

- one obvious primary workspace on every route;
- a compact global navigation rail instead of a permanently crowded sidebar;
- repository switching that is fast without consuming persistent screen space;
- contextual detail that appears in inspectors and drawers only when needed;
- restrained visual chrome, generous whitespace, and clear hierarchy;
- progressive disclosure without hiding freshness, blockers, uncertainty, or
  required evidence;
- consistent interaction patterns across portfolio, repository, remediation,
  analytics, skills, and configuration surfaces.

Success is not measured by fitting less information on screen. It is measured
by whether the user can identify the current context, the primary decision, and
the next safe action without competing panels demanding equal attention.

## Product principles

### One workspace, one context, one inspector

Each route has a single primary surface. Supporting information belongs in a
contextual inspector, a temporary drawer, or an explicit secondary view. Avoid
layouts that combine a page header, metric grid, multiple card collections,
and a permanent right rail at the same visual priority.

### Navigation and repository context are different

The global rail answers **where in Pronto am I?** Repository switching answers
**which project am I looking at?** They must not compete inside the same
permanent sidebar.

The global rail groups destinations by purpose:

- **Overview:** Portfolio;
- **Work:** Remediation and Promotion inbox;
- **Intelligence:** Analytics, Skills, and Activity;
- **Organize:** AI showcase, Groups, and Remote catalog;
- **Configuration:** Settings, anchored at the bottom.

The rail defaults to icon-only presentation with accessible labels and an
intentional expansion control. Group labels appear in the expanded state, not
as persistent chrome.

### Evidence remains visible

Reducing clutter must never turn stale, inferred, conflicted, unknown, blocked,
or unavailable evidence into a clean-looking success state. Freshness and
confidence use a shared compact status language across every route. Additional
explanation can move into an inspector, but the status itself stays adjacent to
the claim it qualifies.

### Selection drives disclosure

Selecting a repository, condition, finding, action, chart point, skill, or
event updates the shared right inspector. The inspector should reuse the same
structure across domains: summary, evidence, relationships, and available
handoffs. Full-screen modals are reserved for consequential confirmation or
focused creation flows.

### Density is deliberate

Default views show the information needed to orient and choose. Tables and
lists may remain dense where comparison is the task, but surrounding chrome
should be quiet. Cards are reserved for independent objects or meaningful
status summaries, not used as the default container for every paragraph.

## Application shell

The refreshed desktop shell has four persistent regions:

1. **Global rail** — narrow, grouped navigation with Settings anchored at the
   bottom.
2. **Context bar** — current destination, repository or portfolio scope,
   freshness, search, refresh, and the most important route action.
3. **Primary workspace** — the route's main list, canvas, table, timeline, or
   builder.
4. **Context inspector** — optional right panel opened by selection and closed
   without losing workspace state.

The current always-visible repository list becomes a repository switcher in the
context bar. It supports search, recent repositories, attention state, and
keyboard selection. A command palette provides destination and repository
navigation from anywhere in the app.

At narrower desktop widths, the inspector becomes an overlay drawer. The
global rail remains available in its compact state. Routes must not introduce
horizontal page scrolling to preserve a three-column layout.

## Visual system

The visual direction is light, neutral, and low-chrome:

- use one canvas background, one raised surface, and hairline separators;
- reserve the blue accent for focus, selection, links, and active flow;
- use semantic colors only for evidence status and never as decoration;
- remove nested borders when spacing or a single divider communicates the same
  grouping;
- use shadows only for temporary layers such as drawers, menus, and floating
  inspectors;
- keep headings short and remove repeated eyebrow, title, and explanatory-copy
  stacks when the context bar already names the page;
- establish a compact body scale, a clear section scale, and one display scale
  rather than route-specific typography;
- standardize spacing, radii, focus rings, status chips, empty states, tooltips,
  and skeletons through semantic design tokens.

Motion should explain state changes: rail expansion, inspector entry, selection
movement, and route continuity. Transitions stay brief and must honor reduced
motion. Ambient or decorative animation is excluded from ordinary evidence
surfaces.

## Route migration

### Portfolio and repository detail

Start with the surfaces that establish the shell. Portfolio becomes a focused
repository workspace with compact summary filters above the primary list.
Attention and recent activity move into the inspector or an explicit secondary
view instead of a permanent right rail.

Opening a repository changes the context bar and primary workspace rather than
presenting a drawer on top of the portfolio. Repository domains become stable
workspace tabs or modes. Their detailed evidence continues to use the shared
inspector and existing guarded handoffs.

### Remediation and promotion work

Treat these as queues. Preserve ranking and blocked-state evidence, but give the
selected plan or candidate one inspector rather than repeating full detail in
every row. Preparation, export, and status-changing actions retain their
existing confirmation and audit boundaries.

### Analytics, skills, and activity

Analytics keeps one primary analysis canvas with controls in a compact toolbar.
Skills and Activity use comparison-friendly lists with selection-driven detail.
The refresh must not turn unavailable telemetry into inferred usage or combine
source compatibility with observed behavior.

### Showcase, groups, remote catalog, and settings

These routes adopt the same shell after the core operational surfaces are
stable. Configuration screens may use forms, but labels, validation, status,
and save boundaries follow the shared inspector and feedback patterns.

### Telescope integration seam

Telescope may add an entity navigator inside its primary workspace. That
navigator is local to the map and does not replace or expand the global rail.
Its repository, workspace, freshness, and lens controls belong in the shared
context bar; selected architecture detail belongs in the shared inspector.

## Implementation sequence

### Phase 1: foundation

- Define semantic tokens for color, typography, spacing, elevation, motion,
  focus, and evidence states.
- Build the compact rail, context bar, repository switcher, command palette,
  primary workspace container, and shared inspector.
- Preserve existing keyboard behavior, accessibility semantics, Mac Control
  target identifiers, and action authorization boundaries.
- Add visual fixtures for empty, loading, stale, conflicted, blocked, and
  selected states before migrating product routes.

### Phase 2: operational core

- Migrate Portfolio and repository detail to prove navigation, repository
  context, responsive behavior, and inspector state.
- Migrate Remediation and Promotion inbox using the shared queue and inspector
  patterns.
- Remove superseded shell and route-specific layout styles as each migrated
  surface becomes authoritative; do not maintain two long-lived UI systems.

### Phase 3: intelligence and configuration

- Migrate Analytics, Skills, and Activity.
- Migrate Showcase, Groups, Remote catalog, and Settings.
- Complete consistency and accessibility passes across all routes, then remove
  transitional tokens and components.

### Phase 4: feature work on the refreshed shell

- Add Telescope or other new workspaces only after the shell contracts are
  stable and directly exercised.
- New features must reuse the context bar, workspace, inspector, evidence, and
  navigation primitives rather than adding permanent rails or bespoke page
  chrome.

## Verification and acceptance

The refresh is complete only when:

- every existing destination remains reachable by pointer and keyboard;
- repository switching preserves destination context and never opens the wrong
  worktree or repository;
- freshness, confidence, applicability, and blocked states retain their current
  meaning;
- consequential actions still use their existing preflight, confirmation,
  authorization, and audit paths;
- selection and inspector state have deterministic back, close, and route-change
  behavior;
- compact-width layouts remain usable without hiding the primary action or
  creating horizontal page scrolling;
- reduced-motion, focus order, landmarks, labels, contrast, and screen-reader
  announcements pass accessibility review;
- representative empty, loading, partial, stale, conflicted, and error states
  receive visual acceptance, not only the populated happy path;
- the packaged Tauri app is exercised directly at normal and compact desktop
  widths.

Use route-level interaction tests for navigation and disclosure, shared
component tests for shell behavior, and a small visual-regression set for the
canonical states above. Source tests and production builds support the claim;
they do not replace direct verification of the running app.

## Explicit non-goals

- Making Telescope the main navigation or information architecture.
- Hiding evidence complexity by removing freshness, confidence, or provenance.
- Rebuilding domain logic while migrating its presentation.
- Introducing new modifying actions or weakening existing safety boundaries.
- Keeping duplicate legacy and refreshed shells indefinitely.
- Copying another product's branding, assets, or ornamental motion.

## Pickup checklist

Before implementation begins:

- re-read the current product decisions and repository operating contract;
- inspect the live packaged app and record baseline screenshots at normal and
  compact widths;
- inventory active ownership and create an isolated worktree from the intended
  integration base;
- identify the shared shell and styling sources rather than duplicating route
  components;
- update the change-surface matrix with the renderer, interaction, desktop,
  documentation, and visual-verification consequences;
- implement and verify Phase 1 before beginning route migrations.
