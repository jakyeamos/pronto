# Pronto agent context

Pronto is the local-first portfolio and Git evidence surface for this machine.
Use this index when a task needs portfolio-wide repository state, workspace
triage, branch integration evidence, quality evidence, or release preparation.

Read the deeper file only when the task matches it:

- `commands.md` — exact CLI invocation, focused JSON projections, freshness
  semantics, and the read/write boundary for agent use.
- `../../docs/implementation-plan.md` — architecture, product boundaries, and
  the source-of-truth relationship between the desktop app and CLI.

The global `$pronto` skill owns the cross-repository workflow. This context
packet owns the current repository contract and should be updated when the CLI
surface or its safety boundary changes.
