# Installed-runtime parity

Pronto can report whether a repository's source, packaged build, installed
artifact, and running process are the same version. A repository opts in with
`.pronto/installed-runtime-parity.json` using schema
`pronto-installed-runtime-parity/v1`.

Each target declares owner-local absolute or `~/` paths for an install manifest
and a running-process manifest, plus the expected executable path. The producing
application owns both manifests. Pronto only reads bounded JSON fields and uses
`/bin/ps -p <pid> -o comm=` to confirm that the recorded PID still addresses the
declared executable. Manifests with group or other permissions are rejected;
Pronto does not execute repository-provided commands.
An optional positive `max_age_hours` adds a producer-specific freshness window;
zero or omission keeps a still-live PID/digest observation valid for long-running
services.

The install manifest (`installed-runtime-build/v1`) records the source revision,
the packaged executable SHA-256, the installed executable SHA-256, and install
time. The process manifest (`installed-runtime-process/v1`) records the source
revision, executable SHA-256 captured at process startup, PID, and start time.

Target states remain causal and actionable:

- `build_stale`: live source differs from the packaged build.
- `install_stale`: the installed executable differs from the packaged build.
- `restart_required`: the active process differs from the installed executable.
- `not_running`: the manifest is absent or the recorded PID/executable pair is
  no longer active.
- `unverifiable`: the contract, revision, manifest, digest, or timestamp cannot
  support a safe comparison.
- `current`: all four identities match and the process evidence is fresh.

The snapshot is included in repository quality detail, agent repository
summaries, the repository drawer, and remediation planning. A missing contract
is explicitly `not_applicable`; missing evidence never becomes current.
