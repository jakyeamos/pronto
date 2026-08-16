# ESLint Anti-Slop real case selection

## Headline case: a committed Pronto UI audit

AS-1 is closed with a real, exact-revision case rather than a seeded component.
The audit reads three committed Pronto components from revision
`133ff8b741a206721a94ca80e1f8fe556a8e0dc0` and runs the unmodified Anti-Slop
rule engine from revision `b32e3d9080c540097870d9b83d089e84c75765e0`.
The machine-readable source and rule-object identities live in
[`pronto-case.json`](pronto-case.json).

The case is stronger than the aspirational “three defects” storyboard because
it produces three candidates that require judgment: two are accepted and one
is rejected as a false positive. The demo should show that distinction instead
of presenting lint output as ground truth.

## Reviewed candidate set

| Candidate | Rule                         | Review                      | Why it matters                                                                                                                 |
| --------- | ---------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| AS-C1     | `require-empty-state-action` | **Reject · false positive** | “Unknown” is a pull-request push-state value, not an empty-state message. Requiring an action here would misunderstand the UI. |
| AS-C2     | `no-generic-stat-label`      | **Accept**                  | “Analytics” does not name the repository health, delivery, quality, and release signals shown below it.                        |
| AS-C3     | `no-defensive-guard-sprawl`  | **Accept**                  | The inline ranking comparator repeats null policy and obscures the sort contract that should have a name and focused proof.    |

## Demo consequence

The original target should become: run Anti-Slop on a polished, working UI
slice; review three concrete candidates; reject the false positive; repair the
two accepted findings; then prove the visible behavior remains unchanged. This
is both more credible and more instructive than arranging three guaranteed
violations in synthetic code.

The reproducible command path uses `ESLint.lintText` on exact Git objects. It
does not read the current Pronto working-tree version of `ShowcaseSurface.tsx`,
which has unrelated uncommitted work and is explicitly outside this case.

## False-positive review

AS-C1 is a genuine product gap, not a showcase inconvenience. The empty-state
rule currently treats the status value “Unknown” / “No upstream” as though it
were a local empty-state message. AS-2 must narrow that heuristic and retain
the real empty-state positives before this case can become a trustworthy live
walkthrough.

## Claim boundary

- Allowed: “The exact-revision audit reported three candidates; review accepted
  two and rejected one false positive.”
- Not allowed: “Anti-Slop found three defects.”
- Allowed after AS-2: “The refined rule no longer flags the reviewed status
  value and retains its intended empty-state coverage.”
- Not allowed before AS-3: “The accepted revisions preserve behavior.”

## Next closure

AS-2: fix the reviewed false positive and make the two accepted findings carry
plain-language rationale and safe direction in the live output. AS-3 will then
own the behavior-preserving revision and proof.
