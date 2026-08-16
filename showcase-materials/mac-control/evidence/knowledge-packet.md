# Knowledge Packet — Mac Control Showcase

## Task

- Output type: feature explainer and capture-ready Showcase packet
- Audience: technical recruiters, hiring managers, and AI product builders
- Domain: technical

## Non-negotiables

Claims allowed:

- Mac Control is a command-first, local macOS control plane with a per-user daemon and owner-only Unix socket. `[fact-001]`
- It exposes typed agent-control capabilities and an explicit approval, execution, verification, and redacted-evidence model. `[fact-002, fact-003]`
- One project-authored System Settings benchmark reports a 134.924 ms median across 3/3 passing samples. `[fact-004]`
- Finder scroll comparisons remained blocked and insufficient. `[fact-005]`
- The installed release check observed on August 12, 2026 did not pass. `[fact-006]`

Claims forbidden:

- Production ready, release ready, universally faster, or safer than every alternative.
- Independently benchmarked, adopted by users, deployed publicly, or proven across all macOS apps.
- A current live demo, public URL, or finished capture exists.

Required uncertainty:

- Attribute the numeric result to the repository's named benchmark and preserve its task and sample size.
- State that current live release evidence is incomplete.
- Treat the pending visual and recording assets as capture work, not completed proof.

## Facts and metrics

The project contract and installed capability report agree on the central mechanism: a CLI sends typed requests to a per-user daemon over an owner-only local socket. The daemon owns the native GUI session and the safety and evidence boundaries. `[fact-001, fact-002, fact-003]`

The strongest bounded performance example is the Phase 2 System Settings focus comparison. The Mac Control lane reports 134.924 ms at 3/3 verified samples; direct System Events reports 479.491 ms; generic GUI control reports 11814.550 ms. This is one named internal comparison, not a general speed ranking. `[fact-004]`

The same report preserves negative evidence. Finder scroll lanes were blocked before a comparable passing result. The current installed release check also failed closed, with one failed storage check and eight blocked checks. `[fact-005, fact-006]`

## Concept

Failure-mode first: explain what blocks and why before describing the happy path as universal. The blocked release check and insufficient Finder comparison are part of the product story because the system is designed not to turn missing evidence into success.

## Template structure

Problem → user → behavior → architecture sketch → limits and risks → what not to claim.

## Style

Direct, analytical, and specific. Avoid hype, architecture adjectives without named components, and latency claims without conditions. One clear idea per sentence.

## Open questions

- Approved public no-auth host
- Approved public repository URL
- Fresh real capture of authorization and verified outcome
