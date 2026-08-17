# AI Code Quality Stack: one finding, three responsibilities

## The case

The example starts with a real Pronto source finding:

```tsx
// src/renderer/src/components/RepositoryAnalyticsPanel.tsx:39
<h2>Analytics</h2>
```

Anti-Slop reports `anti-slop/no-generic-stat-label` because the label repeats
the surrounding navigation concept instead of naming the repository signals in
the panel. Human review accepted the finding and recorded a safer direction:
`Repository trends` or `Repository health and delivery`.

The point of this combined story is what happens next. The three products keep
different jobs:

1. **Detect — ESLint Anti-Slop.** The plugin owns the AST rule and the exact
   line-level diagnostic. A candidate is still a candidate until a person
   reviews it.
2. **Enforce — Pre-CR.** Pre-CR runs the required adapter against the changed
   file set and writes a blocking receipt before review. A passing receipt is
   not merge approval.
3. **Contextualize — Quality Runner.** Quality Runner consumes the canonical
   evidence, adds repository context, and orders remediation. Its advisory text
   fallback is not equivalent to the AST diagnostic.

## What is proven and what is not

The detector case, exact source identity, ownership contract, and fallback
states are recorded locally. The Pre-CR execution and Quality Runner
consumption are the next integration gates; they are deliberately not filled
with a synthetic success.

The eventual proof should show one finding moving through the stack exactly
once. It should also show the uncomfortable paths: the plugin is unavailable,
analysis fails, the advisory fallback runs, or both sources report the same
problem. Each state must remain distinguishable.

The compact synthetic fixture can remain as a reproducibility appendix. The
headline case stays tied to the real Pronto source object so the demo has a
specific line, rule, review decision, and safe direction.

## Current material boundary

This local packet contains the case-study copy, claim ledger, candidate 16:9
preview, and no-auth page source. It is a packaging candidate, not a hosted
case or a cross-repository execution receipt. The combined story also does not
replace the standalone Pre-CR IDE demo or the standalone Quality Runner case.
