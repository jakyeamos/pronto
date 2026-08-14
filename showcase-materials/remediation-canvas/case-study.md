# Remediation Canvas: keep partial work actionable

Remediation Canvas groups stable Quality Lens finding references with human
intent and dispositions. It does not copy the authoritative finding payload or
pretend that changed files prove resolution.

## The local W3 case

Three finding references enter one canvas. The human intent says a focused
parser fixture is needed; one finding is resolved, one blocked, and one remains
unresolved. When the source set replaces one reference, the fingerprint changes
and refresh reports `stale` while the original partial work remains visible.

The story is: **gather → state intent → disposition → refresh → preserve**.

## Evidence boundary

The current `dev` receipt is a local CLI run against a temporary fixture, with a
visually reviewed 1600×900 preview. It proves reference/intent/stale behavior
for one fixture, not automatic remediation, Quality Lens or review integration,
hosting, or publication.
