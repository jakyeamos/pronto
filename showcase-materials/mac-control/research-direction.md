# Mac Control focus-session direction

Research depth: Standard, narrowed to the architecture decision that could
change MC-1.

## Recommendation

Keep the focus-session composer inside Mac Control's existing typed-plan and
digest boundary. Emit a preview-only, deterministic product artifact first;
add executable action routes and observers only in MC-2 and MC-3.

The original concept used a named macOS Research Focus profile as its third
effect. That target was replaced with a bounded TextEdit scratchpad before
MC-3 implementation. Apple's supported Focus status API exposes an authorized,
app-relative `isFocused` value, not the active profile name. A receipt claiming
that the named profile was independently verified would therefore overstate the
available observation. The revised flow still performs three useful effects:
open the brief, open its scratchpad, and arrange both windows.

## Reference findings

| Reference                                                                                            | Observed pattern                                                                                        | Relevance and disposition                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Hammerspoon](https://github.com/Hammerspoon/hammerspoon)                                            | macOS capabilities are exposed to user-authored Lua automation.                                         | Useful evidence that composed desktop automation is practical; rejected as the execution model because arbitrary scripts would bypass Mac Control's typed action and approval boundary. |
| [AeroSpace](https://github.com/nikitabobko/AeroSpace)                                                | A Swift macOS window manager exposes CLI-first, plain-text configuration and multi-command composition. | Useful window-layout reference; rejected as the plan authority because command chaining does not supply Mac Control's per-effect approval and independent verification contract.        |
| [Open Interpreter](https://github.com/openinterpreter/open-interpreter)                              | Natural-language requests can produce local code execution with a confirmation step.                    | Useful confirmation precedent; rejected because broad generated-code execution is materially wider than the bounded focus-session target.                                               |
| [Apple `INFocusStatusCenter`](https://developer.apple.com/documentation/intents/infocusstatuscenter) | With user authorization, an app can read the user's app-relative Focus status.                          | Useful for a future generic interruption-status signal; rejected as evidence for a named Focus profile because the public read surface does not identify that profile.                  |

The existing `TaskPlan`, sorted-key `JSONCodec`, approval stores, and typed
adapter registry are the stronger local seam. No dependency or copied source is
needed. The preview must say `executable=false` until the product owns the
brief-opening, window-layout, and Focus routes plus their independent readback.

## Authorized slice

MC-1 adds one bounded request, one canonical three-effect preview, exact
rollback and verification notes, a deterministic approval digest, a CLI read
surface, positive/normalization/negative tests, and redacted proof fixtures.
MC-2 adds durable progress and partial-failure presentation. MC-3 now adds the
plan-bound, redacted verification-record contract; executable fixture-opening
and live Accessibility observation remain open.
