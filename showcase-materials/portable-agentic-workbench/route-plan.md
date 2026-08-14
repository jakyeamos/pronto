# Portable Agentic Workbench route plan

Status: PW-1 through PW-3, PW-5, and PW-6 are closed in a local,
evidence-bound package. PW-4 was attempted and is explicitly blocked by the
repository's no-provider/no-registration boundary. The selected workflow is
`safe-tool-guards`; generic and Codex are both supported native projections.

The [canonical target](../ideal-demo-targets.md#portable-agentic-workbench) owns
the durable promise and proof gate.

## 1. Ideal target

**North star:** take one bounded review workflow from a public manifest, preview
its effects, install or project it into two clean supported environments, and
show the same safety contract alongside honest provider-specific differences
and an explicitly scoped recovery boundary.

**Non-negotiable:** portability means preserved intent and boundaries, not
pixel-identical configuration or unsupported parity.

## 2. Concept materials

All frames are **concept** until both clean-context validations pass.

| Frame            | Visual                                                                                         | On-screen line                                   | Intended evidence moment                                             |
| ---------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------- |
| 1. One workflow  | The public `safe-tool-guards` manifest sits between two environments                           | “Define the behavior once”                       | The shared safety contract is the hero                               |
| 2. Preview       | Dry-run shows two allowlisted files, manual review, and no mutation                            | “See the change before installing”               | Safety begins before persistence                                     |
| 3. Environment A | Generic projection copies the contract and receipt                                             | “Preserve intent in the first host”              | One real supported path works                                        |
| 4. Environment B | Codex projection copies the same contract; registration stays manual                           | “Adapt without pretending parity”                | Provider edge remains visible                                        |
| 5. Comparison    | File hashes and statuses align; no adapter is silently registered                              | “Same contract. Honest edges.”                   | Portability is behavioral at install                                 |
| 6. Recovery      | Receipt-scoped uninstall previews and reverses the two unchanged files; a modified file blocks | “Recover what you own; preserve what you do not” | Local file recovery is proven without touching an unrelated sentinel |

**Preview concept.** One central manifest branching into two verified receipts,
with a small amber “manual registration” badge on the Codex edge. Headline:
“Carry agent workflows across environments without losing their boundaries.”

**Narrative spine.** Shared contract → preview → projection A → projection B →
behavioral comparison → recovery.

## 3. Build-gap specification

Reviewed baseline: the repository exposes a manifest catalog, a public
`safe-tool-guards` contract, dry-run install, validation commands, and
public/private boundaries. A clean HEAD archive was used because the primary
checkout has unrelated dirty files.

Project disposition: `targeted_gap_closure` — use the existing catalog and
installer, then prove one portable workflow across two clean environments and
its recovery boundary.

Gap classes: content — PW-1; demo_integration — PW-2; evidence — PW-3, PW-4,
PW-5, PW-6.

| ID   | Gap to close                              | Observable acceptance condition                                                                            | Owner                   | Required proof                                                                                                                                  |
| ---- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| PW-1 | Select one public representative workflow | `safe-tool-guards` is manifest-backed, portable, dependency-free, and has a visible manual-review boundary | Product/catalog owner   | `case-fixture.json`, manifest review, safety validation                                                                                         |
| PW-2 | Establish two clean environment fixtures  | Generic and Codex start from empty recorded target roots with no prior installation                        | Demo operations owner   | `validation-receipt.json` baseline snapshots                                                                                                    |
| PW-3 | Prove non-mutating previews               | Both dry-runs report two allowlisted copies and leave target roots empty                                   | Installer owner         | Dry-run receipts and empty post-preview inventories                                                                                             |
| PW-4 | Validate behavioral equivalence           | Shared positive, negative, and boundary triggers produce equivalent workflow behavior                      | Verification owner      | Blocked: install-contract parity is proven; host behavior needs an executable runtime or provider-backed execution that this repository forbids |
| PW-5 | Preserve provider differences             | Generic and Codex are labeled native portable projections; adapter registration remains manual             | Provider-contract owner | `comparison.html`, claim ledger, manual-review receipt                                                                                          |
| PW-6 | Prove recovery                            | Approved install can be reversed without removing unrelated user-owned files                               | Installer/safety owner  | Receipt-scoped dry-run/apply uninstall, unrelated sentinel preservation, modified-file block                                                    |

**Build order:** PW-1 → PW-2 → PW-3/PW-5 → PW-6 → PW-4.

## 4. Evidence package

- [`case-fixture.json`](case-fixture.json) records the real manifest entry,
  clean-source commit, contract expectations, and the two target projections.
- [`expected-results.json`](expected-results.json) makes the observed statuses,
  parity rule, receipt-scoped recovery, and blocked runtime boundary testable
  without claiming host registration.
- [`comparison.html`](comparison.html) is the attractive walkthrough surface;
  [`preview.svg`](preview.svg) is the deterministic thumbnail.
- [`evidence/validation-receipt.json`](evidence/validation-receipt.json) stores
  sanitized dry-run, apply, overwrite-guard, hash, inventory, and recovery
  evidence. Recovery runs use the recorded clean archive plus the local
  recovery patch hash; no unrelated checkout state is included.
- [`evidence/claim-ledger.json`](evidence/claim-ledger.json) separates what the
  package proves from runtime, host, provider, and browser claims it does not.
- [`evidence/runtime-comparison.json`](evidence/runtime-comparison.json) records
  the permitted executable probes, fixed reason codes, and the exact owner for
  the blocked host-runtime comparison.

Browser capture and video remain optional. PW-4 remains blocked until the
runtime/provider owner supplies a documented offline host adapter or separately
authorizes provider-backed execution while preserving the no-registration
boundary. The attempted probes and reason codes are recorded in
[`evidence/runtime-comparison.json`](evidence/runtime-comparison.json).
