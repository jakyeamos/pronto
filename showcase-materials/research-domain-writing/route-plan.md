# Research Domain Writing route plan

Status: the real case, negative path, and visual claim trace are assembled.
RDW-1, RDW-2, and RDW-3 are closed. RDW-0 is partial: the installed pack passes
and matches known repository content, but the release artifact is not bound to
an exact source revision. RDW-4 remains a human editorial gate, and RDW-5
remains open until provenance, hosting, and public verification close.

The [canonical target](../ideal-demo-targets.md#research-domain-writing) owns the
durable promise and proof gate.

## 1. Ideal target

**North star:** let a viewer watch one dangerously plausible claim move from
source packet to draft, fail the claim-safety gate, and return as narrower,
useful prose with a transparent evidence trail.

**Non-negotiable:** the demo must show research judgment, not “AI writes an
article.” Human source selection and publication authority remain explicit.

## 2. Concept materials

The six-frame concept is now implemented in `public/index.html`. It remains a
publication candidate until the open release, editorial, hosting, and owner
gates close.

| Frame                   | Visual                                                            | On-screen line                              | Intended evidence moment                  |
| ----------------------- | ----------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------- |
| 1. Research question    | A concise consequential prompt sits above a small source packet   | “Useful writing starts before the draft”    | Sources and scope precede generation      |
| 2. Claim map            | Supported, inference, and unsupported claims form a visual ledger | “Know what each sentence is allowed to say” | Claim types are legible                   |
| 3. Tempting draft       | A polished but overstrong sentence appears in context             | “Plausible is not proven”                   | The unsafe claim feels realistic          |
| 4. Safety stop          | QA blocks the sentence and opens the missing evidence             | “Stop the claim, not the whole workflow”    | Refusal is precise and useful             |
| 5. Repair               | The claim narrows or is replaced; citations remain attached       | “Write to the evidence you actually have”   | The final prose improves without bluffing |
| 6. Publication boundary | Human review remains required beside the final ledger             | “Research-assisted. Human-published.”       | Responsibility is explicit                |

**Preview concept.** A beautiful draft sentence crossed by an evidence boundary,
resolving into a narrower cited sentence. Headline: “A writing system that knows
when the research is not enough.”

**Narrative spine.** Question → source packet → claim model → unsafe draft →
targeted stop → evidence-safe final.

## 3. Build-gap specification

Reviewed baseline: the installed CLI validates the packet, research readiness,
the failing exact-draft QA receipt, the repaired exact-draft claim ledger, and
the installed basketball pack as `specialized/production`. The installed
domain contract is byte-identical to repository revision `aa46b36`, but the
wheel does not embed that revision, its recorded build worktree no longer
exists, and the active source checkout differs. The visual walkthrough is now
locally addressable; release provenance and public hosting are not proven.

Project disposition: `targeted_gap_closure` — preserve the proven packet and QA
path, reconcile installed/source pack provenance, then package the proof
visually.

Gap classes: demo_integration — RDW-0; content — RDW-1; packaging — RDW-2,
RDW-5; evidence — RDW-3, RDW-4.

| ID    | Gap to close                              | Observable acceptance condition                                                                                                            | Owner                 | Required proof                                              |
| ----- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------- | ----------------------------------------------------------- |
| RDW-0 | Reconcile installed/source pack contract  | **Partial:** installed validation passes and content matches `aa46b36`; the intended release artifact is bound to an exact source revision | Release/product owner | `rdw-0-provenance.json` and rebuilt or attested artifact    |
| RDW-1 | Author the representative source packet   | **Closed:** the Tatum packet supports a real 2023–24 box-score line and intentionally lacks impact evidence                                | Research owner        | `source-packet.yaml`, case and claim ledgers                |
| RDW-2 | Make the claim trace visually addressable | **Closed:** selecting a claim reveals its source, support type, and uncertainty without exposing implementation clutter                    | Product/design owner  | `public/index.html` interactive trace                       |
| RDW-3 | Reproduce the unsafe-claim stop           | **Closed:** fresh QA rejects the unsupported impact inference; both failing and repaired receipts pass exact-draft validation              | Workflow owner        | `rdw-run/unsafe-qa.yaml`, repaired final, validation record |
| RDW-4 | Preserve voice through repair             | The corrected paragraph remains readable and useful after safety intervention                                                              | Editorial owner       | Side-by-side human review                                   |
| RDW-5 | Package public evidence                   | **Partial:** local candidate includes sources, trace, draft, final, and limitations; provenance and no-auth hosting remain open            | Showcase owner        | `final-package.json`, link check, hosted claim audit        |

**Build order:** RDW-0 → RDW-2 → RDW-4 → RDW-5. RDW-1, RDW-2, and RDW-3 are
closed; RDW-0 and RDW-5 are partial, and RDW-4 is the next owner gate.

**Next closure:** the release owner must select and attest or rebuild the exact
source revision for RDW-0. The editorial owner can then close RDW-4 by reviewing
the unsafe and repaired prose side by side. RDW-5 closes only after those gates,
no-auth hosting, responsive public verification, and owner copy approval.
