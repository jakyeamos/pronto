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

Quality analytics keeps detector evidence and maturity evidence on separate
metrics. `findings.detector_total`, `findings.detector_actionable`, and
`findings.detector_unreviewed` describe code-detector findings; `maturity.gaps`
describes maturity gaps and must not be added to those counts. A material
change to imported detector or maturity evidence creates a new deduplicated
observation even inside the normal five-minute refresh window. An unchanged
imported evidence fingerprint deduplicates normally, and blocked or
refresh-required detector evidence remains unavailable for current-count and
delta claims.

`quality refresh` records the imported maturity feed and the latest detector
ledger in the same persisted snapshot before sampling analytics. The sample
fingerprint therefore creates one new deduplicated observation when either
feed materially changes and deduplicates an unchanged rerun.

The curated workspace separates commit volume from branch divergence and also
covers workspace activity composition, quality and evidence cohorts, finding
severity, repository-by-gate coverage, release readiness, and remediation
backlog and progress. Metrics introduced after a historical observation are
shown as unavailable for that observation rather than being backfilled as
zero.

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
