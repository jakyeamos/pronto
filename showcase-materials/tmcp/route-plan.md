# TMCP route plan

Status: **deferred by owner product direction**. The earlier steps 1–3 targeted
a bounded packet-compiler demo and are superseded because that story is too far
from TMCP's intended always-on atomic-node ideal state.

The [canonical target](../ideal-demo-targets.md#tmcp) owns the hold. No Showcase
fixture, visual, rehearsal, or packaging work should proceed while this gate is
open.

Project disposition: `conditional_gate` — resume only after the intended
always-on atomic-node behavior has an owner-approved product contract and a
behavior-verified representative flow.

Gap classes: product — TM-0.

| ID   | Gate to close                               | Observable acceptance condition                                                                                              | Owner         | Required proof                                       |
| ---- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------- | ---------------------------------------------------- |
| TM-0 | Prove the always-on atomic-node ideal state | An owner-approved contract defines the atomic-node behavior and one representative current-build flow verifies it end to end | Product owner | Product contract plus current-build behavior receipt |

**Resume condition:** after TM-0 passes, replace this hold with a new ideal
target, concept, and gap specification derived from the proven atomic-node
system. Do not restore the narrower release-readiness storyboard by default.
