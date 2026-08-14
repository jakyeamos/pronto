# Deletion Proof Workbench route plan

Status: C0 public Showcase admission is complete; real deletion and recovery
proof remain gated.

The [ideal target](../ideal-demo-targets.md#deletion-proof-workbench) owns the
promise and proof gate. The safe result may be “unknown consumers remain.”

## 1. Ideal target

**North star:** a developer previews and verifies one bounded deletion while
preserving unknown consumers, explicit authorization, and recovery evidence.

**Non-negotiable:** no universal non-use claim is made; preview, apply,
verification, and recovery are separate states.

## 2. Concept materials

All frames below are **concept frames** until one real export deletion passes.

| Frame            | Visual                                                       | On-screen line                  | Intended evidence moment   |
| ---------------- | ------------------------------------------------------------ | ------------------------------- | -------------------------- |
| 1. Select        | One exported symbol and target revision are pinned           | “Delete a bounded thing”        | Scope is explicit          |
| 2. Inspect       | Known references and unknown consumer classes are separated  | “Unknown is a result”           | Blind spots remain visible |
| 3. Preview/apply | Human sees the plan and authorizes one change                | “Preview before mutation”       | Authority is explicit      |
| 4. Recover       | Verification and recovery reference appear beside the result | “Every deletion has a way back” | Reversibility is proven    |

**Preview concept.** Use a symbol card, reference list, unknown panel, and
preview/apply/recovery state stack. Avoid a green “safe to delete” badge.

**Narrative spine.** Select → inspect references/unknowns → preview → authorize
→ verify → recover.

## 3. Build-gap specification

Reviewed baseline: the W4 proof MVP supports bounded symbol inspection, unknown
consumer classes, explicit apply, and recovery references. A real deletion and
cross-tool evidence remain open.

Project disposition: `targeted_gap_closure` — preview and apply one real
unreferenced export while preserving unknown consumers.

Gap classes: demo_integration — DPW-0; evidence — DPW-1; product — DPW-2;
packaging — DPW-3.

| ID    | Gap to close                      | Observable acceptance condition                                                     | Owner          | Required proof                  |
| ----- | --------------------------------- | ----------------------------------------------------------------------------------- | -------------- | ------------------------------- |
| DPW-0 | Prove one bounded deletion        | A real export is previewed and applied with explicit authorization                  | Product owner  | Current-branch deletion receipt |
| DPW-1 | Preserve unknown consumer classes | Dynamic, generated, external, and unresolved references remain visible              | Evidence owner | Reference/unknown matrix        |
| DPW-2 | Prove recovery and refusal        | Verification failure or changed target refuses safely and retains recovery evidence | Safety owner   | Negative and recovery receipts  |
| DPW-3 | Package the public case           | Preview, short copy, no-auth deletion case, and proof link agree                    | Showcase owner | Material review and readback    |

**Required build order:** DPW-0 → DPW-1 → DPW-2 → DPW-3. Video is optional
after the evidence gate.
