# Repository remediation order

Generated from `pronto-remediation/v3` at `2026-07-29T17:33:42.324842+00:00`.

This is the active remediation queue, ranked from current Pronto evidence and each repository's intended remediation outcome. Inferred goals remain active until a repository-owned goal contract confirms them. Each plan also classifies every repo-level surface tracked by the UI; unresolved coverage entries link to concrete remediation actions. Repositories leave the active table only after their plan reaches a terminal evidence-backed disposition. Git, provider, publication, and pruning actions still require their own authorization.

Ranking preserves plan status, the earliest unresolved remediation domain, and action priority before fleet leverage. Pronto, AIOS, and Quality Runner receive explicit control-plane or evidence-provider precedence before the intended repository goal and raw action weight are used as tie-breakers.

## Active queue

Active repositories: **42**. Retained closures: **0**.

<!-- prettier-ignore -->
| Rank | Repository | Goal | Goal source | Status | Current stage | Leverage | Tracked gaps | Active actions | First safe action |
| ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | --- |
| 1 | `pronto` | Active maintained repository | inferred | open | scope | Fleet control plane | 9 | 38 | Confirm the repository remediation goal |
| 2 | `AIOS` | Active maintained repository | inferred | open | scope | Agent coordination control plane | 9 | 137 | Confirm the repository remediation goal |
| 3 | `quality-runner` | Public release | inferred | open | scope | Fleet evidence provider | 10 | 62 | Establish the release contract |
| 4 | `eslint-plugin-anti-slop` | Public release | inferred | open | scope | Repository | 10 | 55 | Establish the release contract |
| 5 | `participant-dedup` | Public release | inferred | open | scope | Repository | 9 | 37 | Establish the release contract |
| 6 | `career-ops` | Active maintained repository | inferred | open | scope | Repository | 9 | 49 | Confirm the repository remediation goal |
| 7 | `BBDSE` | Active maintained repository | inferred | open | scope | Repository | 10 | 65 | Confirm the repository remediation goal |
| 8 | `ai-workflow-leverage` | Active maintained repository | inferred | open | scope | Repository | 9 | 61 | Confirm the repository remediation goal |
| 9 | `Terrace` | Active maintained repository | inferred | open | scope | Repository | 9 | 62 | Confirm the repository remediation goal |
| 10 | `portfolio` | Active maintained repository | inferred | open | scope | Repository | 9 | 50 | Confirm the repository remediation goal |
| 11 | `agent-eval-contract` | Active maintained repository | inferred | open | scope | Repository | 9 | 48 | Confirm the repository remediation goal |
| 12 | `Bballedu` | Active maintained repository | inferred | open | scope | Repository | 9 | 47 | Confirm the repository remediation goal |
| 13 | `Crimclock` | Active maintained repository | inferred | open | scope | Repository | 9 | 47 | Confirm the repository remediation goal |
| 14 | `remodelvision` | Active maintained repository | inferred | open | scope | Repository | 9 | 46 | Confirm the repository remediation goal |
| 15 | `BidCamp` | Active maintained repository | inferred | open | scope | Repository | 9 | 46 | Confirm the repository remediation goal |
| 16 | `research-domain-writing` | Active maintained repository | inferred | open | scope | Repository | 9 | 46 | Confirm the repository remediation goal |
| 17 | `BIP-Console` | Active maintained repository | inferred | open | scope | Repository | 9 | 43 | Confirm the repository remediation goal |
| 18 | `Book` | Active maintained repository | inferred | open | scope | Repository | 9 | 43 | Confirm the repository remediation goal |
| 19 | `ai-context-runtime` | Active maintained repository | inferred | open | scope | Repository | 9 | 42 | Confirm the repository remediation goal |
| 20 | `dispatches-from-cyberspace` | Active maintained repository | inferred | open | scope | Repository | 9 | 41 | Confirm the repository remediation goal |
| 21 | `Vaults` | Active maintained repository | inferred | open | scope | Repository | 9 | 40 | Confirm the repository remediation goal |
| 22 | `tm` | Active maintained repository | inferred | open | scope | Repository | 9 | 38 | Confirm the repository remediation goal |
| 23 | `Dsci-proj` | Active maintained repository | inferred | open | scope | Repository | 9 | 39 | Confirm the repository remediation goal |
| 24 | `jakyeamos-agent-skills` | Active maintained repository | inferred | open | scope | Repository | 9 | 40 | Confirm the repository remediation goal |
| 25 | `agent-router` | Active maintained repository | inferred | open | scope | Repository | 9 | 39 | Confirm the repository remediation goal |
| 26 | `Fantasy` | Active maintained repository | inferred | open | scope | Repository | 9 | 37 | Confirm the repository remediation goal |
| 27 | `mac-control` | Active maintained repository | inferred | open | scope | Repository | 9 | 39 | Confirm the repository remediation goal |
| 28 | `LaxDS` | Active maintained repository | inferred | open | scope | Repository | 9 | 38 | Confirm the repository remediation goal |
| 29 | `Hoopscout` | Active maintained repository | inferred | open | scope | Repository | 9 | 38 | Confirm the repository remediation goal |
| 30 | `context-compiler-contract` | Active maintained repository | inferred | open | scope | Repository | 9 | 37 | Confirm the repository remediation goal |
| 31 | `repo-quality-certifier` | Active maintained repository | inferred | open | scope | Repository | 9 | 36 | Confirm the repository remediation goal |
| 32 | `marketing-autoresearch` | Active maintained repository | inferred | open | scope | Repository | 8 | 36 | Confirm the repository remediation goal |
| 33 | `claude-config` | Active maintained repository | inferred | open | scope | Repository | 9 | 35 | Confirm the repository remediation goal |
| 34 | `jakyeamos-profile` | Active maintained repository | inferred | open | scope | Repository | 8 | 31 | Confirm the repository remediation goal |
| 35 | `frmwrklabs` | Active maintained repository | inferred | open | scope | Repository | 9 | 34 | Confirm the repository remediation goal |
| 36 | `relay` | Active maintained repository | inferred | open | scope | Repository | 8 | 34 | Confirm the repository remediation goal |
| 37 | `dotfiles` | Active maintained repository | inferred | open | scope | Repository | 8 | 28 | Confirm the repository remediation goal |
| 38 | `tmcp` | Public release | inferred | open | scope | Repository | 10 | 50 | Establish the release contract |
| 39 | `pre-cr-suite-lsp` | Active maintained repository | inferred | open | scope | Repository | 9 | 47 | Confirm the repository remediation goal |
| 40 | `agent-eval-runtime` | Active maintained repository | inferred | open | scope | Repository | 9 | 38 | Confirm the repository remediation goal |
| 41 | `greenlight` | Active maintained repository | inferred | open | scope | Repository | 9 | 36 | Confirm the repository remediation goal |
| 42 | `quality-evidence-contract` | Active maintained repository | inferred | open | scope | Repository | 9 | 35 | Confirm the repository remediation goal |

## Closure ledger

No repositories have left the active queue in this retained run history.

A later refresh may return a repository to the active queue when new or regressed evidence creates actionable work.
