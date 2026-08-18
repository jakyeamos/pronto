# CI tracker and Codex handoff

Pronto's CI tracker is a read-only projection of GitHub Actions. GitHub remains
the source of truth for workflow runs, jobs, logs, and artifacts; Pronto stores
only bounded run metadata, failed job/step names, a stable failure signature,
and whether the Codex prompt artifact is available.

## Flow

1. A repository's reusable `workflow_run` bridge creates
   `codex-ci-prompt.json` and `codex-ci-prompt.md` and uploads them as an
   artifact after a failed run.
2. `Refresh CI` imports recent GitHub workflow runs, failed jobs, failed step
   names, fork provenance, and the exact matching artifact name.
3. `Diagnose with Codex` downloads the artifact into a bounded temporary
   directory and starts a normal Codex task with `--sandbox read-only` at the
   registered local checkout.

The button does not check out a pull-request head, execute repository code,
edit files, commit, push, comment on GitHub, or merge a pull request. Fork
failures remain diagnosis-only. A remote-only repository stays visible but
cannot start a local Codex handoff until a registered local checkout exists.

## Bridge resolution

The default bridge path is:

```
~/projects/ci-incident-router
```

Set `PRONTO_CI_BRIDGE` when the checkout lives elsewhere. `node` and `codex`
are resolved from `PATH`; `PRONTO_NODE_BIN` and `PRONTO_CODEX_BIN` provide
explicit overrides for a desktop environment with a restricted PATH.

The bridge must expose `bin/codex-ci.mjs`. Its GitHub authentication remains
provider-owned: the bridge uses the existing `gh auth token` fallback when no
`GH_TOKEN` or `GITHUB_TOKEN` is present.

## Evidence states

- `Ready` means the GitHub provider refresh returned a snapshot, not that every
  run has logs or a prompt artifact.
- A missing artifact means the workflow bridge was not installed for that run,
  the artifact expired, or GitHub did not expose it. Pronto leaves the button
  disabled rather than synthesizing a prompt.
- A stable failure signature is derived from workflow name, head SHA, and the
  bounded failure summary. It distinguishes reruns by run ID and attempt in
  the handoff key without requiring a second database.
- Successful runs are retained in the provider snapshot but do not appear in
  the actionable tracker list.

This is a local source/build feature. Installing or updating
`/Applications/Pronto.app` is a separate, explicitly authorized deployment
step.
