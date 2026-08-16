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

`quality refresh --json` records an Analytics observation only after Pronto
accepts and persists the canonical Quality Runner audit. The existing sample
fingerprint suppresses an unchanged repeat inside the deduplication window;
changed accepted evidence appends a new observation, while rejected or
unavailable evidence appends nothing.

The curated workspace separates commit volume from branch divergence and also
covers workspace activity composition, quality and evidence cohorts, finding
severity, repository-by-gate coverage, release readiness, and remediation
backlog and progress. Metrics introduced after a historical observation are
shown as unavailable for that observation rather than being backfilled as
zero.

Workspace activity reserves `Unknown` for incomplete or uncertain process
inspection. A clean, synchronized workspace with a completed inspection and no
associated process is `Idle`; dirty or unpublished work remains `Interrupted`.

Quality Runner findings are deduplicated by stable fingerprint across the
recognized reports in an accepted run. The fingerprinted code-quality scan
remains the preferred report link, but complementary audit findings still
contribute to the total and severity breakdown, so critical and high evidence
cannot disappear merely because the preferred report contains only medium or
observational findings.

Charts use one UTC time contract. Trend and composition views retain the latest
observation for each UTC day, plot those observations on a time scale, and show
full UTC timestamps in tooltips. Metric labels and tooltips round displayed
values to at most two decimal places; repository comparison tooltips always
name the repository.

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
