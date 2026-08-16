# Context Compiler Contract Handshake draft

Status: added to the Handshake profile Projects section. The AI showcase
submission remains blocked by Handshake's preview uploader; no duplicate
showcase submission was created.

## Copy

**Title**

Context Compiler Contract: Make context fail before it becomes agent behavior

**Description**

See [description.txt](description.txt). It is 388 characters and is the exact
copy intended for the Handshake description field.

## Upload materials

- Preview source: [preview-16x9.svg](preview-16x9.svg), a 1600 × 900 crop-safe source
- Raster fallback: [preview-16x9.png](preview-16x9.png), rendered locally after
  Handshake rejected SVG, PNG, and JPEG chooser uploads with the same generic
  error. The Chrome file-URL setting was already enabled, so the failure is
  downstream of the extension permission.
- Local page to host: [comparison.html](../../context-compiler-contract/comparison.html)
- Intended target: [Context Compiler ideal demo](../../ideal-demo-targets.md#context-compiler-contract)

## What the viewer should see

A real AIOS compile result passes. Removing one source-selection reason and
flipping `route_compatible` to `false` create exact field-level failures.
Restoring both fields returns the original digest.

## Role split and limits

The validator owns the contract boundary. AIOS owns context selection and
execution. The two invalid states are synthetic mutations of a real baseline;
they are not presented as production incidents.

## Before posting

1. Put the comparison page on a public no-auth host.
2. Restore a working Handshake preview upload path or provide a verified public
   preview URL.
3. Verify the hosted page and approve the exact description and link before
   creating the single AI showcase submission.
