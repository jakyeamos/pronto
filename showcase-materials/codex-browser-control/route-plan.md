# Codex Browser Control route plan

Status: CB-1 is closed as a labeled synthetic fixture and source-backed
contract receipt. CB-7 is now closed as the local case-study, claim, short-copy,
no-auth page-source, and reviewed binary-preview package. CB-2 is explicitly
blocked at the installed round trip:
the current source build passes, but the connected Chrome provider is an older
extension revision and its doctor response fails protocol validation. The
canonical checkout is `/Users/jakyeamos/projects/browser-control` (the
repository package is `codex-browser-control`).

The [canonical target](../ideal-demo-targets.md#codex-browser-control) owns the
durable promise and proof gate.

## 1. Ideal target

**North star:** on a polished synthetic travel site, an agent observes a booking
draft, proposes one harmless preference change, receives exact human approval,
applies it once, verifies the fresh page state, and refuses to replay after the
user changes the itinerary.

**Non-negotiable:** use only synthetic or disposable state. Human approval must
bind the exact plan and current page state; no real booking, purchase, message,
or account action is authorized.

## 2. Concept materials

All frames are **concept** until the installed extension and native bridge pass.

| Frame               | Visual                                                                           | On-screen line                                 | Intended evidence moment                  |
| ------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------- |
| 1. Observed state   | Synthetic itinerary and side panel show a fresh page digest                      | “Act on the page that is actually open”        | Observation and identity precede planning |
| 2. Exact plan       | “Change seat preference to aisle” names target, old value, new value, and effect | “Review the exact mutation”                    | Scope is understandable                   |
| 3. Human approval   | Single-use approval displays expiry and page-state binding                       | “Approval belongs to this plan and this state” | Authority is precise                      |
| 4. Apply and verify | Page changes once; fresh observation confirms the new value                      | “Apply once. Read it back.”                    | Mutation and verification agree           |
| 5. State changes    | User edits the itinerary manually; digest becomes stale                          | “The page moved on”                            | Race condition is visible                 |
| 6. Refusal          | Old plan is rejected before mutation and requests a new review                   | “Stale state means stop”                       | Safety behavior is decisive               |

**Preview concept.** Synthetic browser page on the left, side-panel plan and
single-use approval in the center, stale-state refusal on the right. Headline:
“Browser actions that expire when the page changes.”

**Narrative spine.** Fresh observation → exact plan → human approval → one apply
→ independent verify → stale-state refusal.

## 3. Build-gap specification

Reviewed baseline: typed observe, inspect, plan, approval, apply, verify, and
rollback surfaces exist over an owner-only native bridge; the current checkout
also contains the in-progress `browser.await_approval` approval-notification
surface. Installed Chrome and live round-trip proof are open. The live source
checkout was dirty at inspection, so its source gate is recorded as current
dirty-checkout evidence rather than a clean release checkpoint.

Project disposition: `targeted_gap_closure` — preserve the typed control path
and add a synthetic environment, exact approval presentation, installed proof,
and public case packaging.

Gap classes: demo_integration — CB-1, CB-2; product — CB-3; evidence — CB-4,
CB-5, CB-6; packaging — CB-7.

| ID   | Gap to close                                    | Observable acceptance condition                                                                                             | Owner                        | Required proof                                               |
| ---- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------ |
| CB-1 | Build the synthetic demo site and fixture       | **Closed:** self-contained synthetic page, semantic state manifest, harmless reversible action, and deliberate stale target | Demo/product owner           | `demo-site.html`, `synthetic-fixture.json`, and CB-1 receipt |
| CB-2 | Prove installed observe-to-plan behavior        | The installed extension and bridge produce a redacted plan bound to the correct tab, target, and state digest               | Browser-provider owner       | Installed build identity and plan receipt                    |
| CB-3 | Make approval exact and comprehensible          | Approval shows mutation, scope, expiry, single-use status, and no hidden follow-up action                                   | Authorization/design owner   | Approval capture and digest validation                       |
| CB-4 | Prove apply-once and fresh verification         | The approved preference changes once and a new observation reads back the expected value                                    | Execution/verification owner | Apply and verify receipts plus DOM readback                  |
| CB-5 | Prove stale-state refusal before mutation       | Manual page change invalidates the old plan; replay leaves the page unchanged                                               | Safety owner                 | Before/after state, refusal receipt, and negative assertion  |
| CB-6 | Validate privacy and external-action boundaries | Captures contain no real history, accounts, tokens, or sensitive page content; mutation authority is fixture-scoped         | Privacy/security owner       | Permission and media review                                  |
| CB-7 | Package the no-auth case                        | **Closed locally 2026-08-14:** synthetic plan, approval, refusal case, short copy, page source, and reviewed 16:9 preview are present | Showcase owner               | `case-study.json`, `claim-ledger.json`, `public-description.txt`, `public/index.html`, `evidence/cb-7-material-checkpoint.json` |

**Build order:** CB-1 **closed** → CB-2/CB-3 → CB-4/CB-5 → CB-6 → CB-7 (local package closed; hosted case remains separate).

## 4. CB-1 closure

CB-1 is closed as a reproducible material packet. The fixture is intentionally
synthetic and remains the primary demo surface until an installed disposable
capture exists:

- [`demo-site.html`](demo-site.html) is a self-contained local HTTP(S) page
  with a visible synthetic label, a reversible seat preference, a reset path,
  and a local itinerary mutation that changes the semantic target name.
- [`synthetic-fixture.json`](synthetic-fixture.json) is the state manifest. It
  distinguishes fixture wiring from the provider's runtime `documentId`,
  `elementRef`, and `elementFingerprint`, and records the exact negative cases.
- [`evidence/cb-1-fixture-receipt.json`](evidence/cb-1-fixture-receipt.json)
  binds the material to the current `dev` checkout and records the dirty source
  state, the passing source gate, and the installed-evidence boundary.

The stale case deliberately changes the accessible name from `Seat preference`
to `Seat preference — itinerary updated`. This is source-aligned: the content
script fingerprints the semantic shape and rejects an old target with
`target_changed`. A value-only change would not be presented as a stale-target
proof because the current semantic shape intentionally does not include the
select's value.

## 5. CB-1 claim boundary

The packet proves the synthetic page and its acceptance contract, not a live
Chrome run. It does not claim an installed extension, Native Messaging bridge,
side-panel capture, MCP round trip, apply/verify receipt, or public hosted case.
Those claims remain CB-2 through CB-7 work. The current source build passes its
documented gates, but the installed provider is not a valid proof surface until
the current extension/native-host pair is reloaded together.

## 6. CB-2 installed round-trip boundary

The [CB-2 blocker receipt](evidence/cb-2-blocker.json) records a read-only
`doctor --json` probe after the current dirty checkout passed
`corepack pnpm@11.9.0 check`. Configuration, native registration, and a
connected provider are present, but the connected extension reports version
`0.3.0` while the current built CLI reports `0.2.0`; the doctor round trip then
fails response-schema validation before an observation or plan receipt exists.

CB-2 is **not closed**. Reloading the current unpacked extension and native host
would change persistent per-user Chrome/Codex state and requires explicit
deployment approval under the repository contract. Park CB-2/CB-3 until that
reviewed reload is authorized, then capture the synthetic observe-to-plan and
exact approval path together.

## 7. Local showcase package

The local candidate distribution layer is now self-contained: [`case-study.json`](case-study.json),
[`claim-ledger.json`](claim-ledger.json), [`public-description.txt`](public-description.txt),
[`preview.html`](preview.html), [`public/index.html`](public/index.html), and the
reviewed 1600x900 binary bound by [`evidence/cb-7-material-checkpoint.json`](evidence/cb-7-material-checkpoint.json).
It intentionally describes CB-1 and the CB-2 blocker without presenting a
synthetic page as an installed browser receipt.
