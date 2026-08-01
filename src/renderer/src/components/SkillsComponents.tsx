import { useMemo, useState, type ReactElement } from "react";
import { ExternalLink, RefreshCw, Search, Sparkles } from "lucide-react";
import { normalizeSkillsSnapshot } from "../skillsSnapshot";
import type { SkillRecord, SkillsSnapshot } from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";
import { SkillsErrorBoundary } from "./SkillsErrorBoundary";

type SortKey = "name" | "recent" | "allTime" | "parity";

type SkillGroup = {
  category: string;
  families: Array<{ family: string; skills: SkillRecord[] }>;
};

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

function categoryRank(category: string): number {
  const index = CATEGORY_ORDER.indexOf(category);
  return index === -1 ? CATEGORY_ORDER.length : index;
}

function providerTone(state?: string): string {
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

function skillCountLabel(count: number): string {
  return String(count) + " " + (count === 1 ? "skill" : "skills");
}

type SkillsSurfaceProps = {
  snapshot: SkillsSnapshot;
  isRefreshing: boolean;
  onRefresh: () => void;
  onOpenSource: (path: string) => void;
};

function SkillsTable({
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
          <th>Evidence</th>
        </tr>
      </thead>
      <tbody>
        {skills.map((skill) => (
          <tr
            key={skill.id}
            className={selectedId === skill.id ? "is-selected" : undefined}
            tabIndex={0}
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
              <strong>{skill.usage.recent_count}</strong>
              <small>
                {skill.usage.all_time_count} all-time
                {skill.usage.last_seen_at
                  ? " · " + formatTime(skill.usage.last_seen_at)
                  : " · no observation"}
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

function SkillsSurfaceContent({
  snapshot,
  isRefreshing,
  onRefresh,
  onOpenSource,
}: SkillsSurfaceProps): ReactElement {
  const normalizedSnapshot = useMemo(
    () => normalizeSkillsSnapshot(snapshot),
    [snapshot],
  );
  const safeSkills = normalizedSnapshot.skills;
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortKey>("name");
  const [selected, setSelected] = useState<SkillRecord | null>(null);
  const [expandedCategories, setExpandedCategories] = useState<
    Record<string, boolean>
  >({});
  const [expandedFamilies, setExpandedFamilies] = useState<
    Record<string, boolean>
  >({});
  const normalizedQuery = query.trim().toLowerCase();
  const skills = useMemo(() => {
    return safeSkills
      .filter(
        (skill) =>
          !normalizedQuery ||
          `${skill.name} ${skill.description}`
            .toLowerCase()
            .includes(normalizedQuery),
      )
      .sort((left, right) => {
        if (sort === "recent")
          return right.usage.recent_count - left.usage.recent_count;
        if (sort === "allTime")
          return right.usage.all_time_count - left.usage.all_time_count;
        if (sort === "parity")
          return (right.parity_score ?? -1) - (left.parity_score ?? -1);
        return left.name.localeCompare(right.name);
      });
  }, [normalizedQuery, safeSkills, sort]);
  const skillGroups = useMemo<SkillGroup[]>(() => {
    const categories = new Map<string, Map<string, SkillRecord[]>>();
    skills.forEach((skill) => {
      const category = skill.category || "Other";
      const family = skill.family || "Standalone";
      const families = categories.get(category) ?? new Map();
      const familySkills = families.get(family) ?? [];
      familySkills.push(skill);
      families.set(family, familySkills);
      categories.set(category, families);
    });
    return Array.from(categories.entries())
      .sort(
        ([left], [right]) =>
          categoryRank(left) - categoryRank(right) || left.localeCompare(right),
      )
      .map(([category, families]) => ({
        category,
        families: Array.from(families.entries())
          .sort(([left], [right]) => left.localeCompare(right))
          .map(([family, familySkills]) => ({
            family,
            skills: familySkills,
          })),
      }));
  }, [skills]);
  const expandableFamilyCount = skillGroups.reduce(
    (total, group) =>
      total +
      group.families.filter((family) => family.skills.length > 1).length,
    0,
  );
  const hostedCount = safeSkills.filter(
    (skill) => skill.hosted_in_jakye_agent_setup,
  ).length;
  const providerStates = safeSkills.reduce<Record<string, number>>(
    (counts, skill) => {
      Object.values(skill.providers).forEach((provider) => {
        counts[provider.state] = (counts[provider.state] ?? 0) + 1;
      });
      return counts;
    },
    {},
  );

  return (
    <div className="skills-surface">
      <div className="skills-overview">
        <div>
          <span>Skills</span>
          <strong>{safeSkills.length}</strong>
          <small>logical skills</small>
        </div>
        <div>
          <span>Hosted</span>
          <strong>{hostedCount}</strong>
          <small>in jakye-agent-setup</small>
        </div>
        <div>
          <span>Observed use</span>
          <strong>
            {safeSkills.reduce(
              (total, skill) => total + skill.usage.recent_count,
              0,
            )}
          </strong>
          <small>last {normalizedSnapshot.recent_days} days</small>
        </div>
        <div>
          <span>Freshness</span>
          <strong>
            {normalizedSnapshot.refreshed_at
              ? formatTime(normalizedSnapshot.refreshed_at)
              : "—"}
          </strong>
          <small>{normalizedSnapshot.freshness}</small>
        </div>
      </div>
      <p className="skills-group-summary">
        {skillGroups.length} categories
        {expandableFamilyCount > 0
          ? " · " + expandableFamilyCount + " family groups"
          : null}{" "}
        · matching {skillCountLabel(skills.length)}
      </p>
      <div className="skills-toolbar">
        <label className="skills-search">
          <Search size={15} />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Find a skill"
            aria-label="Find a skill"
          />
        </label>
        <label className="skills-sort">
          Sort
          <select
            value={sort}
            onChange={(event) => setSort(event.target.value as SortKey)}
          >
            <option value="name">Name</option>
            <option value="recent">Recent usage</option>
            <option value="allTime">All-time usage</option>
            <option value="parity">Parity evidence</option>
          </select>
        </label>
        <button
          className="button button-secondary"
          type="button"
          onClick={onRefresh}
          disabled={isRefreshing}
        >
          <RefreshCw size={15} />
          {isRefreshing ? "Refreshing…" : "Refresh skills"}
        </button>
      </div>
      <p className="skills-freshness">
        <Sparkles size={14} />
        {normalizedSnapshot.telemetry_gap}
      </p>
      <div className="skills-layout">
        <section className="skills-table-wrap" aria-label="Skills inventory">
          {skills.length === 0 ? (
            <div className="empty-state empty-state-compact">
              <h2>No skills indexed yet</h2>
              <p>
                Refresh to scan the configured canonical, provider, managed,
                legacy, and hosted roots.
              </p>
            </div>
          ) : (
            <div className="skills-groups">
              {skillGroups.map((group) => {
                const categoryOpen =
                  normalizedQuery.length > 0 ||
                  expandedCategories[group.category] !== false;
                const singletonSkills = group.families
                  .filter((familyGroup) => familyGroup.skills.length === 1)
                  .flatMap((familyGroup) => familyGroup.skills);
                const expandableFamilies = group.families.filter(
                  (familyGroup) => familyGroup.skills.length > 1,
                );
                return (
                  <details
                    className="skills-category"
                    key={group.category}
                    open={categoryOpen}
                    onToggle={(event) => {
                      if (normalizedQuery) return;
                      const isOpen = event.currentTarget.open;
                      setExpandedCategories((current) => ({
                        ...current,
                        [group.category]: isOpen,
                      }));
                    }}
                  >
                    <summary className="skills-category-summary">
                      <span className="skills-disclosure" aria-hidden="true" />
                      <span>
                        <strong>{group.category}</strong>
                        <small>
                          {expandableFamilies.length > 0
                            ? expandableFamilies.length + " family groups · "
                            : null}
                          {skillCountLabel(
                            group.families.reduce(
                              (total, family) => total + family.skills.length,
                              0,
                            ),
                          )}
                        </small>
                      </span>
                    </summary>
                    <div className="skills-families">
                      {singletonSkills.length > 0 ? (
                        <div className="skills-singletons">
                          <SkillsTable
                            skills={singletonSkills}
                            selectedId={selected?.id ?? null}
                            onSelect={setSelected}
                          />
                        </div>
                      ) : null}
                      {expandableFamilies.map((familyGroup) => {
                        const familyKey =
                          group.category + ":" + familyGroup.family;
                        const familyOpen =
                          normalizedQuery.length > 0 ||
                          expandedFamilies[familyKey] !== false;
                        return (
                          <details
                            className="skills-family"
                            key={familyGroup.family}
                            open={familyOpen}
                            onToggle={(event) => {
                              if (normalizedQuery) return;
                              const isOpen = event.currentTarget.open;
                              setExpandedFamilies((current) => ({
                                ...current,
                                [familyKey]: isOpen,
                              }));
                            }}
                          >
                            <summary className="skills-family-summary">
                              <span
                                className="skills-disclosure"
                                aria-hidden="true"
                              />
                              <strong>{familyGroup.family}</strong>
                              <span>{familyGroup.skills.length}</span>
                            </summary>
                            <SkillsTable
                              skills={familyGroup.skills}
                              selectedId={selected?.id ?? null}
                              onSelect={setSelected}
                            />
                          </details>
                        );
                      })}
                    </div>
                  </details>
                );
              })}
            </div>
          )}
        </section>
        {selected ? (
          <aside
            className="skills-detail"
            aria-label={`${selected.name} details`}
          >
            <div className="skills-detail-heading">
              <div>
                <span className="eyebrow">Selected skill</span>
                <h2>{selected.name}</h2>
              </div>
              <button
                className="icon-button"
                type="button"
                aria-label="Close skill details"
                onClick={() => setSelected(null)}
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
                  {selected.usage.recent_count} recent ·{" "}
                  {selected.usage.all_time_count} all-time
                </dd>
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
        ) : null}
      </div>
      <div className="skills-legend">
        {Object.entries(providerStates).map(([state, count]) => (
          <span key={state}>
            <i className={`skills-dot skills-dot-${providerTone(state)}`} />
            {state} · {count}
          </span>
        ))}
      </div>
    </div>
  );
}

export function SkillsSurface(props: SkillsSurfaceProps): ReactElement {
  const snapshot = normalizeSkillsSnapshot(props.snapshot);
  return (
    <SkillsErrorBoundary onRefresh={props.onRefresh}>
      <SkillsSurfaceContent {...props} snapshot={snapshot} />
    </SkillsErrorBoundary>
  );
}
