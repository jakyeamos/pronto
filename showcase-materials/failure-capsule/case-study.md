# Failure Capsule: make the failure portable

Failure Capsule is a bounded handoff for a failed declared command. It records
what failed and where, removes secret-like values before persistence, names what
was intentionally omitted, and keeps reproduction separate from inspection.

## The current dev case

At the current `dev` head (`516f24f`), an isolated declared command exits with
code `7` and writes a token-like value. Failure Capsule stores
`TOKEN=[REDACTED]`, preserves the failure outcome and Git target, records the
allowlisted report by size and hash, and labels environment, credentials, and
unlisted files as omitted. The capsule opens as inspectable; opening it does
not run the command. Repository tests, lint, and packaging also pass.

The product story is: **failure → bounded capture → redact/label omissions →
inspect → explicit recovery**.

## Evidence boundary

The current-dev checkpoint is a local run from source revision `516f24f`. It
proves the bounded capture slice and binary preview, not Replay handoff,
complete cancellation/recovery coverage, hosting, or publication.
