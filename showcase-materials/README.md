# AI Showcase material workspace

This directory holds the working artifacts for the canonical
[material production route](../docs/showcase-contract.md#material-production-route).
It is an internal production surface, not a publication record.

## Canonical homes

- [Ideal demo targets](ideal-demo-targets.md) own each project's creative north
  star, audience, role split, and proof gate.
- [Public release targets](public-release-targets.md) own the durable channel
  sequence for each eligible project; the paired JSON is machine-readable
  planning data, not an external publication record.
- [Release material inventory](release-material-inventory.md) joins the
  project-level material gaps to the destination rows so previews, copy,
  evidence, case pages, review work, and publication receipts are not hidden
  behind one project-level next step.
- [Postability ledger](postability-ledger.json) is the machine-readable handoff
  for local packet completeness, open gates, and explicit deferrals. It never
  claims that a project was posted; publication still requires a fresh external
  receipt and readback.
- [Local-fix audit](local-fix-audit.md) explains the shared material-fix receipt
  that folds story, evidence, case source, preview, copy, and claim-boundary
  work into one durable pass without closing live or hosted gates.
- Each public project owns one `<project>/route-plan.md` for its concept frames
  and build-gap specification.
- A combined portfolio story owns a separate route and must not replace or
  rename any participating project's standalone route. The current combined
  story is the [AI Code Quality Stack](ai-code-quality-stack/route-plan.md).
- A project package owns evidence, final copy, production assets, and any
  optional video or rehearsal notes once those stages begin.
- The first-wave [Handshake draft packets](handshake/README.md) collect the
  upload copy, crop-safe preview, proof links, role boundaries, and remaining
  publication gates for Pre-CR, Context Compiler Contract, Terrace, and
  Quality Runner. They are review artifacts, not publication records.
- [Optional video readiness](rehearsal-readiness.md) owns the exact 38-project
  video-enhancement ledger; it is not a publication gate and never upgrades
  aspirational material to proven behavior.
- `.pronto/showcase-goal.json` owns eligibility, reviewed readiness, remaining
  materials, and the active next step.

Concepts may lead current implementation, but every unbuilt element remains
labeled until evidence closes its proof gate. Creating a route plan does not by
itself increase product or material readiness.

## Route-plan completion rule

Steps 1–3 are complete only when the route plan:

1. links to the canonical ideal target and sharpens its north star;
2. specifies the storyboard, key-frame content, preview direction, and narrative;
3. labels concept-only behavior;
4. states the reviewed baseline without turning source evidence into live proof;
5. gives every build gap an ID, observable acceptance condition, role owner, and
   required proof;
6. assigns every gap exactly one durable category: `product`,
   `demo_integration`, `evidence`, `content`, or `packaging`; and
7. records the project-level work disposition and orders the gaps so the project
   can enter `gap_closure` without another strategy pass.

`gap_closure` does not imply product construction. A largely product-ready
project can move through the stage using only evidence, content, or packaging
work. Product work is required only where the classified ledger says it is.

Later discoveries can revise a plan. Material promise changes belong in the
ideal target; implementation details stay in the owning repository.
