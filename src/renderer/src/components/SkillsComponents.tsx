import { useMemo, useState, type ReactElement } from "react";
import { ArrowLeft, RefreshCw, Search, Sparkles } from "lucide-react";
import { normalizeSkillsSnapshot } from "../skillsSnapshot";
import type { SkillRecord } from "../types";
import { formatTime } from "./ConsolePrimitives";
import { PapercutSurface } from "./PapercutSurface";
import { SkillsErrorBoundary } from "./SkillsErrorBoundary";

import {
  PAPERCUTS_SKILL_ID,
  SkillDetail,
  SkillsTable,
  categoryRank,
  providerTone,
  skillCountLabel,
  type SkillGroup,
  type SkillsSurfaceProps,
  type SortKey,
} from "./SkillsPresentation";

function SkillsSurfaceContent({
  snapshot,
  isRefreshing,
  onRefresh,
  onOpenSource,
  papercutBacklog,
  onRefreshPapercutBacklog,
  onCreatePapercut,
  onPapercutStatusChange,
  onMultiplierProposalStatusChange,
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
  const papercutsSelected = selected?.id === PAPERCUTS_SKILL_ID;
  const normalizedQuery = query.trim().toLowerCase();
  const usageEvidenceAvailable = safeSkills.some(
    (skill) => skill.usage.state === "observed",
  );
  const effectiveSort =
    !usageEvidenceAvailable && (sort === "recent" || sort === "allTime")
      ? "name"
      : sort;
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
        if (effectiveSort === "recent")
          return right.usage.recent_count - left.usage.recent_count;
        if (effectiveSort === "allTime")
          return right.usage.all_time_count - left.usage.all_time_count;
        if (effectiveSort === "parity")
          return (right.parity_score ?? -1) - (left.parity_score ?? -1);
        return left.name.localeCompare(right.name);
      });
  }, [effectiveSort, normalizedQuery, safeSkills]);
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
          <span>Usage evidence</span>
          <strong>
            {usageEvidenceAvailable
              ? safeSkills.reduce(
                  (total, skill) =>
                    total +
                    (skill.usage.state === "observed"
                      ? skill.usage.recent_count
                      : 0),
                  0,
                )
              : "Unavailable"}
          </strong>
          <small>
            {usageEvidenceAvailable
              ? `recorded in last ${normalizedSnapshot.recent_days} days`
              : "structured provider feed required"}
          </small>
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
      {papercutsSelected ? (
        <div className="skills-papercut-route">
          <div className="skills-papercut-route-heading">
            <div>
              <span className="eyebrow">Skill detail · Design audit</span>
              <h2>Papercuts</h2>
              <p>
                The durable backlog for repeatable small hurts found by the
                design-friction audit.
              </p>
            </div>
            <button
              className="button button-secondary"
              type="button"
              onClick={() => setSelected(null)}
            >
              <ArrowLeft size={15} />
              All skills
            </button>
          </div>
          <PapercutSurface
            backlog={papercutBacklog}
            isRefreshing={isRefreshing}
            onRefresh={onRefreshPapercutBacklog}
            onCreate={onCreatePapercut}
            onStatusChange={onPapercutStatusChange}
            onProposalStatusChange={onMultiplierProposalStatusChange}
          />
        </div>
      ) : (
        <>
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
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Find a skill"
                aria-label="Find a skill"
              />
            </label>
            <label className="skills-sort">
              Sort
              <select
                value={effectiveSort}
                onChange={(event) => setSort(event.target.value as SortKey)}
              >
                <option value="name">Name</option>
                {usageEvidenceAvailable ? (
                  <>
                    <option value="recent">Recent usage</option>
                    <option value="allTime">Recorded usage</option>
                  </>
                ) : null}
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
            <section
              className="skills-table-wrap"
              aria-label="Skills inventory"
            >
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
                          <span
                            className="skills-disclosure"
                            aria-hidden="true"
                          />
                          <span>
                            <strong>{group.category}</strong>
                            <small>
                              {expandableFamilies.length > 0
                                ? expandableFamilies.length +
                                  " family groups · "
                                : null}
                              {skillCountLabel(
                                group.families.reduce(
                                  (total, family) =>
                                    total + family.skills.length,
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
              <SkillDetail
                selected={selected}
                onClose={() => setSelected(null)}
                onOpenSource={onOpenSource}
              />
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
        </>
      )}
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
