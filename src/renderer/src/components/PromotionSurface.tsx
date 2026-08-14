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
  PromotionCandidate,
  PromotionDecision,
  PromotionInbox,
} from "../types";
import { formatExactTime, formatTime, StatusPill } from "./ConsolePrimitives";
import { PromotionCoveragePanel } from "./PromotionCoveragePanel";

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
  if (candidate.candidate_kind === "draft") return "amber";
  return "blue";
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

export function PromotionSurface({
  inbox,
  isRefreshing,
  onRefresh,
  onDecide,
}: PromotionSurfaceProps): ReactElement {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [reason, setReason] = useState("");
  const [submitting, setSubmitting] = useState<PromotionDecision | null>(null);
  const selected =
    inbox.candidates.find(
      (candidate) => candidate.candidate_id === selectedId,
    ) ??
    inbox.candidates[0] ??
    null;
  const jasProjectionReady =
    selected?.candidate_kind === "complete" &&
    selected.jas_projection_status === "ready";

  useEffect(() => {
    if (!selected || selected.candidate_id !== selectedId) {
      setSelectedId(selected?.candidate_id ?? null);
      setReason("");
    }
  }, [selected, selectedId]);

  async function handleDecision(decision: PromotionDecision): Promise<void> {
    if (!selected || submitting) return;
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
              trigger the validated JAS admission/install path; drafts and
              packets without a sanitized projection remain visibly blocked.
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
          <span>Promotion inbox</span>
          <strong>{inbox.counts.pending}</strong>
          <small>
            {inbox.counts.pending} awaiting decision · {inbox.counts.total}{" "}
            total candidates
          </small>
        </div>
        <div className="promotion-card">
          <span>Complete packets</span>
          <strong>{inbox.counts.complete}</strong>
          <small>{inbox.counts.drafts} draft-only candidates</small>
        </div>
        <div className="promotion-card">
          <span>Accepted choices</span>
          <strong>{inbox.counts.accepted}</strong>
          <small>JAS state shown below</small>
        </div>
        <div className="promotion-card">
          <span>Closed choices</span>
          <strong>{inbox.counts.deferred + inbox.counts.rejected}</strong>
          <small>Deferred or rejected in AWL</small>
        </div>
      </section>

      <PromotionCoveragePanel
        coverage={inbox.coverage}
        discovery={inbox.discovery}
        funnel={inbox.funnel}
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
              <p className="eyebrow">Private candidate queue</p>
              <h2>
                {inbox.counts.total} candidate
                {inbox.counts.total === 1 ? "" : "s"}
              </h2>
            </div>
            <Inbox size={19} />
          </div>
          {inbox.candidates.length === 0 ? (
            <div className="surface-empty promotion-empty">
              <Check size={18} />
              <span>
                No evaluated candidates are waiting in the promotion inbox.
              </span>
            </div>
          ) : (
            <div className="promotion-candidate-list">
              {inbox.candidates.map((candidate) => (
                <button
                  className={`promotion-candidate-row ${
                    selected?.candidate_id === candidate.candidate_id
                      ? "is-selected"
                      : ""
                  }`}
                  type="button"
                  key={candidate.candidate_id}
                  onClick={() => {
                    setSelectedId(candidate.candidate_id);
                    setReason("");
                  }}
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
          )}
        </section>

        <section className="surface-panel promotion-detail-panel">
          {selected ? (
            <>
              <div className="promotion-detail-heading">
                <div>
                  <p className="eyebrow">Candidate detail</p>
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
                  : selected.candidate_kind === "complete"
                    ? "Promotion choices are disabled until this complete candidate includes a sanitized JAS projection. You can still defer or reject it."
                    : "Promotion choices are disabled until AWL produces a complete candidate packet with a sanitized JAS projection. You can still defer or reject it."}
              </p>
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
