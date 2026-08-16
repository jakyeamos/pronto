# Repository remediation order

Generated from `pronto-remediation/v3` at `2026-08-04T00:36:45.521759+00:00`.

This is the active remediation queue, ranked from current Pronto evidence and each repository's intended remediation outcome. Inferred goals remain active until a repository-owned goal contract confirms them. Each plan also classifies every repo-level surface tracked by the UI; unresolved coverage entries link to concrete remediation actions. Repositories leave the active table when current evidence produces no actionable work or records an explicit deferral; that is a point-in-time queue transition, not a permanent repository state. Git, provider, publication, and pruning actions still require their own authorization.

Run-specific override: AIOS is on a deprecation path and is being mined into
exactly `ai-context-runtime`, `agent-eval-runtime`, and `ai-workflow-leverage`
before archive and eventual deletion. Its generated `active_maintained` goal is
stale and must not be used as the governing intent for this handoff.
The first reviewed mining slices merged into all three remote `dev` branches on
2026-08-04. AIOS remains first because consumer and automation replacement,
zero-live-caller evidence, retention and rollback proof, freeze, and archive
are still open; integration alone is not cutover or archive readiness.
The authorized automatic-host cutover was applied on 2026-08-04. Stable context,
leverage, and marketing runtimes now own the lifecycle, daily health, and storage
routes; seven direct AIOS Claude hooks, the indirect state-file cap caller, the
AIOS global Git hook path, and all four AIOS cron jobs were removed or replaced
with live smoke evidence and a dated rollback set. The 39-file marketing corpus
was copied with an identical content manifest and its AIOS source remains
untouched. AIOS remains first because 25 external optional TMCP configurations,
manual Claude and documentary callsites, consumer-reviewed retention exports,
and the longer zero-write window remain open. Archive and deletion have not
occurred.

For maturity-applicable goals, **3.0/4 is the minimum maturity threshold and 4.0/4 is the evidence-backed ideal**. Continue only material improvements after the threshold, and never add superficial documentation, configuration, tests, or other artifacts solely to raise the score.

Ranking preserves plan status, the earliest unresolved remediation domain, and action priority before fleet leverage. Pronto, AIOS, and Quality Runner receive explicit control-plane or evidence-provider precedence before the intended repository goal and raw action weight are used as tie-breakers.

## Active queue

Active repositories: **35**. Resolved action history entries: **2**.

<!-- prettier-ignore -->
| Rank | Repository | Goal | Goal source | Status | Current stage | Remaining path | Leverage | Tracked gaps | Active actions | First safe action |
| ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- |
| 1 | `AIOS` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Agent coordination control plane | 8 | 108 | Confirm the repository remediation goal |
| 2 | `quality-runner` | Public release | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Fleet evidence provider | 8 | 49 | Establish the release contract |
| 3 | `eslint-plugin-anti-slop` | Public release | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 37 | Establish the release contract |
| 4 | `tmcp` | Public release | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 37 | Establish the release contract |
| 5 | `participant-dedup` | Public release | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 26 | Establish the release contract |
| 6 | `soundscape-app` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 143 | Confirm the repository remediation goal |
| 7 | `ai-workflow-leverage` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 55 | Confirm the repository remediation goal |
| 8 | `BBDSE` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 54 | Confirm the repository remediation goal |
| 9 | `Terrace` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 42 | Confirm the repository remediation goal |
| 10 | `portfolio` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 36 | Confirm the repository remediation goal |
| 11 | `research-domain-writing` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 36 | Confirm the repository remediation goal |
| 12 | `career-ops` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 35 | Confirm the repository remediation goal |
| 13 | `Crimclock` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 35 | Confirm the repository remediation goal |
| 14 | `pre-cr-suite-lsp` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 34 | Confirm the repository remediation goal |
| 15 | `agent-eval-contract` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 33 | Confirm the repository remediation goal |
| 16 | `BidCamp` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 33 | Confirm the repository remediation goal |
| 17 | `remodelvision` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 33 | Confirm the repository remediation goal |
| 18 | `Bballedu` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 32 | Confirm the repository remediation goal |
| 19 | `mac-control` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 30 | Confirm the repository remediation goal |
| 20 | `ai-context-runtime` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 8 | 29 | Confirm the repository remediation goal |
| 21 | `BIP-Console` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 29 | Confirm the repository remediation goal |
| 22 | `agent-router` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 28 | Confirm the repository remediation goal |
| 23 | `dispatches-from-cyberspace` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 28 | Confirm the repository remediation goal |
| 24 | `jakyeamos-agent-skills` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 28 | Confirm the repository remediation goal |
| 25 | `context-compiler-contract` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 27 | Confirm the repository remediation goal |
| 26 | `Fantasy` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 27 | Confirm the repository remediation goal |
| 27 | `marketing-autoresearch` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 27 | Confirm the repository remediation goal |
| 28 | `Dsci-proj` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 26 | Confirm the repository remediation goal |
| 29 | `LaxDS` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 26 | Confirm the repository remediation goal |
| 30 | `tenure` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 7 | 26 | Confirm the repository remediation goal |
| 31 | `claude-config` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 6 | 25 | Confirm the repository remediation goal |
| 32 | `relay` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 6 | 24 | Confirm the repository remediation goal |
| 33 | `jakyeamos-profile` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 6 | 18 | Confirm the repository remediation goal |
| 34 | `dotfiles` | Active maintained repository | inferred | open | scope | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Repository | 6 | 17 | Confirm the repository remediation goal |
| 35 | `pronto` | Active maintained repository | repository_contract | open | branch_hygiene | Preserve and reconcile repository work → Reconcile product and provider truth → Reach quality and maturity threshold → Refresh and re-evaluate | Fleet control plane | 5 | 24 | Classify integration-ready branch · codex/pronto-final-evidence-closure-20260803 |

## Resolved action history

<!-- prettier-ignore -->
| Repository | Goal | Goal source | Disposition | Resolved at | Resolved actions | Evidence observed at | Summary |
| --- | --- | --- | --- | --- | ---: | --- | --- |
| `pronto` | active_maintained | repository_contract | deferred | `2026-08-03T21:51:22Z` | 3 | 2026-08-01T15:06:18-04:00 | 3 action(s) left the active queue with disposition 'deferred'. |
| `pronto` | active_maintained | repository_contract | deferred | `2026-08-03T21:23:07Z` | 35 | 2026-08-03T21:19:46.150713+00:00 | 35 action(s) left the active queue with disposition 'deferred'. |

A later refresh may return a repository to the active queue when new or regressed evidence creates actionable work.
