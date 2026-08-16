import type { ReactElement } from "react";
import { ExternalLink } from "lucide-react";
import type {
  CreatePapercutInput,
  MultiplierProposalStatus,
  PapercutBacklog,
  PapercutStatus,
  SkillRecord,
  SkillsSnapshot,
} from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

export type SortKey = "name" | "recent" | "allTime" | "parity";

export type SkillGroup = {
  category: string;
  families: Array<{ family: string; skills: SkillRecord[] }>;
};

export const PAPERCUTS_SKILL_ID = "papercuts";

const CATEGORY_ORDER = [
  "Agent Operations",
  "DevOps",
  "UI & Design",
  "Research & Writing",
  "Automation & Data",
  "Career",
  "Quality & Security",
  "Other",
];

export function categoryRank(category: string): number {
  const index = CATEGORY_ORDER.indexOf(category);
  return index === -1 ? CATEGORY_ORDER.length : index;
}

export function providerTone(state?: string): string {
  if (state === "projected" || state === "native") return "mint";
  if (state === "divergent") return "amber";
  if (state === "blocked") return "coral";
  return "slate";
}

function providerSummary(skill: SkillRecord): string {
  return Object.values(skill.providers)
    .map((provider) => provider.state)
    .filter((state, index, values) => values.indexOf(state) === index)
    .join(" · ");
}

function findingCapability(skill: SkillRecord) {
  return (
    skill.finding_capability ?? {
      finding_expectation: "review_required",
      finding_expectation_reason:
        "No reviewed finding profile or Quality Runner capability evidence was found for this skill.",
      finding_classes: [],
      backfill: {
        mode: "not_evidenced",
        phases: [],
        safety: "Review required.",
      },
      quality_runner: {
        status: "unknown",
        adapter: "",
        finding_categories: [],
        coverage: { rule_count: 0, finding_count: 0, statuses: [] },
        evidence: [],
        gaps: [
          "No Quality Runner capability feed or reviewed adapter was found.",
        ],
      },
      gaps: [
        "Review whether this skill should produce findings before adding it to the Quality Runner representation.",
      ],
    }
  );
}

function capabilityTone(status?: string): string {
  if (
    status === "coverage_proven" ||
    status === "adapter_defined" ||
    status === "not_applicable" ||
    status === "native"
  ) {
    return "mint";
  }
  if (
    status === "configured" ||
    status === "available" ||
    status === "required"
  ) {
    return "amber";
  }
  if (status === "unsupported" || status === "blocked") return "coral";
  return "slate";
}

function capabilityStatusLabel(status?: string): string {
  switch (status) {
    case "coverage_proven":
      return "Coverage proven";
    case "adapter_defined":
      return "Adapter defined";
    case "scan_observed":
      return "Scan observed";
    case "not_applicable":
      return "Pronto-native";
    case "configured":
      return "Configured · run evidence pending";
    case "unknown":
      return "Review required";
    default:
      return status || "Unknown";
  }
}

function findingExpectationLabel(expectation?: string): string {
  switch (expectation) {
    case "required":
      return "Should produce findings";
    case "none":
      return "Findings not expected";
    case "optional":
      return "Findings optional";
    default:
      return "Finding expectation unreviewed";
  }
}

export function skillCountLabel(count: number): string {
  return String(count) + " " + (count === 1 ? "skill" : "skills");
}

export type SkillsSurfaceProps = {
  snapshot: SkillsSnapshot;
  isRefreshing: boolean;
  onRefresh: () => void;
  onOpenSource: (path: string) => void;
  papercutBacklog: PapercutBacklog;
  onRefreshPapercutBacklog: () => Promise<void>;
  onCreatePapercut: (input: CreatePapercutInput) => Promise<void>;
  onPapercutStatusChange: (
    papercutId: string,
    status: PapercutStatus,
  ) => Promise<void>;
  onMultiplierProposalStatusChange: (
    proposalId: string,
    status: MultiplierProposalStatus,
  ) => Promise<void>;
};

export function SkillsTable({
  skills,
  selectedId,
  onSelect,
}: {
  skills: SkillRecord[];
  selectedId: string | null;
  onSelect: (skill: SkillRecord) => void;
}): ReactElement {
  return (
    <table className="skills-table">
      <thead>
        <tr>
          <th>Skill</th>
          <th>Providers</th>
          <th>Usage</th>
          <th>Findings</th>
          <th>Evidence</th>
        </tr>
      </thead>
      <tbody>
        {skills.map((skill) => (
          <tr
            key={skill.id}
            id={`skill-${skill.id}`}
            className={selectedId === skill.id ? "is-selected" : undefined}
            tabIndex={0}
            aria-label={`${skill.name} skill`}
            aria-selected={selectedId === skill.id}
            onClick={() => onSelect(skill)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onSelect(skill);
              }
            }}
          >
            <td>
              <strong>{skill.name}</strong>
              <small>{skill.description}</small>
              <span className="skills-badges">
                <StatusPill
                  tone={skill.lifecycle === "canonical" ? "blue" : "slate"}
                >
                  {skill.lifecycle}
                </StatusPill>
                {skill.id === PAPERCUTS_SKILL_ID ? (
                  <StatusPill tone="violet">Pronto skill</StatusPill>
                ) : null}
                {skill.hosted_in_jakye_agent_setup ? (
                  <StatusPill tone="mint">jakye-agent-setup</StatusPill>
                ) : null}
              </span>
            </td>
            <td>
              <div className="skills-provider-pills">
                {Object.entries(skill.providers).map(([provider, value]) => (
                  <StatusPill key={provider} tone={providerTone(value.state)}>
                    {provider}: {value.state}
                  </StatusPill>
                ))}
              </div>
              <small>{providerSummary(skill)}</small>
            </td>
            <td>
              {skill.usage.state === "observed" ? (
                <>
                  <strong>{skill.usage.recent_count}</strong>
                  <small>
                    {skill.usage.all_time_count} recorded
                    {skill.usage.last_seen_at
                      ? " · " + formatTime(skill.usage.last_seen_at)
                      : " · observation time unavailable"}
                  </small>
                </>
              ) : (
                <>
                  <StatusPill tone="slate">Unavailable</StatusPill>
                  <small>No verified provider feed</small>
                </>
              )}
            </td>
            <td className="skills-capability-cell">
              <StatusPill
                tone={capabilityTone(
                  findingCapability(skill).quality_runner.status,
                )}
              >
                {capabilityStatusLabel(
                  findingCapability(skill).quality_runner.status,
                )}
              </StatusPill>
              <small>
                {findingExpectationLabel(
                  findingCapability(skill).finding_expectation,
                )}
              </small>
            </td>
            <td>
              <strong>
                {skill.parity_score == null
                  ? "Unknown"
                  : String(skill.parity_score) + "%"}
              </strong>
              <small>{skill.parity_evidence[0]}</small>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function SkillDetail({
  selected,
  onClose,
  onOpenSource,
}: {
  selected: SkillRecord;
  onClose: () => void;
  onOpenSource: (path: string) => void;
}): ReactElement {
  return (
    <aside className="skills-detail" aria-label={`${selected.name} details`}>
      <div className="skills-detail-heading">
        <div>
          <span className="eyebrow">Selected skill</span>
          <h2>{selected.name}</h2>
        </div>
        <button
          className="icon-button"
          type="button"
          aria-label="Close skill details"
          onClick={onClose}
        >
          ×
        </button>
      </div>
      <p>{selected.description}</p>
      <dl>
        <div>
          <dt>Lifecycle</dt>
          <dd>{selected.lifecycle}</dd>
        </div>
        <div>
          <dt>Category</dt>
          <dd>{selected.category || "Other"}</dd>
        </div>
        <div>
          <dt>Family</dt>
          <dd>{selected.family || "Standalone"}</dd>
        </div>
        <div>
          <dt>Usage</dt>
          <dd>
            {selected.usage.state === "observed" ? (
              <>
                {selected.usage.recent_count} recent ·{" "}
                {selected.usage.all_time_count} recorded
                <small>{selected.usage.reason}</small>
              </>
            ) : (
              <>
                <StatusPill tone="slate">Unavailable</StatusPill>
                <small>{selected.usage.reason}</small>
              </>
            )}
          </dd>
        </div>
        <div>
          <dt>Usage evidence</dt>
          <dd>{selected.usage.telemetry_source}</dd>
        </div>
        <div>
          <dt>Parity</dt>
          <dd>
            {selected.parity_score == null
              ? "Unknown / unverified"
              : `${selected.parity_score}%`}
          </dd>
        </div>
      </dl>
      {(() => {
        const capability = findingCapability(selected);
        return (
          <section
            className="skills-capability"
            aria-label="Finding and backfill analysis"
          >
            <h3>Finding and backfill analysis</h3>
            <div className="skills-capability-status">
              <StatusPill tone={capabilityTone(capability.finding_expectation)}>
                {findingExpectationLabel(capability.finding_expectation)}
              </StatusPill>
              <p>{capability.finding_expectation_reason}</p>
            </div>
            <div className="skills-capability-grid">
              <div>
                <dt>Quality Runner</dt>
                <dd>
                  <StatusPill
                    tone={capabilityTone(capability.quality_runner.status)}
                  >
                    {capabilityStatusLabel(capability.quality_runner.status)}
                  </StatusPill>
                </dd>
              </div>
              <div>
                <dt>Backfill mode</dt>
                <dd>{capability.backfill.mode}</dd>
              </div>
              <div>
                <dt>Adapter</dt>
                <dd>
                  {capability.quality_runner.adapter || "No adapter recorded"}
                </dd>
              </div>
              <div>
                <dt>Coverage</dt>
                <dd>
                  {capability.quality_runner.coverage.rule_count} rules ·{" "}
                  {capability.quality_runner.coverage.finding_count} findings
                </dd>
              </div>
            </div>
            <h4>Finding classes</h4>
            {capability.finding_classes.length > 0 ? (
              <div className="skills-capability-classes">
                {capability.finding_classes.map((findingClass) => (
                  <div key={findingClass.id}>
                    <StatusPill tone={capabilityTone(findingClass.state)}>
                      {findingClass.label}
                    </StatusPill>
                    <small>{findingClass.evidence}</small>
                  </div>
                ))}
              </div>
            ) : (
              <p className="skills-capability-empty">
                No finding classes are represented yet.
              </p>
            )}
            <h4>Backfill phases</h4>
            <div className="skills-capability-phases">
              {capability.backfill.phases.map((phase) => (
                <div key={phase.id}>
                  <StatusPill tone={capabilityTone(phase.state)}>
                    {phase.id}: {phase.state}
                  </StatusPill>
                  <small>{phase.evidence}</small>
                </div>
              ))}
            </div>
            {[
              ...capability.quality_runner.evidence,
              ...capability.quality_runner.gaps,
              ...capability.gaps,
            ].length > 0 ? (
              <>
                <h4>Evidence and gaps</h4>
                <ul className="skills-capability-list">
                  {[
                    ...capability.quality_runner.evidence,
                    ...capability.quality_runner.gaps,
                    ...capability.gaps,
                  ].map((item, index) => (
                    <li key={`${index}-${item}`}>{item}</li>
                  ))}
                </ul>
              </>
            ) : null}
            <p className="skills-capability-safety">
              {capability.backfill.safety}
            </p>
          </section>
        );
      })()}
      <h3>Provider matrix</h3>
      {Object.entries(selected.providers).map(([provider, value]) => (
        <div className="skills-provider-detail" key={provider}>
          <StatusPill tone={providerTone(value.state)}>
            {provider}: {value.state}
          </StatusPill>
          <small>{value.reason}</small>
        </div>
      ))}
      <h3>Sources</h3>
      {selected.sources.map((source) => (
        <button
          className="skills-source"
          type="button"
          key={source.path}
          onClick={() => onOpenSource(source.path)}
        >
          <span>{source.root}</span>
          <small>{source.path}</small>
          <ExternalLink size={14} />
        </button>
      ))}
    </aside>
  );
}
