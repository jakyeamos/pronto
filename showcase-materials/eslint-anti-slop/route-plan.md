# ESLint Anti-Slop supporting-component route

Status: retained as a durable public package and removed as a standalone
Showcase target. Its role is now the detector layer in the
[AI Code Quality Stack route](../ai-code-quality-stack/route-plan.md).

The reviewed [committed Pronto UI case](case-study.md) and its
[exact-revision packet](pronto-case.json) remain useful integration evidence:
three candidates were reported, two were accepted, and one empty-state false
positive was rejected. They are inputs to the combined story, not an
independent material-production queue.

## Durable responsibility

| Layer            | Durable responsibility                                                                                               |
| ---------------- | -------------------------------------------------------------------------------------------------------------------- |
| ESLint Anti-Slop | Own precise, offline AST rule semantics and line-level ESLint diagnostics for JavaScript and TypeScript.             |
| Pre-CR           | Run Anti-Slop against the changed-file set and enforce the configured result before review.                          |
| Quality Runner   | Discover quality capabilities, normalize and contextualize their evidence, audit repositories, and plan remediation. |

Anti-Slop keeps independent package value because standard ESLint consumers can
use it without Pre-CR or Quality Runner. That does not require a separate
Showcase card.

## Product and architecture backlog

These are not standalone Showcase gates:

- Fix the reviewed `require-empty-state-action` false positive without weakening
  intended empty-state coverage.
- Make the accepted findings expose plain-language rationale, safe direction,
  and rule boundary.
- Establish Anti-Slop as the semantic owner for overlapping JS/TS UI rules.
- Make Quality Runner's overlapping text heuristics advisory fallbacks only
  when the plugin is unavailable, and deduplicate them when both sources run.

The combined story must not claim that all three layers are integrated until
the ownership, fallback, deduplication, and evidence handoff are implemented
and behavior-verified.
