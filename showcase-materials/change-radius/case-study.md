# Change Radius: blind spots stay visible

Change Radius is a bounded navigation aid for a changed symbol. It follows
provider-named static references, separates tests from ordinary consumers, and
keeps dynamic, generated, reflective, and external use visible as limitations
instead of collapsing them into a confidence score.

## The current dev case

At the current `dev` head (`a849c8a`), the `parseThing` symbol in a temporary
TypeScript fixture has two static import edges: a consumer and a test. The seed
also performs a dynamic import, so the receipt records `dynamic-import` as an
unknown. The target revision, provider, known edges, limitations, and next
test review action remain together. Repository tests, lint, and packaging pass.

The product story is: **changed symbol → known consumers/tests → explicit
unknowns → source-bound receipt**.

## Evidence boundary

The current-dev graph is a local run from source revision `a849c8a` against
temporary target revision `9977af4`. It proves a bounded static fixture slice
and binary preview, not complete runtime use, downstream parity, a universal
blast radius, or publication.
