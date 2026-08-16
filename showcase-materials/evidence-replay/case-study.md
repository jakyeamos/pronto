# Evidence Replay: inspect before you act

Evidence Replay is the reader side of a durable development claim. It opens a
receipt, shows what the producer actually recorded, compares the receipt target
with the current checkout, and keeps uncertainty visible before a human decides
whether to rerun anything.

## The local W2 case

The Debug Trail fixture says its bounded check passed on a fixture revision. On
the current Evidence Replay checkout, the revision differs, so the historical
success remains visible but the current outcome is `not-run`. The rerun preview
names the command, read-only mutation class, interface, and authorization
requirement, then blocks because freshness is stale.

The product story is: **open → classify freshness → preview rerun → preserve
omissions → hand off**. Opening a receipt never executes its command.

## Evidence boundary

The historical W2 matrix is a local CLI run from source revision `72c29a0`.
The current-dev checkpoint at `780094c` reruns the reader, tests, lint, and
package checks on the live `dev` head. Together they prove the inspect-only
boundary and fail-closed negative contracts, not a hosted viewer, complete
producer/state coverage, direct UI acceptance, or publication.
