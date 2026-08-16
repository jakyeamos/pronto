# Research Domain Writing case selection

## Headline case: a real Tatum line that cannot prove its best-sounding claim

The hero case uses Jayson Tatum's real 2023–24 regular-season line, not a
fabricated writing sample. Boston Celtics game notes and Basketball-Reference
independently record 74 games with averages of 26.9 points, 8.1 rebounds, and
4.9 assists. The source packet deliberately stops there.

That makes the tempting sentence realistic: “Those numbers prove he was an
elite two-way engine.” It is polished, plausible, and too strong. Points,
rebounds, and assists establish box-score volume; this packet contains no
efficiency, matchup, tracking, on/off, or all-in-one impact evidence. It cannot
support “elite,” “two-way,” or “prove.”

## One-sentence consequence

Research Domain Writing should preserve the useful sentence while rejecting
the unsupported leap: fluency survives, but certainty cannot outrun the packet.

## Target demo

1. Open the two-source [research packet](source-packet.yaml) and show the
   supported season line.
2. Select the sentence in the [unsafe draft](unsafe-draft.md).
3. Reveal its claim trace: the four numbers map to `fact-tatum-line`; “elite
   two-way engine” maps to no fact.
4. Run Domain QA and stop on the missing evidence instead of failing the entire
   writing task.
5. Repair the sentence to: “Across 74 regular-season games in 2023–24, Tatum
   averaged 26.9 points, 8.1 rebounds, and 4.9 assists per game. Those figures
   describe recorded scoring, rebounding, and assist volume; they do not
   measure efficiency, defensive impact, or total player value.”
6. End on the claim ledger and the still-required human publication decision.

## Proof boundary

The source packet and research-readiness projection pass. A fresh Domain QA
artifact rejects the unsupported inference with one major issue, and RDW's
deterministic gate validates its exact binding to the unsafe draft. The repaired
draft and byte-identical style-pass final also pass exact-draft claim-ledger
validation.

That does not mean the CLI inferred the semantic problem autonomously: the agent
performed Domain QA, while the deterministic gate verified the receipt's draft
hash, coverage, fact links, and coherent pass/fail state. The installed CLI now
validates its basketball pack as `specialized/production`, and the installed
domain contract is byte-identical to repository revision `aa46b36`. That does
not bind the wheel to that revision: the wheel embeds no source hash, its
recorded build worktree is gone, and the active source checkout differs.
Release provenance therefore remains open. The visual claim trace is assembled
locally, but no-auth hosting is not verified.

The repository already contains related Jayson Tatum acceptance fixtures. They
are implementation evidence and a reproducibility aid; they are not a
substitute for rerunning this evidence-v1 packet on the selected source revision.

## Reproducibility appendix

Keep the repository's deterministic `basketball-vertical-slice` fixture as a
short appendix. It makes the workflow replayable offline, while the Tatum case
remains the public-facing example that explains why the safety boundary matters.
