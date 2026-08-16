# Quality Setup: every setup has a way back

Quality Setup is a local, preview-first setup inspector for the first moment a
repository needs quality tooling. It makes the setup itself inspectable: what
the ecosystem supports, what file will change, who authorizes it, how the
result is verified, and how to recover.

## The local W1 case

An isolated Node repository with `package.json` and `pnpm-lock.yaml` is
recognized as supported. The plan proposes one repository-owned config with
`pnpm test` as its quality command and marks the change reversible.

Apply writes only the owned config and a rollback receipt. Verify passes. A
second plan refuses to overwrite the existing config. Rollback removes only the
owned config and leaves the target supported.

The product story is the sequence: **inspect → preview → apply → verify →
refuse conflict → roll back**. Setup is not complete just because a command
was dispatched.

## Evidence boundary

The current dev checkpoint is `f153fff78575d17eb90fc37174ac4edbe02d73e1` on a
clean tree. Repository tests, lint, and package checks pass. The scenario is an
isolated fixture run on target revision `6c15ea4`; it proves the W1 enabler
contract, not a real quality-command smoke, broad ecosystem support,
installation into a user's repository, hosting, or publication. The 1600×900
preview is available in `assets/preview-16x9.png` with the SVG source beside it.
