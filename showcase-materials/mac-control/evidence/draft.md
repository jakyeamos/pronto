---
domain: technical
output_type: feature_explainer
packet_ids: [mac-control-showcase]
draft_version: 1
---

# Mac Control — AI Showcase Materials

Mac Control gives AI agents a local, inspectable path into native macOS apps. A command-line client sends typed requests over an owner-only Unix socket to a per-user daemon that owns policy checks, GUI execution, verification, and redacted evidence.

The problem is not simply making a cursor move. An agent action can fail because the wrong app is active, a target is ambiguous, permission state changed, or a mutation ran without a verified outcome. Mac Control makes those conditions part of the interface instead of hiding them behind a success-shaped response.

Sensitive workflows follow a prepare, approve, execute, and verify lifecycle with short-lived, single-use authority. The installed capability surface includes app, workflow, keyboard, control, task, shortcut, authorization-notice, and adapter operations. Browser page content remains a separate provider boundary.

One project-authored Phase 2 benchmark makes the behavior concrete. For a named System Settings focus action, the Mac Control lane reported a 134.924 ms median across 3 of 3 passing samples, compared with 479.491 ms for direct System Events and 11814.550 ms for generic GUI control under the same comparison identity. That result is evidence for one bounded task, not a universal speed claim.

The negative cases matter just as much. Finder scroll comparisons were blocked before a comparable passing measurement, and the installed release check observed on August 12, 2026 did not pass: four checks passed, receipt storage failed, and eight daemon-authoritative checks were blocked. The project is therefore presented as a measured working system with incomplete current release evidence—not as production ready.

The human-owned decisions are the safety model, route boundaries, approval semantics, redaction rules, verification oracles, and what counts as enough evidence. AI agents use the runtime control surface, but they do not get to convert missing permission, ambiguous state, or failed verification into success.
