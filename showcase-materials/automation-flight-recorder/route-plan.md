# Automation Flight Recorder route plan

Status: C0 public Showcase admission is complete; the current dev checkpoint
and binary preview are verified, while end-to-end parent/child causality,
recovery breadth, and safe-rerun proof remain gated.

The [canonical target](../ideal-demo-targets.md#automation-flight-recorder)
owns the promise and proof gate. The recorder remains local-first and bounded.

## 1. Ideal target

**North star:** a developer inspects one bounded automation run as a causal,
redacted receipt and safely reruns only the declared step.

**Non-negotiable:** parent/child causality, timing, omissions, redaction, and
rerun eligibility remain explicit; a trace is not permission to replay.

## 2. Concept materials

All frames below are **concept frames** until the Gateboard trace and failure
matrix pass.

| Frame         | Visual                                                            | On-screen line             | Intended evidence moment           |
| ------------- | ----------------------------------------------------------------- | -------------------------- | ---------------------------------- |
| 1. Parent run | One declared gate fans into named child steps                     | “See what caused what”     | Causality is concrete              |
| 2. Evidence   | Timings, bounded outputs, hashes, omissions, and redaction appear | “Keep the useful boundary” | Privacy and scope are visible      |
| 3. Failure    | One child fails without erasing completed siblings                | “A failure stays local”    | Partial results remain trustworthy |
| 4. Rerun      | Only the declared safe step is previewed for rerun                | “Retry by authority”       | Replay is not implicit             |

**Preview concept.** Use a parent/child timeline with one failed child and a
separate rerun-eligibility card. Keep raw output compact.

**Narrative spine.** Declared gate → child causality → bounded evidence → local
failure → safe rerun.

## 3. Build-gap specification

Reviewed baseline: the W3 MVP records local parent/child traces with timing,
redaction, hashes, omissions, and rerun eligibility. The current dev checkpoint
also verifies pass, failure, cancellation, and inspect flows. Gateboard
integration and broader recovery coverage remain open.

Project disposition: `targeted_gap_closure` — trace one Gateboard action from
parent through child and prove safe rerun.

Gap classes: demo_integration — AFR-0; evidence — AFR-1; product — AFR-2;
packaging — AFR-3.

| ID    | Gap to close                            | Observable acceptance condition                                                          | Owner             | Required proof               |
| ----- | --------------------------------------- | ---------------------------------------------------------------------------------------- | ----------------- | ---------------------------- |
| AFR-0 | Prove Gateboard causality               | One declared action records named children, timing, bounded outputs, and parent identity | Integration owner | Gateboard/recorder receipt   |
| AFR-1 | Preserve failure and omission semantics | Failed, cancelled, omitted, and completed steps remain distinct                          | Evidence owner    | Scenario matrix              |
| AFR-2 | Bound rerun authority                   | Only the declared eligible child can be previewed for rerun                              | Safety owner      | Negative rerun probe         |
| AFR-3 | Package the public case                 | Binary preview, short copy, no-auth trace case, and linked receipt agree                 | Showcase owner    | Material review and readback |

**Required build order:** AFR-0 → AFR-1 → AFR-2 → AFR-3. Video is optional
after the evidence gate.
