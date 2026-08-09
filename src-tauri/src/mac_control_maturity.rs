use crate::core::RepositorySnapshot;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAC_CONTROL_GATE_ID: &str = "mac_control_ideal_state";
pub const MAC_CONTROL_GATE_LABEL: &str = "Mac Control ideal state";
pub const MAC_CONTROL_SCHEMA: &str = "pronto-mac-control-ideal-state/v1";
pub const MAC_CONTROL_EVIDENCE_RELATIVE_PATH: &str =
    ".quality-runner/fleet-audit/current/mac-control-ideal-state.json";
pub const MAC_CONTROL_MAX_EVIDENCE_AGE_DAYS: i64 = 7;

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
    pub applicability: String,
    #[serde(default)]
    pub applicability_reason: String,
    #[serde(default)]
    pub observed_at: String,
    #[serde(default)]
    pub observed_commit: String,
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
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
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
    pub attempts: u64,
    #[serde(default)]
    pub successes: u64,
    #[serde(default)]
    pub evidence: Vec<String>,
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
            live_status: "Not configured".to_string(),
            live_task_count: 0,
            live_attempt_count: 0,
            live_success_count: 0,
            criteria: BTreeMap::new(),
            failure_reasons: Vec::new(),
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
    pub implementation_criteria_passed_count: usize,
    #[serde(default)]
    pub implementation_criteria_total: usize,
    #[serde(default)]
    pub live_status: String,
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
            implementation_criteria_passed_count: 0,
            implementation_criteria_total: 0,
            live_status: "Not configured".to_string(),
            live_task_count: 0,
            measured_task_count: 0,
            live_attempt_count: 0,
            live_success_count: 0,
            repository_states: Vec::new(),
            failure_reasons: Vec::new(),
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
    if !fleet_scope && report.repositories.len() != known_repositories.len() {
        scope_reasons.push(format!(
            "The report must account for every repository in Pronto's current maturity scope; expected {}, found {}.",
            known_repositories.len(),
            report.repositories.len()
        ));
    }

    let mut seen_ids = HashSet::new();
    let mut entries = HashMap::new();
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
        entries.insert(repository_id, entry);
    }
    for repository in repositories {
        if !entries.contains_key(&repository.id) {
            scope_reasons.push(format!(
                "The report is missing repository '{}'.",
                repository.id
            ));
        }
    }

    let mut by_repository = HashMap::new();
    let mut states = Vec::with_capacity(repositories.len());
    for repository in repositories {
        let state = entries
            .get(&repository.id)
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
    } else if applicable_repository_count == 0 {
        "Not applicable"
    } else if any_blocked {
        "Blocked"
    } else if any_failed {
        "Failed"
    } else if any_review_required {
        "Review required"
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

    MacControlEvaluation {
        portfolio: MacControlPortfolioSnapshot {
            status: status.to_string(),
            freshness,
            ideal_state,
            applicable_repository_count,
            not_applicable_repository_count,
            evaluated_repository_count: states.len(),
            implementation_status,
            implementation_criteria_passed_count: states
                .iter()
                .map(|state| state.implementation_criteria_passed_count)
                .sum(),
            implementation_criteria_total: states
                .iter()
                .map(|state| state.implementation_criteria_total)
                .sum(),
            live_status,
            live_task_count: states.iter().map(|state| state.live_task_count).sum(),
            measured_task_count: states.iter().map(|state| state.measured_route_count).sum(),
            live_attempt_count: states.iter().map(|state| state.live_attempt_count).sum(),
            live_success_count: states.iter().map(|state| state.live_success_count).sum(),
            repository_states: states,
            failure_reasons,
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
    let mut failure_reasons = Vec::new();
    let mut implementation_reasons = Vec::new();
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
        implementation_reasons.push("repository_name is missing".to_string());
    }
    let provenance_failed = !failure_reasons.is_empty();

    if applicability == "Not applicable" {
        if !entry.supported_tasks.is_empty() {
            implementation_reasons
                .push("not_applicable entries must not contain supported_tasks".to_string());
        }
    } else if applicability == "Applicable" {
        validate_criteria(&entry.criteria, &mut implementation_reasons);
        validate_static_tasks(&entry.supported_tasks, &mut implementation_reasons);
        if entry.evidence.is_empty() {
            implementation_reasons.push("repository evidence references are missing".to_string());
        }
        implementation_reasons.extend(entry.validation_errors.iter().cloned());
        implementation_reasons.extend(
            entry
                .implementation_contract
                .validation_errors
                .iter()
                .cloned(),
        );
        validate_live_tasks(&entry.supported_tasks, &mut live_reasons);
        live_reasons.extend(entry.live_task_evidence.failure_reasons.iter().cloned());
    }

    let measured_route_count = entry
        .supported_tasks
        .iter()
        .filter(|task| task_is_measured(task))
        .count();
    let implementation_status = if applicability == "Not applicable" {
        if implementation_reasons.is_empty() {
            "Not applicable"
        } else {
            "Failed"
        }
    } else if blocked {
        "Blocked"
    } else if implementation_reasons.is_empty() {
        "Passed"
    } else {
        "Failed"
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
    let status = if blocked || live_status == "Blocked" {
        "Blocked"
    } else if provenance_failed || !implementation_reasons.is_empty() || live_status == "Failed" {
        "Failed"
    } else if applicability == "Not applicable" {
        "Not applicable"
    } else if live_status == "Review required" {
        "Review required"
    } else {
        "Passed"
    };
    failure_reasons.extend(
        implementation_reasons
            .iter()
            .map(|reason| format!("Implementation contract: {reason}")),
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
            .filter(|criterion| is_applicable && entry.criteria.get(**criterion) == Some(&true))
            .count(),
        implementation_criteria_total: if is_applicable {
            MAC_CONTROL_CRITERION_IDS.len()
        } else {
            0
        },
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
        observed_at,
        observed_commit,
        report_path,
    }
}

fn validate_criteria(criteria: &BTreeMap<String, bool>, reasons: &mut Vec<String>) {
    for criterion in MAC_CONTROL_CRITERION_IDS {
        match criteria.get(criterion) {
            Some(true) => {}
            Some(false) => reasons.push(format!("criterion {criterion} is false")),
            None => reasons.push(format!("criterion {criterion} is missing")),
        }
    }
    for criterion in criteria.keys() {
        if !MAC_CONTROL_CRITERION_IDS.contains(&criterion.as_str()) {
            reasons.push(format!("unsupported criterion {criterion}"));
        }
    }
}

fn validate_static_tasks(tasks: &[MacControlTaskEvidence], reasons: &mut Vec<String>) {
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
        require_values(
            &task.observable_states,
            &OBSERVABLE_STATE_IDS,
            &format!("task {task_label} observable_states"),
            reasons,
        );
        if task.navigation_strategy.trim().is_empty()
            || normalize_token(&task.navigation_strategy) == "sequential_tabbing"
        {
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

fn validate_live_tasks(tasks: &[MacControlTaskEvidence], reasons: &mut Vec<String>) {
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
    }
}

fn task_is_measured(task: &MacControlTaskEvidence) -> bool {
    task.attempts > 0 && task.successes == task.attempts && !task.evidence.is_empty()
}

fn task_has_failed_attempt(task: &MacControlTaskEvidence) -> bool {
    task.attempts > 0 && task.successes != task.attempts
}

fn blocked_repository_state(
    repository: &RepositorySnapshot,
    report_path: Option<String>,
    reason: &str,
) -> MacControlRepositoryState {
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
        live_status: "Blocked".to_string(),
        live_task_count: 0,
        live_attempt_count: 0,
        live_success_count: 0,
        criteria: BTreeMap::new(),
        failure_reasons: vec![reason.to_string()],
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
            ..MacControlRepositoryState::default()
        })
        .collect::<Vec<_>>();
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

    fn valid_entry(id: &str, commit: &str) -> Value {
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
            .any(|reason| reason.contains("observable_states")));
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
