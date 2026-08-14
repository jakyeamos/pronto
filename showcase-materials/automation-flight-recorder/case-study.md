# Automation Flight Recorder: see what caused what

Automation Flight Recorder records one declared local workflow as a bounded
parent/child trace. It preserves timing-era output, redaction, hashes,
omissions, and rerun guidance without treating a trace as permission to replay.

## The local W3 case

The current `dev` checkpoint reruns pass, failure, cancellation, and inspect
fixtures. The passing fixture records `prepare`, `tests`, and `package` beneath
one workflow identity. A second run fails at `tests`; `prepare` remains passed
and `package` is explicitly `not_run`. Secret-like output is redacted in both
traces, a timed-out child is recorded as `cancelled`, and omitted capture
classes remain named.

The story is: **declare → trace causality → bound evidence → preserve failure →
review rerun**.

## Evidence boundary

The traces are local CLI runs from source revision `51b29f5` against temporary
local fixtures, with a visually reviewed 1600×900 preview. They prove bounded
parent/child, failure, cancellation, redaction, omission, and inspect behavior,
not a Workflow Gateboard handoff, complete recovery coverage, rerun authority,
hosting, or publication.
