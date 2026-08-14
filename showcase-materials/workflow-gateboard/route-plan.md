# Workflow Gateboard route plan

Status: C0 public Showcase admission is complete; the current dev declaration,
syntax receipt, and dependency block are verified, while state handoff, trace
integration, and no-mutation proof remain gated.

The [canonical target](../ideal-demo-targets.md#workflow-gateboard) owns the
promise and proof gate. The board is a human-readable projection of repository-
declared policy, not a second source of authority.

## 1. Ideal target

**North star:** a developer loads repository-declared gates, previews
prerequisites and mutation class, runs one non-mutating gate, and sees a
continuation-ready receipt.

**Non-negotiable:** inspection never executes, local passes never masquerade as
hosted CI, and gate state keeps freshness and blocked reasons visible.

## 2. Concept materials

All frames below are **concept frames** until one declared gate reaches a
verified trace.

| Frame      | Visual                                                | On-screen line                         | Intended evidence moment |
| ---------- | ----------------------------------------------------- | -------------------------------------- | ------------------------ |
| 1. Declare | Three repository gates with prerequisites and owners  | “The repository declares the contract” | Policy is inspectable    |
| 2. Preview | One gate shows command, mutation class, and freshness | “See what will run”                    | Inspection is safe       |
| 3. Run     | A non-mutating gate progresses through bounded states | “Run one declared check”               | Execution is explicit    |
| 4. Receipt | Result links to Flight Recorder and next action       | “Leave a continuation path”            | Causality is preserved   |

**Preview concept.** A board card moves from declared to previewed to running
to receipt, with stale and blocked alternatives visible below the happy path.

**Narrative spine.** Declare → inspect → preview → run → receipt → continue.

## 3. Build-gap specification

Reviewed baseline: the W2 execution MVP exposes repository-declared non-mutating
gates, prerequisites, freshness, and bounded receipts. The current dev
checkpoint at `f160831` proves a syntax receipt and dependent blocking; it also
shows that a passed prerequisite is not yet persisted across separate CLI
invocations. Trace integration and broader IDE/release evidence remain open.

Project disposition: `targeted_gap_closure` — connect one declared gate to the
recorder and keep no-mutation behavior explicit.

Gap classes: demo_integration — WG-0; product — WG-1; evidence — WG-2;
packaging — WG-3.

| ID   | Gap to close                              | Observable acceptance condition                                                       | Owner             | Required proof                  |
| ---- | ----------------------------------------- | ------------------------------------------------------------------------------------- | ----------------- | ------------------------------- |
| WG-0 | Connect a declared gate to trace evidence | One gate exposes prerequisites, freshness, bounded output, and a parent receipt       | Integration owner | Gateboard/Flight Recorder trace |
| WG-1 | Preserve no-mutation behavior             | Preview and a non-mutating run leave repository refs and files unchanged              | Safety owner      | Before/after repository receipt |
| WG-2 | Keep blocked and stale states honest      | Missing prerequisite, stale evidence, failed gate, and not-run states remain distinct; a passed prerequisite survives refresh | Evidence owner    | Negative scenario matrix and state-handoff proof |
| WG-3 | Package the public case                   | Preview, short copy, no-auth case, and linked receipt agree                           | Showcase owner    | Material review and readback    |

**Required build order:** WG-0 → WG-1 → WG-2 → WG-3. Video is optional after
the evidence gate.
