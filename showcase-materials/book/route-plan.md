# Book route plan

Status: steps 1–3 are specified. The local reader seam is documented, and a
rights-safe synthetic appendix is available. BK-1 is blocked because the
checkout does not carry the chapter, soundtrack, font, and AI/human
attribution records needed for a public case study.

The [canonical target](../ideal-demo-targets.md#book) owns the durable promise
and proof gate.

## 1. Ideal target

**North star:** one creator-owned chapter moves from a quiet reading surface
into a carefully timed audiovisual passage, then returns control to the reader
through mute, reduced-motion, and navigation choices without losing the prose.

**Non-negotiable:** the writing remains primary. Authorship, creative direction,
AI assistance, implementation, and asset ownership must be stated separately.

## 2. Concept materials

All frames are **concept** until rights, synchronization, and accessibility pass.

| Frame             | Visual                                                                         | On-screen line                              | Intended evidence moment             |
| ----------------- | ------------------------------------------------------------------------------ | ------------------------------------------- | ------------------------------------ |
| 1. Quiet page     | Beautiful typography and a restrained chapter opening                          | “Begin with the words”                      | Product quality is immediate         |
| 2. Threshold      | A specific sentence approaches while motion and sound cues remain subtle       | “Let the scene gather around the text”      | Enhancement follows narrative intent |
| 3. Transformation | Layered illustration, motion, and audio peak around one authored moment        | “A chapter can become a place”              | Creative payoff is visible           |
| 4. Reader control | Mute and reduced-motion controls change the same scene gracefully              | “Immersion stays optional”                  | Accessibility is functional          |
| 5. Authoring view | Chapter manifest connects passage, timing, layers, and controls                | “Designed as a system—not a one-off effect” | Content tooling becomes legible      |
| 6. Ownership      | Credits separate writing, direction, implementation, assets, and AI assistance | “Authorship stays attributable”             | Contribution boundary is explicit    |

**Preview concept.** A luminous passage at the moment motion enters, framed by
subtle audio-layer and reduced-motion controls. Headline: “A reading experience
where motion and sound follow the prose.”

**Narrative spine.** Quiet reading → narrative threshold → audiovisual payoff →
reader control → authoring system → ownership.

## 3. Build-gap specification

Reviewed baseline: the repository documents a working reader, admin editor,
manifest-driven chapters, motion, layered audio, persistence, and smoke path;
hosted behavior and showcase assets are unproven.

Project disposition: `largely_product_ready` — use the existing reader and
authoring model; close chapter-content, direct-surface proof, rights, and public
packaging gaps.

Gap classes: evidence — BK-1; content — BK-2; product — BK-3, BK-4;
packaging — BK-5, BK-6.

| ID   | Gap to close                                  | Observable acceptance condition                                                                                          | Owner                     | Required proof                                 |
| ---- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------- | ---------------------------------------------- |
| BK-1 | Select and clear one representative chapter   | Text, imagery, audio, fonts, and any generated assets have explicit showcase rights and credits                          | Creative/content owner    | Rights and attribution ledger                  |
| BK-2 | Author the complete transformation arc        | The selected passage has intentional entry, peak, exit, and fallback behavior tied to narrative beats                    | Creative/product owner    | Chapter manifest and design review             |
| BK-3 | Stabilize media synchronization               | Motion, audio layers, text position, and navigation remain aligned across a fresh desktop session and supported viewport | Frontend/audio owner      | Timed run and state assertions                 |
| BK-4 | Prove reader control and accessibility        | Mute, reduced motion, keyboard navigation, focus, and readable fallback work during the same scene                       | Accessibility owner       | Direct-surface audit and capture               |
| BK-5 | Expose the authoring model without admin risk | A sanitized static view explains the manifest-to-scene system without requiring public admin access                      | Product/showcase owner    | Case-study authoring frame and security review |
| BK-6 | Establish a public proof surface              | A no-auth reader or case study loads reliably and identifies hosted versus local evidence                                | Deployment/showcase owner | Link readback and environment identity         |

**Build order:** BK-1 → BK-2 → BK-3/BK-4 → BK-5 → BK-6.

## 4. BK-1 evidence boundary

The live repository is route-ready and clean at the reviewed `dev` checkout.
Chapter 1 (`The Ritual`) is a reasonable candidate because it exercises the
manifest, chapter-specific themes, line-aware audio cues, and reader controls.
It is not yet a cleared showcase asset. The source contains no explicit
chapter authorship/permission statement, no track-level public-use record for
the three referenced MP3 files, no pinned font provenance note for the
external Google Fonts import, and no task-level AI-versus-human contribution
statement.

The detailed boundary is recorded in
[`contribution-ledger.json`](contribution-ledger.json), and the blocked proof
receipt is [`evidence/bk-1-blocker.json`](evidence/bk-1-blocker.json). Until an
owner-approved ledger exists, do not publish the real prose, soundtrack-backed
capture, or an unqualified authorship/AI claim. A sensitive-content review is
also required before selecting a real excerpt for public capture.

## 5. Rights-safe appendix

[`synthetic-fixture.json`](synthetic-fixture.json) specifies a small original
chapter, four narrative beats, synthetic audio tokens, gradient/particle
visuals, and reader controls. It is an appendix for explaining the intended
transformation arc while BK-1 is pending—not a substitute for the product
chapter and not proof of synchronization, hosting, or accessibility behavior.
Keep the synthetic label visible in every preview or walkthrough that uses it.

[`synthetic-preview.html`](synthetic-preview.html) is now the shareable no-auth
surface for that appendix. It makes the prose, media timing, reader controls,
and evidence boundary legible without importing repository text, external
fonts, soundtrack files, provider output, or runtime claims. The
[`bk-1-synthetic-material-receipt.json`](evidence/bk-1-synthetic-material-receipt.json)
records a static HTTP fetch only; this material addition does not clear BK-1.

## 6. Local showcase package

The local candidate package is [`case-study.json`](case-study.json), with the
long-form narrative in [`case-study.md`](case-study.md), the bounded claims in
[`claim-ledger.json`](claim-ledger.json), and the responsive no-auth source at
[`public/index.html`](public/index.html). The 16:9 binary is
[`assets/preview-16x9.png`](assets/preview-16x9.png), with the editable source
in [`assets/preview-16x9.svg`](assets/preview-16x9.svg); the checkpoint is
[`evidence/bk-6-material-checkpoint.json`](evidence/bk-6-material-checkpoint.json).

This package uses the rights-safe synthetic chapter **The Signal Room**. It
makes the four-beat reading-to-media arc and reader-control contract shareable,
but it does not close BK-1 chapter/asset rights, AI-versus-human attribution,
BK-2 authoring, BK-3 synchronization, BK-4 accessibility, hosted no-auth
verification, or external destination readbacks. It is a candidate local
material, not product authorship or a public chapter release.
