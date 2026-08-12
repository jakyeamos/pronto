# Promotion inbox

The Promotion inbox tab is Pronto's local review surface for the
`ai-workflow-leverage` → `jakyeamos-agentic-setup` handoff.

Pronto invokes the fixed local AWL checkout at
`~/projects/ai-workflow-leverage` and reads the private
`leverage-promotion-inbox/v1` projection. The tab can record one of five owner
decisions—`defer`, `reject`, `public`, `private`, or `both`—as an append-only
AWL decision artifact.

AWL owns discovery, forward testing, quantification, and complete candidate
formation. The tab only reads the resulting evidence and records the owner's
choice; it does not create test packets, run tests, calculate measurements, or
assemble candidates.

The tab is the owner-controlled admission boundary. For an accepted complete
candidate, it invokes JAS's explicit `apply` command with a disposable approval
artifact, the selected mode, and the candidate's sanitized projection. JAS
validates, preflights, and applies the public catalog and/or private overlay;
the tab then records a sanitized admission receipt back in AWL. A later refresh
shows `JAS_APPLIED`, `JAS_ALREADY_APPLIED`, or a blocked admission on the
candidate itself. Pronto enables `public`, `private`, and `both` only when the
selected candidate is complete and its sanitized JAS projection is ready.
Drafts and missing projections keep those acceptance controls disabled while
leaving `defer` and `reject` available. They never mutate JAS. If AWL or JAS is
missing or cannot execute, the tab reports that state and does not invent
candidates or decisions.
