import type { ReactElement } from "react";
import {
  CheckCircle2,
  CircleAlert,
  Download,
  FileCheck2,
  GitBranch,
  RefreshCw,
  ShieldAlert,
} from "lucide-react";
import type { RemediationRun } from "../types";
import { formatRemediationTime } from "./RemediationActionRow";

export function RemediationRunOverview({
  run,
  isRefreshing,
  onRefresh,
  onExport,
}: {
  run: RemediationRun;
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onExport: () => Promise<void>;
}): ReactElement {
  const closures = run.closures ?? [];
  const openActionCount = run.plans.reduce(
    (total, plan) =>
      total +
      plan.actions.filter((action) => action.status !== "verified").length,
    0,
  );
  const verifiedActionCount = run.plans.reduce(
    (total, plan) =>
      total +
      plan.actions.filter((action) => action.status === "verified").length,
    0,
  );
  const blockedActionCount = run.plans.reduce(
    (total, plan) =>
      total +
      plan.actions.filter((action) => action.status === "blocked").length,
    0,
  );

  return (
    <>
      <section className="remediation-overview-grid">
        <div className="remediation-overview-card remediation-overview-card-accent">
          <span>Refresh state</span>
          <strong>{run.status.replace("_", " ")}</strong>
          <small>
            {run.source_refresh_id
              ? `Run ${run.source_refresh_id}`
              : "No full refresh recorded yet"}
          </small>
          <FileCheck2 size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Active repositories</span>
          <strong>{run.plans.length}</strong>
          <small>
            Ranked queue · {run.eligible_repository_paths.length} bounded paths
          </small>
          <GitBranch size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Actions to work</span>
          <strong>{openActionCount}</strong>
          <small>
            {verifiedActionCount} verified · {blockedActionCount} blocked
          </small>
          <CircleAlert size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Retained closures</span>
          <strong>{closures.length}</strong>
          <small>Verified or explicitly deferred queue exits</small>
          <CheckCircle2 size={18} />
        </div>
        <div className="remediation-overview-card">
          <span>Scope exclusions</span>
          <strong>{run.excluded_repositories.length}</strong>
          <small>
            {run.excluded_repositories.length === 0
              ? "All registered repositories are eligible"
              : "Intentionally held-out repositories"}
          </small>
          <ShieldAlert size={18} />
        </div>
      </section>

      <section className="surface-panel remediation-control-panel">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Evidence refresh contract</p>
            <h2>Refresh everything before acting</h2>
            <p>
              QR doctor gates the run, then Pronto refreshes local state, QR
              artifacts, replay/report/feed evidence, provider context, and
              per-repository plans.
            </p>
          </div>
          <div className="remediation-control-actions">
            <button
              className="button button-primary"
              type="button"
              disabled={isRefreshing}
              onClick={() => void onRefresh()}
            >
              <RefreshCw
                size={15}
                className={isRefreshing ? "spin" : undefined}
              />
              {isRefreshing ? "Refreshing…" : "Run full refresh"}
            </button>
            <button
              className="button button-secondary"
              type="button"
              disabled={
                isRefreshing ||
                (run.plans.length === 0 && closures.length === 0)
              }
              onClick={() => void onExport()}
            >
              <Download size={15} />
              Export queue
            </button>
          </div>
        </div>
        <div className="remediation-run-meta">
          <span>Generated {formatRemediationTime(run.generated_at)}</span>
          <span>{run.eligible_repository_ids.length} eligible IDs</span>
          {run.message && (
            <span className="remediation-run-message">{run.message}</span>
          )}
        </div>
      </section>
    </>
  );
}
