import type {
  SkillProviderState,
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
  const byProvider = Object.fromEntries(
    Object.entries(asRecord(usage.by_provider)).map(([provider, count]) => [
      provider,
      countValue(count),
    ]),
  );
  const lastSeenAt = optionalString(usage.last_seen_at);
  return {
    recent_count: countValue(usage.recent_count),
    all_time_count: countValue(usage.all_time_count),
    by_provider: byProvider,
    ...(lastSeenAt ? { last_seen_at: lastSeenAt } : {}),
    telemetry_source: stringValue(
      usage.telemetry_source,
      "No local telemetry observed.",
    ),
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
    schema_version: stringValue(snapshot.schema_version, "pronto-skills/v2"),
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
