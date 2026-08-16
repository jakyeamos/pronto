# Rule Lab: a fix must keep its counterexample

Rule Lab makes quality-rule changes inspectable before they become policy. A
finding names the rule and risk, an isolated draft runs beside the canonical
rule, and positive and negative fixtures make the safety boundary visible.

## The local W2 case

The `no-clickable-div` demo rule is evaluated against one positive fixture and
one semantic-button counterexample. The canonical rule and draft both produce
the expected match/no-match results. There are no gained or lost matches, so
the counterexample survives the edit. Rule Lab saves a receipt and re-checks it
as fresh against the temporary target revision.

The product story is: **finding → isolated draft → compare → preserve the
counterexample → save the receipt**. Removing a finding is not success if the
negative fixture breaks.

## Evidence boundary

The current dev checkpoint is `bcddbe0f7615d654c9d0866c603955e67db380e5` on a
clean tree, using Quality Runner `0.6.0`. Pytest, Ruff, and the extension tests
pass; pyright remains open with eight type errors. The receipt is fresh at
verification `4da94293fa1880e8`. This proves the current headless W2 slice, not
direct VS Code acceptance, cross-producer handoff, universal type health,
automatic repair, hosting, or publication. The 1600×900 preview is available in
`assets/preview-16x9.png` with the SVG source beside it.
