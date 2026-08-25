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
    pub resilience_profile_status_counts: BTreeMap<String, usize>,
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
            resilience_profile_status_counts: Default::default(),
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
    pub resilience_profiled_scenario_count: usize,
    pub resilience_profile_status_counts: BTreeMap<String, usize>,
    pub resilience_dimension_counts: BTreeMap<String, BehaviorResilienceDimensionCounts>,
    pub repositories: Vec<BehaviorAssuranceAuditRepository>,
    pub next_safe_step: String,
}

pub const AUDIT_FILTERS: &[&str] = &[
    "missing",
    "legacy",
    "unprofiled",
    "resilience_unprofiled",
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
    let mut resilience_profiled_scenario_count = 0;
    let mut resilience_profile_status_counts = BTreeMap::new();
    let mut resilience_dimension_counts = BTreeMap::new();
    for repository in &selected {
        let mut assurance = repository.quality.behavior_assurance.clone();
        assurance.normalize_state();
        add_counts(&mut coverage, &assurance.coverage.counts);
        resilience_profiled_scenario_count += assurance.coverage.resilience_profiled;
        *resilience_profile_status_counts
            .entry(normalized_resilience_status(&assurance).to_string())
            .or_insert(0) += 1;
        add_resilience_dimensions(
            &mut resilience_dimension_counts,
            &assurance.coverage.per_resilience_dimension,
        );
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
        resilience_profiled_scenario_count,
        resilience_profile_status_counts,
        resilience_dimension_counts,
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
            "Review release, edge-coverage, and systemic-resilience gaps separately, then publish a fresh Quality Runner fleet feed."
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
        Some("resilience_unprofiled") => matches!(
            normalized_resilience_status(&state),
            "missing" | "unprofiled" | "partially_profiled" | "legacy"
        ),
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

pub(crate) fn normalized_resilience_status(state: &BehaviorAssuranceRepositoryState) -> &str {
    if !state.resilience_profile_status.is_empty() {
        state.resilience_profile_status.as_str()
    } else if !state.coverage.resilience_profile_status.is_empty() {
        state.coverage.resilience_profile_status.as_str()
    } else {
        "missing"
    }
}

fn add_resilience_dimensions(
    target: &mut BTreeMap<String, BehaviorResilienceDimensionCounts>,
    source: &BTreeMap<String, BehaviorResilienceDimensionCounts>,
) {
    for (dimension, counts) in source {
        let entry = target.entry(dimension.clone()).or_default();
        entry.scenario_count += counts.scenario_count;
        entry.item_count += counts.item_count;
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
