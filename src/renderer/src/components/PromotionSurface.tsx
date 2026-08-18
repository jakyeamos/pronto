import { useEffect, useState, type ReactElement } from "react";
import {
  AlertTriangle,
  Check,
  Inbox,
  RefreshCw,
  ShieldCheck,
  X,
} from "lucide-react";
import type {
  PromotionCoverage,
  PromotionCandidate,
  PromotionDiscoverySummary,
  PromotionDecision,
  PromotionFunnel,
  PromotionInbox,
} from "../types";
import { formatExactTime, formatTime, StatusPill } from "./ConsolePrimitives";

type PromotionSurfaceProps = {
  inbox: PromotionInbox;
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  onDecide: (
    candidateId: string,
    decision: PromotionDecision,
    reason?: string,
  ) => Promise<void>;
};

const decisionActions: Array<{
  decision: PromotionDecision;
  label: string;
  tone: string;
}> = [
  { decision: "public", label: "Promote public", tone: "mint" },
  { decision: "private", label: "Keep private", tone: "blue" },
  { decision: "both", label: "Promote both", tone: "blue" },
  { decision: "defer", label: "Defer", tone: "amber" },
  { decision: "reject", label: "Reject", tone: "coral" },
];

function isPromotionDecision(decision: PromotionDecision): boolean {
  return decision === "public" || decision === "private" || decision === "both";
}

function decisionLabel(decision?: string | null): string {
  if (decision === "public") return "Public projection";
  if (decision === "private") return "Private projection";
  if (decision === "both") return "Public + private projection";
  if (decision === "defer") return "Deferred";
  if (decision === "reject") return "Rejected";
  return "Awaiting your decision";
}

function candidateTone(candidate: PromotionCandidate): string {
  if (candidate.decision === "reject") return "coral";
  if (candidate.decision === "defer") return "amber";
  if (candidate.decision) return "mint";
  if (candidate.candidate_kind !== "complete") return "amber";
  return "blue";
}

function hasRecordedDecision(candidate: PromotionCandidate): boolean {
  return candidate.decision != null;
}

function isPromotionQueueCandidate(candidate: PromotionCandidate): boolean {
  return (
    candidate.candidate_kind === "complete" && !hasRecordedDecision(candidate)
  );
}

function isPipelineDraft(candidate: PromotionCandidate): boolean {
  return (
    candidate.candidate_kind !== "complete" && !hasRecordedDecision(candidate)
  );
}

function CandidateReferenceList({
  label,
  values,
}: {
  label: string;
  values: string[];
}): ReactElement {
  return (
    <div className="promotion-reference-block">
      <span>{label}</span>
      {values.length === 0 ? (
        <small>None recorded</small>
      ) : (
        <ul className="promotion-reference-list">
          {values.map((value) => (
            <li key={value}>
              <code>{value}</code>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function CandidateList({
  candidates,
  selectedId,
  onSelect,
}: {
  candidates: PromotionCandidate[];
  selectedId: string | null;
  onSelect: (candidate: PromotionCandidate) => void;
}): ReactElement {
  return (
    <div className="promotion-candidate-list">
      {candidates.map((candidate) => (
        <button
          className={`promotion-candidate-row ${
            selectedId === candidate.candidate_id ? "is-selected" : ""
          }`}
          type="button"
          key={candidate.candidate_id}
          onClick={() => onSelect(candidate)}
        >
          <span className="promotion-candidate-row-main">
            <strong>{candidate.title}</strong>
            <small>
              {candidate.asset_kind} · {candidate.candidate_kind} ·{" "}
              {candidate.package_status}
            </small>
          </span>
          <StatusPill tone={candidateTone(candidate)}>
            {decisionLabel(candidate.decision)}
          </StatusPill>
        </button>
      ))}
    </div>
  );
}

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

function FunnelMetric({
  label,
  value,
  note,
}: {
  label: string;
  value: number;
  note: string;
}): ReactElement {
  return (
    <div className="promotion-funnel-metric">
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{note}</small>
    </div>
  );
}

function EvaluationPipelinePanel({
  funnel,
  promotionQueueCount,
  pipelineDraftCount,
  decisionHistoryCount,
}: {
  funnel?: PromotionFunnel | null;
  promotionQueueCount: number;
  pipelineDraftCount: number;
  decisionHistoryCount: number;
}): ReactElement | null {
  if (!funnel) return null;

  const testsCompleted = funnel.forward_test_pass + funnel.forward_test_failed;
  const evaluationsBlocked =
    funnel.forward_test_blocked + funnel.packets_blocked;

  return (
    <section className="surface-panel promotion-funnel-panel">
      <div className="surface-heading">
        <div>
          <p className="eyebrow">AWL evaluation pipeline</p>
          <h2>Inputs, tests, and candidates are separate</h2>
          <p>
            Evaluation inputs are source rows, not pending promotion decisions.
            Only complete, undecided candidate packets enter the owner review
            queue.
          </p>
        </div>
        <StatusPill tone={funnel.status === "pass" ? "mint" : "amber"}>
          {funnel.status === "pass" ? "Projection current" : funnel.status}
        </StatusPill>
      </div>
      <div className="promotion-funnel-grid">
        <FunnelMetric
          label="Evaluation inputs"
          value={funnel.evaluation_candidate_drafts}
          note="source rows; not promotions"
        />
        <FunnelMetric
          label="Behavior identities"
          value={funnel.ready_behavior_identity_clusters}
          note="deduplicated behaviors"
        />
        <FunnelMetric
          label="Forward-test work items"
          value={funnel.selected_forward_test_work_items}
          note="selected execution queue"
        />
        <FunnelMetric
          label="Incomplete candidate drafts"
          value={pipelineDraftCount}
          note="not in promotion queue"
        />
        <FunnelMetric
          label="Promotion-ready packets"
          value={promotionQueueCount}
          note="complete packets awaiting decision"
        />
        <FunnelMetric
          label="Tests completed"
          value={testsCompleted}
          note={`${funnel.forward_test_pass} passed · ${funnel.forward_test_failed} failed`}
        />
        <FunnelMetric
          label="Evaluation blocked"
          value={evaluationsBlocked}
          note={`${funnel.forward_test_blocked} test · ${funnel.packets_blocked} packet`}
        />
        <FunnelMetric
          label="Quantification pending"
          value={funnel.quantification_pending}
          note="awaiting baseline/observed evidence"
        />
        <FunnelMetric
          label="Review packets"
          value={funnel.promotion_packets}
          note={`${funnel.packets_failed} failed packet${funnel.packets_failed === 1 ? "" : "s"}`}
        />
        <FunnelMetric
          label="Candidates formed"
          value={funnel.promotion_candidates}
          note="AWL records; not all promotion-ready"
        />
        <FunnelMetric
          label="Decision history"
          value={decisionHistoryCount}
          note="accepted, deferred, or rejected"
        />
      </div>
    </section>
  );
}

function CoveragePanel({
  coverage,
  discovery,
}: {
  coverage?: PromotionCoverage | null;
  discovery?: PromotionDiscoverySummary | null;
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
            <span>Latest discovery drafts</span>
            <strong>{discovery.candidate_drafts}</strong>
          </div>
          <p>
            Discovery drafts and asset observations are review inputs, not
            formed candidates. They become candidates only after testing,
            quantification, and packet review.
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
export function PromotionSurface({
  inbox,
  isRefreshing,
  onRefresh,
  onDecide,
}: PromotionSurfaceProps): ReactElement {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [reason, setReason] = useState("");
  const [submitting, setSubmitting] = useState<PromotionDecision | null>(null);
  const promotionQueueCandidates = inbox.candidates.filter(
    isPromotionQueueCandidate,
  );
  const pipelineDraftCandidates = inbox.candidates.filter(isPipelineDraft);
  const decisionHistoryCandidates =
    inbox.candidates.filter(hasRecordedDecision);
  const displayedCandidates = [
    ...promotionQueueCandidates,
    ...pipelineDraftCandidates,
    ...decisionHistoryCandidates,
  ];
  const selected =
    displayedCandidates.find(
      (candidate) => candidate.candidate_id === selectedId,
    ) ??
    displayedCandidates[0] ??
    null;
  const selectedIsPromotionQueueCandidate = selected
    ? isPromotionQueueCandidate(selected)
    : false;
  const jasProjectionReady =
    selectedIsPromotionQueueCandidate &&
    selected?.jas_projection_status === "ready";

  const acceptedChoices = decisionHistoryCandidates.filter(
    (candidate) =>
      candidate.decision != null && isPromotionDecision(candidate.decision),
  ).length;

  useEffect(() => {
    if (!selected || selected.candidate_id !== selectedId) {
      setSelectedId(selected?.candidate_id ?? null);
      setReason("");
    }
  }, [selected, selectedId]);

  async function handleDecision(decision: PromotionDecision): Promise<void> {
    if (!selected || !selectedIsPromotionQueueCandidate || submitting) return;
    if (isPromotionDecision(decision) && !jasProjectionReady) return;
    const trimmedReason = reason.trim();
    if ((decision === "defer" || decision === "reject") && !trimmedReason) {
      return;
    }
    setSubmitting(decision);
    try {
      await onDecide(
        selected.candidate_id,
        decision,
        trimmedReason || undefined,
      );
      setReason("");
    } finally {
      setSubmitting(null);
    }
  }

  const actionsDisabled =
    isRefreshing || submitting !== null || inbox.status !== "pass";
  const quantificationEntries = selected?.quantification
    ? Object.entries(selected.quantification)
    : [];
  const jasAdmission = inbox.jas_admission ?? selected?.jas_admission ?? null;
  const jasAdmissionSucceeded =
    jasAdmission?.status === "JAS_APPLIED" ||
    jasAdmission?.status === "JAS_ALREADY_APPLIED";
  const jasReceiptBlocked = jasAdmission?.receipt_status === "blocked";

  return (
    <div className="promotion-surface">
      <section className="surface-panel promotion-boundary">
        <div className="surface-heading">
          <div>
            <p className="eyebrow">Human review boundary</p>
            <h2>AWL finds it. You decide.</h2>
            <p>
              This is a review-only inbox for backend-produced candidate packets
              from ai-workflow-leverage. AWL owns discovery, testing,
              quantification, and packet generation. Accepted complete packets
              trigger the validated JAS admission/install path; drafts remain in
              the AWL pipeline, and packets without a sanitized projection
              remain visibly blocked.
            </p>
          </div>
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void onRefresh()}
            disabled={isRefreshing}
          >
            <RefreshCw className={isRefreshing ? "spin" : ""} size={15} />
            {isRefreshing ? "Refreshing…" : "Refresh inbox"}
          </button>
        </div>
        <div className="promotion-boundary-status">
          <StatusPill
            tone={
              jasReceiptBlocked
                ? "amber"
                : jasAdmissionSucceeded
                  ? "mint"
                  : inbox.jas_mutation
                    ? "blue"
                    : "mint"
            }
            icon={<ShieldCheck size={12} />}
          >
            {jasReceiptBlocked
              ? "JAS applied; receipt persistence blocked"
              : jasAdmission?.status === "JAS_APPLIED"
                ? "JAS applied"
                : jasAdmission?.status === "JAS_ALREADY_APPLIED"
                  ? "JAS already applied"
                  : inbox.jas_mutation
                    ? "JAS mutation reported"
                    : "JAS unchanged"}
          </StatusPill>
          <span>
            Last read {formatTime(inbox.generated_at)} · manual review required
          </span>
        </div>
      </section>

      <section className="promotion-overview-grid">
        <div className="promotion-card">
          <span>Awaiting owner decision</span>
          <strong>{promotionQueueCandidates.length}</strong>
          <small>Complete packets only</small>
        </div>
        <div className="promotion-card">
          <span>AWL pipeline drafts</span>
          <strong>{pipelineDraftCandidates.length}</strong>
          <small>Not eligible for promotion</small>
        </div>
        <div className="promotion-card">
          <span>Accepted choices</span>
          <strong>{acceptedChoices}</strong>
          <small>JAS state shown below</small>
        </div>
        <div className="promotion-card">
          <span>Decision history</span>
          <strong>{decisionHistoryCandidates.length}</strong>
          <small>Accepted, deferred, or rejected</small>
        </div>
      </section>

      <CoveragePanel coverage={inbox.coverage} discovery={inbox.discovery} />
      <EvaluationPipelinePanel
        funnel={inbox.funnel}
        promotionQueueCount={promotionQueueCandidates.length}
        pipelineDraftCount={pipelineDraftCandidates.length}
        decisionHistoryCount={decisionHistoryCandidates.length}
      />

      {inbox.status !== "pass" && (
        <div className="promotion-status-banner" role="alert">
          <AlertTriangle size={17} />
          <div>
            <strong>
              {inbox.status === "unavailable"
                ? "AWL review is unavailable"
                : "AWL returned a blocked inbox"}
            </strong>
            <p>
              {inbox.message ??
                `${inbox.errors.length} source issue${inbox.errors.length === 1 ? "" : "s"} must be resolved before decisions can be recorded.`}
            </p>
          </div>
        </div>
      )}

      {jasAdmission && (
        <div
          className="promotion-status-banner"
          role={jasAdmissionSucceeded ? "status" : "alert"}
        >
          {jasAdmissionSucceeded ? (
            <ShieldCheck size={17} />
          ) : (
            <AlertTriangle size={17} />
          )}
          <div>
            <strong>
              {jasReceiptBlocked
                ? "JAS applied, but AWL receipt persistence is blocked"
                : jasAdmissionSucceeded
                  ? jasAdmission.status === "JAS_ALREADY_APPLIED"
                    ? "JAS is already in the requested state"
                    : "JAS admission/install completed"
                  : "JAS admission/install is blocked"}
            </strong>
            <p>
              {jasAdmission.receipt_message ??
                jasAdmission.message ??
                jasAdmission.reason ??
                (jasAdmission.target
                  ? `Target: ${jasAdmission.target}.`
                  : "The AWL decision remains recorded for review.")}
            </p>
          </div>
        </div>
      )}

      <div className="promotion-layout">
        <section className="surface-panel promotion-list-panel">
          <div className="surface-heading">
            <div>
              <p className="eyebrow">Promotion queue</p>
              <h2>
                {promotionQueueCandidates.length} complete packet
                {promotionQueueCandidates.length === 1 ? "" : "s"}
              </h2>
            </div>
            <Inbox size={19} />
          </div>
          {promotionQueueCandidates.length === 0 ? (
            <div className="surface-empty promotion-empty">
              <Check size={18} />
              <span>
                No complete candidate packets are awaiting your decision.
              </span>
            </div>
          ) : (
            <CandidateList
              candidates={promotionQueueCandidates}
              selectedId={selected?.candidate_id ?? null}
              onSelect={(candidate) => {
                setSelectedId(candidate.candidate_id);
                setReason("");
              }}
            />
          )}
          {pipelineDraftCandidates.length > 0 && (
            <div className="promotion-list-section">
              <div className="promotion-list-section-heading">
                <div>
                  <p className="eyebrow">AWL candidate pipeline</p>
                  <h3>
                    {pipelineDraftCandidates.length} incomplete draft
                    {pipelineDraftCandidates.length === 1 ? "" : "s"}
                  </h3>
                </div>
                <StatusPill tone="amber">Not eligible</StatusPill>
              </div>
              <p>
                These records stay in AWL until testing, quantification, and
                packet completion finish. They are not promotion decisions.
              </p>
              <CandidateList
                candidates={pipelineDraftCandidates}
                selectedId={selected?.candidate_id ?? null}
                onSelect={(candidate) => {
                  setSelectedId(candidate.candidate_id);
                  setReason("");
                }}
              />
            </div>
          )}
          {decisionHistoryCandidates.length > 0 && (
            <div className="promotion-list-section">
              <div className="promotion-list-section-heading">
                <div>
                  <p className="eyebrow">Decision history</p>
                  <h3>
                    {decisionHistoryCandidates.length} recorded decision
                    {decisionHistoryCandidates.length === 1 ? "" : "s"}
                  </h3>
                </div>
                <StatusPill tone="blue">Read-only</StatusPill>
              </div>
              <p>
                Accepted, deferred, and rejected records remain available for
                provenance but are no longer in the promotion queue.
              </p>
              <CandidateList
                candidates={decisionHistoryCandidates}
                selectedId={selected?.candidate_id ?? null}
                onSelect={(candidate) => {
                  setSelectedId(candidate.candidate_id);
                  setReason("");
                }}
              />
            </div>
          )}
        </section>

        <section className="surface-panel promotion-detail-panel">
          {selected ? (
            <>
              <div className="promotion-detail-heading">
                <div>
                  <p className="eyebrow">
                    {selected.decision
                      ? "Decision history"
                      : isPipelineDraft(selected)
                        ? "AWL pipeline draft"
                        : "Promotion candidate"}
                  </p>
                  <h2>{selected.title}</h2>
                  <p>
                    {selected.asset_kind} · {selected.portability} ·{" "}
                    {selected.candidate_kind}
                  </p>
                </div>
                <StatusPill tone={candidateTone(selected)}>
                  {decisionLabel(selected.decision)}
                </StatusPill>
              </div>
              <div className="promotion-detail-grid">
                <div>
                  <span>Package state</span>
                  <strong>{selected.package_status}</strong>
                </div>
                <div>
                  <span>Next action</span>
                  <strong>{selected.next_action}</strong>
                </div>
                <div>
                  <span>Evidence</span>
                  <strong>{selected.evidence_refs.length} references</strong>
                </div>
                <div>
                  <span>Provenance</span>
                  <code>
                    {selected.candidate_provenance_hash.slice(0, 12)}…
                  </code>
                </div>
              </div>
              <div className="promotion-reference-grid">
                <CandidateReferenceList
                  label="Source references"
                  values={selected.source_refs}
                />
                <CandidateReferenceList
                  label="Evidence references"
                  values={selected.evidence_refs}
                />
              </div>
              {quantificationEntries.length > 0 && (
                <div className="promotion-quantification">
                  <span>Quantification</span>
                  <div>
                    {quantificationEntries.map(([key, value]) => (
                      <span key={key}>
                        <strong>{key}</strong> {String(value)}
                      </span>
                    ))}
                  </div>
                </div>
              )}
              {selected.decision_at && (
                <div className="promotion-recorded-decision">
                  <Check size={15} />
                  <span>
                    {decisionLabel(selected.decision)} recorded{" "}
                    {formatExactTime(selected.decision_at)}
                    {selected.decision_reviewer
                      ? ` by ${selected.decision_reviewer}`
                      : ""}
                    .
                  </span>
                </div>
              )}
              {isPipelineDraft(selected) ? (
                <div className="promotion-readiness-note promotion-readiness-note-draft">
                  <AlertTriangle size={15} />
                  <span>
                    This record remains in AWL until testing, quantification,
                    and packet completion finish. It cannot receive an owner
                    decision from this surface.
                  </span>
                </div>
              ) : selected.decision ? (
                <div className="promotion-readiness-note promotion-readiness-note-history">
                  <ShieldCheck size={15} />
                  <span>
                    This record is closed and shown for provenance only. New
                    promotion decisions are not available.
                  </span>
                </div>
              ) : (
                <>
                  <label className="promotion-reason-field">
                    <span>Decision note</span>
                    <textarea
                      value={reason}
                      onChange={(event) => setReason(event.target.value)}
                      placeholder="Required when deferring or rejecting; optional for promotion choices."
                      rows={3}
                    />
                  </label>
                  <div className="promotion-action-grid">
                    {decisionActions.map((action) => (
                      <button
                        className={`button button-${action.tone}`}
                        type="button"
                        key={action.decision}
                        data-decision={action.decision}
                        disabled={
                          actionsDisabled ||
                          (isPromotionDecision(action.decision) &&
                            !jasProjectionReady)
                        }
                        aria-describedby={
                          isPromotionDecision(action.decision) &&
                          !jasProjectionReady
                            ? "promotion-readiness-note"
                            : undefined
                        }
                        title={
                          isPromotionDecision(action.decision) &&
                          !jasProjectionReady
                            ? "A complete candidate with a sanitized JAS projection is required."
                            : undefined
                        }
                        onClick={() => void handleDecision(action.decision)}
                      >
                        {action.decision === "reject" ? (
                          <X size={14} />
                        ) : (
                          <Check size={14} />
                        )}
                        {submitting === action.decision
                          ? "Recording…"
                          : action.label}
                      </button>
                    ))}
                  </div>
                  <p
                    className="promotion-action-note"
                    id="promotion-readiness-note"
                    role={jasProjectionReady ? undefined : "status"}
                  >
                    {jasProjectionReady
                      ? "An accepted choice records the AWL decision and invokes JAS admission/install when the packet is complete and projection-ready. Defer and reject only record the decision."
                      : "Promotion choices are disabled until this complete candidate includes a sanitized JAS projection. You can still defer or reject it."}
                  </p>
                </>
              )}
            </>
          ) : (
            <div className="promotion-detail-empty">
              <Inbox size={22} />
              <strong>Select a candidate to review.</strong>
              <span>
                AWL candidates will appear here after a discovery run.
              </span>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
