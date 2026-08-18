# Fleet Radar route plan

Status: C0 public Showcase admission is complete; a local read-only refresh
candidate packet now includes receipt-contract classification, native-link
metadata, broad registry failure coverage, a current-dev refresh readback, and
a verified 16:9 binary preview.
Real producer parity, direct native navigation, and a public no-auth case URL
remain gated.

The [canonical target](../ideal-demo-targets.md#fleet-radar) owns the promise
and proof gate. Fleet Radar projects evidence; it does not invent readiness for
repositories that lack receipts.

## 1. Ideal target

**North star:** a maintainer sees two repositories' freshness, blockers, and
next actions and refreshes one read-only status without inventing an aggregate
score.

**Non-negotiable:** registry identity, freshness, unavailable paths, receipt
contract status, and native follow-up links remain visible. Real producer
parity and direct navigation block the stronger claim.

### Durable contract distinctions

- **Upstream projection:** Fleet Radar consumes producer receipts without
  inventing status, preserving the producer's target, outcome, freshness, and
  contract state in the fleet snapshot.
- **State coverage:** missing, unreadable, mixed, stale, dirty, duplicate, and
  refresh-race conditions remain distinct, actionable states rather than one
  synthetic red/green result.
- **Native follow-up:** every blocker points back to its receipt or source
  location and carries a concrete next action; Fleet Radar does not claim that
  the owning tool opened successfully until direct navigation is verified.

## 2. Concept materials

The candidate frames now have a current-dev registry refresh behind them, with
W6 contract/failure proof and W7 binary-preview/hosting evidence in the packet.
Real producer parity, native navigation, and public deployment are still proof
gates.

| Frame        | Visual                                               | On-screen line                | Intended evidence moment       |
| ------------ | ---------------------------------------------------- | ----------------------------- | ------------------------------ |
| 1. Registry  | Two registered repositories with source identity     | “Know what is in the fleet”   | Scope is explicit              |
| 2. Freshness | Per-check freshness, blockers, and unavailable state | “Do not flatten the evidence” | Each state has meaning         |
| 3. Deep link | One blocker opens its native repository evidence     | “Follow the owner”            | Projection preserves authority |
| 4. Refresh   | One read-only status refresh updates its timestamp   | “Refresh without mutating”    | Continuity is safe             |

**Preview concept.** Use two repository cards and one selected status receipt;
avoid a fleet-wide health score.

**Narrative spine.** Register → inspect freshness/blockers → follow native link
→ refresh read-only → preserve gaps.

## 3. Build-gap specification

Reviewed baseline: the local MVP records an explicit registry, per-repository
freshness/blockers, read-only snapshot writing, receipt-contract status, native
follow-up metadata, and broad registry failure handling. The candidate packet
proves local projection and ref safety; real producer parity and C5 continuity
proof remain open.

Project disposition: `conditional_gate` — close the real-producer and direct-
navigation conditions before treating fleet continuity as a demonstrated
product result.

Gap classes: evidence — FR-0; demo_integration — FR-1; product — FR-2;
packaging — FR-3.

| ID   | Gap to close                     | Observable acceptance condition                                                   | Owner             | Required proof               |
| ---- | -------------------------------- | --------------------------------------------------------------------------------- | ----------------- | ---------------------------- |
| FR-0 | Prove real producer condition | Two registered repositories expose source-bound evidence, freshness, and blockers through configured upstream receipts | Evidence owner    | Upstream receipt matrix      |
| FR-1 | Prove native deep links          | A blocker opens the owning repository or evidence source without losing identity  | Integration owner | Direct navigation readback   |
| FR-2 | Preserve read-only refresh (verified locally) | Refresh updates a bounded snapshot and never mutates repositories or providers | Safety owner | W5 before/after mutation check |
| FR-3 | Package the public case          | Verified 16:9 PNG, short copy, deployed no-auth fleet case, and proof link agree | Showcase owner    | Material review and credentialless URL readback |

**Required build order:** FR-0 → FR-1 → FR-3. FR-2 is already verified by the
W5 local refresh proof and remains a regression check, not a new product-build
blocker. Video is optional after the evidence gate.
