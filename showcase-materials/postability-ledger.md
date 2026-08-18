# Showcase postability handoff

Reviewed: 2026-08-14. The machine-readable source is
[`postability-ledger.json`](postability-ledger.json); this page is the compact
human handoff.

## Current boundary

- **34/34** public Showcase projects have an explicit posting row and a required
  GitHub, portfolio, and Handshake destination.
- **32/34** have a complete local packet: route, evidence, no-auth source,
  preview/thumbnail, short copy, and an explicit role/claim boundary.
- **2/34** remain locally incomplete because filling the gap would require
  fabricating proof: Mac Control still needs the real installed 16:9 capture and
  redacted authorization/outcome receipt; Chiron's Forge still lacks owner-visible
  repository and deployment provenance.
- **0** are marked externally posted. The ledger intentionally records
  `external_posting_proof: false` for every row.

“Local packet ready” means the materials can be handed to an owner for review;
it does not mean a runtime, rights, hosted URL, destination readback, or
publication authority has been proven.

## Deferral rule

Every open gate has a reason-coded `deferral` or remains an explicit local
package gap. The exact project rows, re-entry conditions, active gate, and next
safe action live in the JSON so this handoff does not flatten them into a single
“make a demo” task. Synthetic fixtures remain labeled as appendices, and no
generated visual is substituted for a required real product capture.

The next execution pass can therefore be linear: review the local packet, close
the named product/evidence/authority gate, then verify hosting and destination
readbacks. No project needs another strategy pass before that work begins.
