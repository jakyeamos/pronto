# Deletion Proof Workbench: unknown is a result

Deletion Proof Workbench previews one exported symbol, separates known
references from dynamic, reflective, generated, and external unknowns, and
requires explicit apply with a recovery reference.

The current-dev checkpoint proves one bounded `unusedThing` deletion in a
temporary Git fixture, blocks `usedThing` when a known consumer remains, and
passes the repository's test, lint, and package gates. Stale-candidate breadth,
downstream Change Radius/Contract Watch parity, hosted access, and external
publication remain open.
