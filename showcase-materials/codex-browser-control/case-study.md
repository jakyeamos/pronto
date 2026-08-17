# Codex Browser Control: browser actions that expire when the page changes

## The case

The demo uses a disposable synthetic travel page. It has one visible seat
preference and a local “change itinerary” control that changes the semantic
accessible name of that target. The fixture is intentionally boring: no account,
payment, submission, network, or real booking state is present.

The intended flow is observe → exact plan → single-use approval → apply once →
fresh verify. The plan names the target, old value, new value, effect, expiry,
and page digest. The source contract also defines the negative case: once the
itinerary changes, the old semantic target is stale and replay must stop before
mutation with `target_changed`.

## Where the proof stops

CB-1 is a source-backed synthetic fixture, not an installed-provider result. The
current source checkout passes its documented quality gate, but the connected
Chrome extension reports `0.3.0` while the current built CLI reports `0.2.0`;
the doctor round trip fails response-schema validation before an observation or
plan receipt exists. Reloading the current extension and native host changes
persistent per-user state and is parked behind explicit deployment approval.

The page therefore shows the safety contract and the desired refusal boundary
without claiming that a browser actually changed. CB-2/CB-3, CB-4/CB-5, and
hosted verification remain separate gates.

## Current material boundary

This local packet contains the synthetic fixture, source receipts, claim ledger,
short copy, candidate 16:9 preview, and no-auth page source. Installed
observe-to-plan, exact approval, apply/verify, stale refusal, hosting, and
external destination readbacks remain open.
