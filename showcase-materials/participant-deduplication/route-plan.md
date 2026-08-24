# Participant Deduplication route plan

Status: PD-1 through PD-5 are closed for the synthetic material and the
repository's deterministic local behavior probes. The fixture is now available
as both a reviewable CSV sheet and a no-auth HTML sheet preview. PD-6 is parked
at the authenticated live-sheet boundary. PD-7 is closed as a local, no-auth
static case built only from the labeled synthetic and local receipts; hosting
and live-provider proof remain open.

The [canonical target](../ideal-demo-targets.md#participant-deduplication) owns
the durable promise and proof gate.

## 1. Ideal target

**North star:** scan a synthetic participant workbook containing exact, fuzzy,
and deliberately ambiguous pairs; explain why each pair was proposed; let the
reviewer reject ambiguity and approve true duplicates; then apply only approved
changes to a copy with a complete recovery trail.

**Non-negotiable:** identity judgment and destructive approval remain human.
The original workbook must stay untouched throughout the demo.

## 2. Concept materials

All frames are **concept** until synthetic and authenticated paths are verified.

| Frame              | Visual                                                          | On-screen line                                | Intended evidence moment              |
| ------------------ | --------------------------------------------------------------- | --------------------------------------------- | ------------------------------------- |
| 1. Messy workbook  | Synthetic rows include typos, shared emails, and similar names  | “Duplicates are rarely exact”                 | Operational pain is recognizable      |
| 2. Review queue    | Candidate pairs rank with field-level reasons and confidence    | “Show why the rows might match”               | Proposals are explainable             |
| 3. Ambiguous pair  | Similar names but conflicting details stay unresolved           | “Confidence is not identity”                  | The system respects uncertainty       |
| 4. Human decision  | Reviewer rejects ambiguity and approves two true pairs          | “A person owns the deletion decision”         | Human authority changes the queue     |
| 5. Copy-only apply | Approved cleanup runs against a clearly named workbook copy     | “Protect the original by design”              | Recovery is structural                |
| 6. Audit proof     | Original/copy comparison and audit timeline show every decision | “Every changed row has a reason and reviewer” | Outcome is inspectable and reversible |

**Preview concept.** A clean review queue with one approved, one rejected, and
one ambiguous pair, beside an “Original untouched” badge. Headline: “Find likely
duplicates without turning confidence into deletion authority.”

**Narrative spine.** Messy data → explainable candidates → meaningful ambiguity
→ human decisions → copy-only apply → audit and recovery.

## 3. Build-gap specification

Reviewed baseline: fuzzy review, reviewer-controlled atomic deletion on a copy,
stale-state refusal, audit history, build output, and substantial tests exist;
authenticated live-sheet UAT remains external.

Project disposition: `targeted_gap_closure` — keep the mature safety behavior
and close synthetic-fixture, explanation, live-sheet proof, and public-case gaps.

Gap classes: content — PD-1; product — PD-2; evidence — PD-3, PD-4, PD-5;
demo_integration — PD-6; packaging — PD-7.

| ID   | Gap to close                           | Observable acceptance condition                                                                                                                                          | Owner                  | Required proof                                                                                                           |
| ---- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| PD-1 | Author the synthetic workbook          | **Closed:** fixture covers exact, fuzzy, shared-contact, and ambiguous non-duplicate cases with expected reviewer outcomes; source matcher probe and privacy review pass; CSV and HTML sheet views are labeled synthetic | Data/product owner     | `synthetic-fixture.json`, `synthetic-sheet.csv`, `synthetic-sheet.html`, and `evidence/pd-1-fixture-receipt.json`          |
| PD-2 | Make candidate reasoning legible       | **Closed:** each proposed pair shows native field contributions, conflicts, confidence limits, and the bounded reviewer action; no mutating action is exposed            | Matching/product owner | `candidate-reasoning.json` and `evidence/pd-2-explanation-review.json`                                                   |
| PD-3 | Prove reviewer-controlled atomic apply | **Closed locally:** only approved candidates change, rejected/ambiguous rows do not, and the deterministic apply path writes a coherent audit batch                      | Workflow owner         | [`evidence/pd-3-atomic-apply-receipt.json`](evidence/pd-3-atomic-apply-receipt.json)                                     |
| PD-4 | Prove stale-state refusal              | **Closed locally:** editing a candidate row after scan invalidates apply before any deletion                                                                             | Safety owner           | [`evidence/pd-4-stale-state-receipt.json`](evidence/pd-4-stale-state-receipt.json)                                       |
| PD-5 | Prove copy-only recovery               | **Closed locally:** original digest remains unchanged; the copied workbook and audit history reconstruct every local action                                              | Data-integrity owner   | [`evidence/pd-5-copy-recovery-receipt.json`](evidence/pd-5-copy-recovery-receipt.json)                                   |
| PD-6 | Complete authenticated live-sheet UAT  | The current deployed Apps Script/backend path passes scan, review, apply, and recovery on the synthetic workbook                                                         | Integration/QA owner   | Authenticated UAT checklist and run evidence                                                                             |
| PD-7 | Build the public synthetic case        | **Closed locally:** a no-auth static viewer exposes the queue, decisions, copy-only result, audit proof, and explicit live-provider limits                               | Showcase owner         | [`case-study.html`](case-study.html), [`evidence/pd-7-public-case-receipt.json`](evidence/pd-7-public-case-receipt.json) |

**Build order:** PD-1 (**closed**) → PD-2 (**closed**) → PD-3/PD-4/PD-5
(**closed locally**) → PD-6 (**parked: authenticated provider**) → PD-7
(**closed locally after the park**). PD-7 does not imply that PD-6 passed.

## 4. PD-1 closure

`synthetic-fixture.json` is the labeled primary fixture until an owner can run
the same story against an authenticated Google Sheets copy. It contains eight
invented rows and four deliberate pair types:

The same rows are exposed in [`synthetic-sheet.csv`](synthetic-sheet.csv) for
spreadsheet review and [`synthetic-sheet.html`](synthetic-sheet.html) for a
no-auth browser preview. Both are disposable showcase fixtures, carry the
`Synthetic showcase fixture · no participant data` label, and must never be
treated as an apply target.

- exact duplicate: approve one retained row and one deletion;
- fuzzy given-name typo: approve only after the reviewer inspects the evidence;
- shared-contact household: keep all because shared context is not identity;
- conflicting valid DOB and address: leave unresolved with no deletion.

The fresh source probe generated all four candidate pairs with no truncation.
The exact and fuzzy pairs scored HIGH, the household pair was EXCLUDED with
`SAME_HOUSEHOLD_ONLY`, and the conflicting-DOB pair stayed LOW with both
`CONFLICTING_VALID_DOB` and `DIFFERENT_ADDRESS` warnings. The receipt records
the repository quality gate and the clean source checkout.

## 5. PD-1 claim boundary

This closes the synthetic content gate, not the product or live integration
gate. No Google account, Apps Script sidebar, Railway service, live workbook,
or copy-only apply was exercised. The fixture must retain its synthetic label
in any future public case.

## 6. PD-2 closure

`candidate-reasoning.json` turns the fresh native matcher probe into a static
queue packet. It preserves the exact score components and the source's bounded
semantics: name, valid DOB, address, ZIP, capped context, and penalties remain
separate; warnings stay visible; and reviewer actions are explicit.

The packet deliberately presents all four cases as different review states:

- exact and fuzzy pairs appear in the default High/Medium review queue;
- the conflicting-DOB pair is visible only through an explicit Low-confidence
  view and remains unresolved;
- the shared-household pair is excluded and labeled as non-identity context.

The explanation receipt verifies that every item has field contributions,
confidence limits, conflicts, and a bounded reviewer action. It is a material
and contract closure, not a claim that the live sidebar has been captured.

## 7. PD-2 claim boundary

No authenticated Google Sheet, Apps Script sidebar, live apply, or provider
identifier is included. The static queue cannot mutate a workbook and always
retains the synthetic label. Continue with PD-3 reviewer-controlled atomic
apply; that gap is the first one that needs execution against a disposable
copy rather than more presentation material.

## 8. PD-3/PD-4/PD-5 behavior closure

The three evidence gaps close against the repository's deterministic
`FakeSheetsGateway` and the checked-in `test/support/participantFixture.ts`.
The probes are intentionally in-memory and non-destructive; they are not a
substitute for a live Google Sheet or deployed backend.

- **PD-3:** a confirmed batch reports two clusters, three deleted rows, one
  filled field, and seven audit events. The pre-confirmation participant digest
  remains unchanged; unselected singleton rows remain present.
- **PD-4:** changing an affected deletion row's ZIP after scan produces
  `STALE_ROW` before apply. The changed row remains present and no deletion is
  performed by the guard.
- **PD-5:** independent original and copy gateways show the original digest
  unchanged while the copy moves from nine to six rows. Deleted-row snapshots
  and apply-batch identifiers remain available in local history.

The receipts keep the fixture identity, test command, exact counts, and
installed-surface boundary together so these claims do not get promoted to
live-provider evidence accidentally.

## 9. PD-3–PD-5 claim boundary

PD-3, PD-4, and PD-5 are closed as **local behavior fixtures**. No authenticated
Google account, live workbook, Apps Script sidebar, Railway deployment, or
external mutation was used. PD-6 remains the next integration gate and is
parked until the owner can provide a safe authenticated UAT path. The local
closures are still useful for the case study because they prove the safety
semantics without touching participant data.

## 10. PD-7 local no-auth case closure

[`case-study.html`](case-study.html) is a self-contained, no-auth local case
surface. It lets a reviewer inspect the four synthetic queue states, the two
reviewer-approved pairs, the keep-all and unresolved outcomes, the copy-only
behavior receipt, the stale guard, and the audit/recovery counts. Every page
section carries the synthetic label or names the local behavior fixture used
for the proof panel.

[`evidence/pd-7-public-case-receipt.json`](evidence/pd-7-public-case-receipt.json)
records the static marker review and keeps hosted publication, live-sheet UAT,
and deployed-provider identity as `not_proven`. The case can therefore be
shared as a local material package without implying a public URL that does not
exist yet.
