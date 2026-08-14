# AI Code Quality Stack route plan

Status: the local candidate package is complete, but this is still a combined
portfolio story, not the Pre-CR demo and not the Quality Runner demo. It owns
only the integration seam among Anti-Slop, Pre-CR's changed-file enforcement,
and Quality Runner's evidence consumption.

The [canonical target](../ideal-demo-targets.md#ai-code-quality-stack) owns the
durable promise and proof gate. Anti-Slop's reviewed
[Pronto case](../eslint-anti-slop/case-study.md) is the real detector input.

## 1. Ideal target

**North star:** show one generated-code problem move through three bounded
responsibilities: Anti-Slop names it at the line, Pre-CR blocks the changed-file
set before review, and Quality Runner places the same evidence into a broader
remediation plan without duplicating the finding.

**Non-negotiable:** each product keeps its standalone identity. This story owns
neither Pre-CR's IDE suite nor Quality Runner's broader audit workflow.

## 2. Concept materials

The target frames are implemented in `concept/index.html`. They remain
**concept** until the integrated case passes.

| Frame            | Product role                                                        | Intended evidence moment                    |
| ---------------- | ------------------------------------------------------------------- | ------------------------------------------- |
| 1. Problem       | One reviewed real generated-code issue                              | The gap is concrete and legible             |
| 2. Detect        | Anti-Slop emits the line-level semantic diagnostic                  | The detector has one clear owner            |
| 3. Enforce       | Pre-CR applies policy to the changed-file set                       | Enforcement happens before review           |
| 4. Contextualize | Quality Runner imports the canonical finding                        | Orchestration adds context, not duplication |
| 5. Repair        | All three layers close the same evidence                            | One repair produces coherent proof          |
| 6. Degrade       | Unavailable, failed-analysis, and dual-source paths remain explicit | Fallback never becomes false confidence     |

## 3. Build-gap specification

Project disposition: `targeted_gap_closure` — retain all three products and
prove their existing integration seam without inventing a fourth product.
Quality Runner's overlapping text heuristics remain an advisory fallback only
when Anti-Slop is unavailable, with deduplication when both sources run.

Gap classes: product — QS-0; demo integration — QS-1, QS-2, QS-3; evidence —
QS-4; packaging — QS-5.

| ID   | Gap to close                                         | Observable acceptance condition                                                                                                  | Owner                    | Required proof                                                         |
| ---- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------- |
| QS-0 | Lock ownership, failure, fallback, and deduplication | Anti-Slop owns AST semantics; unavailable and failed analysis remain distinct; dual-source output produces one canonical finding | Stack owners             | Cross-repo contract and positive/unavailable/failure/dual-source tests |
| QS-1 | Prove Anti-Slop detection                            | The selected case produces one deterministic exact-line diagnostic                                                               | Anti-Slop owner          | Structured ESLint output and focused rule proof                        |
| QS-2 | Prove Pre-CR enforcement                             | Pre-CR runs the adapter against the same changed-file set and writes an attributable blocking receipt                            | Pre-CR enforcement owner | Failing receipt and adapter provenance                                 |
| QS-3 | Prove QR consumption                                 | QR imports the canonical evidence, adds repository context, and emits no duplicate heuristic                                     | Quality Runner owner     | Normalized evidence and deduplication receipt                          |
| QS-4 | Repair and close                                     | The repair clears all three layers and records resolved evidence once                                                            | Stack owners             | Before/after diff and passing receipts                                 |
| QS-5 | Package the combined story                           | **Closed locally 2026-08-14:** the candidate case, claim ledger, crop-safe 16:9 preview, short copy, and no-auth page source distinguish all three responsibilities and point to standalone demos | Showcase/design owner    | `case-study.json`, `case-study.md`, `claim-ledger.json`, `preview.html`, `assets/preview-16x9.png`, and `public/index.html` |

**Build order:** QS-0 → QS-1/QS-2 → QS-3 → QS-4 → QS-5.

**Next closure:** QS-0. The local package is ready for review, but integrated
execution remains blocked until ownership and workspace boundaries are
confirmed and the three layers produce one attributable evidence path.
