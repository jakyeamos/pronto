# Pre-CR Suite preview review

Status: **PCR-4 closed; the standalone static package is locally reviewed.**

The dedicated preview at `showcase-materials/pre-cr-suite/preview.html` was
served from localhost and reviewed directly in external Chrome on August 12, 2026.

| Check             | Result                                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------------------------- |
| Target viewport   | 1600 × 900                                                                                              |
| Target document   | 1600 × 900; no horizontal or vertical overflow                                                          |
| Evidence assets   | Both installed VS Code captures loaded at natural size                                                  |
| Narrative         | Return, Recall, Act, and Prove are visible in one crop                                                  |
| Command legend    | Where Was I?, Quick Actions, and Pre-CR Check are visible                                               |
| Browser console   | 0 application errors                                                                                    |
| Preview encoding  | `preview-16x9.png` contains PNG image data                                                              |
| Small-size review | The 800 × 450 downscale keeps the headline, four stages, before/after labels, and command names legible |

The preview is deliberately a standalone IDE story. It leads with context
recovery and the editor command path, then uses the paired PCR-3 captures as the
proof moment: one uncovered changed line becomes a passing 100% changed-line
receipt after a focused behavior test.

The claim boundary remains visible in `claim-ledger.json`. This package proves
installed local VSIX behavior. It does not claim marketplace distribution, and
the PCR-3 fixture does not prove that the standalone CLI blocks its warning
state.
