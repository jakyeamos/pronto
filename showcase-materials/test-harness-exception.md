# Showcase test-harness exception

`scripts/showcase-materials.test.mjs` is a repository-wide integration corpus,
not production runtime code. It intentionally keeps the cross-project
contract checks together so that the Showcase goal, readiness ledger, release
targets, candidate packets, and postability ledger are evaluated at one
snapshot boundary.

Pre-CR therefore reports its `oversized-source` warning for this file (3,677
nonblank lines). The warning is accepted as a scoped exception for the current
materials pass; splitting the corpus by responsibility is a follow-up
maintenance task and is not required to claim that the candidate packets are
locally verified. The production-source size rule remains enabled for the rest
of the repository.
