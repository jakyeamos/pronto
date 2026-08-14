use crate::core::RepositorySnapshot;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ASSESSMENT_SCHEMA: &str = "quality-runner-behavior-assurance/v2";
pub const SUMMARY_SCHEMA: &str = "quality-runner-behavior-assurance-summary/v2";
pub const AUDIT_SCHEMA: &str = "pronto-behavior-assurance-audit/v2";

fn default_behavior_assurance_state() -> String {
    "unknown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorAssuranceGap {
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub behavior_id: Option<String>,
    #[serde(default)]
    pub scenario_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorAssuranceVerification {
    pub behavior_id: String,
    pub scenario_id: String,
    pub status: String,
    pub verification_level: String,
    pub receipt_id: String,
    pub receipt_commit: String,
    pub carried_forward: bool,
    #[serde(default)]
    pub accepted_defect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorCoverageCounts {
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub profiled: usize,
    #[serde(default)]
    pub verified: usize,
    #[serde(default)]
    pub stale: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub blocked: usize,
    #[serde(default)]
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorScenarioCoverage {
    pub behavior_id: String,
    pub scenario_id: String,
    pub tier: usize,
    #[serde(default)]
    pub profiled: bool,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub side_effects: Option<String>,
    pub status: String,
    #[serde(default)]
    pub verification_level: Option<String>,
    #[serde(default)]
    pub receipt_id: Option<String>,
    pub freshness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorCategoryGap {
    pub category: String,
    pub scenario_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorCoverage {
    #[serde(flatten)]
    pub counts: BehaviorCoverageCounts,
    #[serde(default)]
    pub profile_status: String,
    #[serde(default)]
    pub per_tier: BTreeMap<String, BehaviorCoverageCounts>,
    #[serde(default)]
    pub per_edge_category: BTreeMap<String, BehaviorCoverageCounts>,
    #[serde(default)]
    pub category_gaps: Vec<BehaviorCategoryGap>,
    #[serde(default)]
    pub scenarios: Vec<BehaviorScenarioCoverage>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAssuranceRepositoryState {
    pub schema: String,
    pub applicability: String,
    #[serde(default = "default_behavior_assurance_state")]
    pub state: String,
    pub contract_status: String,
    #[serde(default)]
    pub contract_schema: Option<String>,
    #[serde(default)]
    pub edge_profile_status: String,
    pub result_status: String,
    pub freshness: String,
    pub release_ready: bool,
    pub score: Option<f64>,
    pub contract_path: String,
    pub receipt_directory: String,
    #[serde(default)]
    pub contract_digest: Option<String>,
    #[serde(default)]
    pub target_branch: Option<String>,
    #[serde(default)]
    pub target_commit: Option<String>,
    #[serde(default)]
    pub observed_at: Option<String>,
    #[serde(default)]
    pub required_scenario_count: usize,
    #[serde(default)]
    pub passed_scenario_count: usize,
    #[serde(default)]
    pub accepted_defect_count: usize,
    #[serde(default)]
    pub receipt_count: usize,
    #[serde(default)]
    pub verified: Vec<BehaviorAssuranceVerification>,
    #[serde(default)]
    pub coverage: BehaviorCoverage,
    #[serde(default)]
    pub gaps: Vec<BehaviorAssuranceGap>,
    #[serde(default)]
    pub detail: Option<String>,
    pub next_step: String,
}

impl Default for BehaviorAssuranceRepositoryState {
    fn default() -> Self {
        Self {
            schema: ASSESSMENT_SCHEMA.to_string(),
            applicability: "unknown".to_string(),
            state: "unknown".to_string(),
            contract_status: "missing".to_string(),
            contract_schema: None,
            edge_profile_status: "missing".to_string(),
            result_status: "unknown".to_string(),
            freshness: "unknown".to_string(),
            release_ready: false,
            score: None,
            contract_path: ".pronto/behavior-assurance.json".to_string(),
            receipt_directory: ".quality-runner/behavior-assurance/receipts".to_string(),
            contract_digest: None,
            target_branch: None,
            target_commit: None,
            observed_at: None,
            required_scenario_count: 0,
            passed_scenario_count: 0,
            accepted_defect_count: 0,
            receipt_count: 0,
            verified: Vec::new(),
            coverage: BehaviorCoverage::default(),
            gaps: vec![BehaviorAssuranceGap {
                kind: "evidence_unavailable".to_string(),
                message: "No current Quality Runner behavior-assurance projection is available."
                    .to_string(),
                behavior_id: None,
                scenario_id: None,
            }],
            detail: None,
            next_step: "Run and publish a current Quality Runner fleet audit.".to_string(),
        }
    }
}

impl BehaviorAssuranceRepositoryState {
    pub fn normalize_state(&mut self) {
        if self.state != "unknown" && !self.state.is_empty() {
            return;
        }
        self.state = inferred_state(self).to_string();
    }

    pub fn project_to_target(&mut self, branch: &str, commit: &str) {
        if self.applicability == "not_applicable"
            || self.receipt_count == 0
            || self.target_branch.is_none()
            || self.target_commit.is_none()
            || (self.target_branch.as_deref() == Some(branch)
                && self.target_commit.as_deref() == Some(commit))
        {
            return;
        }
        self.release_ready = false;
        self.result_status = "unknown".to_string();
        self.freshness = "stale".to_string();
        self.state = "stale".to_string();
        mark_coverage_stale(&mut self.coverage);
        self.gaps.push(BehaviorAssuranceGap {
            kind: "target_mismatch".to_string(),
            message: format!(
                "Behavior receipts target {} @ {}, not {branch} @ {commit}.",
                self.target_branch.as_deref().unwrap_or("unknown"),
                self.target_commit.as_deref().unwrap_or("unknown")
            ),
            behavior_id: None,
            scenario_id: None,
        });
        self.next_step =
            "Publish Quality Runner behavior assurance for the configured target.".to_string();
    }
}

fn inferred_state(state: &BehaviorAssuranceRepositoryState) -> &'static str {
    if state.contract_status == "missing"
        && state.gaps.iter().any(|gap| gap.kind == "contract_missing")
    {
        return "missing_contract";
    }
    if state.contract_status == "invalid" || state.result_status == "blocked" {
        return "blocked";
    }
    if state.applicability == "not_applicable" {
        return "not_applicable";
    }
    if state.schema == "quality-runner-behavior-assurance/v1"
        || state.contract_schema.as_deref() == Some("pronto-behavior-assurance/v1")
        || state.edge_profile_status == "legacy"
    {
        return "legacy_v1";
    }
    if state.result_status == "failed" {
        return "failed";
    }
    if state.freshness == "stale" {
        return "stale";
    }
    if state.passed_scenario_count > 0
        && state.passed_scenario_count < state.required_scenario_count
    {
        return "partially_verified";
    }
    if matches!(
        state.edge_profile_status.as_str(),
        "missing" | "unprofiled" | "partially_profiled"
    ) {
        return "unprofiled";
    }
    if state.release_ready {
        return "current";
    }
    "unknown"
}

fn mark_coverage_stale(coverage: &mut BehaviorCoverage) {
    coverage.counts.stale += coverage.counts.verified;
    coverage.counts.verified = 0;
    for counts in coverage
        .per_tier
        .values_mut()
        .chain(coverage.per_edge_category.values_mut())
    {
        counts.stale += counts.verified;
        counts.verified = 0;
    }
    for scenario in &mut coverage.scenarios {
        if scenario.status == "verified" {
            scenario.status = "stale".to_string();
            scenario.freshness = "stale".to_string();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAssurancePortfolioState {
    pub schema: String,
    pub status: String,
    pub repository_count: usize,
    pub ready_repository_count: usize,
    #[serde(default)]
    pub applicability_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub result_status_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub contract_schema_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub edge_profile_status_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub state_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub required_scenario_count: usize,
    #[serde(default)]
    pub passed_scenario_count: usize,
    #[serde(default)]
    pub gap_count: usize,
    #[serde(default)]
    pub coverage: BehaviorCoverageCounts,
}

impl Default for BehaviorAssurancePortfolioState {
    fn default() -> Self {
        Self {
            schema: SUMMARY_SCHEMA.to_string(),
            status: "unavailable".to_string(),
            repository_count: 0,
            ready_repository_count: 0,
            applicability_counts: Default::default(),
            result_status_counts: Default::default(),
            contract_schema_counts: Default::default(),
            edge_profile_status_counts: Default::default(),
            state_counts: Default::default(),
            required_scenario_count: 0,
            passed_scenario_count: 0,
            gap_count: 0,
            coverage: BehaviorCoverageCounts::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BehaviorAssuranceAuditRepository {
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub assurance: BehaviorAssuranceRepositoryState,
}

#[derive(Debug, Clone, Serialize)]
pub struct BehaviorAssuranceAuditReport {
    pub schema_version: String,
    pub generated_at: String,
    pub status: String,
    pub ready: bool,
    pub filter: Option<String>,
    pub fleet_repository_count: usize,
    pub repository_count: usize,
    pub ready_repository_count: usize,
    pub gap_count: usize,
    pub coverage: BehaviorCoverageCounts,
    pub repositories: Vec<BehaviorAssuranceAuditRepository>,
    pub next_safe_step: String,
}

pub const AUDIT_FILTERS: &[&str] = &[
    "missing",
    "legacy",
    "unprofiled",
    "partially_verified",
    "stale",
    "failed",
    "blocked",
    "unknown",
    "current",
    "not_applicable",
];

pub fn audit_report(
    repositories: &[RepositorySnapshot],
    filter: Option<&str>,
) -> BehaviorAssuranceAuditReport {
    let selected: Vec<&RepositorySnapshot> = repositories
        .iter()
        .filter(|repository| matches_filter(&repository.quality.behavior_assurance, filter))
        .collect();
    let repository_count = selected.len();
    let ready_repository_count = selected
        .iter()
        .filter(|repository| repository.quality.behavior_assurance.release_ready)
        .count();
    let gap_count = selected
        .iter()
        .map(|repository| repository.quality.behavior_assurance.gaps.len())
        .sum();
    let mut coverage = BehaviorCoverageCounts::default();
    for repository in &selected {
        let mut assurance = repository.quality.behavior_assurance.clone();
        assurance.normalize_state();
        add_counts(&mut coverage, &assurance.coverage.counts);
    }
    let ready = if filter.is_some() {
        repository_count == 0 || ready_repository_count == repository_count
    } else {
        repository_count > 0 && ready_repository_count == repository_count
    };
    let status = if repository_count == 0 && filter.is_some() {
        "No matches"
    } else if ready {
        "Ready"
    } else {
        "Gaps present"
    };
    BehaviorAssuranceAuditReport {
        schema_version: AUDIT_SCHEMA.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        status: status.to_string(),
        ready,
        filter: filter.map(str::to_string),
        fleet_repository_count: repositories.len(),
        repository_count,
        ready_repository_count,
        gap_count,
        coverage,
        repositories: selected
            .iter()
            .map(|repository| BehaviorAssuranceAuditRepository {
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                assurance: {
                    let mut assurance = repository.quality.behavior_assurance.clone();
                    assurance.normalize_state();
                    assurance
                },
            })
            .collect(),
        next_safe_step: if ready {
            "No selected behavior-assurance action is required."
        } else {
            "Review release gaps and edge-coverage gaps separately, then publish a fresh Quality Runner fleet feed."
        }
        .to_string(),
    }
}

fn matches_filter(state: &BehaviorAssuranceRepositoryState, filter: Option<&str>) -> bool {
    let mut state = state.clone();
    state.normalize_state();
    match filter {
        None => true,
        Some("missing") => state.state == "missing_contract",
        Some("legacy") => {
            state.state == "legacy_v1"
                || state.contract_schema.as_deref() == Some("pronto-behavior-assurance/v1")
                || state.edge_profile_status == "legacy"
        }
        Some("unprofiled") => state.state == "unprofiled",
        Some("partially_verified") => state.state == "partially_verified",
        Some("stale") => {
            state.state == "stale"
                || (state.receipt_count > 0
                    && (state.freshness == "stale" || state.coverage.counts.stale > 0))
        }
        Some("failed") => {
            state.state == "failed"
                || state.result_status == "failed"
                || state.coverage.counts.failed > 0
        }
        Some("blocked") => {
            state.state == "blocked"
                || state.result_status == "blocked"
                || state.coverage.counts.blocked > 0
        }
        Some("unknown") => state.state == "unknown",
        Some("current") => state.state == "current",
        Some("not_applicable") => state.state == "not_applicable",
        Some(_) => false,
    }
}

fn add_counts(target: &mut BehaviorCoverageCounts, source: &BehaviorCoverageCounts) {
    target.total += source.total;
    target.profiled += source.profiled;
    target.verified += source.verified;
    target.stale += source.stale;
    target.failed += source.failed;
    target.blocked += source.blocked;
    target.unknown += source.unknown;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_legacy_missing_and_not_applicable_states_without_collapsing_them() {
        let mut missing = BehaviorAssuranceRepositoryState {
            gaps: vec![BehaviorAssuranceGap {
                kind: "contract_missing".to_string(),
                ..BehaviorAssuranceGap::default()
            }],
            ..BehaviorAssuranceRepositoryState::default()
        };
        missing.normalize_state();
        assert_eq!(missing.state, "missing_contract");

        let mut legacy = BehaviorAssuranceRepositoryState {
            schema: "quality-runner-behavior-assurance/v1".to_string(),
            applicability: "applicable".to_string(),
            contract_status: "current".to_string(),
            freshness: "stale".to_string(),
            ..BehaviorAssuranceRepositoryState::default()
        };
        legacy.normalize_state();
        assert_eq!(legacy.state, "legacy_v1");

        let mut not_applicable = BehaviorAssuranceRepositoryState {
            applicability: "not_applicable".to_string(),
            contract_status: "current".to_string(),
            schema: "quality-runner-behavior-assurance/v1".to_string(),
            ..BehaviorAssuranceRepositoryState::default()
        };
        not_applicable.normalize_state();
        assert_eq!(not_applicable.state, "not_applicable");
    }

    #[test]
    fn target_mismatch_removes_release_and_edge_freshness_claims() {
        let mut state = BehaviorAssuranceRepositoryState {
            applicability: "applicable".to_string(),
            contract_status: "current".to_string(),
            result_status: "passed".to_string(),
            freshness: "current".to_string(),
            release_ready: true,
            target_branch: Some("dev".to_string()),
            target_commit: Some("abc".to_string()),
            receipt_count: 1,
            gaps: Vec::new(),
            coverage: BehaviorCoverage {
                counts: BehaviorCoverageCounts {
                    total: 1,
                    profiled: 1,
                    verified: 1,
                    ..BehaviorCoverageCounts::default()
                },
                scenarios: vec![BehaviorScenarioCoverage {
                    status: "verified".to_string(),
                    freshness: "current".to_string(),
                    ..BehaviorScenarioCoverage::default()
                }],
                ..BehaviorCoverage::default()
            },
            ..BehaviorAssuranceRepositoryState::default()
        };

        state.project_to_target("dev", "def");

        assert!(!state.release_ready);
        assert_eq!(state.freshness, "stale");
        assert_eq!(state.coverage.counts.verified, 0);
        assert_eq!(state.coverage.counts.stale, 1);
        assert_eq!(state.coverage.scenarios[0].status, "stale");
        assert_eq!(state.gaps[0].kind, "target_mismatch");
    }

    #[test]
    fn missing_provenance_is_not_misclassified_as_a_stale_receipt() {
        let mut state = BehaviorAssuranceRepositoryState::default();

        state.project_to_target("dev", "abc");

        assert_eq!(state.freshness, "unknown");
        assert_eq!(state.coverage.counts.stale, 0);
        assert_eq!(state.gaps[0].kind, "evidence_unavailable");
        assert!(!state.gaps.iter().any(|gap| gap.kind == "target_mismatch"));
        assert!(!matches_filter(&state, Some("stale")));
    }
}
