import { useState, type ReactElement } from "react";
import type { AnalyticsSnapshot, MetricDefinition } from "../types";
import {
  DivergenceChart,
  EvidenceHeatmap,
  GovernedBars,
  GovernedTrend,
  QualityCoverageScatter,
} from "./AnalyticsCharts";

function latest<T>(items: T[]): T | undefined {
  return items.at(-1);
}
function count(value: number | null | undefined): string {
  return value == null ? "Unavailable" : new Intl.NumberFormat().format(value);
}
function metric(
  catalog: MetricDefinition[],
  id: string,
): MetricDefinition | undefined {
  return catalog.find((item) => item.id === id);
}
function Card({
  eyebrow,
  title,
  description,
  children,
  wide = false,
}: {
  eyebrow: string;
  title: string;
  description: string;
  children: ReactElement;
  wide?: boolean;
}): ReactElement {
  return (
    <article
      className={
        wide
          ? "analytics-workspace-card analytics-workspace-card-wide"
          : "analytics-workspace-card"
      }
    >
      <header>
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </header>
      {children}
    </article>
  );
}

function RepositoryTable({
  analytics,
  onDrillDown,
}: {
  analytics: AnalyticsSnapshot;
  onDrillDown: (id: string) => void;
}): ReactElement {
  type SortKey =
    "repository" | "maturity" | "evidence" | "findings" | "dirty" | "sync";
  const [sortKey, setSortKey] = useState<SortKey>("findings");
  const [direction, setDirection] = useState<"ascending" | "descending">(
    "descending",
  );
  const rows = analytics.repositories.map((series) => ({
    series,
    sample: latest(series.samples),
  }));
  const value = (row: (typeof rows)[number], key: SortKey): number | string => {
    if (key === "repository") return row.series.name.toLocaleLowerCase();
    if (key === "maturity") return row.sample?.maturity_score ?? -1;
    if (key === "evidence") return row.sample?.ci_readiness_score ?? -1;
    if (key === "findings")
      return (
        row.sample?.detector_findings_total ?? row.sample?.findings_total ?? -1
      );
    if (key === "dirty") return row.sample?.dirty_workspace_count ?? -1;
    return row.sample?.unsynced_workspace_count ?? -1;
  };
  rows.sort((a, b) => {
    const left = value(a, sortKey);
    const right = value(b, sortKey);
    const result =
      typeof left === "string" && typeof right === "string"
        ? left.localeCompare(right)
        : Number(left) - Number(right);
    return direction === "ascending" ? result : -result;
  });
  const sort = (key: SortKey): void => {
    if (key === sortKey)
      setDirection((current) =>
        current === "ascending" ? "descending" : "ascending",
      );
    else {
      setSortKey(key);
      setDirection(key === "repository" ? "ascending" : "descending");
    }
  };
  const header = (label: string, key: SortKey) => (
    <th aria-sort={sortKey === key ? direction : "none"}>
      <button
        type="button"
        className="analytics-table-sort"
        onClick={() => sort(key)}
      >
        {label}
        {sortKey === key ? (direction === "ascending" ? " ↑" : " ↓") : ""}
      </button>
    </th>
  );
  return (
    <div className="analytics-table-wrap">
      <table className="analytics-table">
        <thead>
          <tr>
            {header("Repository", "repository")}
            {header("Maturity", "maturity")}
            {header("Evidence", "evidence")}
            {header("Detector findings", "findings")}
            {header("Dirty", "dirty")}
            {header("Sync", "sync")}
          </tr>
        </thead>
        <tbody>
          {rows.map(({ series, sample }) => (
            <tr key={series.repository_id}>
              <th>
                <button
                  type="button"
                  className="analytics-table-link"
                  onClick={() => onDrillDown(series.repository_id)}
                >
                  {series.name}
                </button>
              </th>
              <td>{count(sample?.maturity_score)}</td>
              <td>{count(sample?.ci_readiness_score)}</td>
              <td>
                {count(
                  sample?.detector_findings_total ?? sample?.findings_total,
                )}
              </td>
              <td>{count(sample?.dirty_workspace_count)}</td>
              <td>{count(sample?.unsynced_workspace_count)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function AnalyticsCuratedWorkspace({
  analytics,
  repositoryFilter,
  onDrillDown,
}: {
  analytics: AnalyticsSnapshot;
  repositoryFilter: string;
  onDrillDown: (id: string) => void;
}): ReactElement {
  const samples = analytics.portfolio_samples;
  const findings = analytics.findings ?? [];
  const latestSample = latest(samples);
  const catalog = analytics.metric_catalog ?? [];
  const repositories =
    repositoryFilter === "all"
      ? analytics.repositories
      : analytics.repositories.filter(
          (series) => series.repository_id === repositoryFilter,
        );
  const pick = (...ids: string[]) =>
    ids
      .map((id) => metric(catalog, id))
      .filter((item): item is MetricDefinition => Boolean(item));
  const trendCards = [
    [
      "Fleet friction",
      "Workspace friction",
      "Dirty and unsynced workspace counts share the same scope, denominator, aggregation, and time semantics.",
      ["workspaces.dirty", "workspaces.unsynced"],
      "Workspace friction over time",
    ],
    [
      "Operational conditions",
      "Active conditions",
      "Open operational conditions remain on their own governed scale.",
      ["conditions.active"],
      "Active operational conditions over time",
    ],
    [
      "Release readiness",
      "Configured versus ready",
      "Ready repositories are compared only with repositories that have configured local release rules.",
      ["release.configured_repositories", "release.ready_repositories"],
      "Configured and release-ready repositories over time",
    ],
    [
      "Remediation progress",
      "Verified eligible weight",
      "Progress is verified action weight divided by non-deferred remediation weight; unavailable history is not rendered as zero.",
      ["remediation.progress_percent"],
      "Remediation progress over time",
    ],
    [
      "Finding trend",
      "Detector finding severity over time",
      "Raw detector findings and high-severity detector findings share source, denominator, aggregation, and time semantics; maturity gaps remain a separate metric.",
      ["findings.detector_total", "findings.high_severity"],
      "Finding severity trend",
    ],
  ] as const;
  return (
    <>
      <aside
        className="analytics-findings"
        aria-label="Deterministic analytics findings"
      >
        <div className="analytics-findings-heading">
          <div>
            <p className="eyebrow">Observed findings</p>
            <h3>What changed or needs evidence</h3>
          </div>
          <span>{findings.length}</span>
        </div>
        {findings.length ? (
          <ol>
            {findings.map((finding) => (
              <li
                key={finding.id}
                className={`analytics-finding analytics-finding-${finding.severity}`}
              >
                <span>{finding.kind.replace("-", " ")}</span>
                <strong>{finding.title}</strong>
                <p>{finding.detail}</p>
              </li>
            ))}
          </ol>
        ) : (
          <div className="analytics-state">
            No deterministic finding was generated for this range.
          </div>
        )}
      </aside>
      <div className="analytics-kpi-grid">
        <div>
          <span>Refresh observations</span>
          <strong>{samples.length}</strong>
          <small>{analytics.range_days}-day query</small>
        </div>
        <div>
          <span>Local commits</span>
          <strong>{count(latestSample?.commits_last_30_days)}</strong>
          <small>Trailing 30-day window</small>
        </div>
        <div>
          <span>High-severity detector findings</span>
          <strong>{count(latestSample?.high_severity_findings)}</strong>
          <small>
            {latestSample?.quality_freshness ?? "Evidence unavailable"}
          </small>
        </div>
        <div>
          <span>Release ready</span>
          <strong>
            {latestSample
              ? `${latestSample.release_ready_repository_count}/${latestSample.release_rule_repository_count}`
              : "Unavailable"}
          </strong>
          <small>Configured rules only</small>
        </div>
      </div>
      <div className="analytics-workspace-grid">
        <Card
          eyebrow="Delivery activity"
          title="Commit activity"
          description="Commit volume uses its own trailing-window scale. Branch divergence is never mixed onto this axis."
          wide
        >
          <GovernedTrend
            samples={samples}
            metrics={pick("git.commits.trailing_30_days")}
            ariaLabel="Commit activity over time"
          />
        </Card>
        <Card
          eyebrow="Branch state"
          title="Ahead and behind"
          description="Diverging bars compare point-in-time branch counts around a shared zero baseline."
          wide
        >
          <DivergenceChart repositories={repositories} />
        </Card>
        <Card
          eyebrow="Cohort posture"
          title="Quality × evidence coverage"
          description="Median lines create cohort-relative quadrants. Missing scores are excluded and remain visible in findings."
          wide
        >
          <QualityCoverageScatter repositories={repositories} />
        </Card>
        {trendCards.map(([eyebrow, title, description, ids, label]) => (
          <Card
            key={title}
            eyebrow={eyebrow}
            title={title}
            description={description}
          >
            <GovernedTrend
              samples={samples}
              metrics={pick(...ids)}
              ariaLabel={label}
            />
          </Card>
        ))}
        <Card
          eyebrow="Remediation backlog"
          title="Action composition"
          description="Open, in-progress, blocked, deferred, and verified actions remain evidence-bound status counts."
        >
          <GovernedBars
            samples={samples}
            metrics={pick(
              "remediation.actions.open",
              "remediation.actions.in_progress",
              "remediation.actions.blocked",
              "remediation.actions.deferred",
              "remediation.actions.verified",
            )}
            ariaLabel="Remediation backlog composition"
            stacked
          />
        </Card>
        <Card
          eyebrow="Evidence matrix"
          title="Repository × gate coverage"
          description="Unavailable evidence is a first-class state, never rendered as zero."
        >
          <EvidenceHeatmap repositories={repositories} />
        </Card>
        <Card
          eyebrow="Repository comparison"
          title="Sortable evidence table"
          description="Use a repository name to drill the workspace down to one series."
          wide
        >
          <RepositoryTable
            analytics={{ ...analytics, repositories }}
            onDrillDown={onDrillDown}
          />
        </Card>
      </div>
    </>
  );
}
