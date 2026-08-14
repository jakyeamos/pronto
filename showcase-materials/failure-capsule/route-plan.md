# Failure Capsule route plan

Status: C0 public Showcase admission is complete; current redaction and
inspect-only proof are verified while Replay handoff, cancellation, and
recovery remain gated.

The [canonical target](../ideal-demo-targets.md#failure-capsule) owns the
promise and proof gate. The capsule is a bounded handoff, not a machine-state
archive.

## 1. Ideal target

**North star:** a failed declared command becomes a portable, redacted capsule
that an inheritor can inspect without receiving the entire machine state.

**Non-negotiable:** capture declares omissions and secrets are removed before
the capsule leaves its source boundary. Inspection never implies reproduction.

## 2. Concept materials

The current redaction/inspection frame is locally verified; Replay handoff and
the complete privacy/recovery matrix remain gated.

| Frame        | Visual                                                      | On-screen line               | Intended evidence moment                   |
| ------------ | ----------------------------------------------------------- | ---------------------------- | ------------------------------------------ |
| 1. Failure   | One named command fails with target and artifact identity   | “Make the failure portable”  | The problem is bounded                     |
| 2. Redaction | Secret-like fields disappear while omissions remain labeled | “Show what was not captured” | Privacy is visible                         |
| 3. Inspect   | Capsule opens without executing anything                    | “Read before you rerun”      | Handoff is safe                            |
| 4. Recovery  | Replay preview and next action remain explicit              | “Continue with authority”    | Recovery does not become hidden automation |

**Preview concept.** A compact capsule card shows failed step, redacted output,
omissions, target identity, and a separate replay-preview button.

**Narrative spine.** Failure → bounded capture → redaction/omissions → inspect
→ explicit recovery.

## 3. Build-gap specification

Reviewed baseline: the current `dev` head passes tests, lint, and packaging and
captures bounded redacted failures with target/tool/artifact identity and
inspect-only opening. Replay integration and the full cancellation/recovery
story remain open.

Project disposition: `targeted_gap_closure` — connect one Debug Trail failure
to Evidence Replay without widening capture authority.

Gap classes: demo_integration — FC-0; evidence — FC-1; product — FC-2;
packaging — FC-3.

| ID   | Gap to close                    | Observable acceptance condition                                         | Owner              | Required proof                        |
| ---- | ------------------------------- | ----------------------------------------------------------------------- | ------------------ | ------------------------------------- |
| FC-0 | Prove the failure handoff       | A Debug Trail failure becomes a bounded capsule and opens in Replay     | Integration owner  | Cross-tool receipt                    |
| FC-1 | Prove privacy and omissions     | Redaction, omission labels, and malformed input handling remain visible | Safety owner       | Redaction and negative fixture matrix |
| FC-2 | Prove cancellation and recovery | Cancelled, failed, and explicitly rerunnable states remain distinct     | Verification owner | Scenario receipts                     |
| FC-3 | Package the public case         | Binary preview, short copy, no-auth capsule case, and proof link agree | Showcase owner     | Material review and readback          |

**Required build order:** FC-0 → FC-1 → FC-2 → FC-3. Video is optional after
the evidence gate.
