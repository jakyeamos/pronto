import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { Plus, Save, Trash2 } from "lucide-react";
import type {
  QualityGate,
  QualityGateRequirement,
  QualityRequirementPolicy,
  QualitySource,
  QualityVerificationLevel,
  ReleaseRuleConfig,
} from "../types";

const canonicalQualityGates: QualityGate[] = [
  {
    id: "build",
    label: "Build",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "runtime_smoke",
    label: "Smoke",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "tests",
    label: "Tests",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "lint",
    label: "Lint",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "formatter",
    label: "Formatter",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "typecheck",
    label: "Typecheck",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "dead_code",
    label: "Dead-code",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "secrets_scan",
    label: "Secrets scan",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "dependency_audit",
    label: "Dependency audit",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
  {
    id: "web_readiness",
    label: "Web readiness",
    status: "Not configured",
    freshness: "Unknown",
    evidence: [],
  },
];

export function qualityGateChoices(
  availableGates: QualityGate[],
): QualityGate[] {
  const choices = new Map(canonicalQualityGates.map((gate) => [gate.id, gate]));
  for (const gate of availableGates) choices.set(gate.id, gate);
  return Array.from(choices.values());
}

export function ReleaseRuleEditor({
  rule,
  availableGates,
  onSave,
}: {
  rule?: ReleaseRuleConfig;
  availableGates: QualityGate[];
  onSave: (rule: ReleaseRuleConfig | null) => Promise<void>;
}): ReactElement {
  const [name, setName] = useState(rule?.name ?? "Release threshold");
  const [operator, setOperator] = useState(rule?.operator ?? "AND");
  const [minCommits, setMinCommits] = useState(
    rule?.min_commits?.toString() ?? "",
  );
  const [minElapsedDays, setMinElapsedDays] = useState(
    rule?.min_elapsed_days?.toString() ?? "",
  );
  const [commitTypes, setCommitTypes] = useState(
    rule?.required_commit_types.join(", ") ?? "",
  );
  const [allowFirstRelease, setAllowFirstRelease] = useState(
    rule?.allow_first_release ?? false,
  );
  const [qualityGates, setQualityGates] = useState<QualityGateRequirement[]>(
    rule?.required_quality_gates ?? [],
  );
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    setName(rule?.name ?? "Release threshold");
    setOperator(rule?.operator ?? "AND");
    setMinCommits(rule?.min_commits?.toString() ?? "");
    setMinElapsedDays(rule?.min_elapsed_days?.toString() ?? "");
    setCommitTypes(rule?.required_commit_types.join(", ") ?? "");
    setAllowFirstRelease(rule?.allow_first_release ?? false);
    setQualityGates(rule?.required_quality_gates ?? []);
  }, [rule]);

  const normalizedTypes = commitTypes
    .split(",")
    .map((value) => value.trim().toLowerCase())
    .filter(Boolean);
  const hasClause = Boolean(
    minCommits.trim() ||
    minElapsedDays.trim() ||
    normalizedTypes.length > 0 ||
    qualityGates.length > 0,
  );
  const gateChoices = qualityGateChoices(availableGates);

  const addQualityGate = (): void => {
    const selected = new Set(
      qualityGates.map((requirement) => requirement.gate_id),
    );
    const firstAvailable = gateChoices.find((gate) => !selected.has(gate.id));
    if (!firstAvailable) return;
    setQualityGates((current) => [
      ...current,
      {
        gate_id: firstAvailable.id,
        source: firstAvailable.id === "web_readiness" ? "QR" : "CI",
        minimum_verification_level:
          firstAvailable.id === "web_readiness"
            ? "deployment_verified"
            : undefined,
        policy: "block",
      },
    ]);
  };

  const save = async (): Promise<void> => {
    if (!name.trim() || !hasClause) return;
    setIsSaving(true);
    try {
      const parsedCommits = Number.parseInt(minCommits, 10);
      const parsedElapsedDays = Number.parseInt(minElapsedDays, 10);
      await onSave({
        name: name.trim(),
        operator,
        min_commits:
          Number.isFinite(parsedCommits) && parsedCommits > 0
            ? parsedCommits
            : undefined,
        min_elapsed_days:
          Number.isFinite(parsedElapsedDays) && parsedElapsedDays > 0
            ? parsedElapsedDays
            : undefined,
        required_commit_types: normalizedTypes,
        allow_first_release: allowFirstRelease,
        required_quality_gates: qualityGates,
      });
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <form
      className="release-rule-form"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label className="field-label">
        Rule name
        <input
          className="text-input"
          value={name}
          maxLength={80}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <div className="release-rule-grid">
        <label className="field-label">
          Combine clauses
          <select
            className="text-input"
            value={operator}
            onChange={(event) => setOperator(event.target.value)}
          >
            <option value="AND">All clauses (AND)</option>
            <option value="OR">Any clause (OR)</option>
          </select>
        </label>
        <label className="field-label">
          Minimum commits
          <input
            className="text-input"
            type="number"
            min={1}
            value={minCommits}
            placeholder="Optional"
            onChange={(event) => setMinCommits(event.target.value)}
          />
        </label>
        <label className="field-label">
          Minimum elapsed days
          <input
            className="text-input"
            type="number"
            min={1}
            value={minElapsedDays}
            placeholder="Optional"
            onChange={(event) => setMinElapsedDays(event.target.value)}
          />
        </label>
      </div>
      <label className="field-label">
        Commit types present
        <input
          className="text-input"
          value={commitTypes}
          placeholder="feat, fix, perf"
          onChange={(event) => setCommitTypes(event.target.value)}
        />
        <small className="field-help">
          Use conventional types: breaking, feat, fix, perf, docs, refactor,
          test, or chore.
        </small>
      </label>
      <label className="checkbox-label">
        <input
          type="checkbox"
          checked={allowFirstRelease}
          onChange={(event) => setAllowFirstRelease(event.target.checked)}
        />
        <span>Allow this rule to evaluate without a published baseline</span>
      </label>
      <div className="quality-rule-gates">
        <div className="quality-rule-gates-heading">
          <div>
            <strong>Required quality gates</strong>
            <small>
              Select one source and minimum evidence level per gate. Blocking
              clauses require a fresh pass; warning clauses remain visible in
              the trace without withholding release eligibility.
            </small>
          </div>
          <button
            className="button button-quiet quality-rule-add"
            type="button"
            onClick={addQualityGate}
            disabled={gateChoices.length === qualityGates.length}
          >
            <Plus size={13} />
            Add gate
          </button>
        </div>
        {qualityGates.length === 0 ? (
          <p className="quality-rule-empty">No quality gates are required.</p>
        ) : (
          <div className="quality-rule-gate-list">
            {qualityGates.map((requirement, index) => (
              <div
                className="quality-rule-gate-row"
                key={`${requirement.gate_id}-${index}`}
              >
                <select
                  className="text-input"
                  aria-label={`Quality gate ${index + 1}`}
                  value={requirement.gate_id}
                  onChange={(event) => {
                    const gateId = event.target.value;
                    setQualityGates((current) =>
                      current.map((item, itemIndex) =>
                        itemIndex === index
                          ? {
                              ...item,
                              gate_id: gateId,
                              ...(gateId === "web_readiness"
                                ? {
                                    source: "QR" as const,
                                    minimum_verification_level:
                                      "deployment_verified" as const,
                                  }
                                : {}),
                            }
                          : item,
                      ),
                    );
                  }}
                >
                  {gateChoices.map((gate) => (
                    <option
                      key={gate.id}
                      value={gate.id}
                      disabled={qualityGates.some(
                        (item, itemIndex) =>
                          itemIndex !== index && item.gate_id === gate.id,
                      )}
                    >
                      {gate.label}
                    </option>
                  ))}
                </select>
                <select
                  className="text-input"
                  aria-label={`Minimum verification for quality gate ${index + 1}`}
                  value={requirement.minimum_verification_level ?? ""}
                  onChange={(event) => {
                    const minimumVerificationLevel = event.target.value as
                      QualityVerificationLevel | "";
                    setQualityGates((current) =>
                      current.map((item, itemIndex) =>
                        itemIndex === index
                          ? {
                              ...item,
                              minimum_verification_level:
                                minimumVerificationLevel || undefined,
                            }
                          : item,
                      ),
                    );
                  }}
                >
                  <option value="">Any evidence level</option>
                  <option value="source_inferred">Source inferred</option>
                  <option value="artifact_inspected">Artifact inspected</option>
                  <option value="browser_rendered">Browser rendered</option>
                  <option value="deployment_verified">
                    Deployment verified
                  </option>
                </select>
                <select
                  className="text-input"
                  aria-label={`Policy for quality gate ${index + 1}`}
                  value={requirement.policy ?? "block"}
                  onChange={(event) => {
                    const policy = event.target
                      .value as QualityRequirementPolicy;
                    setQualityGates((current) =>
                      current.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, policy } : item,
                      ),
                    );
                  }}
                >
                  <option value="block">Block release</option>
                  <option value="warn">Warn only</option>
                </select>
                <select
                  className="text-input"
                  aria-label={`Evidence source for quality gate ${index + 1}`}
                  value={requirement.source}
                  onChange={(event) => {
                    const source = event.target.value as QualitySource;
                    setQualityGates((current) =>
                      current.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, source } : item,
                      ),
                    );
                  }}
                >
                  <option value="CI">CI checks</option>
                  <option value="Local">Local command</option>
                  <option value="QR">QR report</option>
                </select>
                <button
                  className="icon-button quality-gate-remove"
                  type="button"
                  aria-label={`Remove quality gate ${index + 1}`}
                  onClick={() =>
                    setQualityGates((current) =>
                      current.filter((_, itemIndex) => itemIndex !== index),
                    )
                  }
                >
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
      <div className="release-rule-actions">
        <button
          className="button button-primary"
          type="submit"
          disabled={isSaving || !name.trim() || !hasClause}
        >
          <Save size={14} />
          {isSaving ? "Saving…" : "Save deterministic rule"}
        </button>
        {rule && (
          <button
            className="button button-quiet"
            type="button"
            onClick={() => void onSave(null)}
            disabled={isSaving}
          >
            Clear rule
          </button>
        )}
      </div>
    </form>
  );
}
