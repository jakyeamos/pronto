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
assemble candidates. The overview distinguishes the number awaiting an owner
decision from the total candidate inventory (for example, `14 awaiting
decision · 62 total candidates`). When AWL supplies its funnel projection, the
coverage panel separately shows the upstream evaluation drafts → behavior
identities → forward-test work items → review packets counts. Review packets
remain outside the candidate inventory until quantification and owner review
pass.

The Evaluation pipeline panel keeps the upstream funnel separate from the
promotion queue. `evaluation_candidate_drafts` is a count of source rows, not
pending promotions. Behavior identities and forward-test work items are
intermediate AWL stages. Tests completed, blocked evaluations, quantification
pending, review packets, and candidates formed are displayed as separate
counts.

AWL may also return incomplete candidate records and historical decisions in
the same private projection for provenance. Pronto partitions those records:
only undecided `complete` candidates appear in the Promotion queue; incomplete
candidate drafts appear in the AWL candidate pipeline; accepted, deferred, and
rejected records appear in read-only Decision history. Only a complete,
undecided candidate can receive an owner decision or invoke JAS from this
surface, and the native decision bridge rejects non-complete or already-decided
records as well.

The tab is the owner-controlled admission boundary. For an accepted complete
candidate, it invokes JAS's explicit `apply` command with a disposable approval
artifact, the selected mode, and the candidate's sanitized projection. JAS
validates, preflights, and applies the public catalog and/or private overlay;
the tab then records a sanitized admission receipt back in AWL. A later refresh
shows `JAS_APPLIED`, `JAS_ALREADY_APPLIED`, or a blocked admission on the
candidate itself. Pronto enables `public`, `private`, and `both` only when the
selected candidate is complete and its sanitized JAS projection is ready. A
complete candidate with a missing projection remains visible but cannot be
promoted until the projection is ready. Drafts and historical decisions have
no owner decision controls. If AWL or JAS is missing or cannot execute, the
tab reports that state and does not invent candidates or decisions.
