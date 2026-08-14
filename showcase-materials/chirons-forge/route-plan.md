# Chiron's Forge Showcase route plan

Status: steps 1–3 are complete as an aspirational specification. The public
surface is live, but CF-0 is explicitly blocked: the backing repository and
deployed revision are not available in the local portfolio or disclosed by the
public surface. An authenticated end-to-end build has not been verified.

Canonical target: [Chiron's Forge ideal demo](../ideal-demo-targets.md#chirons-forge)

## 1. Ideal demo target

### North star

A user scopes a real domain need, optionally grounds it with sources or data,
several research models investigate in parallel, a different model judges and
drives refinement, and the user downloads a portable expert artifact.

### Non-negotiable proof

The demo must expose a real judge receipt and the finished artifact. A landing
page promise, concept animation, or model-name montage does not prove the
multi-model judge-and-refine loop.

## 2. Attractive concept materials

### Six-frame storyboard

1. **Real brief:** show the domain need, output type, and success criteria.
2. **Grounding:** attach a public-safe source or data packet and show its scope.
3. **Parallel research:** reveal distinct engines investigating complementary
   questions without implying that activity alone is quality.
4. **Judge and refine:** show the independent scorecard, one material gap, and
   the refinement it triggers.
5. **Selection:** compare the initial and refined candidates against the stated
   criteria.
6. **Portable result:** download the finished SKILL.md, report, or Cursor rule
   with source and judge provenance attached.

### Preview concept

Use a crop-safe 16:9 composition centered on the independent judge scorecard
and a visible refinement delta, with the downloadable artifact as the outcome.
Avoid a generic wall of model logos.

### Narrative spine

“Research from several models is only raw material. Chiron's Forge makes the
evaluation and refinement loop inspectable, then ships the result as something
another human or agent can actually use.”

## 3. Gap-closure route

### Reviewed baseline

The live public site presents authentication, per-build pricing, three output
families, optional grounding uploads, and an independent-judge promise. The
backing repository is not registered in Pronto, and no authenticated build,
judge trace, output download, deployment revision, or deletion receipt has been
verified.

Project disposition: `targeted_gap_closure`

Gap classes: evidence — CF-0, CF-3, CF-5; content — CF-1; demo_integration — CF-2, CF-4; packaging — CF-6.

| ID   | Class            | Closure target                                                                      | Completion evidence                                                                                                                                                     |
| ---- | ---------------- | ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CF-0 | evidence         | Resolve the backing repository and deployment provenance.                           | The repository is registered and the live build maps to an exact reviewed revision, or the owner records the precise access blocker.                                    |
| CF-1 | content          | Select one real representative case and define success before running it.           | A public-safe brief identifies the audience, input packet, output type, expected improvement, and claims that must remain bounded.                                      |
| CF-2 | demo_integration | Execute the representative case through an authenticated end-to-end build.          | The current product accepts the brief and grounding material, completes the run, and produces the intended artifact without a manual substitute.                        |
| CF-3 | evidence         | Prove independent judging and refinement.                                           | A receipt names the candidate and judge roles, shows scores or findings, records at least one material refinement, and does not infer independence from marketing copy. |
| CF-4 | demo_integration | Make the artifact, source trail, and judge trail inspectable in a public-safe flow. | The demo can open or download the output and reach its bounded provenance without exposing private inputs or requiring narration to fill missing behavior.              |
| CF-5 | evidence         | Verify the applicable privacy, redaction, and deletion boundary.                    | A current-build receipt or direct readback demonstrates the promised behavior for the representative input; unverified policy claims remain labeled.                    |
| CF-6 | packaging        | Package the proven case for no-auth viewing.                                        | A responsive case-study page, crop-safe preview, concise description, and linked proof agree on the same verified claims; video is optional.                            |

### Build order

CF-0 → CF-1 → CF-2 and CF-3 → CF-4 and CF-5 → CF-6.

## 4. CF-0 provenance disposition

CF-0 was attempted against the owner-visible portfolio and the public surface.
No matching `/Users/jakyeamos/projects` checkout or Pronto route registration
was found. The public page returned successfully and exposes the product
promise, but its HTML and loaded bundles disclose neither a source repository
URL nor an immutable deployed revision. The exact access and provenance record
is [evidence/cf-0-blocker.json](evidence/cf-0-blocker.json).

This is a genuine provenance boundary, not a reason to infer behavior from the
landing page or to fabricate a repository. CF-1 through CF-6 remain open, and
the queue should move to the next runnable project until the owner supplies the
missing source and deployment identity.

### Next closure when unblocked

Return to CF-0: identify the backing repository and bind the live deployment to
an exact reviewed revision before product or demo claims are strengthened.
