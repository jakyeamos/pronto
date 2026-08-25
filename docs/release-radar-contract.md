# Release Radar and visibility-driven profile synchronization

Release Radar is the full-fleet distribution projection. It is not a second
readiness authority and it does not author Showcase materials.

## Projection contract

Pronto emits `pronto-release-radar/v1` in the normal `PortfolioSnapshot`. The
projection forms a deterministic union of registered local repositories and the
fresh GitHub provider repository snapshot, joining rows by normalized GitHub
provider identity. It preserves local-only, GitHub-only, joined, private,
internal, unknown, stale, and blocked rows.

Each row exposes separate dimensions:

- provider visibility and freshness;
- public-framing eligibility;
- profile README link synchronization;
- product readiness from the existing repository/Showcase evidence;
- demo-material readiness from the existing Showcase evidence;
- release readiness from existing release evidence;
- next action and blocker.

Public framing is eligible only when provider visibility is public and fresh and
the selected profile metadata is safe. It does not imply product readiness,
created demo materials, or release readiness. In particular, AIOS can be public
and framing-eligible while retaining its existing `not_applicable` Showcase
eligibility and missing demo-material evidence.

## Visibility control

The visibility action is an explicit provider write from the Release Radar or
Remote catalog surface. It must perform the following sequence for the exact
repository named by the user:

1. record a local `Started` audit;
2. perform a fresh GitHub control read and confirm normalized identity;
3. reject archived repositories, require `permissions.admin=true`, and reject
   no-op targets;
4. record `Preflighted`, then call `gh repo edit` with
   `--accept-visibility-change-consequences`;
5. perform a matching post-write control read;
6. update the local remote snapshot and record `Completed` only after readback
   matches. Failures remain `Unknown` or rejected audit evidence and do not
   trigger profile projection.

Authenticated `gh` access is not mutation authority. The UI requires the exact
`owner/name` confirmation and the provider command remains the credential
boundary.

## Profile publication boundary

The profile repository adapter lives in
`/Users/jakyeamos/projects/jakyeamos-profile/scripts/sync_profile_repository.py`.
It runs the existing catalog refresh, README generator, and validators in a
detached clean worktree from `origin/main`, restricted to the generated
profile repository's `main` branch. It permits only `profile-catalog.json` and
`README.md` output, rejects changes outside the selected repository except for
generated observation metadata, and checks the expected selected link state.

The adapter does not change GitHub visibility. A publication push requires the
explicit `PUBLISH PROFILE README` confirmation. After a successful push it reads
the hosted profile README back and matches the generated marker plus the
selected link/suppression state.

| Confirmed provider state                | Profile behavior                                                                               |
| --------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Public and fresh                        | Public URL may be rendered after safe metadata checks and hosted readback.                     |
| Public but stale                        | Remains visible as stale; no automatic profile sync.                                           |
| Private or internal and fresh           | Public URL and demo/public framing links are suppressed; policy-allowed safe label may remain. |
| Unknown, blocked, failed, or mismatched | Leave the profile README unchanged and expose the blocker.                                     |

The profile catalog never stores local filesystem paths or private remote URLs.
Unknown provider evidence is not treated as private evidence.
