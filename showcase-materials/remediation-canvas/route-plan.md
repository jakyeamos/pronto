# Remediation Canvas route plan

Status: C0 public Showcase admission is complete; the current dev checkpoint
and binary preview are verified, while partial-remediation handoff and
source-authority parity remain gated.

The [canonical target](../ideal-demo-targets.md#remediation-canvas) owns the
promise and proof gate. The canvas organizes authoritative references; it does
not copy finding payloads into a second source of truth.

## 1. Ideal target

**North star:** a developer groups related findings, states intent, verifies
once, and leaves a partial or completed remediation handoff without a second
issue tracker.

**Non-negotiable:** references remain stable, human intent is visible, and a
changed finding set makes the canvas stale rather than silently preserving a
false plan.

## 2. Concept materials

All frames below are **concept frames** until a real finding set and stale
refresh pass.

| Frame            | Visual                                                         | On-screen line                | Intended evidence moment           |
| ---------------- | -------------------------------------------------------------- | ----------------------------- | ---------------------------------- |
| 1. Gather        | Related findings appear as stable references                   | “Work from one evidence set”  | Authority is preserved             |
| 2. Intent        | Human writes the remediation intent and scope                  | “Say what you are changing”   | Judgment is explicit               |
| 3. Verify        | One bounded verification updates several references            | “Verify once, keep the links” | Continuity is useful               |
| 4. Stale handoff | Changed findings mark the plan stale and preserve partial work | “Do not hide drift”           | Incomplete work remains actionable |

**Preview concept.** Use a canvas with three finding references, one intent card,
one verification receipt, and a clearly stale branch.

**Narrative spine.** Gather → state intent → verify → preserve partial work →
refresh.

## 3. Build-gap specification

Reviewed baseline: the W3 composition MVP records stable Quality Lens finding
references, human intent/dispositions, and stale refresh behavior. The current
dev checkpoint reruns fresh and stale refresh flows, and a complete
partial-remediation handoff remains open.

Project disposition: `targeted_gap_closure` — complete one disposition-and-
refresh handoff with partial work preserved.

Gap classes: demo_integration — RC-0; product — RC-1; evidence — RC-2;
packaging — RC-3.

| ID   | Gap to close                     | Observable acceptance condition                                                                    | Owner             | Required proof               |
| ---- | -------------------------------- | -------------------------------------------------------------------------------------------------- | ----------------- | ---------------------------- |
| RC-0 | Complete the disposition handoff | Finding reference, human intent, verification state, and next action survive a partial remediation | Integration owner | Handoff receipt              |
| RC-1 | Preserve source authority        | Canvas refreshes references rather than copying or overriding Quality Lens payloads                | Product owner     | Authority/parity check       |
| RC-2 | Prove stale refresh              | Changed findings make the plan stale while partial work remains visible                            | Evidence owner    | Stale fixture and receipt    |
| RC-3 | Package the public case          | Binary preview, short copy, no-auth canvas case, and proof link agree                              | Showcase owner    | Material review and readback |

**Required build order:** RC-0 → RC-1 → RC-2 → RC-3. Video is optional after
the evidence gate.
