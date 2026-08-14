# Context Compiler Contract route plan

Status: CC-1 through CC-5 closed in a local, evidence-bound package. The
baseline is a real AIOS compile result; the two invalid states are labelled
synthetic mutations. Browser capture and video remain optional enhancements.

The [canonical target](../ideal-demo-targets.md#context-compiler-contract) owns
the durable promise and proof gate.

## 1. Ideal target

**North star:** take a real AIOS context result, remove one source-selection
reason, break its route-compatibility flag, show the exact validator failures,
then restore both fields and finish with a compact valid artifact whose limits
are obvious.

**Non-negotiable:** validator behavior is the product here. Execution behavior
owned by a caller or provider must not be attributed to this contract package.

## 2. Concept materials

The source baseline and corrected result pass the current public contract; the
two invalid states are deterministic mutations used to make the boundary easy
to see.

| Frame                 | Visual                                                                | On-screen line                              | Intended evidence moment        |
| --------------------- | --------------------------------------------------------------------- | ------------------------------------------- | ------------------------------- |
| 1. Plausible bundle   | Three context sources look normal at first glance                     | “Context can be useful and still invalid”   | The issue is not cosmetic       |
| 2. Exact failure      | A source reason and route-compatibility field turn red with paths     | “Fail at the contract boundary”             | Errors are precise              |
| 3. Consequence        | A route-incompatible packet stops before downstream work              | “A bad handoff should stop early”           | The contract’s value is legible |
| 4. Correction         | The source reason and route flag are restored                         | “Fix the contract, not the copy”            | Remediation is understandable   |
| 5. Valid artifact     | Compact bundle shows provenance, selection reason, bounds, and digest | “Portable because the contract is explicit” | The result is inspectable       |
| 6. Ownership boundary | Runtime execution sits outside the package outline                    | “Validation here. Execution elsewhere.”     | Product scope stays honest      |

**Preview concept.** An invalid context bundle splitting into two exact error
paths, then recombining as a compact green contract. Headline: “Make agent
context fail before it becomes agent behavior.”

**Narrative spine.** Plausible invalid input → exact failures → downstream risk
→ source correction → valid bounded artifact → runtime boundary.

## 3. Build-gap specification

Reviewed baseline: public ESM validator contracts are available. A real AIOS
compile result now supplies the baseline, with a static comparison surface and
deterministic expected results for two synthetic mutations.

Project disposition: `largely_product_ready` — the validator is the product;
the remaining work is a representative fixture, bounded explanatory polish,
proof, and a legible comparison surface.

Gap classes: content — CC-1; product — CC-2; packaging — CC-3, CC-6; evidence —
CC-4, CC-5.

| ID   | Gap to close                                  | Observable acceptance condition                                                                                                               | Owner                 | Required proof                                        |
| ---- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ----------------------------------------------------- |
| CC-1 | Author minimal positive and negative fixtures | The real baseline passes; one mutation fails for a missing source reason and one fails for route incompatibility; the corrected result passes | Contract owner        | Versioned fixture and expected results                |
| CC-2 | Make current errors easy to act on            | Each failure pairs the validator’s field path and rule with a local corrective action, without leaking payload content                        | Showcase owner        | Expected-result mapping and claim ledger              |
| CC-3 | Expose the invalid-to-valid diff              | A viewer can see which semantic fields changed without reading the entire bundle                                                              | Showcase/design owner | HTML comparison and SVG preview                       |
| CC-4 | Prove deterministic validation                | Repeated clean invocations return equivalent outcomes and digests for all four cases                                                          | Quality owner         | Validation receipt and artifact hashes                |
| CC-5 | Audit attribution boundaries                  | All copy limits claims to validation and identifies AIOS as the runtime owner                                                                 | Documentation owner   | Claim ledger and reviewer sign-off                    |
| CC-6 | Optional browser-rendered thumbnail           | A permitted browser surface can render the comparison without changing its claim boundary                                                     | Packaging owner       | Browser capture when a permitted surface is available |

**Build order:** CC-1 → CC-2/CC-3 → CC-4 → CC-5 → CC-6 (optional).
