# AI Workflow Leverage case study

## Measure the work before claiming the gain

The honest story is not “AI made this task faster.” The packet first fixes one
real Tenure maintenance task, one protected starting revision, one allowlisted
surface, and one shared quality oracle. Only then can a manual lane and a
bounded assisted lane be compared.

The task is **Expose Tenure review and capture task-state contracts**. Both
lanes must preserve review, approval, export, rollback, and capture semantics
while making stable control IDs and explicit state values observable. The
historical reference implementation is a contract anchor, not a third lane or
a measured outcome.

The local candidate page makes the intended flow visible:

1. fix the same task and baseline;
2. keep manual and assisted work in separate lanes;
3. send both results through the same behavioral and provenance oracle; and
4. stop at AL-2 when the current runtime cannot record the event fields needed
   for a defensible comparison.

The short synthetic appendix is only for offline reproducibility. It supplies a
valid state map plus missing-state and invalid-state fail-closed mutations; it
does not replace the real Tenure task or provide timing data.

## Proof boundary

AL-1 is locally closed as protocol selection. AL-2 remains parked because
aggregate run fields do not distinguish active work, waits, typed human touches,
retries, failures, or outcome evidence. Those paired-measurement fields belong
to `agent-eval-runtime`; adding a second measurement engine here would create a
false comparison path.
