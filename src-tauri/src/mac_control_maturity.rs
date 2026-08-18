use crate::core::RepositorySnapshot;
use crate::evidence_contract::{
    aggregate_contract_coverage, evaluate_repository_contract, EvidenceContractFleetCoverage,
    EvidenceContractRepositoryStatus, EVIDENCE_CONTRACT_STATUS_CURRENT,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const MAC_CONTROL_GATE_ID: &str = "mac_control_ideal_state";
pub const MAC_CONTROL_GATE_LABEL: &str = "Mac Control ideal state";
pub const MAC_CONTROL_SCHEMA: &str = "pronto-mac-control-ideal-state/v1";
pub const MAC_CONTROL_EVIDENCE_RELATIVE_PATH: &str =
    ".quality-runner/fleet-audit/current/mac-control-ideal-state.json";
pub const MAC_CONTROL_MAX_EVIDENCE_AGE_DAYS: i64 = 7;
pub const MAC_CONTROL_TASK_CONTRACT_ID: &str = "mac-control-task-manifest";
pub const MAC_CONTROL_TASK_CONTRACT_LABEL: &str = "Mac Control task evidence";
pub const MAC_CONTROL_TASK_MANIFEST_SCHEMA: &str = "mac-control-task-manifest/v4";

pub const MAC_CONTROL_CRITERION_IDS: [&str; 8] = [
    "stable_identity",
    "correct_semantics",
    "observable_state",
    "useful_hierarchy",
    "efficient_navigation",
    "verifiable_outcomes",
    "route_flexibility",
    "stable_change_behavior",
];

const OBSERVABLE_STATE_IDS: [&str; 7] = [
    "enabled",
    "focused",
    "selected",
    "expanded",
    "visible",
    "loading",
    "completed",
];
const CHANGE_STATE_IDS: [&str; 4] = ["loading", "modal", "disabled", "permission_unavailable"];
const ROUTE_IDS: [&str; 6] = [
    "native_api",
    "adapter",
    "accessibility",
    "keyboard",
    "scrolling",
    "visual_fallback_approved",
];
const PROVIDER_IDS: [&str; 6] = [
    "native",
    "mac_control",
    "app_connector",
    "browser_connector",
    "computer_use",
    "caller",
];
const METHOD_IDS: [&str; 9] = [
    "native_api",
    "adapter",
    "accessibility",
    "keyboard",
    "shortcut",
    "scroll",
    "pointer",
    "visual",
    "drag",
];
const INTERACTION_MODE_IDS: [&str; 6] =
    ["semantic", "keyboard", "pointer", "scroll", "drag", "mixed"];
const ORACLE_KIND_IDS: [&str; 6] = [
    "element_state",
    "window_state",
    "application_state",
    "task_state",
    "receipt_state",
    "provider_readback",
];
const SHORTCUT_DISPOSITION_IDS: [&str; 3] = [
    "built_in_verified",
    "customizable_verified",
    "not_applicable",
];
const SHORTCUT_CUSTOMIZATION_SURFACE_IDS: [&str; 3] =
    ["macos_app_shortcut", "app_managed", "chrome_extension"];
const SHORTCUT_CONFLICT_POLICY_IDS: [&str; 3] =
    ["app_managed", "detect_before_assignment", "system_resolved"];
const FAILURE_BEHAVIOR_IDS: [&str; 4] = [
    "fail_closed",
    "block_and_explain",
    "provider_handoff",
    "retryable_no_change",
];
const IMPLEMENTATION_SOURCE_EXTENSIONS: [&str; 29] = [
    "c",
    "cpp",
    "cs",
    "dart",
    "go",
    "h",
    "html",
    "java",
    "js",
    "json",
    "jsx",
    "kt",
    "kts",
    "m",
    "mm",
    "plist",
    "py",
    "rb",
    "rs",
    "storyboard",
    "svelte",
    "swift",
    "toml",
    "ts",
    "tsx",
    "vue",
    "xib",
    "yaml",
    "yml",
];
const NON_IMPLEMENTATION_SOURCE_COMPONENTS: [&str; 6] = [
    ".mac-control",
    "docs",
    "fixtures",
    "snapshots",
    "test",
    "tests",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlIdealStateReport {
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub producer: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub observed_at: String,
    #[serde(default)]
    pub repositories: Vec<MacControlRepositoryEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlRepositoryEvidence {
    #[serde(default)]
    pub repository_id: String,
    #[serde(default)]
    pub repository_name: String,
    #[serde(default)]
    pub manifest_schema: String,
    #[serde(default)]
    pub applicability: String,
    #[serde(default)]
    pub applicability_reason: String,
    #[serde(default)]
    pub observed_at: String,
    #[serde(default)]
    pub observed_commit: String,
    #[serde(default)]
    pub source_provenance: MacControlSourceProvenance,
    #[serde(default)]
    pub criteria: BTreeMap<String, bool>,
    #[serde(default)]
    pub supported_tasks: Vec<MacControlTaskEvidence>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub implementation_contract: MacControlImplementationContract,
    #[serde(default)]
    pub live_task_evidence: MacControlLiveTaskEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlSourceProvenance {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub dirty_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlImplementationContract {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub criteria: BTreeMap<String, bool>,
    #[serde(default)]
    pub criteria_passed_count: usize,
    #[serde(default)]
    pub criteria_total: usize,
    #[serde(default)]
    pub declaration_criteria_count: usize,
    #[serde(default)]
    pub evidence_level: String,
    #[serde(default)]
    pub dimension_states: BTreeMap<String, MacControlDimensionState>,
    #[serde(default)]
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub grounding_errors: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlDimensionState {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub grounded_task_count: usize,
    #[serde(default)]
    pub task_count: usize,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub failure_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlLiveTaskEvidence {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub task_count: usize,
    #[serde(default)]
    pub measured_task_count: usize,
    #[serde(default)]
    pub attempt_count: u64,
    #[serde(default)]
    pub success_count: u64,
    #[serde(default)]
    pub failure_reasons: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlTaskEvidence {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub surface_kind: String,
    #[serde(default)]
    pub stable_target_id: String,
    #[serde(default)]
    pub hierarchy: String,
    #[serde(default)]
    pub semantic_action: String,
    #[serde(default)]
    pub observable_postcondition: String,
    #[serde(default)]
    pub observable_states: Vec<String>,
    #[serde(default)]
    pub navigation_strategy: String,
    #[serde(default)]
    pub eligible_routes: Vec<String>,
    #[serde(default)]
    pub selected_route: String,
    #[serde(default)]
    pub change_states: Vec<String>,
    #[serde(default)]
    pub accessibility: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub state_exemptions: BTreeMap<String, String>,
    #[serde(default)]
    pub change_state_exemptions: BTreeMap<String, String>,
    #[serde(default)]
    pub focus_policy: String,
    #[serde(default)]
    pub foreground_postcondition: String,
    #[serde(default)]
    pub fallback_policy: String,
    #[serde(default)]
    pub verification_oracle: MacControlVerificationOracle,
    #[serde(default)]
    pub route_candidates: Vec<MacControlRouteCandidate>,
    #[serde(default)]
    pub shortcut_acceleration: MacControlShortcutAcceleration,
    #[serde(default)]
    pub semantic_evidence: BTreeMap<String, MacControlSemanticEvidence>,
    #[serde(default)]
    pub attempts: u64,
    #[serde(default)]
    pub successes: u64,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub measurement_valid: Option<bool>,
    #[serde(default)]
    pub measurement_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MacControlSemanticEvidence {
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub claims: BTreeMap<String, String>,
    #[serde(default)]
    pub source_refs: Vec<MacControlSourceReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MacControlSourceReference {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub anchor: String,
    #[serde(default)]
    pub evidence_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlVerificationOracle {
    #[serde(default)]
    pub oracle_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub expected_state: String,
    #[serde(default)]
    pub independent_readback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlRouteCandidate {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub interaction_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacControlShortcutAcceleration {
    #[serde(default)]
    pub disposition: String,
    #[serde(default)]
    pub command_id: String,
    #[serde(default)]
    pub chord: String,
    #[serde(default)]
    pub menu_path: Vec<String>,
    #[serde(default)]
    pub customization_surface: String,
    #[serde(default)]
    pub conflict_policy: String,
    #[serde(default)]
    pub contextual_availability: Option<bool>,
    #[serde(default)]
    pub reversible_assignment: Option<bool>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacControlRepositoryState {
    pub repository_id: String,
    pub repository_name: String,
    pub applicability: String,
    pub status: String,
    pub freshness: String,
    pub ideal_state: bool,
    pub supported_task_count: usize,
    pub measured_route_count: usize,
    #[serde(default)]
    pub implementation_status: String,
    #[serde(default)]
    pub implementation_criteria_passed_count: usize,
    #[serde(default)]
    pub implementation_criteria_total: usize,
    #[serde(default)]
    pub implementation_declaration_criteria_count: usize,
    #[serde(default)]
    pub implementation_evidence_level: String,
    #[serde(default)]
    pub source_provenance_status: String,
    #[serde(default)]
    pub source_provenance_dirty_paths: Vec<String>,
    #[serde(default)]
    pub live_status: String,
    #[serde(default)]
    pub live_task_count: usize,
    #[serde(default)]
    pub live_attempt_count: u64,
    #[serde(default)]
    pub live_success_count: u64,
    #[serde(default)]
    pub criteria: BTreeMap<String, bool>,
    #[serde(default)]
    pub failure_reasons: Vec<String>,
    #[serde(default)]
    pub evidence_contract: EvidenceContractRepositoryStatus,
    pub observed_at: Option<String>,
    pub observed_commit: Option<String>,
    pub report_path: Option<String>,
}

impl Default for MacControlRepositoryState {
    fn default() -> Self {
        Self {
            repository_id: String::new(),
            repository_name: String::new(),
            applicability: "Unknown".to_string(),
            status: "Not configured".to_string(),
            freshness: "Unknown".to_string(),
            ideal_state: false,
            supported_task_count: 0,
            measured_route_count: 0,
            implementation_status: "Not configured".to_string(),
            implementation_criteria_passed_count: 0,
            implementation_criteria_total: 0,
            implementation_declaration_criteria_count: 0,
            implementation_evidence_level: "Not reported".to_string(),
            source_provenance_status: "Not reported".to_string(),
            source_provenance_dirty_paths: Vec::new(),
            live_status: "Not configured".to_string(),
            live_task_count: 0,
            live_attempt_count: 0,
            live_success_count: 0,
            criteria: BTreeMap::new(),
            failure_reasons: Vec::new(),
            evidence_contract: EvidenceContractRepositoryStatus::default(),
            observed_at: None,
            observed_commit: None,
            report_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacControlPortfolioSnapshot {
    pub status: String,
    pub freshness: String,
    pub ideal_state: bool,
    pub applicable_repository_count: usize,
    pub not_applicable_repository_count: usize,
    pub evaluated_repository_count: usize,
    #[serde(default)]
    pub implementation_status: String,
    #[serde(default)]
    pub implementation_score: Option<f64>,
    #[serde(default)]
    pub implementation_score_display: Option<String>,
    #[serde(default)]
    pub implementation_criteria_passed_count: usize,
    #[serde(default)]
    pub implementation_criteria_total: usize,
    #[serde(default)]
    pub implementation_declaration_criteria_count: usize,
    #[serde(default)]
    pub live_status: String,
    #[serde(default)]
    pub live_score: Option<f64>,
    #[serde(default)]
    pub live_score_display: Option<String>,
    #[serde(default)]
    pub live_task_count: usize,
    #[serde(default)]
    pub measured_task_count: usize,
    #[serde(default)]
    pub live_attempt_count: u64,
    #[serde(default)]
    pub live_success_count: u64,
    #[serde(default)]
    pub repository_states: Vec<MacControlRepositoryState>,
    #[serde(default)]
    pub failure_reasons: Vec<String>,
    #[serde(default)]
    pub evidence_contract: EvidenceContractFleetCoverage,
    pub observed_at: Option<String>,
    pub report_path: Option<String>,
    pub run_id: Option<String>,
}

impl Default for MacControlPortfolioSnapshot {
    fn default() -> Self {
        Self {
            status: "Not configured".to_string(),
            freshness: "Unknown".to_string(),
            ideal_state: false,
            applicable_repository_count: 0,
            not_applicable_repository_count: 0,
            evaluated_repository_count: 0,
            implementation_status: "Not configured".to_string(),
            implementation_score: None,
            implementation_score_display: None,
            implementation_criteria_passed_count: 0,
            implementation_criteria_total: 0,
            implementation_declaration_criteria_count: 0,
            live_status: "Not configured".to_string(),
            live_score: None,
            live_score_display: None,
            live_task_count: 0,
            measured_task_count: 0,
            live_attempt_count: 0,
            live_success_count: 0,
            repository_states: Vec::new(),
            failure_reasons: Vec::new(),
            evidence_contract: EvidenceContractFleetCoverage::default(),
            observed_at: None,
            report_path: None,
            run_id: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MacControlEvaluation {
    pub portfolio: MacControlPortfolioSnapshot,
    pub by_repository: HashMap<String, MacControlRepositoryState>,
}

pub fn canonical_evidence_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(MAC_CONTROL_EVIDENCE_RELATIVE_PATH))
}

/// Evaluate the canonical report against the repositories in Pronto's current
/// maturity scope. The report must account for every scoped repository; a
/// `quality_runner_fleet` report may include additional repositories outside
/// Pronto's current maturity scope. It may explicitly mark a repository
/// `not_applicable` when no Mac Control task is supported there.
pub fn evaluate_canonical(repositories: &[RepositorySnapshot]) -> MacControlEvaluation {
    let Some(path) = canonical_evidence_path() else {
        return unavailable(
            repositories,
            None,
            "The canonical Mac Control evidence path is unavailable.",
        );
    };
    evaluate_report_at(&path, repositories)
}

pub fn evaluate_report_at(
    path: &Path,
    repositories: &[RepositorySnapshot],
) -> MacControlEvaluation {
    let report_path = Some(path.to_string_lossy().to_string());
    if path.is_symlink() || !path.is_file() {
        return unavailable(
            repositories,
            report_path,
            "No canonical Mac Control ideal-state report is available.",
        );
    }
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            return blocked(
                repositories,
                report_path,
                format!("The Mac Control ideal-state report could not be read: {error}."),
            )
        }
    };
    let report = match serde_json::from_str::<MacControlIdealStateReport>(&contents) {
        Ok(report) => report,
        Err(error) => {
            return blocked(
                repositories,
                report_path,
                format!("The Mac Control ideal-state report is invalid JSON: {error}."),
            )
        }
    };
    evaluate_report(report, report_path, repositories)
}

pub fn blocked_for_checkpoint(
    repositories: &[RepositorySnapshot],
    report_path: Option<String>,
    reason: impl Into<String>,
) -> MacControlEvaluation {
    blocked(repositories, report_path, reason.into())
}

fn evaluate_report(
    report: MacControlIdealStateReport,
    report_path: Option<String>,
    repositories: &[RepositorySnapshot],
) -> MacControlEvaluation {
    let mut scope_reasons = Vec::new();
    if report.schema_version != MAC_CONTROL_SCHEMA {
        scope_reasons.push(format!(
            "Expected schema_version {MAC_CONTROL_SCHEMA}, found '{}'.",
            report.schema_version
        ));
    }
    if report.producer.trim() != "mac-control" {
        scope_reasons.push("The report producer must be mac-control.".to_string());
    }
    if report.run_id.trim().is_empty() {
        scope_reasons.push("The report must include a stable run_id.".to_string());
    }
    let report_freshness = freshness_for(&report.observed_at);
    if report_freshness == "Unknown" {
        scope_reasons.push("The report observed_at must be RFC 3339.".to_string());
    }
    let report_scope = normalize_token(&report.scope);
    let fleet_scope = report_scope == "quality_runner_fleet";
    if !report.scope.trim().is_empty() && !fleet_scope {
        scope_reasons.push(format!(
            "The report scope must be empty for a Pronto-scoped report or 'quality_runner_fleet', found '{}'.",
            report.scope.trim()
        ));
    }

    let known_repositories = repositories
        .iter()
        .map(|repository| (repository.id.as_str(), repository))
        .collect::<HashMap<_, _>>();
    let known_repository_name_counts =
        repositories
            .iter()
            .fold(HashMap::<String, usize>::new(), |mut counts, repository| {
                *counts
                    .entry(repository.name.trim().to_ascii_lowercase())
                    .or_default() += 1;
                counts
            });
    if !fleet_scope && report.repositories.len() != known_repositories.len() {
        scope_reasons.push(format!(
            "The report must account for every repository in Pronto's current maturity scope; expected {}, found {}.",
            known_repositories.len(),
            report.repositories.len()
        ));
    }

    let mut seen_ids = HashSet::new();
    let mut entries = HashMap::new();
    let mut entry_ids_by_name = HashMap::new();
    let mut ambiguous_entry_names = HashSet::new();
    for entry in report.repositories {
        let repository_id = entry.repository_id.trim().to_string();
        if repository_id.is_empty() {
            scope_reasons.push("Every report repository must have a repository_id.".to_string());
            continue;
        }
        if !seen_ids.insert(repository_id.clone()) {
            scope_reasons.push(format!(
                "The repository_id '{repository_id}' is duplicated."
            ));
            continue;
        }
        if !known_repositories.contains_key(repository_id.as_str()) && !fleet_scope {
            scope_reasons.push(format!(
                "The report repository '{repository_id}' is not present in Pronto's current maturity scope."
            ));
            continue;
        }
        let repository_name_key = entry.repository_name.trim().to_ascii_lowercase();
        if !repository_name_key.is_empty() && !ambiguous_entry_names.contains(&repository_name_key)
        {
            if entry_ids_by_name
                .insert(repository_name_key.clone(), repository_id.clone())
                .is_some()
            {
                entry_ids_by_name.remove(&repository_name_key);
                ambiguous_entry_names.insert(repository_name_key);
            }
        }
        entries.insert(repository_id, entry);
    }
    let mut entry_ids_by_repository = HashMap::new();
    for repository in repositories {
        let matching_entry_id = if entries.contains_key(&repository.id) {
            Some(repository.id.clone())
        } else if fleet_scope {
            let name_key = repository.name.trim().to_ascii_lowercase();
            (known_repository_name_counts.get(&name_key) == Some(&1))
                .then(|| entry_ids_by_name.get(&name_key).cloned())
                .flatten()
        } else {
            None
        };
        if let Some(entry_id) = matching_entry_id {
            entry_ids_by_repository.insert(repository.id.clone(), entry_id);
        } else {
            scope_reasons.push(format!(
                "The report is missing repository '{}'.",
                repository.id
            ));
        }
    }

    let mut by_repository = HashMap::new();
    let mut states = Vec::with_capacity(repositories.len());
    for repository in repositories {
        let state = entry_ids_by_repository
            .get(&repository.id)
            .and_then(|entry_id| entries.get(entry_id))
            .map(|entry| evaluate_repository(entry, repository, report_path.clone()))
            .unwrap_or_else(|| {
                blocked_repository_state(
                    repository,
                    report_path.clone(),
                    "The report has no entry for this repository.",
                )
            });
        by_repository.insert(repository.id.clone(), state.clone());
        states.push(state);
    }
    states.sort_by(|left, right| left.repository_name.cmp(&right.repository_name));

    let applicable_repository_count = states
        .iter()
        .filter(|state| state.applicability == "Applicable")
        .count();
    let not_applicable_repository_count = states
        .iter()
        .filter(|state| state.applicability == "Not applicable")
        .count();
    let freshness = aggregate_freshness(&report_freshness, &states);
    let evidence_contract = aggregate_contract_coverage(
        MAC_CONTROL_TASK_CONTRACT_ID,
        MAC_CONTROL_TASK_CONTRACT_LABEL,
        MAC_CONTROL_TASK_MANIFEST_SCHEMA,
        &states
            .iter()
            .map(|state| state.evidence_contract.clone())
            .collect::<Vec<_>>(),
    );
    let all_entries_valid = scope_reasons.is_empty();
    let implementation_status = aggregate_lane_status(
        states
            .iter()
            .map(|state| state.implementation_status.as_str()),
        applicable_repository_count,
    );
    let live_status = aggregate_lane_status(
        states.iter().map(|state| state.live_status.as_str()),
        applicable_repository_count,
    );
    let all_applicable_pass = states.iter().all(|state| {
        state.applicability == "Not applicable"
            || (state.applicability == "Applicable"
                && state.status == "Passed"
                && state.implementation_status == "Passed"
                && state.live_status == "Passed"
                && state.freshness == "Fresh")
    });
    let any_blocked = states.iter().any(|state| {
        state.status == "Blocked"
            || state.implementation_status == "Blocked"
            || state.live_status == "Blocked"
    });
    let any_failed = states.iter().any(|state| {
        state.status == "Failed"
            || state.implementation_status == "Failed"
            || state.live_status == "Failed"
    });
    let any_review_required = states
        .iter()
        .any(|state| state.status == "Review required" || state.live_status == "Review required");
    let status = if !all_entries_valid {
        "Blocked"
    } else if any_blocked {
        "Blocked"
    } else if any_failed {
        "Failed"
    } else if evidence_contract.status != EVIDENCE_CONTRACT_STATUS_CURRENT {
        "Review required"
    } else if any_review_required {
        "Review required"
    } else if applicable_repository_count == 0 {
        "Not applicable"
    } else if all_applicable_pass {
        "Passed"
    } else {
        "Failed"
    };
    let ideal_state = status == "Passed" && freshness == "Fresh";

    let mut failure_reasons = scope_reasons;
    for state in &states {
        if state.status != "Passed" && state.status != "Not applicable" {
            failure_reasons.extend(
                state
                    .failure_reasons
                    .iter()
                    .map(|reason| format!("{}: {reason}", state.repository_name)),
            );
        }
        if state.status == "Passed" && state.freshness != "Fresh" {
            failure_reasons.push(format!(
                "{}: Mac Control evidence is {}.",
                state.repository_name, state.freshness
            ));
        }
    }
    failure_reasons.sort();
    failure_reasons.dedup();
    if evidence_contract.status != EVIDENCE_CONTRACT_STATUS_CURRENT {
        failure_reasons.push(evidence_contract.message.clone());
    }

    MacControlEvaluation {
        portfolio: MacControlPortfolioSnapshot {
            status: status.to_string(),
            freshness,
            ideal_state,
            applicable_repository_count,
            not_applicable_repository_count,
            evaluated_repository_count: states.len(),
            implementation_status,
            implementation_score: None,
            implementation_score_display: None,
            implementation_criteria_passed_count: states
                .iter()
                .map(|state| state.implementation_criteria_passed_count)
                .sum(),
            implementation_criteria_total: states
                .iter()
                .map(|state| state.implementation_criteria_total)
                .sum(),
            implementation_declaration_criteria_count: states
                .iter()
                .map(|state| state.implementation_declaration_criteria_count)
                .sum(),
            live_status,
            live_score: None,
            live_score_display: None,
            live_task_count: states.iter().map(|state| state.live_task_count).sum(),
            measured_task_count: states.iter().map(|state| state.measured_route_count).sum(),
            live_attempt_count: states.iter().map(|state| state.live_attempt_count).sum(),
            live_success_count: states.iter().map(|state| state.live_success_count).sum(),
            repository_states: states,
            failure_reasons,
            evidence_contract,
            observed_at: (!report.observed_at.trim().is_empty()).then_some(report.observed_at),
            report_path,
            run_id: (!report.run_id.trim().is_empty()).then_some(report.run_id),
        },
        by_repository,
    }
}

fn evaluate_repository(
    entry: &MacControlRepositoryEvidence,
    repository: &RepositorySnapshot,
    report_path: Option<String>,
) -> MacControlRepositoryState {
    let applicability = normalize_applicability(&entry.applicability);
    let observed_at = (!entry.observed_at.trim().is_empty()).then_some(entry.observed_at.clone());
    let observed_commit =
        (!entry.observed_commit.trim().is_empty()).then_some(entry.observed_commit.clone());
    let freshness = freshness_for(&entry.observed_at);
    let evidence_contract = evaluate_repository_contract(
        MAC_CONTROL_TASK_CONTRACT_ID,
        MAC_CONTROL_TASK_CONTRACT_LABEL,
        MAC_CONTROL_TASK_MANIFEST_SCHEMA,
        Some(&entry.manifest_schema),
        &repository.id,
        &repository.name,
    );
    let mut failure_reasons = Vec::new();
    let mut implementation_failures = Vec::new();
    let mut implementation_reviews = Vec::new();
    let mut live_reasons = Vec::new();
    let mut blocked = false;

    if !matches!(applicability.as_str(), "Applicable" | "Not applicable") {
        blocked = true;
        failure_reasons.push(
            "applicability must be 'applicable' or 'not_applicable' with an explicit reason"
                .to_string(),
        );
    }
    if entry.applicability_reason.trim().is_empty() {
        blocked = true;
        failure_reasons.push("applicability_reason is missing".to_string());
    }
    if freshness == "Unknown" {
        blocked = true;
        failure_reasons.push("observed_at is missing or invalid".to_string());
    }
    if entry.observed_commit.trim().is_empty() {
        blocked = true;
        failure_reasons.push("observed_commit is missing".to_string());
    } else if let Some(current_commit) = repository.workspace.last_commit.as_deref() {
        if current_commit != entry.observed_commit.trim() {
            failure_reasons.push(format!(
                "evidence commit {} does not match current commit {}",
                entry.observed_commit.trim(),
                current_commit
            ));
        }
    } else {
        blocked = true;
        failure_reasons.push("Pronto has no current repository commit to compare".to_string());
    }
    if entry.repository_name.trim().is_empty() {
        implementation_failures.push("repository_name is missing".to_string());
    }
    let provenance_failed = !failure_reasons.is_empty();

    if applicability == "Not applicable" {
        if !entry.supported_tasks.is_empty() {
            implementation_failures
                .push("not_applicable entries must not contain supported_tasks".to_string());
        }
    } else if applicability == "Applicable" {
        let current_manifest = entry.manifest_schema.trim() == MAC_CONTROL_TASK_MANIFEST_SCHEMA;
        if current_manifest {
            match normalize_token(&entry.source_provenance.status).as_str() {
                "clean" => {
                    if entry.source_provenance.digest.trim().is_empty()
                        || entry.source_provenance.paths.is_empty()
                    {
                        implementation_reviews.push(
                            "source provenance is incomplete: a digest and referenced implementation paths are required"
                                .to_string(),
                        );
                    }
                }
                "dirty" => {
                    let paths = if entry.source_provenance.dirty_paths.is_empty() {
                        "the manifest or referenced implementation source".to_string()
                    } else {
                        entry.source_provenance.dirty_paths.join(", ")
                    };
                    implementation_reviews.push(format!(
                        "source provenance is dirty for commit-bound evidence: {paths}"
                    ));
                }
                "unavailable" => implementation_reviews.push(
                    "source provenance could not be verified against the repository worktree"
                        .to_string(),
                ),
                _ => implementation_reviews.push(
                    "source provenance is missing; Quality Runner must bind the manifest and referenced implementation source to the observed commit"
                        .to_string(),
                ),
            }
            validate_criteria(
                &entry.criteria,
                &mut implementation_failures,
                &mut implementation_reviews,
            );
            validate_semantic_lane(
                entry,
                &mut implementation_failures,
                &mut implementation_reviews,
            );
        } else {
            implementation_reviews.push(format!(
                "{} is declaration-only; migrate to {} before any semantic dimension can score",
                entry.manifest_schema.trim(),
                MAC_CONTROL_TASK_MANIFEST_SCHEMA
            ));
        }
        validate_static_tasks(
            &entry.manifest_schema,
            &entry.supported_tasks,
            &mut implementation_failures,
        );
        if entry.evidence.is_empty() {
            implementation_failures.push("repository evidence references are missing".to_string());
        }
        let grounding_errors = entry
            .implementation_contract
            .grounding_errors
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        for reason in entry
            .validation_errors
            .iter()
            .chain(entry.implementation_contract.validation_errors.iter())
        {
            if grounding_errors.contains(reason) {
                implementation_reviews.push(reason.clone());
            } else {
                implementation_failures.push(reason.clone());
            }
        }
        validate_live_tasks(
            &entry.manifest_schema,
            &entry.supported_tasks,
            &mut live_reasons,
        );
        live_reasons.extend(entry.live_task_evidence.failure_reasons.iter().cloned());
    }

    let measured_route_count = entry
        .supported_tasks
        .iter()
        .filter(|task| task_is_measured(&entry.manifest_schema, task))
        .count();
    let mut implementation_status = if applicability == "Not applicable" {
        if implementation_failures.is_empty() {
            "Not applicable"
        } else {
            "Failed"
        }
    } else if blocked {
        "Blocked"
    } else if !implementation_failures.is_empty() {
        "Failed"
    } else if !implementation_reviews.is_empty() {
        "Review required"
    } else {
        "Passed"
    };
    let reported_live_status = normalize_lane_status(&entry.live_task_evidence.status);
    let live_status = if applicability == "Not applicable" {
        "Not applicable"
    } else if blocked || reported_live_status == "blocked" {
        "Blocked"
    } else if reported_live_status == "failed"
        || entry.supported_tasks.iter().any(task_has_failed_attempt)
    {
        "Failed"
    } else if !live_reasons.is_empty() {
        "Review required"
    } else {
        "Passed"
    };
    let mut status = if blocked || live_status == "Blocked" {
        "Blocked"
    } else if provenance_failed || !implementation_failures.is_empty() || live_status == "Failed" {
        "Failed"
    } else if applicability == "Not applicable" {
        "Not applicable"
    } else if !implementation_reviews.is_empty() || live_status == "Review required" {
        "Review required"
    } else {
        "Passed"
    };
    if evidence_contract.status != EVIDENCE_CONTRACT_STATUS_CURRENT {
        if matches!(implementation_status, "Passed" | "Not applicable") {
            implementation_status = "Review required";
        }
        if matches!(status, "Passed" | "Not applicable") {
            status = "Review required";
        }
        failure_reasons.push(format!("Evidence contract: {}", evidence_contract.message));
    }
    failure_reasons.extend(
        implementation_failures
            .iter()
            .map(|reason| format!("Implementation contract: {reason}")),
    );
    failure_reasons.extend(
        implementation_reviews
            .iter()
            .map(|reason| format!("Semantic evidence review: {reason}")),
    );
    failure_reasons.extend(
        live_reasons
            .iter()
            .map(|reason| format!("Live task evidence: {reason}")),
    );
    if reported_live_status == "blocked" && live_reasons.is_empty() {
        failure_reasons.push("Live task evidence: the reported live lane is blocked.".to_string());
    }
    failure_reasons.sort();
    failure_reasons.dedup();
    let ideal_state = status == "Passed" && freshness == "Fresh";
    let is_applicable = applicability == "Applicable";
    let current_manifest = entry.manifest_schema.trim() == MAC_CONTROL_TASK_MANIFEST_SCHEMA;
    MacControlRepositoryState {
        repository_id: repository.id.clone(),
        repository_name: if entry.repository_name.trim().is_empty() {
            repository.name.clone()
        } else {
            entry.repository_name.trim().to_string()
        },
        applicability,
        status: status.to_string(),
        freshness,
        ideal_state,
        supported_task_count: entry.supported_tasks.len(),
        measured_route_count,
        implementation_status: implementation_status.to_string(),
        implementation_criteria_passed_count: MAC_CONTROL_CRITERION_IDS
            .iter()
            .filter(|criterion| {
                is_applicable && current_manifest && entry.criteria.get(**criterion) == Some(&true)
            })
            .count(),
        implementation_criteria_total: if is_applicable {
            MAC_CONTROL_CRITERION_IDS.len()
        } else {
            0
        },
        implementation_declaration_criteria_count: if is_applicable && !current_manifest {
            entry
                .implementation_contract
                .declaration_criteria_count
                .max(entry.criteria.values().filter(|value| **value).count())
        } else {
            0
        },
        implementation_evidence_level: if !is_applicable {
            "Not applicable".to_string()
        } else if current_manifest {
            match normalize_token(&entry.implementation_contract.evidence_level).as_str() {
                "source_grounded" => "Source grounded".to_string(),
                "partially_source_grounded" => "Partially source grounded".to_string(),
                _ => "Not source grounded".to_string(),
            }
        } else {
            "Declaration only".to_string()
        },
        source_provenance_status: match normalize_token(&entry.source_provenance.status).as_str() {
            "clean" => "Clean".to_string(),
            "dirty" => "Dirty".to_string(),
            "unavailable" => "Unavailable".to_string(),
            _ => "Not reported".to_string(),
        },
        source_provenance_dirty_paths: entry.source_provenance.dirty_paths.clone(),
        live_status: live_status.to_string(),
        live_task_count: entry.supported_tasks.len(),
        live_attempt_count: entry.supported_tasks.iter().map(|task| task.attempts).sum(),
        live_success_count: entry
            .supported_tasks
            .iter()
            .map(|task| task.successes)
            .sum(),
        criteria: entry.criteria.clone(),
        failure_reasons,
        evidence_contract,
        observed_at,
        observed_commit,
        report_path,
    }
}

fn validate_criteria(
    criteria: &BTreeMap<String, bool>,
    failures: &mut Vec<String>,
    reviews: &mut Vec<String>,
) {
    for criterion in MAC_CONTROL_CRITERION_IDS {
        match criteria.get(criterion) {
            Some(true) => {}
            Some(false) => reviews.push(format!(
                "{criterion} lacks source-grounded evidence and does not score"
            )),
            None => failures.push(format!("derived criterion {criterion} is missing")),
        }
    }
    for criterion in criteria.keys() {
        if !MAC_CONTROL_CRITERION_IDS.contains(&criterion.as_str()) {
            failures.push(format!("unsupported derived criterion {criterion}"));
        }
    }
}

fn validate_semantic_lane(
    entry: &MacControlRepositoryEvidence,
    failures: &mut Vec<String>,
    reviews: &mut Vec<String>,
) {
    let contract = &entry.implementation_contract;
    let derived_count = MAC_CONTROL_CRITERION_IDS
        .iter()
        .filter(|criterion| entry.criteria.get(**criterion) == Some(&true))
        .count();
    if contract.criteria_total != MAC_CONTROL_CRITERION_IDS.len() {
        failures.push(format!(
            "semantic dimension total must be {}, found {}",
            MAC_CONTROL_CRITERION_IDS.len(),
            contract.criteria_total
        ));
    }
    if contract.criteria_passed_count != derived_count {
        failures.push(format!(
            "reported semantic score {}/{} does not match {} derived dimensions",
            contract.criteria_passed_count, contract.criteria_total, derived_count
        ));
    }
    if !contract.criteria.is_empty() && contract.criteria != entry.criteria {
        failures.push("implementation criteria do not match repository criteria".to_string());
    }
    for criterion in MAC_CONTROL_CRITERION_IDS {
        let Some(state) = contract.dimension_states.get(criterion) else {
            failures.push(format!("semantic dimension state {criterion} is missing"));
            continue;
        };
        let grounded = entry.criteria.get(criterion) == Some(&true);
        if grounded {
            if normalize_token(&state.status) != "source_grounded" {
                failures.push(format!(
                    "{criterion} is scored but its dimension state is not source_grounded"
                ));
            }
            if state.task_count != entry.supported_tasks.len()
                || state.grounded_task_count != entry.supported_tasks.len()
            {
                failures.push(format!(
                    "{criterion} is scored without grounding every supported task"
                ));
            }
            if state.evidence.is_empty() {
                failures.push(format!(
                    "{criterion} is scored without source evidence references"
                ));
            }
        } else if state.failure_reasons.is_empty() {
            reviews.push(format!("{criterion} has no source-grounded evidence"));
        } else {
            reviews.extend(
                state
                    .failure_reasons
                    .iter()
                    .map(|reason| format!("{criterion}: {reason}")),
            );
        }
    }
    let expected_level = if derived_count == MAC_CONTROL_CRITERION_IDS.len() {
        "source_grounded"
    } else if derived_count > 0 {
        "partially_source_grounded"
    } else {
        "not_source_grounded"
    };
    if normalize_token(&contract.evidence_level) != expected_level {
        failures.push(format!(
            "evidence_level must be {expected_level} for {derived_count}/{} dimensions",
            MAC_CONTROL_CRITERION_IDS.len()
        ));
    }
    reviews.extend(contract.grounding_errors.iter().cloned());
}

fn validate_static_tasks(
    manifest_schema: &str,
    tasks: &[MacControlTaskEvidence],
    reasons: &mut Vec<String>,
) {
    let is_v4 = manifest_schema.trim() == MAC_CONTROL_TASK_MANIFEST_SCHEMA;
    let is_v3 = manifest_schema.trim() == "mac-control-task-manifest/v3";
    let is_modern = is_v4 || is_v3 || manifest_schema.trim() == "mac-control-task-manifest/v2";
    let mut task_ids = HashSet::new();
    if tasks.is_empty() {
        reasons.push("supported_tasks is empty".to_string());
    }
    for task in tasks {
        let task_label = if task.task_id.trim().is_empty() {
            "unnamed task".to_string()
        } else {
            task.task_id.trim().to_string()
        };
        if !task_ids.insert(task_label.clone()) {
            reasons.push(format!("task {task_label} is duplicated"));
        }
        if task.stable_target_id.trim().is_empty() {
            reasons.push(format!("task {task_label} has no stable_target_id"));
        }
        if task.hierarchy.trim().is_empty() {
            reasons.push(format!("task {task_label} has no semantic hierarchy"));
        }
        if task.semantic_action.trim().is_empty() {
            reasons.push(format!("task {task_label} has no direct semantic action"));
        }
        if task.observable_postcondition.trim().is_empty() {
            reasons.push(format!("task {task_label} has no observable postcondition"));
        }
        if task.navigation_strategy.trim().is_empty() {
            reasons.push(format!(
                "task {task_label} has no efficient semantic navigation strategy"
            ));
        }
        if is_modern {
            validate_v2_task(task, &task_label, reasons);
            if is_v4 || is_v3 {
                validate_shortcut_acceleration(task, &task_label, reasons);
            }
            if is_v4 {
                validate_v4_task(task, &task_label, reasons);
            }
            continue;
        }
        require_values(
            &task.observable_states,
            &OBSERVABLE_STATE_IDS,
            &format!("task {task_label} observable_states"),
            reasons,
        );
        if normalize_token(&task.navigation_strategy) == "sequential_tabbing" {
            reasons.push(format!(
                "task {task_label} has no efficient semantic navigation strategy"
            ));
        }
        if task.eligible_routes.is_empty() {
            reasons.push(format!("task {task_label} has no eligible route"));
        } else {
            for route in &task.eligible_routes {
                if !ROUTE_IDS.contains(&normalize_token(route).as_str()) {
                    reasons.push(format!("task {task_label} has unsupported route {route}"));
                }
            }
        }
        let selected_route = normalize_token(&task.selected_route);
        if !ROUTE_IDS.contains(&selected_route.as_str())
            || !task
                .eligible_routes
                .iter()
                .any(|route| normalize_token(route) == selected_route)
        {
            reasons.push(format!(
                "task {task_label} does not expose a selected eligible route"
            ));
        }
        require_values(
            &task.change_states,
            &CHANGE_STATE_IDS,
            &format!("task {task_label} change_states"),
            reasons,
        );
    }
}

fn validate_v4_task(task: &MacControlTaskEvidence, task_label: &str, reasons: &mut Vec<String>) {
    let surface = normalize_token(&task.surface_kind);
    let supported_surfaces = [
        "native_app_ui",
        "browser_chrome",
        "web_content",
        "os_dialog",
        "hybrid_transition",
    ];
    if !supported_surfaces.contains(&surface.as_str()) {
        reasons.push(format!(
            "task {task_label} surface_kind must name a supported surface"
        ));
    }
    let providers = task
        .route_candidates
        .iter()
        .map(|candidate| normalize_token(&candidate.provider))
        .collect::<HashSet<_>>();
    let native_providers = ["native", "mac_control", "app_connector"];
    if task.route_candidates.len() < 2 {
        reasons.push(format!(
            "task {task_label} route_flexibility requires at least two route candidates"
        ));
    }
    if surface == "web_content" {
        if !providers.contains("browser_connector") {
            reasons.push(format!(
                "task {task_label} web_content requires a browser_connector route"
            ));
        }
        if native_providers
            .iter()
            .any(|provider| providers.contains(*provider))
        {
            reasons.push(format!(
                "task {task_label} web_content must not claim a native Mac Control route"
            ));
        }
    } else if matches!(
        surface.as_str(),
        "native_app_ui" | "browser_chrome" | "os_dialog"
    ) {
        if !native_providers
            .iter()
            .any(|provider| providers.contains(*provider))
        {
            reasons.push(format!(
                "task {task_label} {surface} requires a native semantic route"
            ));
        }
        if providers.contains("browser_connector") {
            reasons.push(format!(
                "task {task_label} {surface} must not claim a browser_connector route"
            ));
        }
    } else if surface == "hybrid_transition"
        && (!providers.contains("browser_connector")
            || !native_providers
                .iter()
                .any(|provider| providers.contains(*provider)))
    {
        reasons.push(format!(
            "task {task_label} hybrid_transition requires native and browser routes"
        ));
    }
    for candidate in &task.route_candidates {
        let method = normalize_token(&candidate.method);
        if method == "accessibility"
            && task
                .accessibility
                .get("identifier")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|identifier| identifier.trim().is_empty())
        {
            reasons.push(format!(
                "task {task_label} accessibility route requires accessibility.identifier"
            ));
        }
        if matches!(method.as_str(), "pointer" | "visual" | "drag")
            && !matches!(
                normalize_token(&task.fallback_policy).as_str(),
                "explicit_handoff" | "fresh_state_handoff"
            )
        {
            reasons.push(format!(
                "task {task_label} {method} route requires explicit_handoff or fresh_state_handoff"
            ));
        }
    }
    if matches!(
        normalize_token(&task.verification_oracle.expected_state).as_str(),
        "visible" | "readable" | "available" | "success" | "succeeded" | "completed"
    ) {
        reasons.push(format!(
            "task {task_label} verification_oracle expected_state must name a machine-checkable value"
        ));
    }

    for criterion in task.semantic_evidence.keys() {
        if !MAC_CONTROL_CRITERION_IDS.contains(&criterion.as_str()) {
            reasons.push(format!(
                "task {task_label} semantic_evidence contains unsupported key {criterion}"
            ));
        }
    }
    let mut seen_evidence = Vec::new();
    for criterion in MAC_CONTROL_CRITERION_IDS {
        let label = format!("task {task_label} semantic_evidence.{criterion}");
        let Some(evidence) = task.semantic_evidence.get(criterion) else {
            reasons.push(format!("{label} is required"));
            continue;
        };
        if normalize_token(&evidence.level) != "source_grounded" {
            reasons.push(format!("{label}.level must be source_grounded"));
        }
        for claim in semantic_claim_keys(criterion) {
            if evidence
                .claims
                .get(*claim)
                .is_none_or(|value| value.trim().is_empty())
            {
                reasons.push(format!("{label}.claims.{claim} is required"));
            }
        }
        validate_semantic_claims(criterion, evidence, task, &providers, &label, reasons);
        if evidence.source_refs.is_empty() {
            reasons.push(format!(
                "{label}.source_refs requires at least one source reference"
            ));
        }
        for (index, source) in evidence.source_refs.iter().enumerate() {
            let source_label = format!("{label}.source_refs[{index}]");
            let source_path = Path::new(source.path.trim());
            if source.path.trim().is_empty()
                || source_path.is_absolute()
                || source_path
                    .components()
                    .any(|component| component == Component::ParentDir)
            {
                reasons.push(format!(
                    "{source_label}.path must be repository-relative without traversal"
                ));
            }
            let normalized_components = source_path
                .components()
                .filter_map(|component| match component {
                    Component::Normal(value) => value.to_str().map(str::to_lowercase),
                    _ => None,
                })
                .collect::<HashSet<_>>();
            if NON_IMPLEMENTATION_SOURCE_COMPONENTS
                .iter()
                .any(|component| normalized_components.contains(*component))
            {
                reasons.push(format!(
                    "{source_label}.path must reference implementation source, not docs, tests, or fixtures"
                ));
            }
            let extension = source_path
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_lowercase)
                .unwrap_or_default();
            if !IMPLEMENTATION_SOURCE_EXTENSIONS.contains(&extension.as_str()) {
                reasons.push(format!(
                    "{source_label}.path must use a supported implementation-source extension"
                ));
            }
            if source.anchor.trim().is_empty() {
                reasons.push(format!("{source_label}.anchor is required"));
            }
            if source.evidence_tokens.is_empty()
                || source
                    .evidence_tokens
                    .iter()
                    .any(|token| token.trim().is_empty())
            {
                reasons.push(format!(
                    "{source_label}.evidence_tokens requires non-empty source tokens"
                ));
            }
        }
        if seen_evidence.contains(evidence) {
            reasons.push(format!(
                "task {task_label} semantic evidence must be criterion-specific, not cloned"
            ));
        }
        seen_evidence.push(evidence.clone());
    }
}

fn semantic_claim_keys(criterion: &str) -> &'static [&'static str] {
    match criterion {
        "stable_identity" => &["selector_kind", "selector_value", "scope", "uniqueness"],
        "correct_semantics" => &["role", "accessible_name", "action"],
        "observable_state" => &["property", "unavailable_behavior"],
        "useful_hierarchy" => &["container", "relationship", "uniqueness"],
        "efficient_navigation" => &["strategy", "entry_point"],
        "verifiable_outcomes" => &["readback_provider", "property", "operator", "expected"],
        "route_flexibility" => &["primary_provider", "secondary_provider", "fallback_policy"],
        "stable_change_behavior" => &["scenarios", "failure_behavior"],
        _ => &[],
    }
}

fn validate_semantic_claims(
    criterion: &str,
    evidence: &MacControlSemanticEvidence,
    task: &MacControlTaskEvidence,
    providers: &HashSet<String>,
    label: &str,
    reasons: &mut Vec<String>,
) {
    let claims = &evidence.claims;
    match criterion {
        "stable_identity" => {
            let selector_kind =
                normalize_token(claims.get("selector_kind").map_or("", String::as_str));
            if ![
                "ax_identifier",
                "data_attribute",
                "dom_test_id",
                "command_id",
            ]
            .contains(&selector_kind.as_str())
            {
                reasons.push(format!("{label}.claims.selector_kind is unsupported"));
            }
            let surface = normalize_token(&task.surface_kind);
            if surface == "web_content"
                && !["data_attribute", "dom_test_id"].contains(&selector_kind.as_str())
            {
                reasons.push(format!(
                    "{label}.claims.selector_kind must be DOM-native for web_content"
                ));
            }
            if matches!(
                surface.as_str(),
                "native_app_ui" | "browser_chrome" | "os_dialog"
            ) && !["ax_identifier", "command_id"].contains(&selector_kind.as_str())
            {
                reasons.push(format!(
                    "{label}.claims.selector_kind must be native for {surface}"
                ));
            }
            if claims
                .get("selector_value")
                .map_or("", String::as_str)
                .trim()
                != task.stable_target_id.trim()
            {
                reasons.push(format!(
                    "{label}.claims.selector_value must equal stable_target_id"
                ));
            }
            if normalize_token(claims.get("uniqueness").map_or("", String::as_str)) != "exactly_one"
            {
                reasons.push(format!("{label}.claims.uniqueness must be exactly_one"));
            }
        }
        "useful_hierarchy" => {
            if normalize_token(claims.get("uniqueness").map_or("", String::as_str)) != "exactly_one"
            {
                reasons.push(format!("{label}.claims.uniqueness must be exactly_one"));
            }
        }
        "efficient_navigation" => {
            let strategy = normalize_token(claims.get("strategy").map_or("", String::as_str));
            if ![
                "direct_semantic",
                "menu_command",
                "search",
                "shortcut",
                "typed_provider_handoff",
            ]
            .contains(&strategy.as_str())
            {
                reasons.push(format!("{label}.claims.strategy is unsupported"));
            }
            if strategy != normalize_token(&task.navigation_strategy) {
                reasons.push(format!(
                    "{label}.claims.strategy must match task navigation_strategy"
                ));
            }
            if strategy == "direct_semantic"
                && claims.get("entry_point").map_or("", String::as_str).trim()
                    != task.stable_target_id.trim()
            {
                reasons.push(format!(
                    "{label}.claims.entry_point must equal stable_target_id"
                ));
            }
        }
        "verifiable_outcomes" => {
            if !["equals", "not_equals", "contains", "exists"].contains(
                &normalize_token(claims.get("operator").map_or("", String::as_str)).as_str(),
            ) {
                reasons.push(format!("{label}.claims.operator is unsupported"));
            }
            if [
                "visible",
                "readable",
                "available",
                "success",
                "succeeded",
                "completed",
            ]
            .contains(&normalize_token(claims.get("expected").map_or("", String::as_str)).as_str())
            {
                reasons.push(format!(
                    "{label}.claims.expected must name a machine-checkable value"
                ));
            }
            if claims.get("expected").map_or("", String::as_str).trim()
                != task.verification_oracle.expected_state.trim()
            {
                reasons.push(format!(
                    "{label}.claims.expected must match verification_oracle.expected_state"
                ));
            }
            if !providers.contains(&normalize_token(
                claims.get("readback_provider").map_or("", String::as_str),
            )) {
                reasons.push(format!(
                    "{label}.claims.readback_provider must match a route candidate"
                ));
            }
        }
        "route_flexibility" => {
            let primary =
                normalize_token(claims.get("primary_provider").map_or("", String::as_str));
            let secondary =
                normalize_token(claims.get("secondary_provider").map_or("", String::as_str));
            if !providers.contains(&primary) {
                reasons.push(format!(
                    "{label}.claims.primary_provider must match a route candidate"
                ));
            }
            if !providers.contains(&secondary) {
                reasons.push(format!(
                    "{label}.claims.secondary_provider must match a route candidate"
                ));
            }
            if primary == secondary {
                reasons.push(format!(
                    "{label}.claims.secondary_provider must differ from primary_provider"
                ));
            }
            if normalize_token(claims.get("fallback_policy").map_or("", String::as_str))
                != normalize_token(&task.fallback_policy)
            {
                reasons.push(format!(
                    "{label}.claims.fallback_policy must match the task fallback_policy"
                ));
            }
        }
        "stable_change_behavior" => {
            let scenarios = claims
                .get("scenarios")
                .map_or("", String::as_str)
                .split(',')
                .map(normalize_token)
                .collect::<HashSet<_>>();
            for state in CHANGE_STATE_IDS {
                if !scenarios.contains(state) {
                    reasons.push(format!("{label}.claims.scenarios is missing {state}"));
                }
            }
            if !FAILURE_BEHAVIOR_IDS.contains(
                &normalize_token(claims.get("failure_behavior").map_or("", String::as_str))
                    .as_str(),
            ) {
                reasons.push(format!("{label}.claims.failure_behavior is unsupported"));
            }
        }
        _ => {}
    }
}

fn validate_v2_task(task: &MacControlTaskEvidence, task_label: &str, reasons: &mut Vec<String>) {
    if !task.eligible_routes.is_empty() {
        reasons.push(format!(
            "task {task_label} must use route_candidates instead of eligible_routes"
        ));
    }
    require_state_accounting(
        &task.observable_states,
        &task.state_exemptions,
        &OBSERVABLE_STATE_IDS,
        &format!("task {task_label} observable state contract"),
        reasons,
    );
    require_state_accounting(
        &task.change_states,
        &task.change_state_exemptions,
        &CHANGE_STATE_IDS,
        &format!("task {task_label} change state contract"),
        reasons,
    );
    let focus_policy = normalize_token(&task.focus_policy);
    if !matches!(focus_policy.as_str(), "foreground" | "background") {
        reasons.push(format!(
            "task {task_label} focus_policy must be foreground or background"
        ));
    }
    let expected_foreground = if focus_policy == "background" {
        "unrelated_foreground_preserved"
    } else {
        "target_foreground"
    };
    if normalize_token(&task.foreground_postcondition) != expected_foreground {
        reasons.push(format!(
            "task {task_label} foreground_postcondition must be {expected_foreground}"
        ));
    }
    if !matches!(
        normalize_token(&task.fallback_policy).as_str(),
        "none" | "explicit_handoff" | "fresh_state_handoff"
    ) {
        reasons.push(format!(
            "task {task_label} fallback_policy must be none, explicit_handoff, or fresh_state_handoff"
        ));
    }
    if task.verification_oracle.oracle_id.trim().is_empty() {
        reasons.push(format!(
            "task {task_label} verification_oracle has no oracle_id"
        ));
    }
    if !ORACLE_KIND_IDS.contains(&normalize_token(&task.verification_oracle.kind).as_str()) {
        reasons.push(format!(
            "task {task_label} verification_oracle has unsupported kind"
        ));
    }
    if task.verification_oracle.expected_state.trim().is_empty() {
        reasons.push(format!(
            "task {task_label} verification_oracle has no expected_state"
        ));
    }
    if !task.verification_oracle.independent_readback {
        reasons.push(format!(
            "task {task_label} verification_oracle requires independent_readback"
        ));
    }
    if task.route_candidates.is_empty() {
        reasons.push(format!("task {task_label} has no route candidate"));
        return;
    }
    let mut candidate_ids = HashSet::new();
    for candidate in &task.route_candidates {
        let candidate_id = candidate.id.trim();
        if candidate_id.is_empty() {
            reasons.push(format!("task {task_label} route candidate has no id"));
        } else if !candidate_ids.insert(candidate_id) {
            reasons.push(format!(
                "task {task_label} route candidate {candidate_id} is duplicated"
            ));
        }
        if !PROVIDER_IDS.contains(&normalize_token(&candidate.provider).as_str()) {
            reasons.push(format!(
                "task {task_label} route candidate {candidate_id} has unsupported provider"
            ));
        }
        if !METHOD_IDS.contains(&normalize_token(&candidate.method).as_str()) {
            reasons.push(format!(
                "task {task_label} route candidate {candidate_id} has unsupported method"
            ));
        }
        if !INTERACTION_MODE_IDS.contains(&normalize_token(&candidate.interaction_mode).as_str()) {
            reasons.push(format!(
                "task {task_label} route candidate {candidate_id} has unsupported interaction_mode"
            ));
        }
    }
}

fn validate_shortcut_acceleration(
    task: &MacControlTaskEvidence,
    task_label: &str,
    reasons: &mut Vec<String>,
) {
    let shortcut = &task.shortcut_acceleration;
    let disposition = normalize_token(&shortcut.disposition);
    if !SHORTCUT_DISPOSITION_IDS.contains(&disposition.as_str()) {
        reasons.push(format!(
            "task {task_label} shortcut_acceleration has unsupported disposition"
        ));
        return;
    }
    let shortcut_candidates = task
        .route_candidates
        .iter()
        .filter(|candidate| normalize_token(&candidate.method) == "shortcut")
        .collect::<Vec<_>>();
    for candidate in &shortcut_candidates {
        if normalize_token(&candidate.interaction_mode) != "keyboard" {
            reasons.push(format!(
                "task {task_label} shortcut route candidate {} requires interaction_mode keyboard",
                candidate.id
            ));
        }
    }
    if disposition == "not_applicable" {
        if shortcut.reason.trim().is_empty() {
            reasons.push(format!(
                "task {task_label} shortcut_acceleration not_applicable requires reason"
            ));
        }
        if !shortcut_candidates.is_empty() {
            reasons.push(format!(
                "task {task_label} shortcut_acceleration not_applicable must not declare a shortcut route candidate"
            ));
        }
        return;
    }
    if shortcut.command_id.trim().is_empty() {
        reasons.push(format!(
            "task {task_label} shortcut_acceleration requires command_id"
        ));
    }
    if shortcut.contextual_availability != Some(true) {
        reasons.push(format!(
            "task {task_label} shortcut_acceleration requires contextual_availability true"
        ));
    }
    if !SHORTCUT_CONFLICT_POLICY_IDS.contains(&normalize_token(&shortcut.conflict_policy).as_str())
    {
        reasons.push(format!(
            "task {task_label} shortcut_acceleration has unsupported conflict_policy"
        ));
    }
    if disposition == "built_in_verified" {
        if shortcut.chord.trim().is_empty() {
            reasons.push(format!(
                "task {task_label} built_in_verified shortcut_acceleration requires chord"
            ));
        }
        if shortcut_candidates.is_empty() {
            reasons.push(format!(
                "task {task_label} built_in_verified shortcut_acceleration requires a shortcut route candidate"
            ));
        }
        return;
    }
    let customization_surface = normalize_token(&shortcut.customization_surface);
    if !SHORTCUT_CUSTOMIZATION_SURFACE_IDS.contains(&customization_surface.as_str()) {
        reasons.push(format!(
            "task {task_label} customizable_verified shortcut_acceleration has unsupported customization_surface"
        ));
    }
    if customization_surface == "macos_app_shortcut"
        && (shortcut.menu_path.is_empty()
            || shortcut
                .menu_path
                .iter()
                .any(|segment| segment.trim().is_empty()))
    {
        reasons.push(format!(
            "task {task_label} macos_app_shortcut shortcut_acceleration requires an exact menu_path"
        ));
    }
    if shortcut.reversible_assignment != Some(true) {
        reasons.push(format!(
            "task {task_label} customizable_verified shortcut_acceleration requires reversible_assignment true"
        ));
    }
    if !shortcut.chord.trim().is_empty() && shortcut_candidates.is_empty() {
        reasons.push(format!(
            "task {task_label} assigned customizable shortcut requires a shortcut route candidate"
        ));
    }
}

fn require_state_accounting(
    declared: &[String],
    exemptions: &BTreeMap<String, String>,
    supported: &[&str],
    label: &str,
    reasons: &mut Vec<String>,
) {
    let present = declared
        .iter()
        .map(|state| normalize_token(state))
        .collect::<HashSet<_>>();
    let exempt = exemptions
        .iter()
        .map(|(state, reason)| (normalize_token(state), reason))
        .collect::<HashMap<_, _>>();
    for state in &present {
        if !supported.contains(&state.as_str()) {
            reasons.push(format!("{label} contains unsupported state {state}"));
        }
    }
    for (state, reason) in &exempt {
        if !supported.contains(&state.as_str()) {
            reasons.push(format!("{label} exempts unsupported state {state}"));
        }
        if reason.trim().is_empty() {
            reasons.push(format!("{label} exemption for {state} has no reason"));
        }
        if present.contains(state) {
            reasons.push(format!("{label} both declares and exempts {state}"));
        }
    }
    for state in supported {
        if !present.contains(*state) && !exempt.contains_key(*state) {
            reasons.push(format!("{label} must declare or exempt {state}"));
        }
    }
}

fn validate_live_tasks(
    manifest_schema: &str,
    tasks: &[MacControlTaskEvidence],
    reasons: &mut Vec<String>,
) {
    let is_modern = matches!(
        manifest_schema.trim(),
        "mac-control-task-manifest/v2"
            | "mac-control-task-manifest/v3"
            | "mac-control-task-manifest/v4"
    );
    if tasks.is_empty() {
        reasons.push("supported_tasks is empty".to_string());
        return;
    }
    for task in tasks {
        let task_label = if task.task_id.trim().is_empty() {
            "unnamed task".to_string()
        } else {
            task.task_id.trim().to_string()
        };
        if is_modern
            && (task.selected_route.trim().is_empty()
                || !task
                    .route_candidates
                    .iter()
                    .any(|candidate| candidate.id.trim() == task.selected_route.trim()))
        {
            reasons.push(format!(
                "task {task_label} has no runtime-selected route candidate"
            ));
        }
        if task_has_failed_attempt(task) {
            reasons.push(format!(
                "task {task_label} route measurement failed ({}/{})",
                task.successes, task.attempts
            ));
        } else if task.attempts == 0 {
            reasons.push(format!(
                "task {task_label} route measurement is incomplete ({}/{})",
                task.successes, task.attempts
            ));
        } else if task.evidence.is_empty() {
            reasons.push(format!(
                "task {task_label} has no observable evidence reference"
            ));
        }
        if manifest_schema.trim() == MAC_CONTROL_TASK_MANIFEST_SCHEMA
            && task.measurement_valid != Some(true)
        {
            let detail = if task.measurement_errors.is_empty() {
                "the structured receipt, route, source digest, and postcondition checks did not all pass"
                    .to_string()
            } else {
                task.measurement_errors.join("; ")
            };
            reasons.push(format!(
                "task {task_label} structured measurement is not valid: {detail}"
            ));
        }
    }
}

fn task_is_measured(manifest_schema: &str, task: &MacControlTaskEvidence) -> bool {
    task.attempts > 0
        && task.successes == task.attempts
        && !task.selected_route.trim().is_empty()
        && !task.evidence.is_empty()
        && (manifest_schema.trim() != MAC_CONTROL_TASK_MANIFEST_SCHEMA
            || task.measurement_valid == Some(true))
}

fn task_has_failed_attempt(task: &MacControlTaskEvidence) -> bool {
    task.attempts > 0 && task.successes != task.attempts
}

fn blocked_repository_state(
    repository: &RepositorySnapshot,
    report_path: Option<String>,
    reason: &str,
) -> MacControlRepositoryState {
    let evidence_contract = evaluate_repository_contract(
        MAC_CONTROL_TASK_CONTRACT_ID,
        MAC_CONTROL_TASK_CONTRACT_LABEL,
        MAC_CONTROL_TASK_MANIFEST_SCHEMA,
        None,
        &repository.id,
        &repository.name,
    );
    MacControlRepositoryState {
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        applicability: "Unknown".to_string(),
        status: "Blocked".to_string(),
        freshness: "Unknown".to_string(),
        ideal_state: false,
        supported_task_count: 0,
        measured_route_count: 0,
        implementation_status: "Blocked".to_string(),
        implementation_criteria_passed_count: 0,
        implementation_criteria_total: 0,
        implementation_declaration_criteria_count: 0,
        implementation_evidence_level: "Not source grounded".to_string(),
        source_provenance_status: "Not reported".to_string(),
        source_provenance_dirty_paths: Vec::new(),
        live_status: "Blocked".to_string(),
        live_task_count: 0,
        live_attempt_count: 0,
        live_success_count: 0,
        criteria: BTreeMap::new(),
        failure_reasons: vec![reason.to_string()],
        evidence_contract,
        observed_at: None,
        observed_commit: None,
        report_path,
    }
}

fn require_values(values: &[String], required: &[&str], label: &str, reasons: &mut Vec<String>) {
    let normalized = values
        .iter()
        .map(|value| normalize_token(value))
        .collect::<HashSet<_>>();
    for value in required {
        if !normalized.contains(*value) {
            reasons.push(format!("{label} is missing {value}"));
        }
    }
}

fn aggregate_freshness(report_freshness: &str, states: &[MacControlRepositoryState]) -> String {
    if report_freshness == "Unknown" || states.iter().any(|state| state.freshness == "Unknown") {
        return "Unknown".to_string();
    }
    if report_freshness == "Stale" || states.iter().any(|state| state.freshness == "Stale") {
        return "Stale".to_string();
    }
    "Fresh".to_string()
}

fn freshness_for(value: &str) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(value) else {
        return "Unknown".to_string();
    };
    let age = Utc::now().signed_duration_since(parsed.with_timezone(&Utc));
    if age < Duration::zero() {
        return "Unknown".to_string();
    }
    if age <= Duration::days(MAC_CONTROL_MAX_EVIDENCE_AGE_DAYS) {
        "Fresh".to_string()
    } else {
        "Stale".to_string()
    }
}

fn normalize_token(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn normalize_applicability(value: &str) -> String {
    match normalize_token(value).as_str() {
        "applicable" => "Applicable".to_string(),
        "not_applicable" => "Not applicable".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn normalize_lane_status(value: &str) -> String {
    normalize_token(value)
}

fn aggregate_lane_status<'a>(
    statuses: impl Iterator<Item = &'a str>,
    applicable_count: usize,
) -> String {
    let statuses = statuses.collect::<Vec<_>>();
    if applicable_count == 0 {
        return "Not applicable".to_string();
    }
    for status in ["Blocked", "blocked"] {
        if statuses.iter().any(|value| *value == status) {
            return "Blocked".to_string();
        }
    }
    for status in ["Failed", "failed"] {
        if statuses.iter().any(|value| *value == status) {
            return "Failed".to_string();
        }
    }
    for status in ["Review required", "review_required"] {
        if statuses.iter().any(|value| *value == status) {
            return "Review required".to_string();
        }
    }
    "Passed".to_string()
}

fn unavailable(
    repositories: &[RepositorySnapshot],
    report_path: Option<String>,
    reason: &str,
) -> MacControlEvaluation {
    let states = repositories
        .iter()
        .map(|repository| MacControlRepositoryState {
            repository_id: repository.id.clone(),
            repository_name: repository.name.clone(),
            report_path: report_path.clone(),
            failure_reasons: vec![reason.to_string()],
            evidence_contract: evaluate_repository_contract(
                MAC_CONTROL_TASK_CONTRACT_ID,
                MAC_CONTROL_TASK_CONTRACT_LABEL,
                MAC_CONTROL_TASK_MANIFEST_SCHEMA,
                None,
                &repository.id,
                &repository.name,
            ),
            ..MacControlRepositoryState::default()
        })
        .collect::<Vec<_>>();
    let evidence_contract = aggregate_contract_coverage(
        MAC_CONTROL_TASK_CONTRACT_ID,
        MAC_CONTROL_TASK_CONTRACT_LABEL,
        MAC_CONTROL_TASK_MANIFEST_SCHEMA,
        &states
            .iter()
            .map(|state| state.evidence_contract.clone())
            .collect::<Vec<_>>(),
    );
    let by_repository = states
        .iter()
        .map(|state| (state.repository_id.clone(), state.clone()))
        .collect();
    MacControlEvaluation {
        portfolio: MacControlPortfolioSnapshot {
            status: "Not configured".to_string(),
            report_path,
            evaluated_repository_count: repositories.len(),
            repository_states: states,
            failure_reasons: vec![reason.to_string()],
            evidence_contract,
            ..MacControlPortfolioSnapshot::default()
        },
        by_repository,
    }
}

fn blocked(
    repositories: &[RepositorySnapshot],
    report_path: Option<String>,
    reason: String,
) -> MacControlEvaluation {
    let mut evaluation = unavailable(repositories, report_path, &reason);
    evaluation.portfolio.status = "Blocked".to_string();
    evaluation.portfolio.implementation_status = "Blocked".to_string();
    evaluation.portfolio.live_status = "Blocked".to_string();
    evaluation.portfolio.failure_reasons = vec![reason.clone()];
    for state in evaluation.by_repository.values_mut() {
        state.status = "Blocked".to_string();
        state.implementation_status = "Blocked".to_string();
        state.live_status = "Blocked".to_string();
        state.failure_reasons = vec![reason.clone()];
    }
    for state in &mut evaluation.portfolio.repository_states {
        state.status = "Blocked".to_string();
        state.implementation_status = "Blocked".to_string();
        state.live_status = "Blocked".to_string();
        state.failure_reasons = vec![reason.clone()];
    }
    evaluation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::RepositorySnapshot;
    use chrono::Utc;
    use serde_json::{json, Value};

    fn repository(id: &str, commit: &str) -> RepositorySnapshot {
        serde_json::from_value(json!({
            "id": id,
            "name": id,
            "path": format!("/tmp/{id}"),
            "locality": "Local",
            "lifecycle": "Active",
            "lifecycle_candidate": "Active",
            "provider_state": "Unknown",
            "branch": "main",
            "default_branch": "main",
            "target_branch": "main",
            "target_branch_configured": false,
            "workspace": {
                "id": format!("workspace-{id}"),
                "path": format!("/tmp/{id}"),
                "is_primary": true,
                "branch": "main",
                "dirty": false,
                "added": 0,
                "removed": 0,
                "line_totals_partial": false,
                "sync_state": "Synced",
                "remote_freshness": "Unknown",
                "ahead": 0,
                "behind": 0,
                "last_commit": commit,
                "last_commit_at": null,
                "last_activity_at": null,
                "integration_state": "Unknown",
                "target_branch": "main",
                "target_confidence": "High",
                "role": "Primary",
                "role_confidence": "High",
                "activity": {"state": "Unknown", "confidence": "Low", "signals": []}
            },
            "workspaces": [],
            "branches": [],
            "submodules": [],
            "pull_requests": [],
            "releases": [],
            "conditions": [],
            "last_scan_at": Utc::now().to_rfc3339(),
            "last_fetch_at": null,
            "last_activity_at": null
        }))
        .expect("repository fixture should decode")
    }

    fn base_entry(id: &str, commit: &str) -> Value {
        let criteria = MAC_CONTROL_CRITERION_IDS
            .iter()
            .map(|criterion| (criterion.to_string(), json!(true)))
            .collect::<serde_json::Map<_, _>>();
        json!({
            "repository_id": id,
            "repository_name": id,
            "applicability": "applicable",
            "applicability_reason": "The repository contains supported Mac Control tasks.",
            "observed_at": Utc::now().to_rfc3339(),
            "observed_commit": commit,
            "criteria": criteria,
            "supported_tasks": [{
                "task_id": "open-main-window",
                "stable_target_id": "window.main.sidebar",
                "hierarchy": "window.main.sidebar.repositories",
                "semantic_action": "select",
                "observable_postcondition": "selected_repository_id == target",
                "observable_states": OBSERVABLE_STATE_IDS,
                "navigation_strategy": "semantic_target",
                "eligible_routes": ["adapter", "accessibility", "keyboard"],
                "selected_route": "adapter",
                "change_states": CHANGE_STATE_IDS,
                "attempts": 3,
                "successes": 3,
                "evidence": ["macctl://run/task"]
            }],
            "evidence": ["macctl://run/repository"]
        })
    }

    fn valid_v2_entry(id: &str, commit: &str) -> Value {
        let mut entry = base_entry(id, commit);
        entry["manifest_schema"] = json!("mac-control-task-manifest/v2");
        let task = entry["supported_tasks"]
            .as_array_mut()
            .and_then(|tasks| tasks.first_mut())
            .expect("v2 task fixture");
        task["observable_states"] = json!(["enabled", "visible", "completed"]);
        task["state_exemptions"] = json!({
            "focused": "The trigger does not retain focus.",
            "selected": "The trigger is not selectable.",
            "expanded": "The target is not expandable.",
            "loading": "The task completes synchronously."
        });
        task["change_states"] = json!(["loading", "disabled"]);
        task["change_state_exemptions"] = json!({
            "modal": "The task does not present a modal.",
            "permission_unavailable": "The task needs no protected permission."
        });
        task["navigation_strategy"] = json!("sequential_tabbing");
        task["eligible_routes"] = json!([]);
        task["selected_route"] = json!("computer-use-pointer");
        task["focus_policy"] = json!("foreground");
        task["foreground_postcondition"] = json!("target_foreground");
        task["fallback_policy"] = json!("fresh_state_handoff");
        task["verification_oracle"] = json!({
            "oracle_id": "window.main.sidebar.selection",
            "kind": "task_state",
            "expected_state": "selected_repository_id == target",
            "independent_readback": true
        });
        task["route_candidates"] = json!([
            {
                "id": "native-accessibility",
                "provider": "mac_control",
                "method": "accessibility",
                "interaction_mode": "semantic"
            },
            {
                "id": "computer-use-pointer",
                "provider": "computer_use",
                "method": "pointer",
                "interaction_mode": "pointer"
            }
        ]);
        entry
    }

    fn valid_v3_entry(id: &str, commit: &str) -> Value {
        let mut entry = valid_v2_entry(id, commit);
        entry["manifest_schema"] = json!("mac-control-task-manifest/v3");
        let task = entry["supported_tasks"]
            .as_array_mut()
            .and_then(|tasks| tasks.first_mut())
            .expect("v3 task fixture");
        task["route_candidates"]
            .as_array_mut()
            .expect("route candidate fixture")
            .push(json!({
                "id": "verified-shortcut",
                "provider": "mac_control",
                "method": "shortcut",
                "interaction_mode": "keyboard"
            }));
        task["shortcut_acceleration"] = json!({
            "disposition": "built_in_verified",
            "command_id": "window.main.sidebar.open",
            "chord": "cmd+1",
            "conflict_policy": "app_managed",
            "contextual_availability": true
        });
        entry
    }

    fn valid_v4_entry(id: &str, commit: &str) -> Value {
        let mut entry = valid_v3_entry(id, commit);
        entry["manifest_schema"] = json!(MAC_CONTROL_TASK_MANIFEST_SCHEMA);
        entry["source_provenance"] = json!({
            "status": "clean",
            "digest": "sha256:fixture-source-digest",
            "paths": ["src/main-window.tsx"],
            "dirty_paths": []
        });
        let task = entry["supported_tasks"]
            .as_array_mut()
            .and_then(|tasks| tasks.first_mut())
            .expect("v4 task fixture");
        task["surface_kind"] = json!("native_app_ui");
        task["measurement_valid"] = json!(true);
        task["measurement_errors"] = json!([]);
        task["navigation_strategy"] = json!("direct_semantic");
        task["accessibility"] = json!({
            "identifier": "window.main.sidebar",
            "role": "AXGroup"
        });
        let claims = json!({
            "stable_identity": {
                "selector_kind": "ax_identifier",
                "selector_value": "window.main.sidebar",
                "scope": "MainWindow",
                "uniqueness": "exactly_one"
            },
            "correct_semantics": {
                "role": "AXGroup",
                "accessible_name": "Repositories",
                "action": "AXPress"
            },
            "observable_state": {
                "property": "selected_repository_id",
                "unavailable_behavior": "permission_unavailable"
            },
            "useful_hierarchy": {
                "container": "MainWindow",
                "relationship": "Sidebar.Repositories",
                "uniqueness": "exactly_one"
            },
            "efficient_navigation": {
                "strategy": "direct_semantic",
                "entry_point": "window.main.sidebar"
            },
            "verifiable_outcomes": {
                "readback_provider": "mac_control",
                "property": "selected_repository_id",
                "operator": "equals",
                "expected": "selected_repository_id == target"
            },
            "route_flexibility": {
                "primary_provider": "mac_control",
                "secondary_provider": "computer_use",
                "fallback_policy": "fresh_state_handoff"
            },
            "stable_change_behavior": {
                "scenarios": "loading,modal,disabled,permission_unavailable",
                "failure_behavior": "fail_closed"
            }
        });
        let semantic_evidence = MAC_CONTROL_CRITERION_IDS
            .iter()
            .map(|criterion| {
                (
                    criterion.to_string(),
                    json!({
                        "level": "source_grounded",
                        "claims": claims[*criterion],
                        "source_refs": [{
                            "path": "src/main-window.tsx",
                            "anchor": criterion,
                            "evidence_tokens": [criterion, "window.main.sidebar"]
                        }]
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        task["semantic_evidence"] = Value::Object(semantic_evidence);
        let criteria = MAC_CONTROL_CRITERION_IDS
            .iter()
            .map(|criterion| (criterion.to_string(), json!(true)))
            .collect::<serde_json::Map<_, _>>();
        let dimension_states = MAC_CONTROL_CRITERION_IDS
            .iter()
            .map(|criterion| {
                (
                    criterion.to_string(),
                    json!({
                        "status": "source_grounded",
                        "grounded_task_count": 1,
                        "task_count": 1,
                        "evidence": [format!("source:src/main-window.tsx#{criterion}")],
                        "failure_reasons": []
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        entry["criteria"] = Value::Object(criteria.clone());
        entry["implementation_contract"] = json!({
            "status": "passed",
            "criteria": criteria,
            "criteria_passed_count": 8,
            "criteria_total": 8,
            "declaration_criteria_count": 0,
            "evidence_level": "source_grounded",
            "dimension_states": dimension_states,
            "validation_errors": [],
            "grounding_errors": [],
            "evidence": ["source:src/main-window.tsx"]
        });
        entry
    }

    fn valid_entry(id: &str, commit: &str) -> Value {
        valid_v4_entry(id, commit)
    }

    fn valid_report(ids: &[(&str, &str)]) -> String {
        let repositories = ids
            .iter()
            .map(|(id, commit)| valid_entry(id, commit))
            .collect::<Vec<_>>();
        serde_json::to_string(&json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-1",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": repositories
        }))
        .expect("fixture should serialize")
    }

    #[test]
    fn current_scope_passes_without_a_fixed_repository_count() {
        let ids = [("repo/one", "commit-1"), ("repo/two", "commit-2")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let directory =
            std::env::temp_dir().join(format!("pronto-mac-control-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temporary directory should be writable");
        let path = directory.join("report.json");
        fs::write(&path, valid_report(&ids)).expect("report should be writable");

        let evaluation = evaluate_report_at(&path, &repositories);

        assert_eq!(evaluation.portfolio.status, "Passed");
        assert!(evaluation.portfolio.ideal_state);
        assert_eq!(evaluation.portfolio.applicable_repository_count, 2);
        assert_eq!(evaluation.portfolio.not_applicable_repository_count, 0);
        assert_eq!(evaluation.portfolio.implementation_status, "Passed");
        assert_eq!(evaluation.portfolio.live_status, "Passed");
        assert_eq!(
            evaluation.portfolio.implementation_criteria_passed_count,
            16
        );
        assert_eq!(evaluation.portfolio.implementation_criteria_total, 16);
        assert_eq!(evaluation.portfolio.live_task_count, 2);
        assert_eq!(evaluation.portfolio.measured_task_count, 2);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn v2_task_surface_accepts_task_specific_states_and_runtime_route() {
        let repository = repository("repo/one", "commit-1");
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v2",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [valid_v2_entry("repo/one", "commit-1")]
        }))
        .expect("v2 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Review required");
        assert_eq!(state.implementation_criteria_passed_count, 0);
        assert_eq!(
            state.implementation_criteria_total,
            MAC_CONTROL_CRITERION_IDS.len()
        );
        assert_eq!(state.live_status, "Passed");
        assert_eq!(state.measured_route_count, 1);
        assert_eq!(
            state.evidence_contract.status,
            crate::evidence_contract::EVIDENCE_CONTRACT_STATUS_AUDIT_REQUIRED
        );
    }

    #[test]
    fn v2_missing_runtime_route_is_live_review_not_static_failure() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v2_entry("repo/one", "commit-1");
        entry["supported_tasks"][0]["selected_route"] = json!("");
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v2-unmeasured",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v2 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Review required");
        assert_eq!(state.live_status, "Review required");
        assert_eq!(state.measured_route_count, 0);
    }

    #[test]
    fn v3_shortcut_acceleration_is_readable_but_declaration_only() {
        let repository = repository("repo/one", "commit-1");
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v3",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [valid_v3_entry("repo/one", "commit-1")]
        }))
        .expect("v3 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Review required");
        assert_eq!(state.implementation_criteria_passed_count, 0);
        assert_eq!(state.implementation_declaration_criteria_count, 8);
        assert_eq!(state.implementation_evidence_level, "Declaration only");
        assert_eq!(state.live_status, "Passed");
    }

    #[test]
    fn v3_cannot_spoof_a_source_grounded_eight_of_eight() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v3_entry("repo/one", "commit-1");
        entry["implementation_contract"] = json!({
            "status": "passed",
            "criteria_passed_count": 8,
            "criteria_total": 8,
            "declaration_criteria_count": 8,
            "evidence_level": "source_grounded"
        });
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v3-spoofed",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v3 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Review required");
        assert_eq!(state.implementation_criteria_passed_count, 0);
        assert_eq!(state.implementation_declaration_criteria_count, 8);
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("declaration-only")));
    }

    #[test]
    fn v4_missing_source_anchor_is_review_required_and_non_scoring() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v4_entry("repo/one", "commit-1");
        entry["criteria"]["stable_identity"] = json!(false);
        entry["implementation_contract"]["criteria"]["stable_identity"] = json!(false);
        entry["implementation_contract"]["criteria_passed_count"] = json!(7);
        entry["implementation_contract"]["evidence_level"] = json!("partially_source_grounded");
        entry["implementation_contract"]["dimension_states"]["stable_identity"] = json!({
            "status": "not_source_grounded",
            "grounded_task_count": 0,
            "task_count": 1,
            "evidence": [],
            "failure_reasons": ["anchor not found in src/main-window.tsx: stable_identity"]
        });
        entry["implementation_contract"]["grounding_errors"] =
            json!(["anchor not found in src/main-window.tsx: stable_identity"]);
        entry["implementation_contract"]["validation_errors"] =
            json!(["anchor not found in src/main-window.tsx: stable_identity"]);
        entry["validation_errors"] =
            json!(["anchor not found in src/main-window.tsx: stable_identity"]);
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v4-partial",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v4 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Review required");
        assert_eq!(state.implementation_criteria_passed_count, 7);
        assert_eq!(
            state.implementation_evidence_level,
            "Partially source grounded"
        );
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("anchor not found")));
    }

    #[test]
    fn v4_dirty_source_provenance_cannot_pass_as_commit_bound_evidence() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v4_entry("repo/one", "commit-1");
        entry["source_provenance"] = json!({
            "status": "dirty",
            "digest": "sha256:dirty-source-digest",
            "paths": ["src/main-window.tsx"],
            "dirty_paths": ["src/main-window.tsx"]
        });
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v4-dirty-source",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v4 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.status, "Review required");
        assert_eq!(state.implementation_status, "Review required");
        assert_eq!(state.source_provenance_status, "Dirty");
        assert_eq!(state.source_provenance_dirty_paths, ["src/main-window.tsx"]);
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("dirty for commit-bound evidence")));
    }

    #[test]
    fn v4_aggregate_attempt_counts_cannot_spoof_structured_measurement() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v4_entry("repo/one", "commit-1");
        entry["supported_tasks"][0]["measurement_valid"] = json!(false);
        entry["supported_tasks"][0]["measurement_errors"] =
            json!(["receipt source digest does not match the audited source"]);
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v4-invalid-measurement",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v4 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.live_status, "Review required");
        assert_eq!(state.status, "Review required");
        assert_eq!(state.measured_route_count, 0);
        assert!(state.failure_reasons.iter().any(|reason| {
            reason.contains("receipt source digest does not match the audited source")
        }));
    }

    #[test]
    fn v4_rejects_cross_field_spoofing_and_nonimplementation_sources() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v4_entry("repo/one", "commit-1");
        let task = &mut entry["supported_tasks"][0];
        task["navigation_strategy"] = json!("shortcut");
        task["semantic_evidence"]["verifiable_outcomes"]["claims"]["expected"] =
            json!("different_state");
        task["semantic_evidence"]["verifiable_outcomes"]["claims"]["readback_provider"] =
            json!("caller");
        task["semantic_evidence"]["route_flexibility"]["claims"]["secondary_provider"] =
            json!("mac_control");
        task["semantic_evidence"]["stable_change_behavior"]["claims"]["failure_behavior"] =
            json!("ignore_and_continue");
        task["semantic_evidence"]["stable_identity"]["source_refs"][0]["path"] =
            json!("docs/mac-control.md");
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v4-spoofed",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v4 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Failed");
        for expected in [
            "strategy must match task navigation_strategy",
            "expected must match verification_oracle.expected_state",
            "readback_provider must match a route candidate",
            "secondary_provider must differ",
            "failure_behavior is unsupported",
            "must reference implementation source",
            "implementation-source extension",
        ] {
            assert!(
                state
                    .failure_reasons
                    .iter()
                    .any(|reason| reason.contains(expected)),
                "missing failure reason: {expected}"
            );
        }
    }

    #[test]
    fn v3_rejects_unverifiable_custom_shortcut_surface() {
        let repository = repository("repo/one", "commit-1");
        let mut entry = valid_v3_entry("repo/one", "commit-1");
        entry["supported_tasks"][0]["shortcut_acceleration"] = json!({
            "disposition": "customizable_verified",
            "command_id": "window.main.sidebar.open",
            "customization_surface": "macos_app_shortcut",
            "conflict_policy": "detect_before_assignment",
            "contextual_availability": false,
            "reversible_assignment": false
        });
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "run_id": "macctl-run-v3-invalid",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("v3 fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository state");

        assert_eq!(state.implementation_status, "Failed");
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("exact menu_path")));
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("reversible_assignment true")));
    }

    #[test]
    fn static_contract_passes_while_unattempted_tasks_remain_review_required() {
        let ids = [("repo/one", "commit-1")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let mut report = serde_json::from_str::<MacControlIdealStateReport>(&valid_report(&ids))
            .expect("fixture should parse");
        report.repositories[0].supported_tasks[0].attempts = 0;
        report.repositories[0].supported_tasks[0].successes = 0;
        report.repositories[0].supported_tasks[0].evidence.clear();

        let evaluation = evaluate_report(report, None, &repositories);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository should be evaluated");

        assert_eq!(state.implementation_status, "Passed");
        assert_eq!(state.live_status, "Review required");
        assert_eq!(state.status, "Review required");
        assert!(!state.ideal_state);
        assert_eq!(state.implementation_criteria_passed_count, 8);
        assert_eq!(state.implementation_criteria_total, 8);
        assert_eq!(state.live_task_count, 1);
        assert_eq!(state.measured_route_count, 0);
        assert_eq!(evaluation.portfolio.implementation_status, "Passed");
        assert_eq!(evaluation.portfolio.live_status, "Review required");
        assert_eq!(evaluation.portfolio.status, "Review required");
        assert!(!evaluation.portfolio.ideal_state);
    }

    #[test]
    fn fleet_scope_accepts_extra_repositories_but_requires_current_scope_entries() {
        let ids = [("repo/one", "commit-1"), ("repo/two", "commit-2")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let mut report = serde_json::from_str::<MacControlIdealStateReport>(&valid_report(&ids))
            .expect("fixture should parse");
        report.scope = "quality_runner_fleet".to_string();
        report.repositories.push(
            serde_json::from_value(valid_entry("repo/three", "commit-3"))
                .expect("fleet-only repository should decode"),
        );

        let evaluation = evaluate_report(report, None, &repositories);

        assert_eq!(evaluation.portfolio.status, "Passed");
        assert!(evaluation.portfolio.ideal_state);
        assert_eq!(evaluation.portfolio.evaluated_repository_count, 2);
        assert_eq!(evaluation.portfolio.applicable_repository_count, 2);
    }

    #[test]
    fn fleet_scope_reconciles_opaque_ids_by_unique_repository_name() {
        let mut repository = repository("repository:/tmp/pronto", "commit-1");
        repository.name = "Pronto".to_string();
        let mut entry = valid_entry("repo-opaque-123", "commit-1");
        entry["repository_name"] = json!("Pronto");
        let report = serde_json::from_value::<MacControlIdealStateReport>(json!({
            "schema_version": MAC_CONTROL_SCHEMA,
            "producer": "mac-control",
            "scope": "quality_runner_fleet",
            "run_id": "macctl-run-opaque",
            "observed_at": Utc::now().to_rfc3339(),
            "repositories": [entry]
        }))
        .expect("fleet fixture should decode");

        let evaluation = evaluate_report(report, None, &[repository]);

        assert_eq!(evaluation.portfolio.status, "Passed");
        assert!(evaluation.portfolio.failure_reasons.is_empty());
        assert_eq!(evaluation.portfolio.applicable_repository_count, 1);
        assert_eq!(
            evaluation
                .by_repository
                .get("repository:/tmp/pronto")
                .expect("Pronto repository should be reconciled")
                .status,
            "Passed"
        );
    }

    #[test]
    fn missing_current_scope_entry_is_blocked() {
        let ids = [("repo/one", "commit-1"), ("repo/two", "commit-2")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let report = serde_json::from_str::<MacControlIdealStateReport>(&valid_report(&ids))
            .expect("fixture should parse");
        let report = MacControlIdealStateReport {
            repositories: report.repositories.into_iter().take(1).collect(),
            ..report
        };

        let evaluation = evaluate_report(report, None, &repositories);

        assert_eq!(evaluation.portfolio.status, "Blocked");
        assert!(!evaluation.portfolio.ideal_state);
        assert!(evaluation
            .portfolio
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("every repository")));
    }

    #[test]
    fn missing_observable_state_and_route_measurement_fail_closed() {
        let ids = [("repo/one", "commit-1"), ("repo/two", "commit-2")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let mut report = serde_json::from_str::<MacControlIdealStateReport>(&valid_report(&ids))
            .expect("fixture should parse");
        report.repositories[0].supported_tasks[0]
            .observable_states
            .clear();
        report.repositories[0].supported_tasks[0]
            .state_exemptions
            .clear();
        report.repositories[0].supported_tasks[0].successes = 2;

        let evaluation = evaluate_report(report, None, &repositories);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository should be evaluated");

        assert_eq!(state.status, "Failed");
        assert_eq!(state.freshness, "Fresh");
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("observable state contract")));
        assert!(state
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("route measurement")));
        assert_eq!(evaluation.portfolio.status, "Failed");
        assert!(!evaluation.portfolio.ideal_state);
    }

    #[test]
    fn not_applicable_is_explicit_and_does_not_count_as_an_ideal_app() {
        let ids = [("repo/one", "commit-1"), ("repo/two", "commit-2")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let mut report = serde_json::from_str::<MacControlIdealStateReport>(&valid_report(&ids))
            .expect("fixture should parse");
        report.repositories[1].applicability = "not_applicable".to_string();
        report.repositories[1].applicability_reason =
            "This repository has no supported Mac Control task surface.".to_string();
        report.repositories[1].criteria.clear();
        report.repositories[1].evidence.clear();
        report.repositories[1].supported_tasks.clear();

        let evaluation = evaluate_report(report, None, &repositories);

        assert_eq!(evaluation.portfolio.status, "Passed");
        assert!(evaluation.portfolio.ideal_state);
        assert_eq!(evaluation.portfolio.applicable_repository_count, 1);
        assert_eq!(evaluation.portfolio.not_applicable_repository_count, 1);
    }

    #[test]
    fn stale_evidence_is_visible_and_does_not_pass_the_ideal_gate() {
        let ids = [("repo/one", "commit-1"), ("repo/two", "commit-2")];
        let repositories = ids
            .iter()
            .map(|(id, commit)| repository(id, commit))
            .collect::<Vec<_>>();
        let mut report = serde_json::from_str::<MacControlIdealStateReport>(&valid_report(&ids))
            .expect("fixture should parse");
        report.repositories[0].observed_at = (Utc::now() - Duration::days(8)).to_rfc3339();

        let evaluation = evaluate_report(report, None, &repositories);
        let state = evaluation
            .by_repository
            .get("repo/one")
            .expect("repository should be evaluated");

        assert_eq!(state.status, "Passed");
        assert_eq!(state.freshness, "Stale");
        assert_eq!(evaluation.portfolio.status, "Failed");
        assert_eq!(evaluation.portfolio.freshness, "Stale");
        assert!(!evaluation.portfolio.ideal_state);
    }
}
