import type {
  SkillBackfillCapability,
  SkillBackfillPhase,
  SkillFindingCapability,
  SkillFindingClass,
  SkillProviderState,
  SkillQualityRunnerCoverage,
  SkillQualityRunnerRepresentation,
  SkillRecord,
  SkillSource,
  SkillUsage,
  SkillsSnapshot,
} from "./types";

type JsonRecord = Record<string, unknown>;

function asRecord(value: unknown): JsonRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as JsonRecord)
    : {};
}

function stringValue(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim() ? value : fallback;
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value : undefined;
}

function countValue(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.max(0, Math.round(value))
    : 0;
}

function exactCount(value: unknown): number | null {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0
    ? value
    : null;
}

function validTimestamp(value: unknown): string | undefined {
  const timestamp = optionalString(value);
  return timestamp && !Number.isNaN(Date.parse(timestamp))
    ? timestamp
    : undefined;
}

function stringList(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter(
        (item): item is string =>
          typeof item === "string" && item.trim().length > 0,
      )
    : [];
}

function normalizeFindingClass(value: unknown): SkillFindingClass | null {
  const item = asRecord(value);
  const id = optionalString(item.id);
  if (!id) return null;
  return {
    id,
    label: stringValue(item.label, id),
    state: stringValue(item.state, "unknown"),
    evidence: stringValue(
      item.evidence,
      "Finding-class evidence is unavailable.",
    ),
  };
}

function normalizeBackfillPhase(value: unknown): SkillBackfillPhase | null {
  const item = asRecord(value);
  const id = optionalString(item.id);
  if (!id) return null;
  return {
    id,
    state: stringValue(item.state, "unknown"),
    evidence: stringValue(item.evidence, "Backfill evidence is unavailable."),
  };
}

function unknownFindingCapability(): SkillFindingCapability {
  return {
    finding_expectation: "review_required",
    finding_expectation_reason:
      "No reviewed finding profile or Quality Runner capability evidence was found for this skill.",
    finding_classes: [],
    backfill: {
      mode: "not_evidenced",
      phases: [],
      safety:
        "Inventory evidence only; review is required before treating this skill as a finding source.",
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
  };
}

function normalizeFindingCapability(value: unknown): SkillFindingCapability {
  const capability = asRecord(value);
  const fallback = unknownFindingCapability();
  const backfill = asRecord(capability.backfill);
  const qualityRunner = asRecord(capability.quality_runner);
  const coverage = asRecord(qualityRunner.coverage);
  const normalizedBackfill: SkillBackfillCapability = {
    mode: stringValue(backfill.mode, fallback.backfill.mode),
    phases: Array.isArray(backfill.phases)
      ? backfill.phases
          .map(normalizeBackfillPhase)
          .filter((phase): phase is SkillBackfillPhase => phase !== null)
      : fallback.backfill.phases,
    safety: stringValue(backfill.safety, fallback.backfill.safety),
  };
  const normalizedCoverage: SkillQualityRunnerCoverage = {
    rule_count: countValue(coverage.rule_count),
    finding_count: countValue(coverage.finding_count),
    statuses: stringList(coverage.statuses),
  };
  const normalizedQualityRunner: SkillQualityRunnerRepresentation = {
    status: stringValue(qualityRunner.status, fallback.quality_runner.status),
    adapter: stringValue(
      qualityRunner.adapter,
      fallback.quality_runner.adapter,
    ),
    finding_categories: stringList(qualityRunner.finding_categories),
    coverage: normalizedCoverage,
    evidence: stringList(qualityRunner.evidence),
    gaps: stringList(qualityRunner.gaps),
  };
  return {
    finding_expectation: stringValue(
      capability.finding_expectation,
      fallback.finding_expectation,
    ),
    finding_expectation_reason: stringValue(
      capability.finding_expectation_reason,
      fallback.finding_expectation_reason,
    ),
    finding_classes: Array.isArray(capability.finding_classes)
      ? capability.finding_classes
          .map(normalizeFindingClass)
          .filter((item): item is SkillFindingClass => item !== null)
      : [],
    backfill: normalizedBackfill,
    quality_runner: normalizedQualityRunner,
    gaps: stringList(capability.gaps),
  };
}

function normalizeProvider(value: unknown): SkillProviderState {
  const provider = asRecord(value);
  const sourcePath = optionalString(provider.source_path);
  return {
    state: stringValue(provider.state, "unknown"),
    reason: stringValue(provider.reason, "Provider evidence is unavailable."),
    ...(sourcePath ? { source_path: sourcePath } : {}),
  };
}

function normalizeSource(value: unknown): SkillSource | null {
  const source = asRecord(value);
  const path = optionalString(source.path);
  if (!path) return null;
  return {
    path,
    root: stringValue(source.root, "unknown root"),
    provenance: stringValue(source.provenance, "Provenance is unavailable."),
    sha256: stringValue(source.sha256, "unknown"),
    hosted_in_jakye_agent_setup: source.hosted_in_jakye_agent_setup === true,
  };
}

function normalizeUsage(value: unknown): SkillUsage {
  const usage = asRecord(value);
  const state = usage.state === "observed" ? "observed" : "unavailable";
  const unavailable = (reason?: string): SkillUsage => {
    return {
      state: "unavailable",
      recent_count: 0,
      all_time_count: 0,
      by_provider: {},
      telemetry_source:
        "Unavailable; catalog, prompt, and transcript text are never counted as invocations.",
      reason: stringValue(
        reason ?? usage.reason,
        "No installed provider exposes a structured local skill-invocation feed that Pronto can verify.",
      ),
    };
  };
  if (state === "unavailable") {
    return unavailable();
  }
  const telemetrySource = optionalString(usage.telemetry_source);
  const recentCount = exactCount(usage.recent_count);
  const allTimeCount = exactCount(usage.all_time_count);
  const providerEntries = Object.entries(asRecord(usage.by_provider));
  const validProviderEntries = providerEntries.every(
    ([provider, count]) =>
      provider.trim().length > 0 && exactCount(count) !== null,
  );
  const byProvider = Object.fromEntries(
    providerEntries.map(([provider, count]) => [
      provider,
      exactCount(count) ?? 0,
    ]),
  );
  const providerTotal = Object.values(byProvider).reduce(
    (total, count) => total + count,
    0,
  );
  const lastSeenAt = validTimestamp(usage.last_seen_at);
  const timestampIsConsistent =
    allTimeCount === 0
      ? usage.last_seen_at === undefined || usage.last_seen_at === null
      : lastSeenAt !== undefined;
  if (
    !telemetrySource ||
    recentCount === null ||
    allTimeCount === null ||
    recentCount > allTimeCount ||
    providerEntries.length === 0 ||
    !validProviderEntries ||
    providerTotal !== allTimeCount ||
    !timestampIsConsistent
  ) {
    return unavailable(
      "Structured usage evidence was malformed or internally inconsistent, so Pronto discarded its counts.",
    );
  }
  return {
    state,
    recent_count: recentCount,
    all_time_count: allTimeCount,
    by_provider: byProvider,
    ...(lastSeenAt ? { last_seen_at: lastSeenAt } : {}),
    telemetry_source: telemetrySource,
    reason: stringValue(usage.reason, "Structured usage evidence observed."),
  };
}

function normalizeSkill(value: unknown, index: number): SkillRecord {
  const skill = asRecord(value);
  const name = stringValue(skill.name, `Unnamed skill ${index + 1}`);
  const parityScore =
    typeof skill.parity_score === "number" &&
    Number.isFinite(skill.parity_score)
      ? skill.parity_score
      : null;
  const parityEvidence = Array.isArray(skill.parity_evidence)
    ? skill.parity_evidence.filter(
        (evidence): evidence is string =>
          typeof evidence === "string" && evidence.trim().length > 0,
      )
    : [];

  return {
    id: stringValue(skill.id, name),
    name,
    description: stringValue(
      skill.description,
      "No description recorded for this skill.",
    ),
    category: stringValue(skill.category, "Other"),
    family: stringValue(skill.family, "Standalone"),
    lifecycle: stringValue(skill.lifecycle, "unknown"),
    hosted_in_jakye_agent_setup: skill.hosted_in_jakye_agent_setup === true,
    sources: Array.isArray(skill.sources)
      ? skill.sources
          .map(normalizeSource)
          .filter((source): source is SkillSource => source !== null)
      : [],
    providers: Object.fromEntries(
      Object.entries(asRecord(skill.providers)).map(([provider, value]) => [
        provider,
        normalizeProvider(value),
      ]),
    ),
    parity_score: parityScore,
    parity_evidence:
      parityEvidence.length > 0
        ? parityEvidence
        : ["Parity evidence is unavailable."],
    usage: normalizeUsage(skill.usage),
    finding_capability: normalizeFindingCapability(skill.finding_capability),
  };
}

export function normalizeSkillsSnapshot(value: unknown): SkillsSnapshot {
  const snapshot = asRecord(value);
  const generatedAt = stringValue(
    snapshot.generated_at,
    new Date().toISOString(),
  );
  const refreshedAt = optionalString(snapshot.refreshed_at);

  return {
    schema_version: stringValue(snapshot.schema_version, "pronto-skills/v4"),
    generated_at: generatedAt,
    ...(refreshedAt ? { refreshed_at: refreshedAt } : {}),
    freshness: stringValue(
      snapshot.freshness,
      "Freshness is unavailable until the next skills refresh.",
    ),
    source: stringValue(snapshot.source, "Local skill roots"),
    recent_days: countValue(snapshot.recent_days) || 30,
    roots: Array.isArray(snapshot.roots)
      ? snapshot.roots.filter(
          (root): root is string =>
            typeof root === "string" && root.trim().length > 0,
        )
      : [],
    skills: Array.isArray(snapshot.skills)
      ? snapshot.skills.map(normalizeSkill)
      : [],
    telemetry_gap: stringValue(
      snapshot.telemetry_gap,
      "No local telemetry gap has been recorded.",
    ),
  };
}
