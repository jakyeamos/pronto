# Mac Control route plan

Status: steps 1–3 complete as an aspirational specification. This is not live
or installed-product proof.

The [canonical target](../ideal-demo-targets.md#mac-control) owns the promise,
audience, ownership split, and proof gate. This plan turns it into a concept and
a bounded build target.

## 1. Ideal target

**North star:** a person asks an agent to prepare a focused work session. Mac
Control presents one inspectable multi-action plan, receives one scope-bound
approval, arranges the workspace, verifies each result, and refuses to reuse
the approval after the user changes the environment.

This is intentionally more ambitious than a single menu click. The demo should
make safe orchestration feel useful, not merely show that accessibility APIs can
move a control.

**Non-negotiable:** approval, execution, verification, and stale-state refusal
must remain visible. The story cannot imply unattended or blanket control.

## 2. Concept materials

All frames below are **concept frames** until the proof gate passes.

| Frame            | Visual                                                                                   | On-screen line                     | Intended evidence moment                                  |
| ---------------- | ---------------------------------------------------------------------------------------- | ---------------------------------- | --------------------------------------------------------- |
| 1. The ask       | Clean desktop beside a compact Control Center; “Prepare my research session” is entered  | “One request. A bounded Mac plan.” | The outcome is understandable before architecture appears |
| 2. The plan      | Three action cards: open the synthetic brief, open its scratchpad, arrange both windows  | “Review exactly what will change”  | Targets, effects, and reversibility are explicit          |
| 3. Human control | Approval sheet highlights scope, expiry, and single-use behavior                         | “Nothing runs until you approve”   | Human authority is unmistakable                           |
| 4. Execution     | Desktop transforms while the plan advances action by action                              | “Apply once”                       | The product performs a useful coordinated flow            |
| 5. Proof         | Before/after desktop and a redacted receipt show three verified outcomes                 | “Every action earns a receipt”     | Visible state and structured evidence agree               |
| 6. Refusal       | The user changes one window; replaying the old plan returns “State changed—review again” | “Stale approval is not approval”   | Safety is demonstrated as behavior, not copy              |

**Preview concept.** A 16:9 split frame: transformed desktop on the left; the
approval-to-verified receipt stack on the right. Headline: “Agents can act on
your Mac without taking control away.” Keep the approval badge and verified
outcome legible at card size.

**Narrative spine.** Intent → inspectable plan → human approval → useful
multi-action change → independent proof → stale-state refusal.

**Art direction.** Native, calm, and high-contrast. Use one synthetic project
name and a clean account. Avoid terminal-led framing, private paths, floating
security theater, or a wall of architecture labels.

## 3. Build-gap specification

Reviewed baseline: the owner-only control plane and authorization boundaries
are product-capable. MC-1 has a deterministic preview-only product surface.
MC-2's progress projection and safe partial-failure behavior, MC-3's
plan-bound verification contract, and MC-4's approval/refusal behavior now pass
at the source-build level. Their live observations remain open. The bounded
document-open and two-window-layout executors required by MC-2 are explicitly
still a product gap; the preview is not executable. Live focus-session
execution, direct current-surface evidence, and installed proof remain open.

Project disposition: `targeted_gap_closure` — compose and prove one ambitious
multi-action story on the existing control plane, with only the UI and behavior
changes demanded by that story.

Gap classes: product — MC-1, MC-2; evidence — MC-3, MC-4, MC-6;
demo_integration — MC-5.

| ID   | Gap to close                                                                                                             | Observable acceptance condition                                                                                                                                                                        | Owner                                | Required proof                                                         |
| ---- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------ | ---------------------------------------------------------------------- |
| MC-1 | Compose the focus-session actions into one previewable bounded plan                                                      | A clean demo request produces the same ordered targets, effects, rollback notes, and approval digest twice                                                                                             | Product/implementation owner         | Redacted plan fixture and deterministic comparison                     |
| MC-2 | Implement the bounded document-open and layout routes, then make their multi-action progress and partial failure legible | The exact build opens only the two fixture documents, arranges their windows, and shows pending, running, verified, and stopped states; a seeded failure stops safely without hiding completed actions | Product, UI, and control-plane owner | Direct current-surface readback plus structured partial-result receipt |
| MC-3 | Verify each intended postcondition independently                                                                         | Brief, scratchpad, and two-window layout outcomes are read back through target-appropriate observers rather than assumed from dispatch success                                                         | Verification owner                   | Three redacted verification records tied to the plan                   |
| MC-4 | Bind approval to current state and one execution                                                                         | A manually changed target causes replay to refuse before any second mutation                                                                                                                           | Authorization owner                  | Approval digest, changed-state evidence, and refusal receipt           |
| MC-5 | Establish a clean capture environment                                                                                    | A fresh demo account can run the story without private paths, notifications, tokens, or unrelated windows                                                                                              | Demo operations owner                | Capture checklist and privacy review                                   |
| MC-6 | Prove the current installed surface                                                                                      | The exact build used for capture completes the positive and stale-state paths from the visible app                                                                                                     | Release/verification owner           | Build identity, live run receipts, and direct visual readback          |

**Build order:** MC-1 → MC-2/MC-3 → MC-4 → MC-5 → MC-6. Rehearsal starts
only after all six acceptance conditions pass.

## Closure ledger

- **MC-1 — passed at source-build level, 2026-08-12.** Two normalized forms of
  the clean request produced byte-identical output and approval digest
  `22c4bbe245b375ee99a5d44aa2d7376956a5b3d4ed87ee346d46b82e4118723e`.
  The unsupported-request check failed closed. See
  [the redacted plan](evidence/focus-session-plan.json) and
  [the deterministic comparison](evidence/focus-session-determinism.json).
- **MC-2 — product behavior implemented; acceptance still open, 2026-08-12.**
  The source-built candidate now contains three product-owned, allowlisted
  operations with no caller-controlled path, content, application, script, or
  frame. The Control Center derives `Pending`, `Running`, `Verified`, and
  `Stopped` rows from the durable task checkpoint. A seeded postcondition
  failure retained the first verified action, stopped the second, released
  input authority, and kept the redacted result visible for 60 seconds. The
  focused suite passed 19 tests and the full Swift suite passed 252. See the
  [structured source evidence](evidence/control-center-progress-source.json)
  [bounded executor evidence](evidence/focus-session-execution-source.json),
  and [inspected source-built render](evidence/control-center-progress-source.png).
  The required installed-build screen recording and live focus-session receipt
  are not yet proven, so MC-2 is not closed.
- **MC-3 — verification behavior implemented; acceptance still open,
  2026-08-12.** The source verifier accepts only post-dispatch observations,
  binds every record to the exact plan digest, checks fixture identities and
  independent Accessibility window state, compares both frames with an explicit
  tolerance, redacts titles and paths, and fails all records for a stale plan.
  Five focused tests pass. See the
  [structured source evidence](evidence/focus-session-verification-source.json).
  Live Preview/TextEdit observations and three current-build records do not yet
  exist, so MC-3 is not closed.
- **MC-4 — authorization behavior covered; acceptance still open,
  2026-08-12.** A deterministic changed-target test refused with
  `precondition_failed:target_changed` before the executor ran, kept the task
  prepared, completed once after state restoration, and rejected a second run
  as `invalid_state:completed`. The focused suite passed six tests. See the
  [structured source evidence](evidence/focus-session-approval-replay-source.json).
  The target change was injected, not observed from a live desktop, so the
  changed-state evidence and installed refusal receipt remain open.
- **MC-5 and MC-6 — open.** The focus-session preview still declares
  `executable=false` and `blocked_by=["MC-2", "MC-3"]`; it is not live or
  installed-product proof. The documented install step was attempted after all
  252 source tests passed and correctly stopped for explicit authority to
  replace the persistent installed CLI and daemon. No workaround was used.
