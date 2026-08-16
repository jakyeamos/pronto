# Contract Watch: facts are not decisions

Contract Watch compares two explicit OpenAPI contracts and records semantic
certainty separately from policy consequence. A human disposition remains
required; an undocumented consumer is not treated as a clean result.

## The local W3 case

The fixture removes `GET /users`, adds `POST /reports`, changes and deprecates
`GET /orders`, and records four explicit changes. The removal is mitigated and
the response change verified; the addition and deprecation remain unreviewed.
No merge or release verdict is produced.

The story is: **compare → classify → expose unknowns → disposition → hand off**.

## Evidence boundary

The current `dev` receipt is a local CLI run against a temporary OpenAPI fixture,
with a visually reviewed 1600×900 preview. It proves bounded comparison and
human disposition, not complete consumer impact, Review Attention Map or
Deletion Proof integration, a merge/release verdict, hosting, or publication.
