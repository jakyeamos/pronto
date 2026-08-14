import type { ReactElement } from "react";
import type {
  PromotionCoverage,
  PromotionDiscoverySummary,
  PromotionFunnel,
} from "../types";
import { StatusPill } from "./ConsolePrimitives";

function coverageTone(status?: string): string {
  if (status === "assessed") return "mint";
  if (status === "blocked") return "coral";
  if (status === "partial") return "amber";
  return "blue";
}

function coverageLabel(status?: string): string {
  if (status === "assessed") return "Coverage assessed";
  if (status === "partial") return "Partial coverage";
  if (status === "blocked") return "Coverage blocked";
  return "Coverage not assessed";
}

function sourceLabel(value: string): string {
  return value.replaceAll("_", " ");
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

export function PromotionCoveragePanel({
  coverage,
  discovery,
  funnel,
}: {
  coverage?: PromotionCoverage | null;
  discovery?: PromotionDiscoverySummary | null;
  funnel?: PromotionFunnel | null;
}): ReactElement {
  const sourceManifest = coverage?.source_manifest ?? [];
  const unknownSources = coverage?.unknown_sources ?? [];
  const coverageStatus = coverage?.coverage_status ?? "unassessed";
  return (
    <section className="surface-panel promotion-coverage-panel">
      <div className="surface-heading">
        <div>
          <p className="eyebrow">Discovery coverage</p>
          <h2>Candidate counts are bounded by AWL's source inventory</h2>
          <p>
            This run inventories explicit roots and file metadata; candidate
            extraction is a separate review step. Unassessed sources are shown
            explicitly so zero candidates never means “nothing exists.”
          </p>
        </div>
        <StatusPill tone={coverageTone(coverageStatus)}>
          {coverageLabel(coverageStatus)}
        </StatusPill>
      </div>
      <div className="promotion-coverage-grid">
        <div>
          <span>Assessed sources</span>
          <strong>{coverage?.assessed_sources ?? 0}</strong>
        </div>
        <div>
          <span>Unassessed sources</span>
          <strong>{coverage?.unassessed_sources ?? 0}</strong>
        </div>
        <div>
          <span>Files inventoried</span>
          <strong>{coverage?.files_seen ?? 0}</strong>
        </div>
        <div>
          <span>Bytes inventoried</span>
          <strong>{formatBytes(coverage?.bytes_seen ?? 0)}</strong>
        </div>
      </div>
      {discovery && (
        <div className="promotion-discovery-summary">
          <div>
            <span>AWL observations</span>
            <strong>{discovery.observations_seen}</strong>
          </div>
          <div>
            <span>Asset observations</span>
            <strong>{discovery.asset_observation_documents}</strong>
          </div>
          <div>
            <span>Candidate drafts</span>
            <strong>{discovery.candidate_drafts}</strong>
          </div>
          <p>
            Asset observations are review inputs, not candidates. They become
            candidates only after testing, quantification, and packet review.
          </p>
        </div>
      )}
      {funnel && (
        <div className="promotion-funnel-summary">
          <div>
            <span>Evaluation drafts</span>
            <strong>{funnel.evaluation_candidate_drafts}</strong>
          </div>
          <div>
            <span>Behavior identities</span>
            <strong>{funnel.ready_behavior_identity_clusters}</strong>
          </div>
          <div>
            <span>Forward-test queue</span>
            <strong>{funnel.selected_forward_test_work_items}</strong>
          </div>
          <div>
            <span>Review packets</span>
            <strong>{funnel.promotion_packets}</strong>
          </div>
          <p>
            Upstream funnel: {funnel.evaluation_candidate_drafts} evaluation
            drafts → {funnel.ready_behavior_identity_clusters} behavior
            identities → {funnel.selected_forward_test_work_items} forward-test
            work items. Packets stay outside the {funnel.promotion_candidates}
            -candidate inbox until quantification and review pass.
          </p>
        </div>
      )}
      {unknownSources.length > 0 && (
        <div className="promotion-coverage-unknowns">
          <span>Not assessed in this run</span>
          <div>
            {unknownSources.map((source) => (
              <span key={source}>{sourceLabel(source)}</span>
            ))}
          </div>
        </div>
      )}
      {sourceManifest.length > 0 && (
        <div className="promotion-coverage-source-list">
          {sourceManifest.map((source) => (
            <div key={source.source_id}>
              <span>{sourceLabel(source.category)}</span>
              <small>
                {source.files_seen} file{source.files_seen === 1 ? "" : "s"}
                {source.notes ? ` · ${source.notes}` : ""}
              </small>
              <StatusPill tone={coverageTone(source.status)}>
                {source.status}
              </StatusPill>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
