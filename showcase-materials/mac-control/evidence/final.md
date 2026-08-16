---
domain: technical
output_type: feature_explainer
packet_ids: [mac-control-showcase]
source_draft: draft.md
qa: qa.yaml
style_pass: humanizer-blader
---

# Mac Control

Mac Control gives AI agents a local, inspectable path into native macOS apps. A command-line client sends typed requests over an owner-only Unix socket to a per-user daemon. That daemon owns policy checks, GUI execution, verification, and redacted evidence.

Moving a cursor is the easy part. An agent action can still fail because the wrong app is active, a target is ambiguous, permission state changed, or the system never verified the result. Mac Control makes those conditions explicit instead of hiding them behind a success-shaped response.

Sensitive workflows follow a prepare, approve, execute, and verify lifecycle with short-lived, single-use authority. The installed surface exposes typed app, workflow, keyboard, control, task, shortcut, authorization-notice, and adapter operations. Browser page content stays outside this native provider boundary.

One project-authored Phase 2 benchmark makes the behavior concrete. For a named System Settings focus action, the Mac Control lane reported a 134.924 ms median across 3 of 3 passing samples. Direct System Events reported 479.491 ms, and generic GUI control reported 11814.550 ms under the same comparison identity. That is evidence for one bounded task, not a universal speed claim.

The negative cases matter just as much. Finder scroll comparisons were blocked before a comparable passing measurement. The installed release check observed on August 12, 2026 also did not pass: four checks passed, receipt storage failed, and eight daemon-authoritative checks were blocked.

The project is therefore presented as a measured working system with incomplete current release evidence—not as production ready. The human-owned decisions are the safety model, route boundaries, approval semantics, redaction rules, verification oracles, and what counts as enough evidence. Agents can use the runtime surface. They cannot turn missing permission, ambiguous state, or failed verification into success.

## Style pass notes

- Opened with the product outcome before architecture.
- Shortened long clauses and varied sentence rhythm.
- Preserved every metric, qualifier, limitation, and provider boundary.
- Added no facts, examples, comparisons, or citations.
