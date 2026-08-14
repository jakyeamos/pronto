# Behavior Coverage Atlas: know what the test proves

Behavior Coverage Atlas maps named product behaviors to repository-owned test
evidence. It keeps an assertion, an execution, a failure, a missing link, and
a stale receipt as different states instead of compressing them into a line
coverage percentage.

## The current dev case

At the current `dev` head (`8c052b1`), the fixture declares checkout, profile,
billing, and search behaviors. The fresh run shows a verified assertion,
execution without assertion, a failed assertion, and missing evidence. A
duplicate test link remains a diagnostic. Changing the results target makes the
entire historical receipt stale without rewriting the underlying evidence.
Repository tests, lint, and packaging pass.

The story is: **declare → link → classify → review**.

## Evidence boundary

The current-dev matrix is a local CLI run from source revision `8c052b1` against
fixture revision `40f738b`. It proves explicit state classification and the
binary preview, not line coverage, complete runtime behavior, Review Attention
Map integration, hosting, or publication.
