# Marketing Autoresearch route plan

Status: steps 1–3 complete as an aspirational specification. MA-1 is closed
locally with a real public portfolio decision brief and privacy review. MA-2
is parked because the current runtime cannot bind that brief to a
source-to-claim receipt; the decision-useful report and publication-block
proof do not yet exist.

The [canonical target](../ideal-demo-targets.md#marketing-autoresearch) owns the
durable promise and proof gate.

## 1. Ideal target

**North star:** a marketer asks a consequential campaign question, watches a
shadow agent gather and grade public evidence, receives a concise opportunity
report, and sees an unreviewed draft stop at a cryptographically or
permission-backed publication gate.

**Non-negotiable:** “the agent did not publish” must be enforced and observable,
not merely promised in narration.

## 2. Concept materials

All frames are **concept** until the shadow run and refusal are independently
verified.

| Frame                  | Visual                                                                   | On-screen line                                  | Intended evidence moment       |
| ---------------------- | ------------------------------------------------------------------------ | ----------------------------------------------- | ------------------------------ |
| 1. Campaign question   | Sanitized brief asks where a specific audience is underserved            | “Start with a decision, not content volume”     | Research has a purpose         |
| 2. Evidence sweep      | Sources enter with freshness, relevance, and claim-use labels            | “Collect evidence you can inspect”              | Source quality is visible      |
| 3. Opportunity map     | Themes, contradictions, and unknowns form a ranked map                   | “Show the white space—and the uncertainty”      | Synthesis is decision-oriented |
| 4. Report              | Recommendation, supporting claims, and limitations resolve into one page | “A report before a campaign”                    | Output is concrete             |
| 5. Publication attempt | Draft reaches a locked publish stage requiring human review              | “Shadow means no external mutation”             | Boundary is exercised          |
| 6. Human handoff       | Reviewer sees sources, claims, draft, and explicit approve-later path    | “Research can automate. Accountability cannot.” | Human ownership is clear       |

**Preview concept.** An evidence map flowing into a polished report, stopping at
a locked “Publish” boundary. Headline: “Automate the research loop. Keep the
publication decision human.”

**Narrative spine.** Decision question → graded sources → opportunity synthesis
→ report → blocked publication → inspectable handoff.

## 3. Build-gap specification

Reviewed baseline: a report-only fail-closed loop exists; no public demo packet,
sanitized run, report artifact, or publication-block proof was found.

Project disposition: `targeted_gap_closure` — retain the fail-closed loop while
adding a safe campaign case, inspectable run, human handoff, and public packet.

Gap classes: content — MA-1, MA-3; demo_integration — MA-2, MA-5; evidence —
MA-4; packaging — MA-6.

| ID   | Gap to close                            | Observable acceptance condition                                                                                     | Owner                    | Required proof                                                    |
| ---- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------- |
| MA-1 | Create a sanitized consequential brief  | The prompt is realistic, contains no client data, and has explicit decision and evidence criteria                   | Marketing/product owner  | Brief and privacy review                                          |
| MA-2 | Produce a traceable shadow research run | Every material report claim links to an allowed source and uncertain findings remain labeled                        | Research workflow owner  | Source packet, claim ledger, and run receipt                      |
| MA-3 | Make the report decision-useful         | A reviewer can identify recommendation, evidence, contradiction, unknown, and next experiment in one page           | Editorial/product owner  | Reviewed report artifact                                          |
| MA-4 | Enforce the publication boundary        | An attempted publish action fails before external mutation because authority is absent or the review gate is closed | Safety/integration owner | Permission evidence, refusal receipt, and external-state readback |
| MA-5 | Design the human handoff                | Reviewer controls factual edits, brand judgment, approval, and any later external action                            | Product/design owner     | Handoff prototype and role review                                 |
| MA-6 | Create a no-auth synthetic case study   | Public viewers can inspect the brief, evidence, report, and refusal without credentials                             | Showcase owner           | Link and redaction checks                                         |

**Build order:** MA-1 → MA-2/MA-3 → MA-4 → MA-5/MA-6.

## 4. MA-1 closure

**Closed 2026-08-13.** The case is a real, public-safe portfolio maintenance
question: which AI Showcase story should be foregrounded next for
evidence-seeking technical hiring teams, and what proof gap limits trust? The
brief compares the Quality Runner, AI Workflow Leverage, and AI Code Quality
Stack lanes without claiming that any lane wins.

- [Sanitized consequential brief](brief.json) binds the audience, decision,
  approved source registry, evidence rules, exclusions, stop conditions, and
  report contract.
- [Privacy review](privacy-review.json) passes the brief for a report-only
  shadow run and keeps future provider payloads, private paths, and external
  actions conditional.
- [Synthetic appendix](synthetic-fixture.json) exercises private-source,
  unlinked-claim, and publish-request refusals without becoming market
  evidence.

MA-1 does not establish audience demand, a campaign recommendation, or a
publication candidate. The next eligible gap is MA-2: produce a traceable
shadow research run with source-to-claim receipts.

## 5. MA-2 blocker

The isolated no-live probe run is a valid report-only execution, but it records
missing market observations and no external mutations. The current runtime
accepts a profile-set ID, not the MA-1 brief; its `MarketObservation` record
contains a topic, status, source, summary, and raw-artifact path, but no
source-to-claim ID, retrieval date, allowed-source check, or uncertainty
classification. Enabling the optional research adapter would not close that
contract by itself.

The exact owner boundary and probe evidence are recorded in
[`evidence/ma-2-blocker.json`](evidence/ma-2-blocker.json). Do not add a second
claim-ledger engine in Pronto or call the missing-observation run a research
result. Resume MA-2 only after the marketing-autoresearch owner approves a
brief-aware run and receipt contract (or an explicitly owned adapter with the
same redaction and source rules).

## 6. Local showcase package

The local material layer is now assembled around the real MA-1 public portfolio
brief. `case-study.json` and `case-study.md` describe the decision question,
allowed source registry, report-only probe, and human handoff. `claim-ledger.json`
keeps the brief, privacy boundary, missing source-to-claim contract, report,
publication boundary, and external release at separate evidence levels.

`public/index.html` is a candidate no-auth source page and `preview.html` is a
fixed 1600x900 capture surface. The reviewed binary preview is the visual
thumbnail; `synthetic-fixture.json` remains only a short reproducibility
appendix. `evidence/ma-7-material-checkpoint.json` records the package and its
open gates. This is a local candidate, not a hosted or published case: MA-2,
the report artifact, runtime refusal receipt, hosting, and destination readbacks
remain open.
