# Workflow Gateboard: see what will run

Workflow Gateboard is a human-readable projection of a repository's declared
quality contract. It shows dependencies and mutation class before a command is
requested, refuses a dependent run whose prerequisite has not passed, and
leaves a bounded receipt for the next person.

## The local W2 case

The manifest declares `syntax`, `tests`, and `package` as read-only gates. On
the current dev head, `syntax` writes a target-bound receipt while separate
`tests` and `package` invocations remain blocked because the passed prerequisite
is not yet persisted across the CLI boundary. The historical W2 fixture records
the intended full pass chain at `4ce8f60`; the current repository ref remains
unchanged and the receipt is the expected evidence artifact.

The product story is: **declare → inspect → preview → run → receipt →
continue**. Inspection never executes a gate.

## Evidence boundary

The historical matrix is a local CLI run from source revision `4ce8f60`. The
current-dev checkpoint at `f160831` adds fresh checks, declaration proof, syntax
receipt, and the explicit persistence gap. Together they prove the W2
declared-gate boundary, not a persisted pass sequence, hosted CI parity, Flight
Recorder integration, concurrent refresh recovery, or publication.
