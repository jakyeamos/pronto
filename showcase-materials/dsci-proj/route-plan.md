# Dsci-proj route plan

Status: DS-0 is closed as a versioned product/decision contract; the current
repository still implements only the narrower research-issue profile. The
original research backlog is the first use case, and DS-1 is now the next
implementation gap.

The [canonical target](../ideal-demo-targets.md#dsci-proj) owns the durable
promise and proof gate.

## 1. Ideal target

**North star:** a user imports a crowded backlog, chooses or edits the criteria
that matter, receives an explainable priority queue, tests a changed assumption,
and exports the chosen next-work list with decision provenance.

**Non-negotiable:** prioritization remains advisory and configurable. The user
owns the criteria, weights, constraints, and final decision; the system must
show how each recommendation follows from the supplied evidence.

## 2. Concept materials

All frames are **concept** until the generalized prioritization flow is proven
against two meaningfully different backlogs.

| Frame                   | Visual                                                        | On-screen line                                 | Intended evidence moment                  |
| ----------------------- | ------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------- |
| 1. Unordered backlog    | Competing items arrive from a research or product backlog     | “Everything cannot be first”                   | The decision problem is concrete          |
| 2. Criteria contract    | Editable criteria, weights, constraints, and evidence fields  | “Priority starts with what matters”            | Judgment stays user-owned                 |
| 3. Ranked queue         | Items reorder with score bands and uncertainty visible        | “Turn a backlog into a defensible next move”   | The output is immediately actionable      |
| 4. Explanation          | One item opens into factor-level score and source evidence    | “Every recommendation shows its work”          | Ranking is traceable                      |
| 5. Scenario comparison  | A changed weight produces a deterministic before/after view   | “Test the assumption before committing”        | Tradeoffs are explorable                  |
| 6. Generalization proof | A second backlog maps into the same engine without code edits | “One decision system, different kinds of work” | The product exceeds its original use case |

**Preview concept.** A ranked backlog sits beside an explanation panel and a
before/after scenario comparison. Headline: “Choose what to do next—and show
why.”

**Narrative spine.** Backlog → criteria → ranking → explanation → scenario →
chosen next-work queue.

## 3. Build-gap specification

Reviewed baseline: a research pipeline and dashboard exist for one backlog;
current runnable proof, configurable prioritization, and cross-backlog
generalization are limited.

Project disposition: `material_build_or_restoration` — preserve the original
analysis as a use case while building a configurable, explainable backlog-triage
system around its reusable decision model.

Gap classes: product — DS-0, DS-1, DS-2, DS-3; evidence — DS-4; packaging —
DS-5.

| ID   | Gap to close                                | Observable acceptance condition                                                                                    | Owner                  | Required proof                                 |
| ---- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------- | ---------------------------------------------- |
| DS-0 | Define the reusable prioritization contract | Backlog fields, criteria, weights, constraints, scoring, explanations, uncertainty, and output are configurable    | Product owner          | Versioned product and decision contract        |
| DS-1 | Normalize multiple backlog inputs           | Two meaningfully different backlog schemas map into one internal item model without source-specific engine changes | Implementation owner   | Input adapters, fixtures, and mapping receipts |
| DS-2 | Build configurable ranking and scenarios    | A user can edit criteria or weights and compare deterministic before/after rankings                                | Implementation owner   | Scenario tests and stable ranking receipts     |
| DS-3 | Explain every recommendation                | Each ranked item exposes factor contributions, source evidence, missing inputs, and uncertainty                    | Product/design owner   | Explanation view and claim ledger              |
| DS-4 | Prove cross-backlog generalization          | The same engine produces useful, reviewable queues for the original research backlog and a second backlog type     | Verification owner     | Cross-backlog evaluation and reviewer receipt  |
| DS-5 | Package the decision story                  | A non-specialist can import, rank, inspect, adjust, compare, and export without notebook narration                 | Design/editorial owner | Walkthrough prototype and comprehension review |

**Build order:** DS-0 → DS-1 → DS-2/DS-3 → DS-4 → DS-5. Do not claim a
generalized product until DS-4 proves the same engine against both backlog
types.

## 4. DS-0 closure

DS-0 is closed as a product contract, not as a code-complete generalized
engine. [`decision-contract.json`](decision-contract.json) defines the reusable
item model, configurable criteria and weights, constraints, deterministic
scoring, per-item explanations, uncertainty, scenario comparison, and the
human-owned export. [`synthetic-fixture.json`](synthetic-fixture.json) provides
the short reproducibility appendix with the original research-issue shape, a
meaningfully different product-backlog shape, and three fail-closed mutations.

The contract is grounded in the current native surface: fixed impact
components, predicted-time/reopen/velocity difficulty, ROI, repo-local
quadrants, and deterministic why bullets. The current code does not yet accept
user-edited weights, a second adapter, scenario comparisons, or a decision
contract export. Those are implementation gaps DS-1 onward, not claims hidden
behind this material.

The evidence boundary is recorded in
[`evidence/ds-0-contract-receipt.json`](evidence/ds-0-contract-receipt.json).

## 5. Rights-safe contract preview

[`synthetic-preview.html`](synthetic-preview.html) is now a self-contained,
no-auth visual target for the DS-0 contract. It shows the two synthetic
backlog shapes, their canonical item model, user-owned criteria, an
illustrative queue, and the explanation/handoff boundary. The page is labeled
**Synthetic contract appendix · not current product output** and uses no
credentials, external assets, or network calls.

The static material receipt is
[`evidence/ds-0-synthetic-material-receipt.json`](evidence/ds-0-synthetic-material-receipt.json).
It records the HTTP surface probe and preserves the claim boundary: the
preview makes the target shareable, but it does not close DS-1 or claim a
current ranking run, scenario execution, cross-backlog generalization, or
hosted dashboard behavior. The next durable step remains DS-1.
