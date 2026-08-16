# Review Attention Map: follow the reason

## Target

Review Attention Map places explicit contract and behavior signals on a changed
region, keeps source and freshness boundaries visible, and leaves the decision
with the reviewer instead of inventing a risk score.

## Current local checkpoint

- Source: `dev` at `2f387be0b30f32a37d28cb3fd4d09f7b68cbf787`.
- A temporary diff identified `src/api.ts:1` with Contract Watch and Behavior
  Coverage Atlas signals, while an outside-diff signal stayed unmatched.
- The direct review command recorded `reviewed` with a human note; the missing
  diff path returned a blocked result rather than silently producing a map.
- Tests, lint, and package checks passed. The crop-safe 1600×900 PNG/SVG
  preview was visually reviewed.

## Boundaries

This is current-dev local evidence, not a complete producer/freshness handoff
or runtime coverage claim. Hosted no-auth access and GitHub, portfolio, and
Handshake readbacks remain open.
