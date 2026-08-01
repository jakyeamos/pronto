use crate::core::RepositorySnapshot;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

pub const REMEDIATION_SCHEMA: &str = "pronto-remediation/v3";
pub const REMEDIATION_GOAL_SCHEMA: &str = "pronto-remediation-goal/v1";
pub const REMEDIATION_GOAL_PATH: &str = ".pronto/remediation-goal.json";
pub const MATURITY_TARGET: f64 = 3.0;
pub const EXCLUDED_REPOSITORY_NAMES: [&str; 2] = ["soundscape", "tenure"];
const ALL_GATE_IDS: [&str; 9] = [
    "build",
    "tests",
    "runtime_smoke",
    "lint",
    "formatter",
    "typecheck",
    "dead_code",
    "secrets_scan",
    "dependency_audit",
];

const STAGE_ORDER: [&str; 11] = [
    "scope",
    "product_truth",
    "repository_health",
    "branch_hygiene",
    "provider",
    "evidence_refresh",
    "ci_ideal",
    "qr_findings",
    "maturity",
    "verification",
    "complete",
];

const UI_TRACKED_SURFACE_IDS: [&str; 16] = [
    "scope",
    "project_compass",
    "provider",
    "pull_requests",
    "releases",
    "quality_evidence",
    "ci_gates",
    "quality_findings",
    "maturity",
    "workspaces",
    "branches",
    "submodules",
    "conditions",
    "release_preparation",
    "agent_permission",
    "analytics",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationEvidence {
    pub source: String,
    pub label: String,
    pub status: String,
    pub freshness: String,
    pub observed_at: Option<String>,
    pub report_path: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationAction {
    pub id: String,
    pub stable_key: String,
    pub repository_id: String,
    pub domain: String,
    pub title: String,
    pub summary: String,
    pub severity: String,
    pub priority: String,
    pub weight: u64,
    pub status: String,
    pub acceptance_criteria: Vec<String>,
    pub evidence: Vec<RemediationEvidence>,
    pub related_finding_ids: Vec<String>,
    pub source_run_id: Option<String>,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationProgress {
    pub verified_weight: u64,
    pub total_weight: u64,
    pub deferred_weight: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationTrack {
    pub domain: String,
    pub label: String,
    pub status: String,
    pub action_ids: Vec<String>,
    pub verified_weight: u64,
    pub total_weight: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationCoverage {
    pub surface: String,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub action_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationGoalProfile {
    pub schema_version: String,
    pub target_state: String,
    pub label: String,
    pub source: String,
    pub confidence: String,
    pub reason: String,
    pub contract_path: String,
    pub required_gate_ids: Vec<String>,
    pub optional_gate_ids: Vec<String>,
    pub evidence_max_age_days: u64,
    pub closure_criteria: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlan {
    pub schema_version: String,
    pub id: String,
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub generated_at: String,
    pub source_refresh_id: Option<String>,
    #[serde(default)]
    pub goal: RemediationGoalProfile,
    pub current_stage: String,
    pub status: String,
    pub progress: RemediationProgress,
    #[serde(default)]
    pub coverage: Vec<RemediationCoverage>,
    pub tracks: Vec<RemediationTrack>,
    pub actions: Vec<RemediationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationClosure {
    pub id: String,
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub plan_id: String,
    #[serde(default)]
    pub target_state: String,
    #[serde(default)]
    pub goal_source: String,
    pub closed_at: String,
    pub source_refresh_id: Option<String>,
    pub disposition: String,
    pub summary: String,
    pub resolved_action_count: usize,
    pub verified_action_count: usize,
    pub deferred_action_count: usize,
    pub last_evidence_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationExclusion {
    pub repository_id: String,
    pub repository_name: String,
    pub repository_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationRefreshStep {
    pub id: String,
    pub label: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub detail: String,
    pub evidence_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationRun {
    pub schema_version: String,
    pub id: String,
    pub generated_at: String,
    pub source_refresh_id: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub eligible_repository_ids: Vec<String>,
    #[serde(default)]
    pub eligible_repository_paths: Vec<String>,
    #[serde(default)]
    pub refresh_steps: Vec<RemediationRefreshStep>,
    pub excluded_repositories: Vec<RemediationExclusion>,
    #[serde(default)]
    pub closures: Vec<RemediationClosure>,
    pub plans: Vec<RemediationPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationExport {
    pub run_id: String,
    pub output_path: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
struct QrRunEvidence {
    id: String,
    run_dir: PathBuf,
    observed_at: Option<String>,
    findings: Vec<ParsedFinding>,
    maturity_score: Option<f64>,
    dimension_scores: BTreeMap<String, f64>,
    maturity_report_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedFinding {
    id: String,
    group_key: String,
    category: String,
    pack: Option<String>,
    severity: String,
    title: String,
    summary: String,
    file: Option<String>,
    line: Option<u64>,
    verification: Option<String>,
    report_path: String,
}

#[derive(Debug, Clone)]
struct ActionSeed {
    stable_key: String,
    domain: String,
    title: String,
    summary: String,
    severity: String,
    priority: String,
    weight: u64,
    acceptance_criteria: Vec<String>,
    evidence: Vec<RemediationEvidence>,
    related_finding_ids: Vec<String>,
    source_run_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RepositoryGoalContract {
    schema_version: String,
    target_state: String,
    reason: String,
    #[serde(default)]
    additional_required_gate_ids: Vec<String>,
    #[serde(default)]
    optional_gate_ids: Vec<String>,
    evidence_max_age_days: Option<u64>,
}

pub fn is_excluded_repository(repository: &RepositorySnapshot) -> bool {
    let name = repository.name.trim().to_ascii_lowercase();
    let path_name = Path::new(&repository.path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    [name, path_name].iter().any(|candidate| {
        EXCLUDED_REPOSITORY_NAMES.iter().any(|excluded| {
            candidate == excluded
                || candidate.starts_with(&format!("{excluded}-"))
                || candidate.starts_with(&format!("{excluded}_"))
        })
    })
}

pub fn exclusion_reason(repository: &RepositorySnapshot) -> Option<String> {
    if is_excluded_repository(repository) {
        Some("Currently in progress; excluded from this refresh and remediation plan.".to_string())
    } else {
        None
    }
}

fn goal_definition(target_state: &str) -> Option<RemediationGoalProfile> {
    let (label, required, optional, evidence_max_age_days, closure_criteria) = match target_state {
        "public_release" => (
            "Public release",
            ALL_GATE_IDS.to_vec(),
            Vec::new(),
            7,
            vec![
                "The canonical branch and provider state are reconciled.",
                "All release-quality gates have fresh passing evidence.",
                "The repository has an explicit release rule and release recipe.",
                "Packaging, documentation, versioning, and release evidence are verified.",
            ],
        ),
        "deployed_product" => (
            "Deployed product",
            vec![
                "build",
                "tests",
                "runtime_smoke",
                "lint",
                "typecheck",
                "secrets_scan",
                "dependency_audit",
            ],
            vec!["formatter", "dead_code"],
            7,
            vec![
                "The canonical branch and provider state are reconciled.",
                "Build, test, security, and runtime evidence are fresh and passing.",
                "The deployed operating surface is verified.",
            ],
        ),
        "active_maintained" => (
            "Active maintained repository",
            vec![
                "build",
                "tests",
                "lint",
                "formatter",
                "typecheck",
                "dead_code",
                "secrets_scan",
                "dependency_audit",
            ],
            vec!["runtime_smoke"],
            14,
            vec![
                "The canonical branch and provider state are reconciled.",
                "The maintained development gates have fresh passing evidence.",
                "No active quality or maturity blocker remains.",
            ],
        ),
        "clean_only" => (
            "Clean and preserved",
            Vec::new(),
            vec!["tests", "secrets_scan", "dependency_audit"],
            30,
            vec![
                "All work is intentionally committed, published, or explicitly preserved.",
                "The canonical branch and provider relationship are understood.",
                "No dirty, divergent, unpublished, or ambiguous workspace remains.",
            ],
        ),
        "prototype" => (
            "Preserved prototype",
            Vec::new(),
            vec!["build", "tests", "secrets_scan", "dependency_audit"],
            30,
            vec![
                "The prototype state and limitations are documented.",
                "Unique work is intentionally preserved.",
                "No unresolved safety blocker remains.",
            ],
        ),
        "archived" => (
            "Archived repository",
            Vec::new(),
            Vec::new(),
            30,
            vec![
                "Final work is published or explicitly preserved.",
                "The repository lifecycle is confirmed as archived.",
                "No active worktree, branch operation, or ambiguous unpublished work remains.",
            ],
        ),
        _ => return None,
    };
    Some(RemediationGoalProfile {
        schema_version: REMEDIATION_GOAL_SCHEMA.to_string(),
        target_state: target_state.to_string(),
        label: label.to_string(),
        required_gate_ids: required.into_iter().map(str::to_string).collect(),
        optional_gate_ids: optional.into_iter().map(str::to_string).collect(),
        evidence_max_age_days,
        closure_criteria: closure_criteria.into_iter().map(str::to_string).collect(),
        contract_path: REMEDIATION_GOAL_PATH.to_string(),
        ..RemediationGoalProfile::default()
    })
}

fn inferred_goal(repository: &RepositorySnapshot) -> (&'static str, String) {
    let lifecycle = repository.lifecycle.to_ascii_lowercase();
    let candidate = repository.lifecycle_candidate.to_ascii_lowercase();
    if lifecycle.contains("archiv") || candidate.contains("archiv") {
        return (
            "archived",
            "The repository lifecycle indicates archived or archive-candidate status.".to_string(),
        );
    }
    if ["prototype", "experimental", "incubator"]
        .iter()
        .any(|signal| lifecycle.contains(signal) || candidate.contains(signal))
    {
        return (
            "prototype",
            "The repository lifecycle indicates prototype or experimental work.".to_string(),
        );
    }
    if repository.release_rule.is_some()
        || repository.release_recipe.is_some()
        || repository.confirmed_release_version.is_some()
        || !repository.releases.is_empty()
    {
        return (
            "public_release",
            "Release configuration or release history is present in the Pronto snapshot."
                .to_string(),
        );
    }
    if lifecycle.contains("active") || candidate.contains("active") {
        return (
            "active_maintained",
            "The repository lifecycle indicates active maintained work.".to_string(),
        );
    }
    (
        "clean_only",
        "No release, deployment, or active-product signal is confirmed; clean preservation is the conservative default.".to_string(),
    )
}

fn normalized_gate_ids(values: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for value in values {
        let gate_id = value.trim().to_ascii_lowercase();
        if !ALL_GATE_IDS.contains(&gate_id.as_str()) {
            return Err(format!("Unknown remediation gate '{value}'."));
        }
        normalized.push(gate_id);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn inferred_goal_profile(
    repository: &RepositorySnapshot,
    error: Option<String>,
) -> RemediationGoalProfile {
    let (target_state, reason) = inferred_goal(repository);
    let mut profile = goal_definition(target_state).expect("inferred goal must be supported");
    profile.source = "inferred".to_string();
    profile.confidence = "Medium".to_string();
    profile.reason = reason;
    profile.error = error;
    profile
}

fn resolve_goal_profile(repository: &RepositorySnapshot) -> RemediationGoalProfile {
    let contract_path = Path::new(&repository.path).join(REMEDIATION_GOAL_PATH);
    if !contract_path.is_file() {
        return inferred_goal_profile(repository, None);
    }
    let contract = fs::read_to_string(&contract_path)
        .map_err(|error| format!("Could not read the remediation goal contract: {error}"))
        .and_then(|contents| {
            serde_json::from_str::<RepositoryGoalContract>(&contents)
                .map_err(|error| format!("Invalid remediation goal JSON: {error}"))
        });
    let contract = match contract {
        Ok(contract) => contract,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    if contract.schema_version != REMEDIATION_GOAL_SCHEMA {
        return inferred_goal_profile(
            repository,
            Some(format!(
                "Unsupported remediation goal schema '{}'; expected '{}'.",
                contract.schema_version, REMEDIATION_GOAL_SCHEMA
            )),
        );
    }
    if contract.reason.trim().is_empty() {
        return inferred_goal_profile(
            repository,
            Some("The remediation goal contract must include a non-empty reason.".to_string()),
        );
    }
    let Some(mut profile) = goal_definition(contract.target_state.trim()) else {
        return inferred_goal_profile(
            repository,
            Some(format!(
                "Unknown remediation target state '{}'.",
                contract.target_state
            )),
        );
    };
    let additional_required = match normalized_gate_ids(&contract.additional_required_gate_ids) {
        Ok(gates) => gates,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    let optional = match normalized_gate_ids(&contract.optional_gate_ids) {
        Ok(gates) => gates,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    profile.required_gate_ids.extend(additional_required);
    profile.required_gate_ids.sort();
    profile.required_gate_ids.dedup();
    profile.optional_gate_ids.extend(optional);
    profile.optional_gate_ids.sort();
    profile.optional_gate_ids.dedup();
    profile
        .optional_gate_ids
        .retain(|gate| !profile.required_gate_ids.contains(gate));
    if let Some(days) = contract.evidence_max_age_days {
        if !(1..=90).contains(&days) {
            return inferred_goal_profile(
                repository,
                Some("evidence_max_age_days must be between 1 and 90.".to_string()),
            );
        }
        profile.evidence_max_age_days = days;
    }
    profile.source = "repository_contract".to_string();
    profile.confidence = "High".to_string();
    profile.reason = contract.reason.trim().to_string();
    profile
}

fn goal_requires_provider(goal: &RemediationGoalProfile) -> bool {
    goal.target_state != "prototype"
}

fn goal_requires_quality_evidence(goal: &RemediationGoalProfile) -> bool {
    matches!(
        goal.target_state.as_str(),
        "public_release" | "deployed_product" | "active_maintained"
    )
}

fn goal_requires_maturity(goal: &RemediationGoalProfile) -> bool {
    matches!(
        goal.target_state.as_str(),
        "public_release" | "deployed_product" | "active_maintained"
    )
}

fn goal_queue_rank(target_state: &str) -> u8 {
    match target_state {
        "public_release" => 0,
        "deployed_product" => 1,
        "active_maintained" => 2,
        "clean_only" => 3,
        "prototype" => 4,
        "archived" => 5,
        _ => 6,
    }
}

pub fn empty_run() -> RemediationRun {
    RemediationRun {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        status: "not_run".to_string(),
        ..RemediationRun::default()
    }
}

pub fn rebuild_run(
    repositories: &[RepositorySnapshot],
    previous: &RemediationRun,
    source_refresh_id: Option<&str>,
) -> RemediationRun {
    rebuild_run_with_fleet_root(repositories, previous, source_refresh_id, None)
}

pub fn rebuild_run_with_fleet_root(
    repositories: &[RepositorySnapshot],
    previous: &RemediationRun,
    source_refresh_id: Option<&str>,
    fleet_audit_root: Option<&Path>,
) -> RemediationRun {
    let generated_at = Utc::now().to_rfc3339();
    let previous_by_repository = previous
        .plans
        .iter()
        .map(|plan| (plan.repository_id.as_str(), plan))
        .collect::<HashMap<_, _>>();
    let mut closures = previous.closures.clone();
    let mut exclusions = Vec::new();
    let mut plans = Vec::new();
    let mut eligible_repository_ids = Vec::new();
    let mut eligible_repository_paths = Vec::new();
    for repository in repositories {
        if let Some(reason) = exclusion_reason(repository) {
            exclusions.push(RemediationExclusion {
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                reason,
            });
            continue;
        }
        eligible_repository_ids.push(repository.id.clone());
        eligible_repository_paths.push(repository.path.clone());
        let previous_plan = previous_by_repository.get(repository.id.as_str()).copied();
        let plan = build_plan(
            repository,
            previous_plan,
            source_refresh_id,
            &generated_at,
            fleet_audit_root,
        );
        if plan_is_terminal(&plan) {
            if let Some(previous_plan) = previous_plan {
                closures.push(closure_from_transition(
                    repository,
                    previous_plan,
                    &plan,
                    &generated_at,
                    source_refresh_id,
                ));
            }
        } else {
            plans.push(plan);
        }
    }
    rank_active_plans(&mut plans);
    deduplicate_closures(&mut closures);
    exclusions.sort_by(|left, right| left.repository_name.cmp(&right.repository_name));
    eligible_repository_ids.sort();
    eligible_repository_paths.sort();
    let run_id = stable_id(
        &format!("run:{}:{}", generated_at, plans.len()),
        "remediation-run",
    );
    RemediationRun {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        id: run_id,
        generated_at,
        source_refresh_id: source_refresh_id.map(str::to_string),
        status: if previous.status.is_empty() {
            "not_run".to_string()
        } else {
            previous.status.clone()
        },
        message: previous.message.clone(),
        eligible_repository_ids,
        eligible_repository_paths,
        refresh_steps: previous.refresh_steps.clone(),
        excluded_repositories: exclusions,
        closures,
        plans,
    }
}

pub fn sync_scope_metadata(run: &mut RemediationRun, repositories: &[RepositorySnapshot]) {
    let mut exclusions = Vec::new();
    let mut eligible_repository_ids = Vec::new();
    let mut eligible_repository_paths = Vec::new();
    for repository in repositories {
        if let Some(reason) = exclusion_reason(repository) {
            exclusions.push(RemediationExclusion {
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                reason,
            });
        } else {
            eligible_repository_ids.push(repository.id.clone());
            eligible_repository_paths.push(repository.path.clone());
        }
    }
    exclusions.sort_by(|left, right| left.repository_name.cmp(&right.repository_name));
    eligible_repository_ids.sort();
    eligible_repository_paths.sort();
    run.excluded_repositories = exclusions;
    run.eligible_repository_ids = eligible_repository_ids;
    run.eligible_repository_paths = eligible_repository_paths;
}

pub fn set_refresh_metadata(
    run: &mut RemediationRun,
    refresh_id: &str,
    status: &str,
    message: Option<String>,
    eligible_repository_ids: Vec<String>,
    eligible_repository_paths: Vec<String>,
    refresh_steps: Vec<RemediationRefreshStep>,
) {
    run.source_refresh_id = Some(refresh_id.to_string());
    run.status = status.to_string();
    run.message = message;
    run.eligible_repository_ids = eligible_repository_ids;
    run.eligible_repository_paths = eligible_repository_paths;
    run.refresh_steps = refresh_steps;
}

pub fn recompute_plan_derived(plan: &mut RemediationPlan) {
    plan.progress = calculate_progress(&plan.actions);
    plan.tracks = build_tracks(&plan.actions);
    plan.status = plan_status(&plan.actions);
    plan.current_stage = current_stage(&plan.actions);
}

pub fn normalize_queue(run: &mut RemediationRun, closed_at: &str) {
    let mut active_plans = Vec::new();
    for plan in std::mem::take(&mut run.plans) {
        if plan_is_terminal(&plan) {
            run.closures.push(closure_from_plan(
                &plan,
                closed_at,
                plan.source_refresh_id.as_deref(),
            ));
        } else {
            active_plans.push(plan);
        }
    }
    rank_active_plans(&mut active_plans);
    deduplicate_closures(&mut run.closures);
    run.plans = active_plans;
}

pub fn action_has_fresh_evidence(action: &RemediationAction) -> bool {
    action.evidence.iter().any(|item| {
        item.freshness.eq_ignore_ascii_case("fresh") && {
            let status = item.status.to_ascii_lowercase();
            ![
                "failed",
                "blocked",
                "missing",
                "stale",
                "unknown",
                "open",
                "dirty",
                "unconfirmed",
                "not configured",
            ]
            .iter()
            .any(|blocked| status.contains(blocked))
                && !status.contains("finding")
                && !status.contains("below target")
                && !status.contains("ahead")
                && !status.contains("behind")
        }
    })
}

fn plan_is_terminal(plan: &RemediationPlan) -> bool {
    matches!(plan.status.as_str(), "complete" | "deferred")
}

fn queue_status_rank(status: &str) -> u8 {
    match status {
        "blocked" => 0,
        "in_progress" => 1,
        _ => 2,
    }
}

fn queue_domain_rank(domain: &str) -> u8 {
    match domain {
        "scope" => 0,
        "product_truth" => 1,
        "branch_hygiene" => 2,
        "repository_health" => 3,
        "provider" => 4,
        "evidence_refresh" => 5,
        "ci_ideal" => 6,
        "qr_findings" => 7,
        "maturity" => 8,
        "verification" => 9,
        _ => 10,
    }
}

fn queue_priority_rank(priority: &str) -> u8 {
    match priority {
        "P0" => 0,
        "P1" => 1,
        "P2" => 2,
        "P3" => 3,
        _ => 4,
    }
}

fn queue_leverage(repository_name: &str) -> (u8, &'static str) {
    match repository_name.to_ascii_lowercase().as_str() {
        "pronto" => (0, "Fleet control plane"),
        "aios" => (1, "Agent coordination control plane"),
        "quality-runner" => (2, "Fleet evidence provider"),
        _ => (3, "Repository"),
    }
}

fn plan_queue_key(plan: &RemediationPlan) -> (u8, u8, u8, u8, u8, std::cmp::Reverse<u64>, String) {
    let active_actions = plan
        .actions
        .iter()
        .filter(|action| !matches!(action.status.as_str(), "verified" | "deferred"))
        .collect::<Vec<_>>();
    let domain_rank = active_actions
        .iter()
        .map(|action| queue_domain_rank(&action.domain))
        .min()
        .unwrap_or(u8::MAX);
    let priority_rank = active_actions
        .iter()
        .map(|action| queue_priority_rank(&action.priority))
        .min()
        .unwrap_or(u8::MAX);
    let active_weight = active_actions.iter().map(|action| action.weight).sum();
    (
        queue_status_rank(&plan.status),
        domain_rank,
        priority_rank,
        queue_leverage(&plan.repository_name).0,
        goal_queue_rank(&plan.goal.target_state),
        std::cmp::Reverse(active_weight),
        plan.repository_name.to_ascii_lowercase(),
    )
}

fn rank_active_plans(plans: &mut [RemediationPlan]) {
    plans.sort_by_key(plan_queue_key);
}

fn latest_plan_evidence_at(plan: &RemediationPlan) -> Option<String> {
    plan.actions
        .iter()
        .flat_map(|action| action.evidence.iter())
        .filter_map(|item| item.observed_at.clone())
        .max()
}

fn closure_from_plan(
    plan: &RemediationPlan,
    closed_at: &str,
    source_refresh_id: Option<&str>,
) -> RemediationClosure {
    let deferred_action_count = plan
        .actions
        .iter()
        .filter(|action| action.status == "deferred")
        .count();
    let verified_action_count = plan
        .actions
        .iter()
        .filter(|action| action.status == "verified")
        .count();
    let disposition = if plan.status == "deferred" {
        "deferred"
    } else {
        "verified"
    };
    let summary = if plan.actions.is_empty() {
        "A fresh remediation projection found no active actions.".to_string()
    } else {
        format!(
            "{} action(s) left the active queue with disposition '{}'.",
            plan.actions.len(),
            disposition
        )
    };
    RemediationClosure {
        id: stable_id(
            &format!(
                "closure:{}:{}:{}",
                plan.repository_id, closed_at, disposition
            ),
            "remediation-closure",
        ),
        repository_id: plan.repository_id.clone(),
        repository_name: plan.repository_name.clone(),
        repository_path: plan.repository_path.clone(),
        plan_id: plan.id.clone(),
        target_state: plan.goal.target_state.clone(),
        goal_source: plan.goal.source.clone(),
        closed_at: closed_at.to_string(),
        source_refresh_id: source_refresh_id.map(str::to_string),
        disposition: disposition.to_string(),
        summary,
        resolved_action_count: plan.actions.len(),
        verified_action_count,
        deferred_action_count,
        last_evidence_at: latest_plan_evidence_at(plan),
    }
}

fn closure_from_transition(
    repository: &RepositorySnapshot,
    previous: &RemediationPlan,
    current: &RemediationPlan,
    closed_at: &str,
    source_refresh_id: Option<&str>,
) -> RemediationClosure {
    let mut closure = closure_from_plan(current, closed_at, source_refresh_id);
    closure.resolved_action_count = previous.actions.len();
    closure.last_evidence_at =
        latest_plan_evidence_at(current).or_else(|| Some(repository.last_scan_at.clone()));
    if current.actions.is_empty() {
        closure.summary = format!(
            "Fresh evidence removed {} prior action(s) from the active remediation queue.",
            previous.actions.len()
        );
    }
    closure
}

fn deduplicate_closures(closures: &mut Vec<RemediationClosure>) {
    closures.sort_by(|left, right| {
        right
            .closed_at
            .cmp(&left.closed_at)
            .then_with(|| left.repository_name.cmp(&right.repository_name))
    });
    let mut seen = std::collections::HashSet::new();
    closures.retain(|closure| seen.insert(closure.id.clone()));
}

fn build_plan(
    repository: &RepositorySnapshot,
    previous: Option<&RemediationPlan>,
    source_refresh_id: Option<&str>,
    generated_at: &str,
    fleet_audit_root: Option<&Path>,
) -> RemediationPlan {
    let goal = resolve_goal_profile(repository);
    let qr_run = latest_qr_run(Path::new(&repository.path), fleet_audit_root);
    let mut seeds = Vec::new();
    add_scope_seed(repository, &mut seeds);
    add_goal_seeds(repository, &goal, &mut seeds);
    add_release_evidence_seeds(repository, &goal, &mut seeds);
    add_project_compass_seeds(repository, &mut seeds);
    if goal_requires_provider(&goal) {
        add_provider_seed(repository, &mut seeds);
        add_pull_request_seeds(repository, &mut seeds);
    }
    if goal_requires_quality_evidence(&goal) {
        add_evidence_seed(repository, qr_run.as_ref(), &goal, &mut seeds);
    }
    add_ci_ideal_seeds(repository, &goal, &mut seeds);
    add_qr_finding_seeds(repository, qr_run.as_ref(), &goal, &mut seeds);
    add_branch_hygiene_seeds(repository, &mut seeds);
    add_submodule_seeds(repository, &mut seeds);
    if goal_requires_maturity(&goal) {
        add_maturity_seeds(repository, qr_run.as_ref(), &goal, &mut seeds);
    }

    if !seeds.is_empty() {
        seeds.push(ActionSeed {
            stable_key: "verification:recheck-after-remediation".to_string(),
            domain: "verification".to_string(),
            title: "Verify the repository after remediation".to_string(),
            summary: "Re-run the eligible evidence sources and confirm the plan is clear before closing it.".to_string(),
            severity: "verification".to_string(),
            priority: "P2".to_string(),
            weight: 1,
            acceptance_criteria: vec![
                "A fresh local snapshot is recorded.".to_string(),
                "Project Compass, workspace, branch, submodule, condition, provider, quality, CI, and maturity evidence are rechecked where applicable.".to_string(),
                "No unresolved blocking action remains.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Derived verification gate",
                "Open",
                "Unknown",
                None,
                None,
                "Verification is required after the source gaps are addressed.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: qr_run.as_ref().map(|run| run.id.clone()),
        });
    }

    let previous_actions = previous
        .map(|plan| {
            plan.actions
                .iter()
                .map(|action| (action.stable_key.as_str(), action))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let actions = seeds
        .into_iter()
        .map(|seed| materialize_action(repository, seed, &previous_actions, generated_at))
        .collect::<Vec<_>>();
    let progress = calculate_progress(&actions);
    let coverage = build_ui_coverage(repository, &goal, &actions);
    let tracks = build_tracks(&actions);
    let status = plan_status(&actions);
    let current_stage = current_stage(&actions);
    let plan_id = stable_id(&format!("plan:{}", repository.id), "remediation-plan");
    RemediationPlan {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        id: plan_id,
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        generated_at: generated_at.to_string(),
        source_refresh_id: source_refresh_id
            .map(str::to_string)
            .or_else(|| qr_run.as_ref().map(|run| run.id.clone())),
        goal,
        current_stage,
        status,
        progress,
        coverage,
        tracks,
        actions,
    }
}

fn add_goal_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    if goal.source != "repository_contract" {
        let detail = goal.error.as_deref().unwrap_or(
            "No repository-owned remediation goal contract is present; this profile is inferred.",
        );
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-remediation-goal".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the repository remediation goal".to_string(),
            summary: format!(
                "Pronto inferred '{}' for this repository. Confirm the intended outcome before using it as the closure contract.",
                goal.label
            ),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                format!(
                    "Create {} with schema {}.",
                    REMEDIATION_GOAL_PATH, REMEDIATION_GOAL_SCHEMA
                ),
                "Record the intended target_state and a non-empty reason.".to_string(),
                "Refresh Pronto and confirm the profile source is repository_contract."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Remediation goal",
                "Inferred",
                "Unknown",
                Some(&repository.last_scan_at),
                Some(REMEDIATION_GOAL_PATH),
                detail,
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if goal.target_state == "public_release"
        && (repository.release_rule.is_none() || repository.release_recipe.is_none())
    {
        seeds.push(ActionSeed {
            stable_key: "scope:release-contract".to_string(),
            domain: "scope".to_string(),
            title: "Establish the release contract".to_string(),
            summary: "A public-release repository needs both an explicit release rule and an executable release recipe before it can leave remediation.".to_string(),
            severity: "release".to_string(),
            priority: "P1".to_string(),
            weight: 3,
            acceptance_criteria: vec![
                "A normalized release rule defines when a release is eligible.".to_string(),
                "A release recipe defines validation, generation, commit, and publication steps."
                    .to_string(),
                "Release preview can evaluate the repository without inventing provider evidence."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Release contract",
                "Incomplete",
                "Fresh",
                Some(&repository.last_scan_at),
                None,
                &format!(
                    "Release rule: {} · release recipe: {}.",
                    if repository.release_rule.is_some() {
                        "configured"
                    } else {
                        "missing"
                    },
                    if repository.release_recipe.is_some() {
                        "configured"
                    } else {
                        "missing"
                    }
                ),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if goal.target_state == "archived"
        && !repository.lifecycle.to_ascii_lowercase().contains("archiv")
    {
        seeds.push(ActionSeed {
            stable_key: "scope:align-archived-lifecycle".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the archived lifecycle".to_string(),
            summary: "The remediation goal is archived, but the repository lifecycle does not yet record an archived state.".to_string(),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The repository lifecycle is explicitly confirmed as archived.".to_string(),
                "No active ownership or release obligation remains.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Lifecycle alignment",
                &repository.lifecycle,
                "Fresh",
                Some(&repository.last_scan_at),
                Some(REMEDIATION_GOAL_PATH),
                "The archived goal and repository lifecycle must agree.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_scope_seed(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let lifecycle = repository.lifecycle.to_ascii_lowercase();
    let candidate = repository.lifecycle_candidate.to_ascii_lowercase();
    if lifecycle.contains("unconfirmed") || candidate != lifecycle && !candidate.is_empty() {
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-lifecycle".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the repository scope".to_string(),
            summary: format!(
                "Pronto has lifecycle evidence of '{}' with candidate '{}'; confirm the repository's intended scope before prioritizing remediation.",
                repository.lifecycle, repository.lifecycle_candidate
            ),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The repository lifecycle is explicitly confirmed.".to_string(),
                "The decision is recorded in Pronto before work is planned.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Lifecycle and scope state",
                &repository.lifecycle,
                "Fresh",
                Some(&repository.last_scan_at),
                None,
                &format!("Candidate lifecycle: {}", repository.lifecycle_candidate),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if repository.default_branch.is_none() {
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-default-branch".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the canonical integration branch".to_string(),
            summary: "Pronto cannot determine the repository's default branch, so branch comparisons and integration targets are ambiguous.".to_string(),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The canonical integration branch is confirmed from repository or provider evidence.".to_string(),
                "Pronto records the default branch and can evaluate branch integration against it.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Default branch",
                "Missing",
                "Unknown",
                Some(&repository.last_scan_at),
                None,
                "No default branch was resolved for this repository.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_project_compass_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let compass = &repository.project_compass;
    match compass.status.as_str() {
        "Missing" => seeds.push(ActionSeed {
            stable_key: "product_truth:project-compass".to_string(),
            domain: "product_truth".to_string(),
            title: "Establish the Project Compass contract".to_string(),
            summary: "The repository has no Project Compass contract, so Pronto cannot relate remediation work to the intended product outcome.".to_string(),
            severity: "product_truth".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Use the Project Compass workflow to make an explicit product-truth decision; do not infer or silently invent the contract.".to_string(),
                format!("A valid contract is present at {}.", compass.contract_path),
                "Refresh Pronto and confirm Project Compass is Ready.".to_string(),
            ],
            evidence: vec![evidence(
                "Project Compass",
                "Product truth contract",
                "Missing",
                "Fresh",
                Some(&repository.last_scan_at),
                Some(&compass.contract_path),
                "The UI tracks Project Compass for this repository, but no contract exists.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        }),
        "Invalid" => seeds.push(ActionSeed {
            stable_key: "product_truth:project-compass".to_string(),
            domain: "product_truth".to_string(),
            title: "Repair the Project Compass contract".to_string(),
            summary: "The repository's Project Compass contract is present but invalid, so its product progress cannot be trusted.".to_string(),
            severity: "product_truth".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Repair the existing contract through the Project Compass workflow without inventing product truth.".to_string(),
                "The contract validates and Pronto reports Project Compass as Ready.".to_string(),
            ],
            evidence: vec![evidence(
                "Project Compass",
                "Product truth contract",
                "Invalid",
                "Fresh",
                compass.updated_at.as_deref().or(Some(&repository.last_scan_at)),
                Some(&compass.contract_path),
                compass.error.as_deref().unwrap_or("The contract is invalid."),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        }),
        "Ready" => {
            if compass.open_blockers > 0 {
                seeds.push(ActionSeed {
                    stable_key: "product_truth:project-compass-blockers".to_string(),
                    domain: "product_truth".to_string(),
                    title: "Resolve Project Compass blockers".to_string(),
                    summary: format!(
                        "Project Compass records {} open product blocker(s).",
                        compass.open_blockers
                    ),
                    severity: "product_truth".to_string(),
                    priority: "P1".to_string(),
                    weight: compass.open_blockers.max(1) as u64,
                    acceptance_criteria: vec![
                        "Each blocker is resolved or explicitly dispositioned in the canonical Project Compass contract.".to_string(),
                        "Pronto refreshes the contract and reports no unexplained open blocker.".to_string(),
                    ],
                    evidence: vec![evidence(
                        "Project Compass",
                        "Open blockers",
                        &compass.open_blockers.to_string(),
                        "Fresh",
                        compass.updated_at.as_deref(),
                        Some(&compass.contract_path),
                        "Open product blockers are tracked as remediation work.",
                    )],
                    related_finding_ids: Vec::new(),
                    source_run_id: None,
                });
            }
            if compass.open_drift > 0 {
                seeds.push(ActionSeed {
                    stable_key: "product_truth:project-compass-drift".to_string(),
                    domain: "product_truth".to_string(),
                    title: "Reconcile Project Compass drift".to_string(),
                    summary: format!(
                        "Project Compass records {} open drift item(s) between product truth and implementation.",
                        compass.open_drift
                    ),
                    severity: "product_truth".to_string(),
                    priority: "P1".to_string(),
                    weight: compass.open_drift.max(1) as u64,
                    acceptance_criteria: vec![
                        "Each drift item is reconciled in implementation or explicitly dispositioned in Project Compass.".to_string(),
                        "Pronto refreshes the contract and reports no unexplained open drift.".to_string(),
                    ],
                    evidence: vec![evidence(
                        "Project Compass",
                        "Open drift",
                        &compass.open_drift.to_string(),
                        "Fresh",
                        compass.updated_at.as_deref(),
                        Some(&compass.contract_path),
                        "Open product-to-implementation drift is tracked as remediation work.",
                    )],
                    related_finding_ids: Vec::new(),
                    source_run_id: None,
                });
            }
        }
        _ => seeds.push(ActionSeed {
            stable_key: "product_truth:project-compass".to_string(),
            domain: "product_truth".to_string(),
            title: "Resolve the unknown Project Compass state".to_string(),
            summary: format!(
                "Pronto received an unrecognized Project Compass state: '{}'.",
                compass.status
            ),
            severity: "product_truth".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Inspect the canonical contract and Project Compass tooling.".to_string(),
                "Refresh Pronto and confirm the state is Ready, Missing, or Invalid.".to_string(),
            ],
            evidence: vec![evidence(
                "Project Compass",
                "Product truth contract",
                &compass.status,
                "Unknown",
                compass.updated_at.as_deref(),
                Some(&compass.contract_path),
                "The Project Compass state is not recognized by the remediation planner.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        }),
    }
}

fn add_release_evidence_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    if goal.target_state != "public_release"
        || !repository.releases.is_empty()
        || repository
            .release_rule
            .as_ref()
            .is_none_or(|rule| rule.allow_first_release)
    {
        return;
    }
    seeds.push(ActionSeed {
        stable_key: "release_evidence:published-baseline".to_string(),
        domain: "provider".to_string(),
        title: "Resolve the missing published-release baseline".to_string(),
        summary: "The public-release goal requires release evidence, but no published baseline is available and the release rule does not allow a first release.".to_string(),
        severity: "release".to_string(),
        priority: "P1".to_string(),
        weight: 2,
        acceptance_criteria: vec![
            "Confirm whether this is an intentional first release or a missing provider snapshot."
                .to_string(),
            "If it is a first release, explicitly authorize that case in the release rule; otherwise refresh the published release evidence.".to_string(),
            "Release preparation reports an evidence-ready baseline disposition.".to_string(),
        ],
        evidence: vec![evidence(
            "Pronto release preparation",
            "Published release baseline",
            "Missing",
            if repository.last_fetch_at.is_some() {
                "Fresh"
            } else {
                "Unknown"
            },
            repository.last_fetch_at.as_deref(),
            None,
            "No published release is present and allow_first_release is false.",
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn add_provider_seed(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let provider_state = repository.provider_state.to_ascii_lowercase();
    let connected = provider_state.contains("connected") && repository.last_fetch_at.is_some();
    let (title, summary, detail) = if repository.remote_url.is_none() {
        (
            "Confirm the repository remote/provider identity",
            "No remote URL is recorded, so provider freshness and CI context cannot be matched to this repository.",
            "Remote URL is missing from the local snapshot.",
        )
    } else if !connected {
        (
            "Refresh the provider snapshot",
            "A GitHub remote is known locally, but Pronto does not have a confirmed fresh provider snapshot for it.",
            "Remote detected; provider snapshot is not confirmed fresh.",
        )
    } else {
        return;
    };
    seeds.push(ActionSeed {
        stable_key: "provider:remote-freshness".to_string(),
        domain: "provider".to_string(),
        title: title.to_string(),
        summary: summary.to_string(),
        severity: "provider".to_string(),
        priority: "P1".to_string(),
        weight: 2,
        acceptance_criteria: vec![
            "The local remote maps to the intended provider repository.".to_string(),
            "A provider refresh records a successful fetch timestamp.".to_string(),
            "Remote evidence remains read-only and is not treated as a source edit.".to_string(),
        ],
        evidence: vec![evidence(
            "Pronto",
            "Provider freshness",
            &repository.provider_state,
            if connected { "Fresh" } else { "Unknown" },
            repository.last_fetch_at.as_deref(),
            None,
            detail,
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn add_pull_request_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    for pull_request in repository
        .pull_requests
        .iter()
        .filter(|pull_request| pull_request.state.eq_ignore_ascii_case("open"))
    {
        let checks_ready = pull_request.checks_state.eq_ignore_ascii_case("passed");
        let reviews_ready = pull_request
            .reviews_state
            .to_ascii_lowercase()
            .contains("approved");
        let mergeability = pull_request.mergeability.to_ascii_lowercase();
        let merge_ready = matches!(mergeability.as_str(), "clean" | "mergeable");
        if !pull_request.draft && checks_ready && reviews_ready && merge_ready {
            continue;
        }
        seeds.push(ActionSeed {
            stable_key: format!("provider:pull-request:{}", pull_request.number),
            domain: "provider".to_string(),
            title: format!("Resolve pull request evidence · #{}", pull_request.number),
            summary: format!(
                "Open pull request #{} is not fully ready: draft {} · checks {} · reviews {} · mergeability {}.",
                pull_request.number,
                pull_request.draft,
                pull_request.checks_state,
                pull_request.reviews_state,
                pull_request.mergeability
            ),
            severity: "provider".to_string(),
            priority: if pull_request.checks_state.eq_ignore_ascii_case("failed") {
                "P1"
            } else {
                "P2"
            }
            .to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Refresh provider-native pull request evidence for the exact head and base."
                    .to_string(),
                "Required checks, reviews, draft state, and mergeability are explicitly resolved or dispositioned.".to_string(),
                "Pronto records the resulting pull request snapshot without inferring provider success."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "GitHub",
                &format!("Pull request #{}", pull_request.number),
                if pull_request.draft {
                    "Draft"
                } else {
                    &pull_request.checks_state
                },
                "Fresh",
                Some(&pull_request.last_refreshed_at),
                Some(&pull_request.html_url),
                &format!(
                    "{} → {} · reviews {} · mergeability {}.",
                    pull_request.head_branch,
                    pull_request.base_branch,
                    pull_request.reviews_state,
                    pull_request.mergeability
                ),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_evidence_seed(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let Some(run) = qr_run else {
        seeds.push(ActionSeed {
            stable_key: "evidence_refresh:qr-run".to_string(),
            domain: "evidence_refresh".to_string(),
            title: "Run a fresh Quality Runner audit".to_string(),
            summary: "No repository-local QR run is available, so findings and verification evidence are not current.".to_string(),
            severity: "evidence".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "QR doctor passes before execution.".to_string(),
                "A full QR audit is run for this repository.".to_string(),
                "The new run is replay-valid and its artifacts are imported by Pronto.".to_string(),
            ],
            evidence: vec![evidence(
                "Quality Runner",
                "Latest repository run",
                "Missing",
                "Unknown",
                None,
                None,
                "No .quality-runner/runs artifact was found.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
        return;
    };
    let freshness = freshness_for(run.observed_at.as_deref(), goal.evidence_max_age_days);
    if freshness != "Fresh" {
        seeds.push(ActionSeed {
            stable_key: "evidence_refresh:qr-run".to_string(),
            domain: "evidence_refresh".to_string(),
            title: "Refresh the Quality Runner evidence".to_string(),
            summary:
                "The latest QR artifact is present but no longer fresh for this remediation run."
                    .to_string(),
            severity: "evidence".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "QR doctor passes before execution.".to_string(),
                "A new full QR run is written for this repository.".to_string(),
                "The run timestamp and commit match the current local snapshot.".to_string(),
            ],
            evidence: vec![evidence(
                "Quality Runner",
                "Latest repository run",
                "Present",
                &freshness,
                run.observed_at.as_deref(),
                run.run_dir.to_str(),
                "The repository has QR artifacts, but they are outside the fresh-evidence window.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
    if repository.quality.findings.freshness.as_str() != "Fresh"
        && !seeds
            .iter()
            .any(|seed| seed.stable_key == "evidence_refresh:qr-run")
    {
        seeds.push(ActionSeed {
            stable_key: "evidence_refresh:pronto-import".to_string(),
            domain: "evidence_refresh".to_string(),
            title: "Re-import the QR result into Pronto".to_string(),
            summary: "A QR run exists, but the Pronto quality projection is not fresh against the current repository evidence.".to_string(),
            severity: "evidence".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Pronto ingests the latest QR run without changing source files.".to_string(),
                "The finding report path and observed timestamp are present.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Imported QR findings",
                "Stale",
                "Stale",
                repository.quality.findings.observed_at.as_deref(),
                repository.quality.findings.report_path.as_deref(),
                "The local quality projection is not fresh.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
}

fn add_ci_ideal_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let readiness = &repository.quality.ci_readiness;
    let mut gate_ids = readiness.unconfigured_gate_ids.clone();
    gate_ids.extend(readiness.missing_gate_ids.iter().cloned());
    gate_ids.extend(readiness.stale_gate_ids.iter().cloned());
    gate_ids.extend(readiness.failed_gate_ids.iter().cloned());
    gate_ids.extend(readiness.blocked_gate_ids.iter().cloned());
    gate_ids.sort();
    gate_ids.dedup();
    gate_ids.retain(|gate_id| goal.required_gate_ids.contains(gate_id));
    for gate_id in gate_ids {
        let status = if readiness.failed_gate_ids.contains(&gate_id) {
            "Failed"
        } else if readiness.blocked_gate_ids.contains(&gate_id) {
            "Blocked"
        } else if readiness.stale_gate_ids.contains(&gate_id) {
            "Stale"
        } else if readiness.unconfigured_gate_ids.contains(&gate_id) {
            "Not configured"
        } else {
            "Missing"
        };
        let label = gate_id.replace('_', " ");
        let gate_evidence = repository
            .quality
            .gates
            .iter()
            .find(|gate| gate.id == gate_id)
            .and_then(|gate| gate.evidence.first());
        let freshness = repository
            .quality
            .gates
            .iter()
            .find(|gate| gate.id == gate_id)
            .map(|gate| gate.freshness.as_str())
            .unwrap_or_else(|| {
                if readiness.stale_gate_ids.contains(&gate_id) {
                    "Stale"
                } else {
                    "Unknown"
                }
            });
        seeds.push(ActionSeed {
            stable_key: format!("ci_ideal:gate:{gate_id}"),
            domain: "ci_ideal".to_string(),
            title: format!("Bring the {label} gate to the ideal state"),
            summary: format!(
                "The '{}' remediation goal requires {label}; its current state is {status}.",
                goal.label
            ),
            severity: if matches!(status, "Failed" | "Blocked") {
                "high".to_string()
            } else {
                "quality".to_string()
            },
            priority: if matches!(status, "Failed" | "Blocked") {
                "P1".to_string()
            } else {
                "P2".to_string()
            },
            weight: 2,
            acceptance_criteria: vec![
                format!(
                    "The {label} gate is configured according to the repository ideal profile."
                ),
                format!("A fresh passing {label} result is recorded where the gate is executable."),
                "The gate is represented in Pronto with its source, status, and freshness."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "Pronto quality",
                &format!("Ideal CI gate · {label}"),
                status,
                freshness,
                gate_evidence.and_then(|item| item.observed_at.as_deref()),
                gate_evidence.and_then(|item| item.report_path.as_deref()),
                &format!(
                    "Required by the repository's '{}' remediation goal.",
                    goal.target_state
                ),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_qr_finding_seeds(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let Some(run) = qr_run else {
        return;
    };
    let mut groups = BTreeMap::<String, Vec<&ParsedFinding>>::new();
    for finding in &run.findings {
        groups
            .entry(finding.group_key.clone())
            .or_default()
            .push(finding);
    }
    for (group_key, findings) in groups {
        let severity = findings
            .iter()
            .max_by_key(|finding| severity_weight(&finding.severity))
            .map(|finding| finding.severity.clone())
            .unwrap_or_else(|| "warning".to_string());
        let max_weight = findings
            .iter()
            .map(|finding| severity_weight(&finding.severity))
            .max()
            .unwrap_or(2);
        let count = findings.len() as u64;
        let first = findings[0];
        let locations = findings
            .iter()
            .filter_map(|finding| {
                finding.file.as_ref().map(|file| match finding.line {
                    Some(line) => format!("{file}:{line}"),
                    None => file.clone(),
                })
            })
            .take(3)
            .collect::<Vec<_>>();
        let detail = if locations.is_empty() {
            format!("{} finding(s) in the current QR report.", findings.len())
        } else {
            format!(
                "{} finding(s); examples: {}",
                findings.len(),
                locations.join(", ")
            )
        };
        let source_path = first.report_path.clone();
        let source_label = first
            .pack
            .as_deref()
            .map(|pack| format!("QR · {pack} · {}", first.category))
            .unwrap_or_else(|| format!("QR · {}", first.category));
        seeds.push(ActionSeed {
            stable_key: format!("qr_findings:group:{group_key}"),
            domain: "qr_findings".to_string(),
            title: if first.title.is_empty() {
                format!("Resolve {} QR findings", first.category)
            } else {
                first.title.clone()
            },
            summary: format!("{} {}", first.summary, detail),
            severity: severity.clone(),
            priority: priority_for_weight(max_weight),
            weight: max_weight.saturating_mul(count).max(1),
            acceptance_criteria: findings
                .iter()
                .find_map(|finding| finding.verification.clone())
                .map(|verification| vec![verification])
                .unwrap_or_else(|| {
                    vec![
                        "The grouped QR findings are resolved in source or configuration."
                            .to_string(),
                        "The relevant verification command or evidence is rerun.".to_string(),
                        "A fresh QR report no longer includes these finding identities."
                            .to_string(),
                    ]
                }),
            evidence: vec![evidence(
                "Quality Runner",
                &source_label,
                &format!("{count} finding(s)"),
                &freshness_for(run.observed_at.as_deref(), goal.evidence_max_age_days),
                run.observed_at.as_deref(),
                Some(&source_path),
                &detail,
            )],
            related_finding_ids: findings.iter().map(|finding| finding.id.clone()).collect(),
            source_run_id: Some(run.id.clone()),
        });
    }
    if run.findings.is_empty() && repository.quality.findings.total > 0 {
        seeds.push(ActionSeed {
            stable_key: "qr_findings:aggregate-report".to_string(),
            domain: "qr_findings".to_string(),
            title: "Resolve the findings in the QR report".to_string(),
            summary: format!(
                "Pronto imported {} QR findings, but the current artifact does not expose leaf identities for grouping.",
                repository.quality.findings.total
            ),
            severity: if repository.quality.findings.high_severity_total > 0 {
                "high".to_string()
            } else {
                "warning".to_string()
            },
            priority: "P1".to_string(),
            weight: repository.quality.findings.high_severity_total.max(1),
            acceptance_criteria: vec![
                "The current QR report is reviewed and its findings are addressed.".to_string(),
                "A fresh QR report is rerun to verify the result.".to_string(),
            ],
            evidence: vec![evidence(
                "Quality Runner",
                "QR aggregate report",
                &repository.quality.findings.total.to_string(),
                repository.quality.findings.freshness.as_str(),
                repository.quality.findings.observed_at.as_deref(),
                repository.quality.findings.report_path.as_deref(),
                "Leaf finding identities were not available in the current report.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
}

fn add_branch_hygiene_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let workspaces = if repository.workspaces.is_empty() {
        vec![&repository.workspace]
    } else {
        repository.workspaces.iter().collect::<Vec<_>>()
    };
    for workspace in workspaces {
        if workspace_activity_requires_coordination(&workspace.activity) {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:activity:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Coordinate workspace ownership · {}", workspace.branch),
                summary: format!(
                    "{} reports {} activity; preserve ownership before changing its branch or worktree.",
                    workspace.path, workspace.activity.state
                ),
                severity: "ownership".to_string(),
                priority: "P0".to_string(),
                weight: 3,
                acceptance_criteria: vec![
                    "The active or interrupted owner is identified and coordinated with."
                        .to_string(),
                    "No worktree, branch, or unpublished work is changed while ownership is ambiguous."
                        .to_string(),
                    "A refreshed snapshot no longer reports unresolved active ownership.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Workspace activity · {}", workspace.branch),
                    &workspace.activity.state,
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    &format!(
                        "{} activity signal(s); confidence {}.",
                        workspace.activity.signals.len(),
                        workspace.activity.confidence
                    ),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if let Some(operation) = workspace.operation.as_deref() {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:operation:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Resolve interrupted Git operation · {}", workspace.branch),
                summary: format!(
                    "{} has an interrupted {operation} operation that must be resolved before reconciliation.",
                    workspace.path
                ),
                severity: "operation".to_string(),
                priority: "P0".to_string(),
                weight: 3,
                acceptance_criteria: vec![
                    "Inspect and intentionally complete or abort the interrupted Git operation."
                        .to_string(),
                    "Preserve all unrelated and unpublished work.".to_string(),
                    "A refreshed snapshot reports no interrupted operation.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Git operation · {}", workspace.branch),
                    operation,
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    "An interrupted Git operation is a hard stop for branch mutation.",
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if workspace.dirty {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:dirty:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Resolve dirty workspace · {}", workspace.branch),
                summary: format!("{} has uncommitted changes that must be reviewed before remediation can be verified.", workspace.path),
                severity: "workspace".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "The changes are intentionally committed, stashed, or explicitly preserved.".to_string(),
                    "No unreviewed local changes are hidden by the remediation run.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Dirty workspace · {}", workspace.branch),
                    "Dirty",
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    &format!("{} changed lines added and {} removed.", workspace.added, workspace.removed),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if workspace.sync_state != "Synced" {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:sync:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Reconcile branch sync · {}", workspace.branch),
                summary: format!(
                    "The workspace is {} relative to its upstream/default branch.",
                    workspace.sync_state
                ),
                severity: "sync".to_string(),
                priority: "P2".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "The branch relationship is understood and intentional.".to_string(),
                    "The final verification run records the resulting branch state.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Branch sync · {}", workspace.branch),
                    &workspace.sync_state,
                    "Fresh",
                    workspace.last_commit_at.as_deref(),
                    None,
                    &format!("Ahead {} · behind {}", workspace.ahead, workspace.behind),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        let remote_freshness = workspace.remote_freshness.to_ascii_lowercase();
        if remote_freshness.contains("not fetched")
            || remote_freshness.contains("stale")
            || remote_freshness.contains("unknown")
            || remote_freshness.contains("unavailable")
        {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:remote-freshness:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Refresh remote branch evidence · {}", workspace.branch),
                summary: "The workspace's remote comparison is not fresh enough to support reconciliation or pruning decisions.".to_string(),
                severity: "freshness".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "Fetch remote evidence through the authorized Pronto workflow.".to_string(),
                    "Recompute ahead, behind, upstream, and integration state from fresh evidence."
                        .to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Remote freshness · {}", workspace.branch),
                    &workspace.remote_freshness,
                    &workspace.remote_freshness,
                    workspace.last_commit_at.as_deref(),
                    None,
                    "Stale or unavailable remote evidence cannot prove branch closure.",
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
        if workspace.integration_state == "Integration eligible" {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:integrate:{}", workspace.branch),
                domain: "branch_hygiene".to_string(),
                title: format!("Classify integration-ready branch · {}", workspace.branch),
                summary: format!(
                    "{} has unique commits eligible for integration into {}.",
                    workspace.branch,
                    workspace.target_branch.as_deref().unwrap_or("the canonical branch")
                ),
                severity: "integration".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "Review the complete diff and confirm whether the work is wanted, superseded, or intentionally preserved.".to_string(),
                    "Fold wanted work through the documented integration lane and verify the canonical branch.".to_string(),
                    "Treat pruning as a separate, later authorization after integration equivalence is proven.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Branch integration · {}", workspace.branch),
                    &workspace.integration_state,
                    "Fresh",
                    workspace.last_commit_at.as_deref(),
                    None,
                    &format!(
                        "Target: {} · confidence: {}.",
                        workspace.target_branch.as_deref().unwrap_or("Unknown"),
                        workspace.target_confidence
                    ),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
    }
    for branch in &repository.branches {
        if branch.integration_state == "Integration eligible"
            && !seeds
                .iter()
                .any(|seed| seed.stable_key == format!("branch_hygiene:integrate:{}", branch.name))
        {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:integrate:{}", branch.name),
                domain: "branch_hygiene".to_string(),
                title: format!("Classify integration-ready branch · {}", branch.name),
                summary: format!(
                    "{} has unique commits eligible for integration into {}.",
                    branch.name,
                    branch.target_branch.as_deref().unwrap_or("the canonical branch")
                ),
                severity: "integration".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "Review the complete diff and confirm whether the work is wanted, superseded, or intentionally preserved.".to_string(),
                    "Fold wanted work through the documented integration lane and verify the canonical branch.".to_string(),
                    "Treat pruning as a separate, later authorization after integration equivalence is proven.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Branch integration · {}", branch.name),
                    &branch.integration_state,
                    "Fresh",
                    branch.last_commit_at.as_deref(),
                    None,
                    &format!(
                        "Ahead {} · behind {} · target {}.",
                        branch.ahead,
                        branch.behind,
                        branch.target_branch.as_deref().unwrap_or("Unknown")
                    ),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        }
    }
    const REPRESENTED_CONDITIONS: [&str; 9] = [
        "active-agent-workspace",
        "interrupted-operation",
        "dirty-workspace",
        "diverged-branch",
        "unpushed-commits",
        "behind-remote",
        "remote-stale",
        "no-upstream",
        "integration-eligible",
    ];
    for condition in repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .filter(|condition| !REPRESENTED_CONDITIONS.contains(&condition.kind.as_str()))
    {
        seeds.push(ActionSeed {
            stable_key: format!("repository_health:condition:{}", condition.id),
            domain: "repository_health".to_string(),
            title: format!("Resolve repository condition · {}", condition.title),
            summary: condition.summary.clone(),
            severity: "condition".to_string(),
            priority: if condition.priority <= 1 { "P1" } else { "P2" }.to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Inspect the condition's rule, evidence, and missing prerequisites.".to_string(),
                "Resolve or explicitly disposition the condition and refresh Pronto.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto condition",
                &condition.title,
                &condition.status,
                condition.freshness.as_deref().unwrap_or("Unknown"),
                Some(&repository.last_scan_at),
                None,
                &condition.summary,
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn workspace_activity_requires_coordination(activity: &crate::core::WorkspaceActivity) -> bool {
    if activity.state.eq_ignore_ascii_case("active")
        || activity
            .signals
            .iter()
            .any(|signal| signal.summary == "Activity state uncertain")
    {
        return true;
    }
    activity
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.status.as_deref())
        .is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "active" | "running" | "started" | "paused" | "interrupted"
            )
        })
}

fn add_submodule_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    for submodule in &repository.submodules {
        if submodule.status == "Checked out" {
            continue;
        }
        seeds.push(ActionSeed {
            stable_key: format!("repository_health:submodule:{}", submodule.path),
            domain: "repository_health".to_string(),
            title: format!("Resolve submodule state · {}", submodule.path),
            summary: format!(
                "The submodule is '{}', so the repository snapshot is not in its expected checked-out state.",
                submodule.status
            ),
            severity: "repository_health".to_string(),
            priority: if submodule.status == "Merge conflict" {
                "P0"
            } else {
                "P1"
            }
            .to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Confirm the intended submodule commit from repository-owned evidence.".to_string(),
                "Resolve the submodule without discarding unrelated local work.".to_string(),
                "A refreshed Pronto snapshot reports the submodule as Checked out.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                &format!("Submodule · {}", submodule.path),
                &submodule.status,
                "Fresh",
                Some(&repository.last_scan_at),
                Some(&submodule.path),
                submodule
                    .commit
                    .as_deref()
                    .unwrap_or("No checked-out submodule commit was recorded."),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_maturity_seeds(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let maturity = &repository.quality.maturity;
    let score = maturity
        .score
        .or_else(|| qr_run.and_then(|run| run.maturity_score));
    let observed_at = maturity
        .observed_at
        .as_deref()
        .or_else(|| qr_run.and_then(|run| run.observed_at.as_deref()));
    let report_path = maturity
        .report_path
        .as_deref()
        .or_else(|| qr_run.and_then(|run| run.maturity_report_path.as_deref()));
    let freshness = if maturity.score.is_some() {
        maturity.freshness.as_str().to_string()
    } else {
        qr_run
            .map(|run| freshness_for(run.observed_at.as_deref(), goal.evidence_max_age_days))
            .unwrap_or_else(|| "Unknown".to_string())
    };
    let empty_dimensions = BTreeMap::new();
    let dimension_scores = if maturity.dimension_scores.is_empty() {
        qr_run
            .map(|run| &run.dimension_scores)
            .unwrap_or(&empty_dimensions)
    } else {
        &maturity.dimension_scores
    };
    let audit_id = maturity
        .audit_id
        .clone()
        .or_else(|| qr_run.map(|run| run.id.clone()));
    match score {
        None => seeds.push(maturity_seed(
            "maturity:score",
            "Get a current maturity score",
            "No repository maturity score is available in the imported feed.",
            "Missing",
            &freshness,
            observed_at,
            report_path,
            2,
        )),
        Some(score) if freshness != "Fresh" => seeds.push(maturity_seed(
            "maturity:score",
            "Refresh the maturity score",
            &format!("The maturity score is {score:.3}/4 but its evidence is {freshness}."),
            "Stale",
            &freshness,
            observed_at,
            report_path,
            2,
        )),
        Some(score) if score < MATURITY_TARGET => seeds.push(maturity_seed(
            "maturity:score",
            "Raise the repository maturity score",
            &format!("The current maturity score is {score:.3}/4; the Pronto target is {MATURITY_TARGET:.1}/4."),
            "Below target",
            &freshness,
            observed_at,
            report_path,
            3,
        )),
        Some(_) => {}
    }
    for (dimension, score) in dimension_scores {
        if *score < MATURITY_TARGET {
            seeds.push(ActionSeed {
                stable_key: format!("maturity:dimension:{dimension}"),
                domain: "maturity".to_string(),
                title: format!("Improve the {dimension} maturity dimension"),
                summary: format!("The dimension score is {score:.3}/4; the Pronto target is {MATURITY_TARGET:.1}/4."),
                severity: "maturity".to_string(),
                priority: "P2".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    format!("Address the evidence-backed gaps for {dimension}."),
                    format!("A fresh maturity feed reports {dimension} at or above {MATURITY_TARGET:.1}/4."),
                ],
                evidence: vec![evidence(
                    "Quality Runner maturity feed",
                    &format!("Maturity dimension · {dimension}"),
                    &format!("{score:.3}/4"),
                    &freshness,
                    observed_at,
                    report_path,
                    "Dimension-level score imported from the latest QR maturity evidence.",
                )],
                related_finding_ids: Vec::new(),
                source_run_id: audit_id.clone(),
            });
        }
    }
}

fn maturity_seed(
    stable_key: &str,
    title: &str,
    summary: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    weight: u64,
) -> ActionSeed {
    ActionSeed {
        stable_key: stable_key.to_string(),
        domain: "maturity".to_string(),
        title: title.to_string(),
        summary: summary.to_string(),
        severity: "maturity".to_string(),
        priority: if weight >= 3 { "P1" } else { "P2" }.to_string(),
        weight,
        acceptance_criteria: vec![
            "The canonical maturity feed is refreshed after the relevant work.".to_string(),
            format!("The resulting maturity evidence is fresh and at or above {MATURITY_TARGET:.1}/4 where applicable."),
        ],
        evidence: vec![evidence(
            "Quality Runner maturity feed",
            "Repository maturity",
            status,
            freshness,
            observed_at,
            report_path,
            summary,
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    }
}

fn materialize_action(
    repository: &RepositorySnapshot,
    seed: ActionSeed,
    previous: &HashMap<&str, &RemediationAction>,
    generated_at: &str,
) -> RemediationAction {
    let previous_action = previous.get(seed.stable_key.as_str()).copied();
    let preserved_status = previous_action
        .filter(|action| {
            matches!(
                action.status.as_str(),
                "in_progress" | "blocked" | "deferred"
            ) || (action.status == "verified"
                && seed
                    .evidence
                    .iter()
                    .any(|item| item.freshness.eq_ignore_ascii_case("fresh")))
        })
        .map(|action| action.status.clone())
        .unwrap_or_else(|| "open".to_string());
    let notes = previous_action.and_then(|action| action.notes.clone());
    RemediationAction {
        id: stable_id(
            &format!("{}:{}", repository.id, seed.stable_key),
            "remediation-action",
        ),
        stable_key: seed.stable_key,
        repository_id: repository.id.clone(),
        domain: seed.domain,
        title: seed.title,
        summary: seed.summary,
        severity: seed.severity,
        priority: seed.priority,
        weight: seed.weight,
        status: preserved_status,
        acceptance_criteria: seed.acceptance_criteria,
        evidence: seed.evidence,
        related_finding_ids: seed.related_finding_ids,
        source_run_id: seed.source_run_id,
        updated_at: generated_at.to_string(),
        completed_at: previous_action.and_then(|action| action.completed_at.clone()),
        notes,
    }
}

fn build_ui_coverage(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    actions: &[RemediationAction],
) -> Vec<RemediationCoverage> {
    let compass = &repository.project_compass;
    let active_conditions = repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .count();
    let unhealthy_submodules = repository
        .submodules
        .iter()
        .filter(|submodule| submodule.status != "Checked out")
        .count();
    let eligible_branches = repository
        .branches
        .iter()
        .filter(|branch| branch.integration_state == "Integration eligible")
        .count();
    let workspace_count = repository.workspaces.len().max(1);
    let coverage = vec![
        coverage_for_prefixes(
            "scope",
            "Repository scope",
            &format!(
                "Lifecycle: {} · candidate: {} · default branch: {}.",
                repository.lifecycle,
                repository.lifecycle_candidate,
                repository.default_branch.as_deref().unwrap_or("Unknown")
            ),
            &["scope:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "project_compass",
            "Project Compass",
            &format!(
                "Status: {} · blockers: {} · drift: {} · MVP progress: {}.",
                compass.status,
                compass.open_blockers,
                compass.open_drift,
                compass
                    .mvp
                    .progress_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "Unknown".to_string())
            ),
            &["product_truth:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "provider",
            "Provider and remote evidence",
            &format!(
                "Provider: {} · remote: {} · last fetch: {}.",
                repository.provider_state,
                repository.remote_url.as_deref().unwrap_or("Missing"),
                repository.last_fetch_at.as_deref().unwrap_or("Unknown")
            ),
            &["provider:remote-freshness"],
            !goal_requires_provider(goal),
            actions,
        ),
        coverage_for_prefixes(
            "pull_requests",
            "Pull request evidence",
            &format!(
                "{} pull request snapshot(s); {} open.",
                repository.pull_requests.len(),
                repository
                    .pull_requests
                    .iter()
                    .filter(|pull_request| pull_request.state.eq_ignore_ascii_case("open"))
                    .count()
            ),
            &["provider:pull-request:"],
            !goal_requires_provider(goal),
            actions,
        ),
        coverage_for_prefixes(
            "releases",
            "Published release evidence",
            &format!(
                "{} release snapshot(s); latest fetch {}.",
                repository.releases.len(),
                repository.last_fetch_at.as_deref().unwrap_or("Unknown")
            ),
            &["release_evidence:"],
            goal.target_state != "public_release",
            actions,
        ),
        coverage_for_prefixes(
            "quality_evidence",
            "Quality evidence",
            &format!(
                "Findings freshness: {} · observed: {}.",
                repository.quality.findings.freshness.as_str(),
                repository
                    .quality
                    .findings
                    .observed_at
                    .as_deref()
                    .unwrap_or("Unknown")
            ),
            &["evidence_refresh:"],
            !goal_requires_quality_evidence(goal),
            actions,
        ),
        coverage_for_prefixes(
            "ci_gates",
            "CI gates",
            &format!(
                "{} required gate(s) for the '{}' goal.",
                goal.required_gate_ids.len(),
                goal.label
            ),
            &["ci_ideal:"],
            goal.required_gate_ids.is_empty(),
            actions,
        ),
        coverage_for_prefixes(
            "quality_findings",
            "Quality findings",
            &format!(
                "{} finding(s), including {} high-severity.",
                repository.quality.findings.total,
                repository.quality.findings.high_severity_total
            ),
            &["qr_findings:"],
            !goal_requires_quality_evidence(goal),
            actions,
        ),
        coverage_for_prefixes(
            "maturity",
            "Repository maturity",
            &format!(
                "Score: {} · freshness: {}.",
                repository
                    .quality
                    .maturity
                    .score
                    .map(|score| format!("{score:.3}/4"))
                    .unwrap_or_else(|| "Unknown".to_string()),
                repository.quality.maturity.freshness.as_str()
            ),
            &["maturity:"],
            !goal_requires_maturity(goal),
            actions,
        ),
        coverage_for_prefixes(
            "workspaces",
            "Workspaces",
            &format!("{workspace_count} workspace(s) inspected for ownership, operations, dirt, sync, and remote freshness."),
            &[
                "branch_hygiene:activity:",
                "branch_hygiene:operation:",
                "branch_hygiene:dirty:",
                "branch_hygiene:sync:",
                "branch_hygiene:remote-freshness:",
            ],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "branches",
            "Branches and integration",
            &format!(
                "{} branch record(s); {} integration-eligible.",
                repository.branches.len(),
                eligible_branches
            ),
            &["branch_hygiene:integrate:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "submodules",
            "Submodules",
            &format!(
                "{} submodule(s); {} require attention.",
                repository.submodules.len(),
                unhealthy_submodules
            ),
            &["repository_health:submodule:"],
            repository.submodules.is_empty(),
            actions,
        ),
        coverage_for_prefixes(
            "conditions",
            "Repository conditions",
            &format!(
                "{} condition(s); {} active.",
                repository.conditions.len(),
                active_conditions
            ),
            &["branch_hygiene:", "repository_health:condition:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "release_preparation",
            "Release preparation",
            &format!(
                "Rule: {} · recipe: {} · confirmed version: {}.",
                if repository.release_rule.is_some() {
                    "configured"
                } else {
                    "missing"
                },
                if repository.release_recipe.is_some() {
                    "configured"
                } else {
                    "missing"
                },
                repository
                    .confirmed_release_version
                    .as_deref()
                    .unwrap_or("Unknown")
            ),
            &["scope:release-contract", "release_evidence:"],
            goal.target_state != "public_release",
            actions,
        ),
        coverage_for_prefixes(
            "agent_permission",
            "Agent permission",
            &format!(
                "Current repository permission: {}. This remains a safety boundary, not inferred remediation work.",
                repository.ai_permission
            ),
            &[],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "analytics",
            "Repository analytics",
            "Historical activity and trend analytics are represented as informational evidence; they do not create remediation work by themselves.",
            &[],
            false,
            actions,
        ),
    ];
    debug_assert_eq!(
        coverage
            .iter()
            .map(|entry| entry.surface.as_str())
            .collect::<Vec<_>>(),
        UI_TRACKED_SURFACE_IDS
    );
    coverage
}

fn coverage_for_prefixes(
    surface: &str,
    label: &str,
    detail: &str,
    prefixes: &[&str],
    not_applicable: bool,
    actions: &[RemediationAction],
) -> RemediationCoverage {
    let matching = actions
        .iter()
        .filter(|action| {
            prefixes
                .iter()
                .any(|prefix| action.stable_key.starts_with(prefix))
        })
        .collect::<Vec<_>>();
    let status = if not_applicable {
        "not_applicable"
    } else if matching.is_empty() {
        "clear"
    } else if matching.iter().any(|action| action.status == "blocked") {
        "blocked"
    } else if matching
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress"))
    {
        "attention"
    } else if matching.iter().any(|action| action.status == "deferred") {
        "deferred"
    } else {
        "verified"
    };
    RemediationCoverage {
        surface: surface.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        detail: detail.to_string(),
        action_ids: matching.iter().map(|action| action.id.clone()).collect(),
    }
}

fn evidence(
    source: &str,
    label: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    detail: &str,
) -> RemediationEvidence {
    RemediationEvidence {
        source: source.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        freshness: freshness.to_string(),
        observed_at: observed_at.map(str::to_string),
        report_path: report_path.map(str::to_string),
        detail: detail.to_string(),
    }
}

fn build_tracks(actions: &[RemediationAction]) -> Vec<RemediationTrack> {
    STAGE_ORDER
        .iter()
        .filter(|domain| **domain != "complete")
        .filter_map(|domain| {
            let matching = actions
                .iter()
                .filter(|action| action.domain == *domain)
                .collect::<Vec<_>>();
            if matching.is_empty() {
                return None;
            }
            Some(RemediationTrack {
                domain: (*domain).to_string(),
                label: domain_label(domain),
                status: track_status(&matching),
                action_ids: matching.iter().map(|action| action.id.clone()).collect(),
                verified_weight: matching
                    .iter()
                    .filter(|action| action.status == "verified")
                    .map(|action| action.weight)
                    .sum(),
                total_weight: matching
                    .iter()
                    .filter(|action| action.status != "deferred")
                    .map(|action| action.weight)
                    .sum(),
            })
        })
        .collect()
}

fn calculate_progress(actions: &[RemediationAction]) -> RemediationProgress {
    let deferred_weight = actions
        .iter()
        .filter(|action| action.status == "deferred")
        .map(|action| action.weight)
        .sum();
    let total_weight = actions
        .iter()
        .filter(|action| action.status != "deferred")
        .map(|action| action.weight)
        .sum();
    let verified_weight = actions
        .iter()
        .filter(|action| action.status == "verified")
        .map(|action| action.weight)
        .sum();
    let percentage = if total_weight == 0 {
        100.0
    } else {
        (verified_weight as f64 / total_weight as f64 * 100.0).round()
    };
    RemediationProgress {
        verified_weight,
        total_weight,
        deferred_weight,
        percentage,
    }
}

fn plan_status(actions: &[RemediationAction]) -> String {
    if actions.iter().any(|action| action.status == "blocked") {
        return "blocked".to_string();
    }
    if actions
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress"))
    {
        return if actions.iter().any(|action| action.status == "in_progress") {
            "in_progress".to_string()
        } else {
            "open".to_string()
        };
    }
    if !actions.is_empty()
        && actions
            .iter()
            .all(|action| matches!(action.status.as_str(), "verified" | "deferred"))
    {
        return if actions.iter().any(|action| action.status == "deferred") {
            "deferred".to_string()
        } else {
            "complete".to_string()
        };
    }
    if actions.is_empty() {
        "complete".to_string()
    } else {
        "open".to_string()
    }
}

fn current_stage(actions: &[RemediationAction]) -> String {
    STAGE_ORDER
        .iter()
        .find(|domain| {
            actions.iter().any(|action| {
                action.domain == **domain
                    && action.status != "verified"
                    && action.status != "deferred"
            })
        })
        .unwrap_or(&"complete")
        .to_string()
}

fn track_status(actions: &[&RemediationAction]) -> String {
    if actions.iter().any(|action| action.status == "blocked") {
        "blocked".to_string()
    } else if actions
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress"))
    {
        "in_progress".to_string()
    } else if actions.iter().all(|action| action.status == "deferred") {
        "deferred".to_string()
    } else {
        "complete".to_string()
    }
}

fn domain_label(domain: &str) -> String {
    match domain {
        "evidence_refresh" => "Evidence refresh".to_string(),
        "ci_ideal" => "CI ideal state".to_string(),
        "qr_findings" => "QR findings".to_string(),
        "branch_hygiene" => "Branch hygiene".to_string(),
        value => value
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn stable_id(value: &str, prefix: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let hex = format!("{digest:x}");
    format!("{prefix}-{}", &hex[..16])
}

fn priority_for_weight(weight: u64) -> String {
    if weight >= 4 {
        "P0".to_string()
    } else if weight >= 3 {
        "P1".to_string()
    } else if weight >= 2 {
        "P2".to_string()
    } else {
        "P3".to_string()
    }
}

fn severity_weight(severity: &str) -> u64 {
    match severity.trim().to_ascii_lowercase().as_str() {
        "blocker" | "critical" => 4,
        "error" | "high" => 3,
        "warning" | "p1" => 2,
        "observation" | "info" | "p2" => 1,
        _ => 2,
    }
}

fn freshness_for(observed_at: Option<&str>, max_age_days: u64) -> String {
    let Some(observed_at) = observed_at else {
        return "Unknown".to_string();
    };
    let Ok(timestamp) = DateTime::parse_from_rfc3339(observed_at) else {
        return "Unknown".to_string();
    };
    let age = Utc::now() - timestamp.with_timezone(&Utc);
    if age >= Duration::zero() && age <= Duration::days(max_age_days as i64) {
        "Fresh".to_string()
    } else if age < Duration::zero() {
        "Unknown".to_string()
    } else {
        "Stale".to_string()
    }
}

fn latest_qr_run(repository_path: &Path, fleet_audit_root: Option<&Path>) -> Option<QrRunEvidence> {
    let local = latest_local_qr_run(repository_path);
    let fleet = fleet_qr_run(repository_path, fleet_audit_root);
    match (local, fleet) {
        (Some(local), Some(fleet)) => {
            if fleet.observed_at > local.observed_at
                || (fleet.observed_at == local.observed_at
                    && fleet.findings.len() >= local.findings.len())
            {
                Some(fleet)
            } else {
                Some(local)
            }
        }
        (Some(local), None) => Some(local),
        (None, Some(fleet)) => Some(fleet),
        (None, None) => None,
    }
}

fn latest_local_qr_run(repository_path: &Path) -> Option<QrRunEvidence> {
    let runs = repository_path.join(".quality-runner").join("runs");
    let entries = fs::read_dir(runs).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let run_dir = entry.path();
            let manifest = read_json(&run_dir.join("run-manifest.json"))?;
            let observed_at = first_string(
                &manifest,
                &[
                    &["created_at"],
                    &["started_at"],
                    &["completed_at"],
                    &["finished_at"],
                    &["generated_at"],
                    &["as_of"],
                ],
            );
            let id = first_string(&manifest, &[&["run_id"], &["id"]])
                .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
            Some(QrRunEvidence {
                id,
                run_dir: run_dir.clone(),
                observed_at,
                findings: parse_findings(&run_dir),
                maturity_score: None,
                dimension_scores: BTreeMap::new(),
                maturity_report_path: None,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.observed_at
            .cmp(&right.observed_at)
            .then_with(|| left.run_dir.cmp(&right.run_dir))
    });
    candidates.pop()
}

fn fleet_qr_run(repository_path: &Path, fleet_audit_root: Option<&Path>) -> Option<QrRunEvidence> {
    let root = fleet_audit_root?;
    let findings_dir = root.join("findings");
    let entries = fs::read_dir(&findings_dir).ok()?;
    let summary = read_json(&root.join("summary.json"));
    let summary_id = summary
        .as_ref()
        .and_then(|value| first_string(value, &[&["audit_id"]]));
    let summary_observed_at = summary
        .as_ref()
        .and_then(|value| first_string(value, &[&["as_of"]]));
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .filter_map(|entry| {
            let path = entry.path();
            let payload = read_json(&path)?;
            let repository_payload = payload.get("repository")?;
            let candidate_path = first_string(repository_payload, &[&["primary_path"], &["path"]])
                .or_else(|| {
                    repository_payload
                        .get("checkouts")
                        .and_then(Value::as_array)
                        .and_then(|checkouts| checkouts.first())
                        .and_then(|checkout| first_string(checkout, &[&["path"]]))
                });
            if !path_matches(candidate_path.as_deref(), repository_path) {
                return None;
            }
            let findings = parse_fleet_findings(&path, &payload);
            let (dimension_scores, derived_score) = fleet_dimension_scores(&payload);
            let observed_at = first_string(&payload, &[&["as_of"]]).or(summary_observed_at.clone());
            let id = first_string(&payload, &[&["audit_id"], &["id"]])
                .or(summary_id.clone())
                .unwrap_or_else(|| path.display().to_string());
            Some(QrRunEvidence {
                id,
                run_dir: root.to_path_buf(),
                observed_at,
                findings,
                maturity_score: first_f64(&payload, &[&["mean_maturity"], &["maturity_score"]])
                    .or(derived_score),
                dimension_scores,
                maturity_report_path: Some(path.to_string_lossy().to_string()),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.observed_at
            .cmp(&right.observed_at)
            .then_with(|| left.run_dir.cmp(&right.run_dir))
    });
    candidates.pop()
}

fn path_matches(candidate: Option<&str>, repository_path: &Path) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    canonical_path(Path::new(candidate)) == canonical_path(repository_path)
}

fn canonical_path(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn parse_fleet_findings(path: &Path, payload: &Value) -> Vec<ParsedFinding> {
    let Some(items) = payload.get("findings").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            !item
                .get("applicable")
                .and_then(Value::as_bool)
                .is_some_and(|applicable| !applicable)
        })
        .map(|(index, item)| {
            let category =
                first_string(item, &[&["dimension"], &["category"], &["kind"], &["type"]])
                    .unwrap_or_else(|| "quality".to_string());
            let bucket = first_string(item, &[&["dimension"], &["status"], &["bucket"]])
                .unwrap_or_else(|| category.clone());
            let id = first_string(
                item,
                &[&["finding_id"], &["id"], &["fingerprint"], &["rule_id"]],
            )
            .unwrap_or_else(|| format!("fleet:{index}"));
            let pack = first_string(item, &[&["schema"], &["pack"], &["pack_id"]]);
            let severity = first_string(item, &[&["severity"], &["priority"], &["risk"]])
                .unwrap_or_else(|| "warning".to_string());
            let title = first_string(item, &[&["label"], &["title"], &["rule_id"]])
                .unwrap_or_else(|| format!("Resolve {category} finding"));
            let summary = first_string(
                item,
                &[
                    &["message"],
                    &["summary"],
                    &["description"],
                    &["recommended_fix"],
                ],
            )
            .unwrap_or_else(|| "Review the evidence and apply the recommended fix.".to_string());
            let file = first_string(item, &[&["file"], &["path"], &["file_path"]])
                .or_else(|| first_evidence_path(item));
            let verification = first_string(
                item,
                &[&["verification"], &["verification_command"], &["verify"]],
            )
            .or_else(|| first_array_string(item, "validation_commands"));
            ParsedFinding {
                id,
                group_key: format!(
                    "{}|{}|{}",
                    category.to_ascii_lowercase(),
                    bucket.to_ascii_lowercase(),
                    pack.as_deref().unwrap_or("unknown").to_ascii_lowercase()
                ),
                category,
                pack,
                severity,
                title,
                summary,
                file,
                line: first_u64(item, &[&["line"], &["line_number"]]),
                verification,
                report_path: path.to_string_lossy().to_string(),
            }
        })
        .collect()
}

fn first_evidence_path(value: &Value) -> Option<String> {
    value
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|item| first_string(item, &[&["path"], &["file"], &["file_path"]]))
}

fn first_array_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(Value::as_str))
        .map(str::to_string)
}

fn fleet_dimension_scores(payload: &Value) -> (BTreeMap<String, f64>, Option<f64>) {
    if let Some(scores) = payload.get("dimension_scores").and_then(Value::as_object) {
        let scores = scores
            .iter()
            .filter_map(|(dimension, score)| score.as_f64().map(|value| (dimension.clone(), value)))
            .collect::<BTreeMap<_, _>>();
        let mean = (!scores.is_empty()).then(|| scores.values().sum::<f64>() / scores.len() as f64);
        return (scores, mean);
    }
    let scores = payload
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|finding| {
            !finding
                .get("applicable")
                .and_then(Value::as_bool)
                .is_some_and(|applicable| !applicable)
        })
        .filter_map(|finding| {
            let dimension = finding.get("dimension").and_then(Value::as_str)?;
            let score = finding.get("score").and_then(Value::as_f64)?;
            Some((dimension.to_string(), score))
        })
        .collect::<BTreeMap<_, _>>();
    let mean = (!scores.is_empty()).then(|| scores.values().sum::<f64>() / scores.len() as f64);
    (scores, mean)
}

fn parse_findings(run_dir: &Path) -> Vec<ParsedFinding> {
    let mut findings = Vec::new();
    for file_name in ["code-quality-scan.json", "quality-audit.json"] {
        let path = run_dir.join(file_name);
        let Some(payload) = read_json(&path) else {
            continue;
        };
        let Some(items) = payload.get("findings").and_then(Value::as_array) else {
            continue;
        };
        for (index, item) in items.iter().enumerate() {
            let id = first_string(
                item,
                &[&["id"], &["fingerprint"], &["finding_id"], &["rule_id"]],
            )
            .unwrap_or_else(|| format!("{file_name}:{index}"));
            let category = first_string(item, &[&["category"], &["kind"], &["type"]])
                .unwrap_or_else(|| "quality".to_string());
            let bucket = first_string(item, &[&["remediation_bucket"], &["bucket"], &["rule_id"]])
                .unwrap_or_else(|| category.clone());
            let pack = first_string(item, &[&["pack"], &["pack_id"], &["pack_name"]]);
            let severity = first_string(item, &[&["severity"], &["priority"], &["risk"]])
                .unwrap_or_else(|| "warning".to_string());
            let title = first_string(item, &[&["title"], &["summary"], &["rule"], &["rule_id"]])
                .unwrap_or_else(|| format!("Resolve {category} finding"));
            let summary = first_string(
                item,
                &[
                    &["summary"],
                    &["message"],
                    &["description"],
                    &["recommended_fix"],
                ],
            )
            .unwrap_or_else(|| "Review the evidence and apply the recommended fix.".to_string());
            let file = first_string(item, &[&["file"], &["path"], &["file_path"]]);
            let line = first_u64(item, &[&["line"], &["line_number"]]);
            let verification = first_string(
                item,
                &[&["verification"], &["verification_command"], &["verify"]],
            );
            findings.push(ParsedFinding {
                id,
                group_key: format!(
                    "{}|{}|{}",
                    category.to_ascii_lowercase(),
                    bucket.to_ascii_lowercase(),
                    pack.as_deref().unwrap_or("unknown").to_ascii_lowercase()
                ),
                category,
                pack,
                severity,
                title,
                summary,
                file,
                line,
                verification,
                report_path: path.to_string_lossy().to_string(),
            });
        }
        if !findings.is_empty() {
            break;
        }
    }
    findings
}

fn read_json(path: &Path) -> Option<Value> {
    if !path.is_file() || path.is_symlink() {
        return None;
    }
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn first_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for segment in *path {
            current = current.get(*segment)?;
        }
        current.as_str().map(str::to_string)
    })
}

fn first_u64(value: &Value, paths: &[&[&str]]) -> Option<u64> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for segment in *path {
            current = current.get(*segment)?;
        }
        current.as_u64()
    })
}

fn first_f64(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for segment in *path {
            current = current.get(*segment)?;
        }
        current.as_f64()
    })
}

fn markdown_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

fn first_active_action(plan: &RemediationPlan) -> Option<&RemediationAction> {
    plan.actions
        .iter()
        .filter(|action| !matches!(action.status.as_str(), "verified" | "deferred"))
        .min_by_key(|action| {
            (
                queue_domain_rank(&action.domain),
                queue_priority_rank(&action.priority),
                std::cmp::Reverse(action.weight),
                action.title.to_ascii_lowercase(),
            )
        })
}

pub fn render_active_queue_markdown(run: &RemediationRun) -> String {
    let mut output = format!(
        "# Repository remediation order\n\n\
Generated from `{}` at `{}`.\n\n\
This is the active remediation queue, ranked from current Pronto evidence and \
each repository's intended remediation outcome. Inferred goals remain active \
until a repository-owned goal contract confirms them. \
Each plan also classifies every repo-level surface tracked by the UI; unresolved \
coverage entries link to concrete remediation actions. \
Repositories leave the active table only after their plan reaches a terminal \
evidence-backed disposition. Git, provider, publication, and pruning actions \
still require their own authorization.\n\n\
Ranking preserves plan status, the earliest unresolved remediation domain, \
and action priority before fleet leverage. Pronto, AIOS, and Quality Runner \
receive explicit control-plane or evidence-provider precedence before the \
intended repository goal and raw action weight are used as tie-breakers.\n\n\
## Active queue\n\n\
Active repositories: **{}**. Retained closures: **{}**.\n\n\
<!-- prettier-ignore -->\n\
| Rank | Repository | Goal | Goal source | Status | Current stage | Leverage | Tracked gaps | Active actions | First safe action |\n\
| ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | --- |\n",
        run.schema_version,
        run.generated_at,
        run.plans.len(),
        run.closures.len()
    );
    if run.plans.is_empty() {
        output.push_str("| — | _No active remediation remains_ | — | — | complete | complete | — | 0 | 0 | Refresh scoped evidence before treating this as current. |\n");
    } else {
        for (index, plan) in run.plans.iter().enumerate() {
            let active_action_count = plan
                .actions
                .iter()
                .filter(|action| !matches!(action.status.as_str(), "verified" | "deferred"))
                .count();
            let tracked_gap_count = plan
                .coverage
                .iter()
                .filter(|entry| matches!(entry.status.as_str(), "attention" | "blocked"))
                .count();
            let first_action = first_active_action(plan)
                .map(|action| action.title.as_str())
                .unwrap_or("Refresh scoped evidence and recheck the plan.");
            let leverage = queue_leverage(&plan.repository_name).1;
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                index + 1,
                markdown_cell(&plan.repository_name),
                markdown_cell(&plan.goal.label),
                markdown_cell(&plan.goal.source),
                markdown_cell(&plan.status),
                markdown_cell(&plan.current_stage),
                markdown_cell(leverage),
                tracked_gap_count,
                active_action_count,
                markdown_cell(first_action),
            ));
        }
    }
    output.push_str("\n## Closure ledger\n\n");
    if run.closures.is_empty() {
        output
            .push_str("No repositories have left the active queue in this retained run history.\n");
    } else {
        output.push_str(
            "<!-- prettier-ignore -->\n\
| Repository | Goal | Goal source | Disposition | Closed at | Resolved actions | Evidence observed at | Summary |\n\
| --- | --- | --- | --- | --- | ---: | --- | --- |\n",
        );
        for closure in &run.closures {
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | `{}` | {} | {} | {} |\n",
                markdown_cell(&closure.repository_name),
                markdown_cell(&closure.target_state),
                markdown_cell(&closure.goal_source),
                markdown_cell(&closure.disposition),
                markdown_cell(&closure.closed_at),
                closure.resolved_action_count,
                closure
                    .last_evidence_at
                    .as_deref()
                    .map(markdown_cell)
                    .unwrap_or_else(|| "Not recorded".to_string()),
                markdown_cell(&closure.summary),
            ));
        }
    }
    output.push_str(
        "\nA later refresh may return a repository to the active queue when new or \
regressed evidence creates actionable work.\n",
    );
    output
}

pub fn export_run(run: &RemediationRun, output_dir: &Path) -> Result<RemediationExport, String> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("Could not create remediation export directory: {error}"))?;
    let mut files = Vec::new();
    let manifest_path = output_dir.join("remediation-run.json");
    write_json(&manifest_path, run)?;
    files.push(manifest_path.to_string_lossy().to_string());
    let markdown_path = output_dir.join("repository-remediation-order.md");
    fs::write(&markdown_path, render_active_queue_markdown(run)).map_err(|error| {
        format!(
            "Could not write remediation queue {}: {error}",
            markdown_path.display()
        )
    })?;
    files.push(markdown_path.to_string_lossy().to_string());
    for plan in &run.plans {
        let file_name = format!("repo-{}.json", safe_file_component(&plan.repository_id));
        let plan_path = output_dir.join(file_name);
        write_json(&plan_path, plan)?;
        files.push(plan_path.to_string_lossy().to_string());
    }
    if !run.closures.is_empty() {
        let closures_path = output_dir.join("remediation-closures.json");
        write_json(&closures_path, &run.closures)?;
        files.push(closures_path.to_string_lossy().to_string());
    }
    Ok(RemediationExport {
        run_id: run.id.clone(),
        output_path: output_dir.to_string_lossy().to_string(),
        files,
    })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Could not encode remediation export: {error}"))?;
    fs::write(path, contents).map_err(|error| {
        format!(
            "Could not write remediation export {}: {error}",
            path.display()
        )
    })
}

fn safe_file_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        ActivitySignal, BranchSummary, Condition, SubmoduleSummary, WorkspaceActivity,
        WorkspaceSummary,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn fixture_repository(name: &str) -> RepositorySnapshot {
        RepositorySnapshot {
            id: format!("repo-{name}"),
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            locality: "Local".to_string(),
            lifecycle: "Active".to_string(),
            lifecycle_candidate: "Active".to_string(),
            remote_url: Some(format!("https://github.com/example/{name}")),
            provider_state: "GitHub connected".to_string(),
            branch: "dev".to_string(),
            default_branch: Some("dev".to_string()),
            workspace: WorkspaceSummary {
                id: format!("workspace-{name}"),
                path: format!("/tmp/{name}"),
                is_primary: true,
                branch: "dev".to_string(),
                dirty: false,
                added: 0,
                removed: 0,
                line_totals_partial: false,
                sync_state: "Synced".to_string(),
                remote_freshness: "Fresh".to_string(),
                ahead: 0,
                behind: 0,
                upstream: Some("origin/dev".to_string()),
                operation: None,
                last_commit: Some("abc".to_string()),
                last_commit_at: Some(Utc::now().to_rfc3339()),
                last_activity_at: None,
                integration_state: "Ready".to_string(),
                target_branch: Some("dev".to_string()),
                target_confidence: "High".to_string(),
                role: "Primary".to_string(),
                role_confidence: "High".to_string(),
                activity: WorkspaceActivity::default(),
            },
            workspaces: Vec::new(),
            branches: Vec::<BranchSummary>::new(),
            submodules: Vec::new(),
            pull_requests: Vec::new(),
            releases: Vec::new(),
            quality: Default::default(),
            project_compass: Default::default(),
            release_rule: None,
            release_recipe: None,
            confirmed_release_version: None,
            ai_permission: "Blocked".to_string(),
            conditions: Vec::new(),
            last_scan_at: Utc::now().to_rfc3339(),
            last_fetch_at: Some(Utc::now().to_rfc3339()),
            last_activity_at: None,
        }
    }

    fn fixture_action(
        id: &str,
        domain: &str,
        priority: &str,
        weight: u64,
        status: &str,
    ) -> RemediationAction {
        RemediationAction {
            id: id.to_string(),
            stable_key: id.to_string(),
            repository_id: "repo".to_string(),
            domain: domain.to_string(),
            title: format!("Action {id}"),
            summary: String::new(),
            severity: "test".to_string(),
            priority: priority.to_string(),
            weight,
            status: status.to_string(),
            acceptance_criteria: Vec::new(),
            evidence: vec![evidence(
                "Pronto",
                "Fixture evidence",
                "Passed",
                "Fresh",
                Some("2026-07-29T12:00:00Z"),
                None,
                "Fixture evidence is current.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some("refresh-1".to_string()),
            updated_at: "2026-07-29T12:00:00Z".to_string(),
            completed_at: (status == "verified").then(|| "2026-07-29T12:00:00Z".to_string()),
            notes: None,
        }
    }

    fn fixture_plan(
        repository_name: &str,
        status: &str,
        actions: Vec<RemediationAction>,
    ) -> RemediationPlan {
        let repository_id = format!("repo-{repository_name}");
        let mut goal =
            goal_definition("active_maintained").expect("fixture goal should be supported");
        goal.source = "repository_contract".to_string();
        goal.confidence = "High".to_string();
        goal.reason = "Fixture repository is actively maintained.".to_string();
        RemediationPlan {
            schema_version: REMEDIATION_SCHEMA.to_string(),
            id: format!("plan-{repository_name}"),
            repository_id,
            repository_name: repository_name.to_string(),
            repository_path: format!("/tmp/{repository_name}"),
            generated_at: "2026-07-29T12:00:00Z".to_string(),
            source_refresh_id: Some("refresh-1".to_string()),
            goal,
            current_stage: actions
                .first()
                .map(|action| action.domain.clone())
                .unwrap_or_else(|| "complete".to_string()),
            status: status.to_string(),
            progress: calculate_progress(&actions),
            coverage: Vec::new(),
            tracks: build_tracks(&actions),
            actions,
        }
    }

    #[test]
    fn excludes_in_progress_repositories_by_name() {
        assert!(is_excluded_repository(&fixture_repository("soundscape")));
        assert!(is_excluded_repository(&fixture_repository("tenure")));
        assert!(!is_excluded_repository(&fixture_repository("pronto")));
    }

    #[test]
    fn terminal_plans_leave_the_active_queue_and_retain_a_closure() {
        let mut run = empty_run();
        run.plans = vec![fixture_plan(
            "pronto",
            "complete",
            vec![fixture_action(
                "verified",
                "verification",
                "P2",
                1,
                "verified",
            )],
        )];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert!(run.plans.is_empty());
        assert_eq!(run.closures.len(), 1);
        assert_eq!(run.closures[0].repository_name, "pronto");
        assert_eq!(run.closures[0].disposition, "verified");
        assert_eq!(run.closures[0].resolved_action_count, 1);
        assert_eq!(
            run.closures[0].last_evidence_at.as_deref(),
            Some("2026-07-29T12:00:00Z")
        );
    }

    #[test]
    fn active_queue_ranks_preservation_before_later_quality_work() {
        let mut run = empty_run();
        run.plans = vec![
            fixture_plan(
                "quality",
                "open",
                vec![fixture_action("quality", "qr_findings", "P1", 8, "open")],
            ),
            fixture_plan(
                "preserve",
                "open",
                vec![fixture_action(
                    "preserve",
                    "branch_hygiene",
                    "P2",
                    2,
                    "open",
                )],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "preserve");
        assert_eq!(run.plans[1].repository_name, "quality");
    }

    #[test]
    fn active_queue_uses_fleet_leverage_before_raw_action_weight() {
        let mut run = empty_run();
        run.plans = vec![
            fixture_plan(
                "ordinary-heavy-repo",
                "open",
                vec![fixture_action(
                    "many-findings",
                    "scope",
                    "P1",
                    10_000,
                    "open",
                )],
            ),
            fixture_plan(
                "quality-runner",
                "open",
                vec![fixture_action(
                    "evidence-provider",
                    "scope",
                    "P1",
                    3,
                    "open",
                )],
            ),
            fixture_plan(
                "AIOS",
                "open",
                vec![fixture_action("coordination", "scope", "P1", 2, "open")],
            ),
            fixture_plan(
                "pronto",
                "open",
                vec![fixture_action("control-plane", "scope", "P1", 1, "open")],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(
            run.plans
                .iter()
                .map(|plan| plan.repository_name.as_str())
                .collect::<Vec<_>>(),
            vec!["pronto", "AIOS", "quality-runner", "ordinary-heavy-repo"]
        );
    }

    #[test]
    fn active_queue_keeps_blocked_work_ahead_of_fleet_leverage() {
        let mut run = empty_run();
        run.plans = vec![
            fixture_plan(
                "pronto",
                "open",
                vec![fixture_action("control-plane", "scope", "P1", 100, "open")],
            ),
            fixture_plan(
                "ordinary-blocker",
                "blocked",
                vec![fixture_action("blocker", "scope", "P1", 1, "open")],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "ordinary-blocker");
        assert_eq!(run.plans[1].repository_name, "pronto");
    }

    #[test]
    fn active_queue_uses_fleet_leverage_before_repository_goal() {
        let mut release = fixture_plan(
            "ordinary-public-release",
            "open",
            vec![fixture_action("release", "scope", "P1", 100, "open")],
        );
        release.goal =
            goal_definition("public_release").expect("public release goal should be supported");
        let mut run = empty_run();
        run.plans = vec![
            release,
            fixture_plan(
                "pronto",
                "open",
                vec![fixture_action("control-plane", "scope", "P1", 1, "open")],
            ),
        ];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "pronto");
        assert_eq!(run.plans[1].repository_name, "ordinary-public-release");
    }

    #[test]
    fn active_queue_uses_goal_priority_within_the_same_safety_stage() {
        let mut release = fixture_plan(
            "release",
            "open",
            vec![fixture_action(
                "release-provider",
                "provider",
                "P1",
                2,
                "open",
            )],
        );
        release.goal = goal_definition("public_release").expect("release goal should be supported");
        let mut clean = fixture_plan(
            "clean",
            "open",
            vec![fixture_action(
                "clean-provider",
                "provider",
                "P1",
                2,
                "open",
            )],
        );
        clean.goal = goal_definition("clean_only").expect("clean goal should be supported");
        let mut run = empty_run();
        run.plans = vec![clean, release];

        normalize_queue(&mut run, "2026-07-29T13:00:00Z");

        assert_eq!(run.plans[0].repository_name, "release");
        assert_eq!(run.plans[1].repository_name, "clean");
    }

    #[test]
    fn repository_goal_contract_controls_required_gates_and_freshness() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-goal-contract-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("goal fixture should be writable");
        fs::write(
            contract_dir.join("remediation-goal.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": REMEDIATION_GOAL_SCHEMA,
                "target_state": "clean_only",
                "reason": "This utility only needs a clean, preserved canonical branch.",
                "additional_required_gate_ids": ["tests"],
                "optional_gate_ids": ["secrets_scan"],
                "evidence_max_age_days": 21
            }))
            .expect("goal fixture should encode"),
        )
        .expect("goal contract should be writable");
        let mut repository = fixture_repository("goal-contract");
        repository.path = root.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();

        let goal = resolve_goal_profile(&repository);

        assert_eq!(goal.target_state, "clean_only");
        assert_eq!(goal.source, "repository_contract");
        assert_eq!(goal.evidence_max_age_days, 21);
        assert_eq!(goal.required_gate_ids, vec!["tests"]);
        assert!(goal.optional_gate_ids.contains(&"secrets_scan".to_string()));
        fs::remove_dir_all(root).expect("goal fixture should be removable");
    }

    #[test]
    fn clean_only_goal_does_not_inherit_universal_quality_or_maturity_work() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-clean-goal-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("goal fixture should be writable");
        fs::write(
            contract_dir.join("remediation-goal.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": REMEDIATION_GOAL_SCHEMA,
                "target_state": "clean_only",
                "reason": "This repository only needs clean preservation."
            }))
            .expect("goal fixture should encode"),
        )
        .expect("goal contract should be writable");
        let mut repository = fixture_repository("clean-only");
        repository.path = root.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-clean"),
            "2026-07-29T12:00:00Z",
            None,
        );

        assert_eq!(plan.goal.target_state, "clean_only");
        assert!(plan.actions.iter().all(|action| !matches!(
            action.domain.as_str(),
            "evidence_refresh" | "ci_ideal" | "maturity"
        )));
        fs::remove_dir_all(root).expect("goal fixture should be removable");
    }

    #[test]
    fn missing_goal_contract_stays_visible_as_confirmation_work() {
        let repository = fixture_repository("inferred-goal");

        let run = rebuild_run(&[repository], &empty_run(), Some("refresh-goal"));
        let plan = &run.plans[0];

        assert_eq!(plan.goal.source, "inferred");
        assert!(plan
            .actions
            .iter()
            .any(|action| action.stable_key == "scope:confirm-remediation-goal"));
    }

    #[test]
    fn plan_covers_every_repo_level_ui_surface_with_linked_gap_actions() {
        let mut repository = fixture_repository("ui-coverage");
        repository.workspace.dirty = true;
        repository.workspace.sync_state = "Ahead by 1".to_string();
        repository.workspace.remote_freshness = "Not fetched by Pronto".to_string();
        repository.workspace.operation = Some("rebase".to_string());
        repository.workspace.integration_state = "Integration eligible".to_string();
        repository.workspace.activity.state = "Active".to_string();
        repository.branches.push(BranchSummary {
            name: "feature/coverage".to_string(),
            role: "Feature".to_string(),
            role_confidence: "High".to_string(),
            target_branch: Some("dev".to_string()),
            target_confidence: "High".to_string(),
            ahead: 1,
            behind: 0,
            integration_state: "Integration eligible".to_string(),
            workspace_id: None,
            last_commit: Some("def".to_string()),
            last_commit_at: Some(Utc::now().to_rfc3339()),
        });
        repository.submodules.push(SubmoduleSummary {
            path: "vendor/example".to_string(),
            commit: Some("123".to_string()),
            status: "Modified commit".to_string(),
        });
        repository.conditions.push(Condition {
            id: "condition-custom".to_string(),
            kind: "new-ui-condition".to_string(),
            title: "New UI condition".to_string(),
            summary: "A newly tracked condition needs a remediation disposition.".to_string(),
            priority: 1,
            status: "Active".to_string(),
            fingerprint: "condition-custom".to_string(),
            rule: "fixture".to_string(),
            evidence: Vec::new(),
            missing: Vec::new(),
            confidence: Some("High".to_string()),
            freshness: Some("Fresh".to_string()),
        });

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-coverage"),
            "2026-07-29T12:00:00Z",
            None,
        );

        assert_eq!(
            plan.coverage
                .iter()
                .map(|entry| entry.surface.as_str())
                .collect::<Vec<_>>(),
            UI_TRACKED_SURFACE_IDS
        );
        assert!(plan
            .coverage
            .iter()
            .filter(|entry| matches!(entry.status.as_str(), "attention" | "blocked"))
            .all(|entry| !entry.action_ids.is_empty()));
        for stable_key in [
            "product_truth:project-compass",
            "branch_hygiene:activity:workspace-ui-coverage",
            "branch_hygiene:operation:workspace-ui-coverage",
            "branch_hygiene:remote-freshness:workspace-ui-coverage",
            "branch_hygiene:integrate:feature/coverage",
            "repository_health:submodule:vendor/example",
            "repository_health:condition:condition-custom",
        ] {
            assert!(
                plan.actions
                    .iter()
                    .any(|action| action.stable_key == stable_key),
                "missing action {stable_key}"
            );
        }
    }

    #[test]
    fn dirty_workspace_without_an_owner_does_not_create_an_ownership_blocker() {
        let mut repository = fixture_repository("dirty-without-owner");
        repository.workspace.dirty = true;
        repository.workspace.activity = WorkspaceActivity {
            state: "Interrupted with dirty work".to_string(),
            confidence: "Medium".to_string(),
            signals: vec![ActivitySignal {
                source: "Process".to_string(),
                summary: "No associated process detected".to_string(),
                confidence: "Medium".to_string(),
                observed_at: Utc::now().to_rfc3339(),
                process_name: None,
                process_id: None,
                started_at: None,
                working_directory: None,
            }],
            manifest: None,
        };

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-dirty"),
            "2026-07-29T18:00:00Z",
            None,
        );

        assert!(plan.actions.iter().any(|action| {
            action.stable_key == "branch_hygiene:dirty:workspace-dirty-without-owner"
        }));
        assert!(!plan.actions.iter().any(|action| {
            action.stable_key == "branch_hygiene:activity:workspace-dirty-without-owner"
        }));
    }

    #[test]
    fn verified_and_deferred_actions_form_a_terminal_deferred_plan() {
        let actions = vec![
            fixture_action("verified", "qr_findings", "P1", 3, "verified"),
            fixture_action("deferred", "maturity", "P2", 2, "deferred"),
        ];

        assert_eq!(plan_status(&actions), "deferred");
        assert_eq!(current_stage(&actions), "complete");
    }

    #[test]
    fn markdown_export_separates_active_queue_from_closure_history() {
        let mut run = empty_run();
        run.generated_at = "2026-07-29T13:00:00Z".to_string();
        run.plans = vec![fixture_plan(
            "active-repo",
            "open",
            vec![fixture_action(
                "preserve",
                "branch_hygiene",
                "P1",
                2,
                "open",
            )],
        )];
        run.closures = vec![closure_from_plan(
            &fixture_plan(
                "closed-repo",
                "complete",
                vec![fixture_action(
                    "verified",
                    "verification",
                    "P2",
                    1,
                    "verified",
                )],
            ),
            "2026-07-29T13:00:00Z",
            Some("refresh-1"),
        )];

        let markdown = render_active_queue_markdown(&run);

        assert!(markdown.contains(
            "| 1 | `active-repo` | Active maintained repository | repository_contract | open |"
        ));
        assert!(markdown
            .contains("| `closed-repo` | active_maintained | repository_contract | verified |"));
        assert!(!markdown.contains("| 2 | `closed-repo` |"));
    }

    #[test]
    fn remediation_export_writes_markdown_and_retained_closure_data() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let output_dir =
            std::env::temp_dir().join(format!("pronto-remediation-export-{fixture_id}"));
        let mut run = empty_run();
        run.id = "run-export".to_string();
        run.generated_at = "2026-07-29T13:00:00Z".to_string();
        run.plans = vec![fixture_plan(
            "active-repo",
            "open",
            vec![fixture_action(
                "preserve",
                "branch_hygiene",
                "P1",
                2,
                "open",
            )],
        )];
        run.closures = vec![closure_from_plan(
            &fixture_plan(
                "closed-repo",
                "complete",
                vec![fixture_action(
                    "verified",
                    "verification",
                    "P2",
                    1,
                    "verified",
                )],
            ),
            "2026-07-29T13:00:00Z",
            Some("refresh-1"),
        )];

        let exported = export_run(&run, &output_dir).expect("queue export should succeed");

        assert!(exported
            .files
            .iter()
            .any(|path| path.ends_with("repository-remediation-order.md")));
        assert!(exported
            .files
            .iter()
            .any(|path| path.ends_with("remediation-closures.json")));
        let markdown = fs::read_to_string(output_dir.join("repository-remediation-order.md"))
            .expect("markdown queue should be readable");
        assert!(markdown.contains("| 1 | `active-repo` | Active maintained repository |"));
        assert!(markdown
            .contains("| `closed-repo` | active_maintained | repository_contract | verified |"));

        fs::remove_dir_all(output_dir).expect("export fixture should be removable");
    }

    #[test]
    fn progress_excludes_deferred_weight() {
        let actions = vec![
            RemediationAction {
                id: "a".to_string(),
                stable_key: "a".to_string(),
                repository_id: "repo".to_string(),
                domain: "qr_findings".to_string(),
                title: "A".to_string(),
                summary: String::new(),
                severity: "high".to_string(),
                priority: "P1".to_string(),
                weight: 3,
                status: "verified".to_string(),
                acceptance_criteria: Vec::new(),
                evidence: Vec::new(),
                related_finding_ids: Vec::new(),
                source_run_id: None,
                updated_at: String::new(),
                completed_at: None,
                notes: None,
            },
            RemediationAction {
                id: "b".to_string(),
                stable_key: "b".to_string(),
                repository_id: "repo".to_string(),
                domain: "maturity".to_string(),
                title: "B".to_string(),
                summary: String::new(),
                severity: "maturity".to_string(),
                priority: "P2".to_string(),
                weight: 4,
                status: "deferred".to_string(),
                acceptance_criteria: Vec::new(),
                evidence: Vec::new(),
                related_finding_ids: Vec::new(),
                source_run_id: None,
                updated_at: String::new(),
                completed_at: None,
                notes: None,
            },
        ];
        let progress = calculate_progress(&actions);
        assert_eq!(progress.total_weight, 3);
        assert_eq!(progress.deferred_weight, 4);
        assert_eq!(progress.percentage, 100.0);
    }

    #[test]
    fn severity_weights_are_deterministic() {
        assert_eq!(severity_weight("critical"), 4);
        assert_eq!(severity_weight("error"), 3);
        assert_eq!(severity_weight("warning"), 2);
        assert_eq!(severity_weight("observation"), 1);
    }

    #[test]
    fn fleet_qr_artifact_supplies_findings_and_maturity_to_plan() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-remediation-fleet-{fixture_id}"));
        let repository_path = root.join("repo");
        let findings_dir = root.join("findings");
        fs::create_dir_all(&repository_path).expect("repository fixture should be writable");
        fs::create_dir_all(&findings_dir).expect("findings fixture should be writable");
        let observed_at = Utc::now().to_rfc3339();
        let mut repository = fixture_repository("fleet-repo");
        repository.path = repository_path.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();
        repository.workspace.branch = "dev".to_string();
        repository.branch = "dev".to_string();
        repository.workspace.last_commit = Some("abc".to_string());
        let finding_path = findings_dir.join("repo.json");
        fs::write(
            &finding_path,
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-fleet",
                "as_of": observed_at,
                "repository": {
                    "primary_path": repository.path.clone(),
                    "checkouts": [{"path": repository.workspace.path.clone(), "head": "abc", "branch": "dev"}]
                },
                "findings": [
                    {
                        "applicable": true,
                        "dimension": "quality_commands",
                        "finding_id": "finding-quality-commands",
                        "label": "quality commands",
                        "message": "Quality commands are not fully legible.",
                        "priority": "P1",
                        "score": 2,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "validation_commands": ["pnpm test"]
                    },
                    {
                        "applicable": false,
                        "dimension": "deployment_rollback",
                        "finding_id": "finding-deployment",
                        "score": null,
                        "severity": "observation"
                    }
                ]
            }))
            .expect("fleet finding should encode"),
        )
        .expect("fleet finding should be writable");

        let run = rebuild_run_with_fleet_root(
            &[repository],
            &empty_run(),
            Some("refresh-fleet"),
            Some(&root),
        );
        let plan = &run.plans[0];
        assert!(plan
            .actions
            .iter()
            .any(|action| action.stable_key.starts_with("qr_findings:group:")));
        assert!(plan.actions.iter().any(
            |action| action.stable_key == "maturity:score" && action.summary.contains("2.000")
        ));
        assert!(plan
            .actions
            .iter()
            .any(|action| action.stable_key == "maturity:dimension:quality_commands"));
        assert!(!plan
            .actions
            .iter()
            .any(|action| action.stable_key == "evidence_refresh:qr-run"));
        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.domain == "qr_findings")
                .count(),
            1
        );
        fs::remove_dir_all(root).expect("fleet fixture should be removable");
    }
}
