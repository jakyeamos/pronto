# Quality Setup route plan

Status: C0 public Showcase admission is complete; one current Node/pnpm setup
case, current dev checks, and local visual packaging are verified. Real quality
command smoke and hosted delivery remain gated.

The [canonical target](../ideal-demo-targets.md#quality-setup) owns the promise
and proof gate. The route is explicitly preview-first and recoverable.

## 1. Ideal target

**North star:** a developer previews a supported Quality Runner setup, applies
it explicitly, verifies the installed result, refuses a conflict, and can
recover through a rollback receipt.

**Non-negotiable:** setup is an explicit mutation with a target, authority,
conflict policy, and recovery path. “Installed” cannot be inferred from a
successful command dispatch.

## 2. Concept materials

The local frame below is now a verified binary preview; the scenario and
rollback receipt still remain local fixture evidence.

| Frame             | Visual                                                   | On-screen line                       | Intended evidence moment         |
| ----------------- | -------------------------------------------------------- | ------------------------------------ | -------------------------------- |
| 1. Inspect        | Ecosystem and missing capability are identified          | “Know what this repository supports” | Scope is explicit                |
| 2. Preview        | Planned files, conflicts, and rollback are shown         | “See the change before it happens”   | Mutation is inspectable          |
| 3. Apply          | Human confirms a bounded setup action                    | “Apply with authority”               | No silent installation occurs    |
| 4. Verify/recover | Verification passes or rollback restores the prior state | “Every setup has a way back”         | Result and recovery are receipts |

**Preview concept.** Use a four-state setup card—inspect, preview, apply,
verify—with a visible conflict refusal and rollback branch.

**Narrative spine.** Support check → preview → explicit apply → verify →
conflict/refusal → rollback.

## 3. Build-gap specification

Reviewed baseline: the W1 enabler has preview, explicit apply, conflict refusal,
verify, and rollback receipt behavior in a local slice. The current `dev`
revision is clean and repository checks pass; a standalone public case and
current release evidence are not yet complete.

Project disposition: `targeted_gap_closure` — close one bounded supported setup
case, then package its proof.

Gap classes: demo_integration — QS-0; evidence — QS-1; content — QS-2;
packaging — QS-3.

| ID   | Gap to close                        | Observable acceptance condition                                                         | Owner                     | Required proof                            |
| ---- | ----------------------------------- | --------------------------------------------------------------------------------------- | ------------------------- | ----------------------------------------- |
| QS-0 | Demonstrate one supported ecosystem | **Verified locally:** inspect, preview, apply, conflict refusal, verify, and rollback complete on one fixture | Product/integration owner | `evidence/qs-w1-current-dev-checkpoint.json` |
| QS-1 | Bind setup evidence to the target   | **Verified locally:** current branch/revision, fixture target, result, and rollback state are recorded | Evidence owner            | Current checkpoint + scenario matrix       |
| QS-2 | Write the public explanation        | **Draft complete:** non-specialist case copy and claim boundary are reviewed | Content owner             | Case study + claim ledger                 |
| QS-3 | Package the release surfaces        | **Local partial:** preview image and source are verified; hosted no-auth and readbacks remain open | Showcase owner            | PNG/SVG + hosted/readback evidence        |

**Required build order:** QS-0 → QS-1 → QS-2 → QS-3. QS-0 through QS-2 are
locally closed for this bounded case. QS-3 is locally packaged but not release
complete. Video is optional after the evidence gate.
