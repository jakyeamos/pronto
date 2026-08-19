# Pronto agent context

Last reviewed: 2026-08-18.

Pronto is the local-first portfolio and Git evidence surface for this machine.
Use this index when a task needs portfolio-wide repository state, workspace
triage, branch integration evidence, quality evidence, or release preparation.

The canonical branch is `main`. Feature and remediation work must use an
isolated branch and reach `main` through the repository's reviewed integration
lane. Preserve unpublished or dirty work before branch folding, and do not
prune until integration or patch equivalence is proven.

Read the deeper file only when the task matches it:

- [Agent command contract](commands.md) — exact CLI invocation, focused JSON
  projections, freshness semantics, and the read/write boundary for agent use.
- [Quality prevention contract](quality-prevention.md) — task-scoped QR
  baseline/completion checkpoints and the boundary between candidate and
  certified native checks.
- [Repository operating contract](../../docs/repository-contract.md) —
  architecture ownership, coding and security constraints, failure recovery,
  approval gates, definition of done, and installation/release rollback.
- [Repository Telescope](../../docs/telescope.md) — source-derived living-city
  projection, categorical readiness and Map Workshop contract, building-local
  source detail, freshness/privacy rules, and independently composable lenses.
- [Implementation examples](../../docs/implementation-examples.md) — existing
  end-to-end patterns for projection, quality, remediation, and change-surface
  work.
- [Agent usability maturity](../../docs/agent-usability-maturity.md) — the
  tool-to-documentation-to-skill relationship, four evidence lanes, and
  documentation/skill-family growth-health contract.
- [Sequential remediation handoff](../../docs/remediation-sequential-handoff.md)
  — the Sol orchestrator, Luna Max executor, one-at-a-time dispatch, result,
  verification, and resume contract.
- [Implementation plan](../../docs/implementation-plan.md) — product
  boundaries and the source-of-truth relationship between the desktop app and
  CLI.
- [CI tracker](../../docs/ci-tracker.md) — GitHub Actions failure projection,
  prompt-artifact availability, and the user-controlled read-only Codex
  handoff.
- [Branch-sensitive quality verification](branch-sensitive-quality-verification.md)
  — required when a branch or commit selector is adjacent to displayed quality
  or tenure statistics.

The global `$pronto` skill owns the cross-repository workflow. This context
packet owns the current repository contract and should be updated when the CLI
surface or its safety boundary changes.
