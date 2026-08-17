# Analytics workspace

Pronto Analytics is a dedicated, local-only evidence workspace. Portfolio does
not render charts; it links to Analytics and remains focused on current
operations.

The `pronto-analytics/v2` response contains a governed metric catalog, adapted
legacy samples, repository series, deterministic findings, and local saved
views. A metric definition records its unit, denominator, scope, time
semantics, window, aggregation, polarity, source, freshness contract, and
allowed visualizations. Two metrics may share an axis only when unit,
denominator, aggregation, time semantics, and window are identical. Dual-axis
charts and arbitrary formulas are unsupported.

Use a bounded query range:

```sh
pnpm --silent run cli analytics --range-days 30 --json
```

The requested range defaults to 30 days and is capped by configured retention.
Existing v1 sample rows remain untouched and are adapted into governed metric
observations when read.

The curated workspace separates commit volume from branch divergence and also
covers dirty and unsynced workspace friction, quality and evidence cohorts,
findings evidence coverage, finding severity, repository-by-gate coverage,
release readiness, and remediation backlog and progress. Findings evidence
coverage compares repositories with a detector-level findings source against
repositories where a count is unavailable; an evidenced zero remains distinct
from missing evidence. Metrics introduced after a historical observation are
derived from the stored sample when possible, and otherwise shown as
unavailable rather than being backfilled as zero. The observed-findings rail is
omitted when the selected range contains no deterministic findings.

Saved views are stored only in Pronto's SQLite database:

```sh
pnpm --silent run cli analytics view list --json
pnpm --silent run cli analytics view save --config-json @view.json --json
pnpm --silent run cli analytics view default <view-id> --json
pnpm --silent run cli analytics view delete <view-id> --json
```

The built-in `curated` view cannot be overwritten or deleted. Remote-provider
data must remain timestamped and labeled; it is not silently merged into local
refresh history. Findings describe observed state, change, freshness, or
coverage only and do not claim an unproven cause.
