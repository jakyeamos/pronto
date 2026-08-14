# Mac Control — AI Showcase packet

## One-line promise

A local safety and verification layer between AI intent and native macOS action.

## Handshake description

Mac Control gives AI agents a local, inspectable path into native macOS apps. A per-user daemon routes typed actions through approval, execution, verification, and redacted receipts. In one project-authored System Settings benchmark, a verified focus action recorded a 134.924 ms median across 3/3 samples. Current release evidence is incomplete, so this is measured capability—not a production-readiness claim.

Character count: **411 / 500**

## Case study

### The problem

Moving a cursor is the easy part. An agent action can still fail because the wrong app is active, a target is ambiguous, permission state changed, or the system never verified the outcome. A success-shaped response hides those failures.

Mac Control turns them into explicit interface states. A command-line client sends typed requests over an owner-only Unix socket to a per-user daemon. That daemon owns native GUI execution, policy checks, verification, and redacted evidence.

### How it works

Sensitive workflows follow a prepare, approve, execute, and verify lifecycle with short-lived, single-use authority. The installed surface exposes typed app, workflow, keyboard, control, task, shortcut, authorization-notice, and adapter operations. Browser page content stays outside this native provider boundary.

The creator-owned work is the safety model: route boundaries, approval semantics, verification oracles, redaction rules, and the threshold for enough evidence. Agents can request and consume the runtime surface. They cannot turn missing permission, ambiguous state, or failed verification into success.

### Bounded evidence

In one project-authored Phase 2 comparison, a verified System Settings focus action recorded these medians:

| Lane                 |       Median | Verified samples | Scope                            |
| -------------------- | -----------: | ---------------: | -------------------------------- |
| Mac Control          |   134.924 ms |              3/3 | Named System Settings focus task |
| Direct System Events |   479.491 ms |              3/3 | Same comparison identity         |
| Generic GUI control  | 11814.550 ms |              3/3 | Same comparison identity         |

This is one task under the repository's documented benchmark method. It is not a universal speed ranking.

### Limits that stay visible

The same benchmark set leaves Finder scroll comparisons unranked because measurement was blocked before a comparable passing result. The installed release check observed on August 12, 2026 also did not pass: four checks passed, receipt storage failed, and eight daemon-authoritative checks were blocked.

The right claim is measured working capability with incomplete current release evidence. “Production ready,” “universally faster,” and “proven across macOS” are not supported.

## 70-second demo script

The capture should use two labeled proof moments until one reviewed workflow can safely combine authorization and native action.

| Time   | Picture                                                                                  | Caption or voiceover                                                                                   | Proof                           |
| ------ | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------- |
| 0–5s   | Clean title over a real Control Center state                                             | “AI can operate a Mac. The harder problem is proving what it was allowed to do—and whether it worked.” | Five-second problem and outcome |
| 5–14s  | Terminal and menu-bar Control Center, with account and path details hidden               | “Mac Control keeps native actions local and routes them through an owner-only daemon.”                 | Local boundary                  |
| 14–28s | Prepare `approval.smoke`; show the pending operation in Control Center                   | “This smoke path changes no app or account. It demonstrates explicit, short-lived approval.”           | Intent and human control        |
| 28–38s | Human approves in Control Center; crop out tokens and identifiers                        | “Authority comes from the user-facing control—not from the agent prompt.”                              | Authorization boundary          |
| 38–55s | Run a reviewed System Settings native-control action and show the focused control change | “The daemon performs one bounded action and checks the postcondition.”                                 | Real native interaction         |
| 55–64s | Show a redacted machine-readable outcome with `verified` and route fields highlighted    | “The result names the route, outcome, and verification state without retaining private UI content.”    | Verified result and evidence    |
| 64–70s | Show the non-passing release summary beside the product                                  | “This is measured capability, not a production-readiness claim. Missing live evidence stays blocked.”  | Honest current status           |

Recording rules:

- Record the product, not slides.
- Use a fresh external macOS session after daemon connectivity and permissions are confirmed.
- Do not hide the native action behind a cut.
- Add burned-in captions; assume silent playback.
- Keep the final edit between 45 and 90 seconds.
- Do not record approval tokens, request IDs, account names, private paths, window contents, or notification history.

## 16:9 preview brief

Status: **capture pending**

Use a real 1920×1080 capture with a safe center crop. The dominant state should be Mac Control's native Control Center showing one bounded operation. Pair it with a tightly cropped, redacted outcome panel that makes `verified` legible at roughly 400 pixels wide.

Headline: **Native Mac actions agents can verify**

Composition:

- Left two-thirds: Control Center, with the pending or completed operation as the focal point.
- Right one-third: redacted outcome with only route, status, and verification fields visible.
- One headline, no feature collage, no generic robot imagery.
- High contrast; remove incidental desktop, browser chrome, usernames, paths, tokens, and unrelated notifications.
- Export a full 16:9 source and a 400-pixel-wide legibility proof.

Do not replace this with generated product UI. The Showcase gate requires immediate proof of a real product state.

## Capture checklist

Before recording:

- Confirm `macctl` resolves to `/Users/jakyeamos/.local/bin/macctl`.
- Confirm the daemon socket is reachable from the external macOS session.
- Run the read-only doctor and release checks; record their exact status.
- Close unrelated apps and notifications.
- Use a clean macOS account state with no private documents visible.
- Rehearse the action and recovery path without recording.

During recording:

- Keep the approval decision human-controlled.
- Show intent, approval, execution, and verification as separate visible states.
- Keep raw JSON cropped to the few fields the audience needs.
- Stop if the target is ambiguous, the active app differs, or verification is indeterminate.
- Do not retry a possibly dispatched mutation.

After any capture:

- Inspect every frame at full size for tokens, request IDs, usernames, paths, document titles, message bodies, notifications, and private selector text.
- If video is produced, verify captions match the observed action.
- Verify the benchmark caption includes the named task, sample count, and project-authored qualifier.
- Verify the limitation slide matches the latest release report.
- Recheck the 400-pixel preview and mobile case-study layout.

## Publication gate

This packet is ready for review and capture, not submission. Publication remains blocked until all of the following are true:

- A real preview image is captured and reviewed.
- Redacted authorization and verified-outcome proof are present.
- A public no-auth case-study URL is selected and tested on mobile and desktop.
- The repository disclosure boundary is approved.
- The description, preview, and page make the same factual promise; any optional
  recording matches it too.
- The current release status and benchmark qualifiers are refreshed immediately before publication.

## Evidence map

- Research packet: [evidence/research-packet.yaml](evidence/research-packet.yaml)
- Research gaps: [evidence/research-missing.yaml](evidence/research-missing.yaml)
- Drafting packet: [evidence/knowledge-packet.md](evidence/knowledge-packet.md)
- Exact QA ledger: [evidence/qa.yaml](evidence/qa.yaml)
