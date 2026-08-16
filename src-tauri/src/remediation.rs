use crate::core::{RemoteRepositorySnapshot, RepositorySnapshot};
use crate::mac_control_maturity;
use crate::quality;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const REMEDIATION_SCHEMA: &str = "pronto-remediation/v3";
pub const REMEDIATION_GOAL_SCHEMA: &str = "pronto-remediation-goal/v1";
pub const REMEDIATION_GOAL_PATH: &str = ".pronto/remediation-goal.json";
pub const MATURITY_CLOSURE_TARGET: f64 = 3.0;
pub const MATURITY_IDEAL_SCORE: f64 = 4.0;
pub const EXCLUDED_REPOSITORY_NAMES: [&str; 0] = [];
pub const GITHUB_ONLY_LOCALITY: &str = "GitHub only";
pub const GITHUB_ONLY_REMEDIATION_TASK: &str = "GitHub only";
const VERIFICATION_ACTION_KEY: &str = "verification:recheck-after-remediation";
pub const GITHUB_ONLY_VERIFICATION_ACTION_KEY: &str = "verification:github-only";
const RESOLVED_BY_REFRESH_LABEL: &str = "Resolved by remediation refresh";
const PROJECT_COMPASS_OPEN_ITEMS_KEY: &str = "product_truth:project-compass-open-items";
const DEBLOAT_GATE_ACTION_KEY: &str = "qr_findings:debloat-maturity-gate";
const PUBLIC_RELEASE_BOUNDARY_ACTION_KEY: &str = "release_boundary:public-distribution";
const DEBLOAT_GROUP_CATEGORY_PREFIX: &str = "qr_findings:group:debloat|";
const DEBLOAT_GROUP_KEY_PREFIX: &str = "qr_findings:group:debloat|debloat candidate review|";
const LEGACY_DEBLOAT_GROUP_KEY_PREFIX: &str =
    "qr_findings:group:simplify|simplification and shrink pass|";
const FLEET_MATURITY_FINDING_PACK_PREFIX: &str = quality::FLEET_MATURITY_FINDING_SCHEMA_PREFIX;
const LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS: [&str; 2] = [
    "product_truth:project-compass-blockers",
    "product_truth:project-compass-drift",
];
const DEFAULT_REMEDIATION_PHASE_IDS: [&str; 5] = [
    "preserve_and_reconcile",
    "product_and_provider_truth",
    "quality_and_maturity",
    "public_distribution_boundary",
    "verify_and_close",
];
const UNCLASSIFIED_REMEDIATION_PHASE_ID: &str = "unclassified_remediation";
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
const ALL_MATURITY_GATE_IDS: [&str; 1] = [mac_control_maturity::MAC_CONTROL_GATE_ID];

const STAGE_ORDER: [&str; 12] = [
    "scope",
    "product_truth",
    "repository_health",
    "branch_hygiene",
    "provider",
    "evidence_refresh",
    "ci_ideal",
    "qr_findings",
    "maturity",
    "release_boundary",
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
    #[serde(default)]
    pub scanned_commit: Option<String>,
    #[serde(default)]
    pub scanned_branch: Option<String>,
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
pub struct RemediationMaturityPolicy {
    pub minimum_closure_score: f64,
    pub ideal_score: f64,
    pub scoring_owner: String,
    pub improvement_rule: String,
    pub integrity_rule: String,
    #[serde(default)]
    pub ideal_gate_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationPhaseDefinition {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub domains: Vec<String>,
    pub completion_criterion: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_phase_id: Option<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maturity_gate_ids: Vec<String>,
    pub evidence_max_age_days: u64,
    pub closure_criteria: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remediation_phases: Vec<RemediationPhaseDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maturity_policy: Option<RemediationMaturityPolicy>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationExplanationStep {
    pub action_id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub completion_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationExplanationPhase {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub steps: Vec<RemediationExplanationStep>,
    pub completion_criterion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationHealthySurface {
    pub surface: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationExplanation {
    pub authority: String,
    pub summary: String,
    pub phases: Vec<RemediationExplanationPhase>,
    pub healthy_surfaces: Vec<RemediationHealthySurface>,
    pub closure_requirements: Vec<String>,
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
    #[serde(default)]
    pub integration_only_remaining: bool,
    pub progress: RemediationProgress,
    #[serde(default)]
    pub coverage: Vec<RemediationCoverage>,
    #[serde(default)]
    pub explanation: RemediationExplanation,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maturity_policy: Option<RemediationMaturityPolicy>,
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
pub struct GitHubOnlyCandidate {
    pub repository_id: String,
    pub provider: String,
    pub full_name: String,
    pub html_url: String,
    pub archived: bool,
    pub label: String,
    pub status: String,
    pub last_remediation_task: String,
    pub observed_at: String,
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
    #[serde(default)]
    pub github_only_candidates: Vec<GitHubOnlyCandidate>,
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
    scanned_branch: Option<String>,
    scanned_commit: Option<String>,
    findings: Vec<ParsedFinding>,
}

#[derive(Debug, Clone)]
struct ParsedFinding {
    id: String,
    fingerprint: Option<String>,
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
    #[serde(default)]
    additional_maturity_gate_ids: Vec<String>,
    evidence_max_age_days: Option<u64>,
    #[serde(default)]
    remediation_phases: Vec<RemediationPhaseDefinition>,
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
                "Public, optional-adapter, and local-only surfaces are classified and the built distribution passes the public-release boundary checks.",
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
        "github_only" => (
            "GitHub only",
            Vec::new(),
            Vec::new(),
            30,
            vec![
                "The repository is intentionally retained on GitHub only because local storage is constrained.",
                "A fresh provider snapshot confirms the GitHub repository and its remote identity.",
                "The terminal remediation task is recorded as GitHub only.",
            ],
        ),
        _ => return None,
    };
    let maturity_policy = maturity_policy_for_target(target_state);
    let maturity_gate_ids = maturity_policy
        .as_ref()
        .map(|_| vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()])
        .unwrap_or_default();
    Some(RemediationGoalProfile {
        schema_version: REMEDIATION_GOAL_SCHEMA.to_string(),
        target_state: target_state.to_string(),
        label: label.to_string(),
        required_gate_ids: required.into_iter().map(str::to_string).collect(),
        optional_gate_ids: optional.into_iter().map(str::to_string).collect(),
        maturity_gate_ids,
        evidence_max_age_days,
        closure_criteria: closure_criteria.into_iter().map(str::to_string).collect(),
        maturity_policy,
        contract_path: REMEDIATION_GOAL_PATH.to_string(),
        ..RemediationGoalProfile::default()
    })
}

fn maturity_improvement_rule() -> String {
    format!(
        "Reaching {MATURITY_CLOSURE_TARGET:.1}/4 clears blocking maturity remediation; a {MATURITY_IDEAL_SCORE:.1}/4 ideal claim additionally requires every configured maturity gate, including Mac Control where applicable, to be fresh and passing. Continue material, evidence-backed improvements toward the ideal without keeping the repository in the active queue solely for stretch work."
    )
}

fn maturity_integrity_rule() -> String {
    "Do not add or accept superficial documentation, configuration, tests, tab stops, Accessibility-only routes, or visual evidence solely to raise the score; each claimed improvement must close a real applicable gap and preserve structural evidence versus behavior-proof boundaries.".to_string()
}

fn maturity_policy_for_target(target_state: &str) -> Option<RemediationMaturityPolicy> {
    matches!(
        target_state,
        "public_release" | "deployed_product" | "active_maintained"
    )
    .then(|| RemediationMaturityPolicy {
        minimum_closure_score: MATURITY_CLOSURE_TARGET,
        ideal_score: MATURITY_IDEAL_SCORE,
        scoring_owner: "Quality Runner canonical maturity feed".to_string(),
        improvement_rule: maturity_improvement_rule(),
        integrity_rule: maturity_integrity_rule(),
        ideal_gate_ids: vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()],
    })
}

fn inferred_goal(repository: &RepositorySnapshot) -> (&'static str, String) {
    let lifecycle = repository.lifecycle.to_ascii_lowercase();
    let candidate = repository.lifecycle_candidate.to_ascii_lowercase();
    if is_github_only_label(&repository.lifecycle)
        || is_github_only_label(&repository.lifecycle_candidate)
    {
        return (
            "github_only",
            "The repository lifecycle records a storage-preserving GitHub-only disposition."
                .to_string(),
        );
    }
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

fn is_github_only_label(value: &str) -> bool {
    value.trim().to_ascii_lowercase().replace(['_', '-'], " ") == "github only"
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

fn normalized_maturity_gate_ids(values: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for value in values {
        let gate_id = value.trim().to_ascii_lowercase();
        if !ALL_MATURITY_GATE_IDS.contains(&gate_id.as_str()) {
            return Err(format!("Unknown remediation maturity gate '{value}'."));
        }
        normalized.push(gate_id);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalized_phase_token(value: &str, label: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || !normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(format!(
            "Remediation phase {label} '{value}' must contain only letters, numbers, underscores, or hyphens."
        ));
    }
    Ok(normalized)
}

fn normalized_remediation_phases(
    values: &[RemediationPhaseDefinition],
) -> Result<Vec<RemediationPhaseDefinition>, String> {
    let mut known_phase_ids = DEFAULT_REMEDIATION_PHASE_IDS
        .iter()
        .map(|value| (*value).to_string())
        .collect::<HashSet<_>>();
    known_phase_ids.insert(UNCLASSIFIED_REMEDIATION_PHASE_ID.to_string());
    let mut claimed_domains = HashSet::new();
    let mut normalized = Vec::new();

    for value in values {
        let id = normalized_phase_token(&value.id, "id")?;
        if !known_phase_ids.insert(id.clone()) {
            return Err(format!(
                "Remediation phase id '{}' is duplicated or reserved by the planner.",
                value.id
            ));
        }
        if value.title.trim().is_empty()
            || value.summary.trim().is_empty()
            || value.completion_criterion.trim().is_empty()
        {
            return Err(format!(
                "Remediation phase '{id}' requires a non-empty title, summary, and completion_criterion."
            ));
        }
        let mut domains = Vec::new();
        for domain in &value.domains {
            let domain = normalized_phase_token(domain, "domain")?;
            if !claimed_domains.insert(domain.clone()) {
                return Err(format!(
                    "Remediation action domain '{domain}' is claimed by more than one repository phase."
                ));
            }
            domains.push(domain);
        }
        domains.sort();
        domains.dedup();
        if domains.is_empty() {
            return Err(format!(
                "Remediation phase '{id}' must claim at least one action domain."
            ));
        }
        let after_phase_id = value
            .after_phase_id
            .as_deref()
            .map(|after| normalized_phase_token(after, "after_phase_id"))
            .transpose()?;
        if let Some(after) = after_phase_id.as_ref() {
            if after == &id || !known_phase_ids.contains(after) {
                return Err(format!(
                    "Remediation phase '{id}' references unknown or later phase '{after}'."
                ));
            }
        }
        normalized.push(RemediationPhaseDefinition {
            id,
            title: value.title.trim().to_string(),
            summary: value.summary.trim().to_string(),
            domains,
            completion_criterion: value.completion_criterion.trim().to_string(),
            after_phase_id,
        });
    }

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
    let additional_maturity =
        match normalized_maturity_gate_ids(&contract.additional_maturity_gate_ids) {
            Ok(gates) => gates,
            Err(error) => return inferred_goal_profile(repository, Some(error)),
        };
    let remediation_phases = match normalized_remediation_phases(&contract.remediation_phases) {
        Ok(phases) => phases,
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
    profile.maturity_gate_ids.extend(additional_maturity);
    profile.maturity_gate_ids.sort();
    profile.maturity_gate_ids.dedup();
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
    profile.remediation_phases = remediation_phases;
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

pub(crate) fn repository_requires_maturity(repository: &RepositorySnapshot) -> bool {
    goal_requires_maturity(&resolve_goal_profile(repository))
}

pub(crate) fn repository_requires_public_release_boundary(repository: &RepositorySnapshot) -> bool {
    resolve_goal_profile(repository).target_state == "public_release"
}

pub(crate) fn repository_requires_maturity_gate(
    repository: &RepositorySnapshot,
    gate_id: &str,
) -> bool {
    resolve_goal_profile(repository)
        .maturity_gate_ids
        .iter()
        .any(|configured| configured == gate_id)
}

fn goal_queue_rank(target_state: &str) -> u8 {
    match target_state {
        "public_release" => 0,
        "deployed_product" => 1,
        "active_maintained" => 2,
        "github_only" => 3,
        "clean_only" => 4,
        "prototype" => 5,
        "archived" => 6,
        _ => 7,
    }
}

pub fn empty_run() -> RemediationRun {
    RemediationRun {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        status: "not_run".to_string(),
        ..RemediationRun::default()
    }
}

pub fn sync_github_only_candidates(
    run: &mut RemediationRun,
    remote_repositories: &[RemoteRepositorySnapshot],
) {
    let mut candidates = remote_repositories
        .iter()
        .filter(|repository| {
            repository.provider.eq_ignore_ascii_case("github")
                && repository
                    .locality
                    .eq_ignore_ascii_case(GITHUB_ONLY_LOCALITY)
        })
        .map(|repository| GitHubOnlyCandidate {
            repository_id: repository.id.clone(),
            provider: repository.provider.clone(),
            full_name: repository.full_name.clone(),
            html_url: repository.html_url.clone(),
            archived: repository.archived,
            label: GITHUB_ONLY_LOCALITY.to_string(),
            status: "candidate".to_string(),
            last_remediation_task: GITHUB_ONLY_REMEDIATION_TASK.to_string(),
            observed_at: repository.last_refreshed_at.clone(),
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.full_name.cmp(&right.full_name));
    run.github_only_candidates = candidates;
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
        let retained_closure_is_current = previous_plan.is_none()
            && closures
                .iter()
                .filter(|closure| closure.repository_id == repository.id)
                .max_by(|left, right| left.closed_at.cmp(&right.closed_at))
                .is_some_and(|closure| closure_covers_plan(closure, &plan));
        if retained_closure_is_current {
            continue;
        }
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
        github_only_candidates: previous.github_only_candidates.clone(),
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
    plan.integration_only_remaining = integration_only_remaining(&plan.actions);
    plan.explanation = build_remediation_explanation(&plan.goal, &plan.actions, &plan.coverage);
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

fn closure_covers_plan(closure: &RemediationClosure, plan: &RemediationPlan) -> bool {
    if closure.source_refresh_id != plan.source_refresh_id
        || closure.target_state != plan.goal.target_state
        || closure.goal_source != plan.goal.source
    {
        return false;
    }
    let Some(closed_at) = DateTime::parse_from_rfc3339(&closure.closed_at).ok() else {
        return false;
    };
    plan.actions
        .iter()
        .flat_map(|action| action.evidence.iter())
        .filter_map(|item| item.observed_at.as_deref())
        .all(|observed_at| {
            DateTime::parse_from_rfc3339(observed_at)
                .is_ok_and(|observed_at| observed_at <= closed_at)
        })
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
        maturity_policy: plan.goal.maturity_policy.clone(),
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
    add_scope_seed(repository, &goal, &mut seeds);
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
    add_debloat_gate_seed(repository, qr_run.as_ref(), &mut seeds);
    add_branch_hygiene_seeds(repository, &mut seeds);
    add_submodule_seeds(repository, &mut seeds);
    add_evidence_contract_seeds(repository, &mut seeds);
    if goal_requires_maturity(&goal) {
        add_maturity_seeds(repository, &mut seeds);
        add_maturity_gate_seeds(repository, &goal, &mut seeds);
    }

    if !seeds.is_empty() || goal.target_state == "github_only" {
        let github_only_terminal_task = goal.target_state == "github_only";
        seeds.push(ActionSeed {
            stable_key: if github_only_terminal_task {
                GITHUB_ONLY_VERIFICATION_ACTION_KEY.to_string()
            } else {
                VERIFICATION_ACTION_KEY.to_string()
            },
            domain: "verification".to_string(),
            title: if github_only_terminal_task {
                GITHUB_ONLY_REMEDIATION_TASK.to_string()
            } else {
                "Verify the repository after remediation".to_string()
            },
            summary: if github_only_terminal_task {
                "Record the storage-preserving terminal disposition as GitHub only; no local checkout is required for this repository.".to_string()
            } else {
                "Re-run the eligible evidence sources and confirm the plan is clear before closing it.".to_string()
            },
            severity: "verification".to_string(),
            priority: "P2".to_string(),
            weight: 1,
            acceptance_criteria: if github_only_terminal_task {
                vec![
                    "A fresh provider snapshot confirms the GitHub repository and remote identity.".to_string(),
                    "The local checkout is intentionally absent or no longer required because local storage is constrained.".to_string(),
                    "The terminal remediation task is recorded as GitHub only.".to_string(),
                ]
            } else {
                vec![
                    "A fresh local snapshot is recorded.".to_string(),
                    "Project Compass, workspace, branch, submodule, condition, provider, quality, CI, and maturity evidence are rechecked where applicable.".to_string(),
                    "No unresolved blocking action remains.".to_string(),
                ]
            },
            evidence: vec![evidence(
                if github_only_terminal_task { "GitHub" } else { "Pronto" },
                if github_only_terminal_task {
                    "GitHub-only disposition"
                } else {
                    "Derived verification gate"
                },
                "Open",
                if github_only_terminal_task
                    && repository.provider_state.contains("GitHub connected")
                {
                    "Fresh"
                } else {
                    "Unknown"
                },
                repository
                    .last_fetch_at
                    .as_deref()
                    .or(Some(repository.last_scan_at.as_str())),
                None,
                if github_only_terminal_task {
                    "The provider snapshot must support the intentional GitHub-only storage disposition."
                } else {
                    "Verification is required after the source gaps are addressed."
                },
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
    let mut actions = seeds
        .into_iter()
        .map(|seed| materialize_action(repository, seed, &previous_actions, generated_at))
        .collect::<Vec<_>>();
    retain_resolved_action_history(&mut actions, previous, generated_at);
    let progress = calculate_progress(&actions);
    let coverage = build_ui_coverage(repository, &goal, &actions);
    let explanation = build_remediation_explanation(&goal, &actions, &coverage);
    let tracks = build_tracks(&actions);
    let status = plan_status(&actions);
    let current_stage = current_stage(&actions);
    let integration_only_remaining = integration_only_remaining(&actions);
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
        integration_only_remaining,
        progress,
        coverage,
        explanation,
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
    if goal.target_state == "public_release"
        && !repository.quality.release_boundary.is_release_ready()
    {
        let boundary = &repository.quality.release_boundary;
        let blocker_detail = if boundary.blocking_check_ids.is_empty() {
            boundary.detail.clone()
        } else {
            format!(
                "Blocking checks: {}. {}",
                boundary.blocking_check_ids.join(", "),
                boundary.detail
            )
        };
        seeds.push(ActionSeed {
            stable_key: PUBLIC_RELEASE_BOUNDARY_ACTION_KEY.to_string(),
            domain: "release_boundary".to_string(),
            title: if boundary.status == "Missing" {
                "Create and pass the public-release boundary receipt".to_string()
            } else {
                "Refresh and pass the public-release boundary receipt".to_string()
            },
            summary: blocker_detail.clone(),
            severity: "release".to_string(),
            priority: "P1".to_string(),
            weight: 3,
            acceptance_criteria: vec![
                "Every release-relevant source, configuration, and documentation surface is classified as public_core, public_adapter, or local_only; unclassified surfaces block release preparation.".to_string(),
                "Tracked source and documentation contain no personal absolute paths, private repository inventories, credentials, or private operational defaults.".to_string(),
                "Every built distribution is inspected against an explicit artifact allowlist, including wheel and sdist contents for Python packages where applicable.".to_string(),
                "The packaged artifact installs and runs with an isolated temporary home while Pronto, Leverage, Mac Control, and other private workspace peers are absent.".to_string(),
                "Optional integrations are verified through sanitized contract fixtures or consumer-owned tests without copying private data or setup-specific policy into the public release.".to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Quality Runner release boundary",
                "Public-release receipt",
                &boundary.status,
                &boundary.freshness,
                boundary.generated_at.as_deref(),
                boundary.report_path.as_deref(),
                &blocker_detail,
                boundary.scanned_branch.as_deref(),
                boundary.scanned_commit.as_deref(),
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

fn add_scope_seed(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let lifecycle = repository.lifecycle.to_ascii_lowercase();
    let candidate = repository.lifecycle_candidate.to_ascii_lowercase();
    let github_only = goal.target_state == "github_only"
        || is_github_only_label(&repository.lifecycle)
        || is_github_only_label(&repository.lifecycle_candidate);
    if !github_only
        && (lifecycle.contains("unconfirmed") || candidate != lifecycle && !candidate.is_empty())
    {
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
    if !github_only
        && repository
            .target_branch
            .as_ref()
            .or(repository.default_branch.as_ref())
            .is_none()
    {
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-target-branch".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the canonical integration branch".to_string(),
            summary: "Pronto has no configured target or observed default branch, so branch comparisons and integration targets are ambiguous.".to_string(),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The canonical integration branch is confirmed from repository or provider evidence.".to_string(),
                "Pronto records the target branch and can evaluate branch integration against it.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Target branch",
                "Missing",
                "Unknown",
                Some(&repository.last_scan_at),
                None,
                "No target or default branch was resolved for this repository.",
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
            if compass.open_blockers > 0 || compass.open_drift > 0 {
                let mut compass_evidence = Vec::new();
                if compass.open_blockers > 0 {
                    compass_evidence.push(evidence(
                        "Project Compass",
                        "Open blockers",
                        &compass.open_blockers.to_string(),
                        "Fresh",
                        compass.updated_at.as_deref(),
                        Some(&compass.contract_path),
                        "Open product blockers require reconciliation in the canonical contract.",
                    ));
                }
                if compass.open_drift > 0 {
                    compass_evidence.push(evidence(
                        "Project Compass",
                        "Open drift",
                        &compass.open_drift.to_string(),
                        "Fresh",
                        compass.updated_at.as_deref(),
                        Some(&compass.contract_path),
                        "Open product-to-implementation drift requires reconciliation in the canonical contract.",
                    ));
                }
                seeds.push(ActionSeed {
                    stable_key: PROJECT_COMPASS_OPEN_ITEMS_KEY.to_string(),
                    domain: "product_truth".to_string(),
                    title: "Reconcile open Project Compass items".to_string(),
                    summary: format!(
                        "Project Compass records {} open blocker(s) and {} open drift item(s). They are one product-truth reconciliation action, not independent evidence of additional remediation work.",
                        compass.open_blockers, compass.open_drift
                    ),
                    severity: "product_truth".to_string(),
                    priority: "P1".to_string(),
                    weight: severity_weight("warning"),
                    acceptance_criteria: vec![
                        "Each blocker is resolved or explicitly dispositioned in the canonical Project Compass contract.".to_string(),
                        "Each drift item is reconciled in implementation or explicitly dispositioned in Project Compass.".to_string(),
                        "Pronto refreshes the contract and reports no unexplained open blocker or drift.".to_string(),
                    ],
                    evidence: compass_evidence,
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
            .map_or(true, |rule| rule.allow_first_release)
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
            evidence: vec![evidence_with_provenance(
                "Quality Runner",
                "Latest repository run",
                "Present",
                &freshness,
                run.observed_at.as_deref(),
                run.run_dir.to_str(),
                "The repository has QR artifacts, but they are outside the fresh-evidence window.",
                run.scanned_branch.as_deref(),
                run.scanned_commit.as_deref(),
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
            evidence: vec![evidence_with_provenance(
                "Pronto",
                "Imported QR findings",
                "Stale",
                "Stale",
                repository.quality.findings.observed_at.as_deref(),
                repository.quality.findings.report_path.as_deref(),
                "The local quality projection is not fresh.",
                repository.quality.findings.scanned_branch.as_deref(),
                repository.quality.findings.scanned_commit.as_deref(),
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
            evidence: vec![evidence_with_provenance(
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
                gate_evidence.and_then(|item| item.scanned_branch.as_deref()),
                gate_evidence.and_then(|item| item.scanned_commit.as_deref()),
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
    let non_actionable = if repository.quality.findings.disposition_status == "Ready" {
        quality::non_actionable_finding_fingerprints(Path::new(&repository.path))
            .unwrap_or_default()
    } else {
        Default::default()
    };
    let mut groups = BTreeMap::<String, Vec<&ParsedFinding>>::new();
    for finding in run.findings.iter().filter(|finding| {
        let maturity_projection_owns_action = is_fleet_maturity_finding(finding)
            && (!goal_requires_maturity(goal)
                || repository
                    .quality
                    .maturity
                    .dimension_scores
                    .contains_key(&finding.category));
        !maturity_projection_owns_action
            && !finding
                .fingerprint
                .as_ref()
                .is_some_and(|fingerprint| non_actionable.contains(fingerprint))
    }) {
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
            // Repeated instances describe the scope of one remediation action; they must
            // not make that action dominate the entire plan's completion percentage.
            weight: max_weight.max(1),
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
            evidence: vec![evidence_with_provenance(
                "Quality Runner",
                &source_label,
                &format!("{count} finding(s)"),
                &freshness_for(run.observed_at.as_deref(), goal.evidence_max_age_days),
                run.observed_at.as_deref(),
                Some(&source_path),
                &detail,
                run.scanned_branch.as_deref(),
                run.scanned_commit.as_deref(),
            )],
            related_finding_ids: findings.iter().map(|finding| finding.id.clone()).collect(),
            source_run_id: Some(run.id.clone()),
        });
    }
    if run.findings.is_empty() && repository.quality.findings.actionable_total > 0 {
        seeds.push(ActionSeed {
            stable_key: "qr_findings:aggregate-report".to_string(),
            domain: "qr_findings".to_string(),
            title: "Resolve the findings in the QR report".to_string(),
            summary: format!(
                "Pronto imported {} actionable QR findings, but the current artifact does not expose leaf identities for grouping.",
                repository.quality.findings.actionable_total
            ),
            severity: if repository.quality.findings.high_severity_total > 0 {
                "high".to_string()
            } else {
                "warning".to_string()
            },
            priority: "P1".to_string(),
            weight: if repository.quality.findings.high_severity_total > 0 {
                severity_weight("high")
            } else {
                severity_weight("warning")
            },
            acceptance_criteria: vec![
                "The current QR report is reviewed and its findings are addressed.".to_string(),
                "A fresh QR report is rerun to verify the result.".to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Quality Runner",
                "QR aggregate report",
                &repository.quality.findings.actionable_total.to_string(),
                repository.quality.findings.freshness.as_str(),
                repository.quality.findings.observed_at.as_deref(),
                repository.quality.findings.report_path.as_deref(),
                "Leaf finding identities were not available in the current report.",
                repository.quality.findings.scanned_branch.as_deref(),
                repository.quality.findings.scanned_commit.as_deref(),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
}

fn add_debloat_gate_seed(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    seeds: &mut Vec<ActionSeed>,
) {
    let Some(gate) = repository
        .quality
        .gates
        .iter()
        .find(|gate| gate.id == "debloat")
    else {
        return;
    };
    if gate.status == quality::QualityGateStatus::Passed
        || seeds.iter().any(|seed| {
            seed.stable_key.starts_with(DEBLOAT_GROUP_CATEGORY_PREFIX)
                || seed.stable_key == "qr_findings:aggregate-report"
        })
    {
        return;
    }

    let detected = repository
        .quality
        .findings
        .category_counts
        .get("debloat")
        .copied()
        .unwrap_or_default();
    let actionable = repository
        .quality
        .findings
        .actionable_category_counts
        .get("debloat")
        .copied()
        .unwrap_or(detected);
    let gate_evidence = gate.evidence.first();
    seeds.push(ActionSeed {
        stable_key: DEBLOAT_GATE_ACTION_KEY.to_string(),
        domain: "qr_findings".to_string(),
        title: "Review the repository debloat signals".to_string(),
        summary: format!(
            "The debloat review gate is {} with {detected} detected structural signal(s) and {actionable} unresolved signal(s). No leaf remediation action remains after finding dispositions, so this action preserves an explicit path to review or refresh the signals without claiming architectural maturity.",
            gate.status.as_str()
        ),
        severity: "warning".to_string(),
        priority: "P2".to_string(),
        weight: severity_weight("warning"),
        acceptance_criteria: vec![
            "Each unresolved structural signal receives an ownership-pressure audit that checks duplicated engines, reusable helpers, parallel or legacy surfaces, and obsolete ownership paths."
                .to_string(),
            "Each audit finding records confidence independently from implementation or deletion readiness."
                .to_string(),
            "A fresh QR scan reports no unresolved structural debloat signals before the review gate passes."
                .to_string(),
            "Any deletion or structural rewrite is separately authorized and behavior-verified."
                .to_string(),
        ],
        evidence: vec![evidence_with_provenance(
            "Pronto quality",
            "Repository debloat review",
            gate.status.as_str(),
            gate.freshness.as_str(),
            gate_evidence.and_then(|item| item.observed_at.as_deref()),
            gate_evidence.and_then(|item| item.report_path.as_deref()),
            gate_evidence
                .map(|item| item.detail.as_str())
                .unwrap_or("The debloat review gate has no leaf evidence detail."),
            gate_evidence.and_then(|item| item.scanned_branch.as_deref()),
            gate_evidence.and_then(|item| item.scanned_commit.as_deref()),
        )],
        related_finding_ids: Vec::new(),
        source_run_id: qr_run.map(|run| run.id.clone()),
    });
}

fn is_fleet_maturity_finding(finding: &ParsedFinding) -> bool {
    finding
        .pack
        .as_deref()
        .is_some_and(|pack| pack.starts_with(quality::FLEET_MATURITY_FINDING_SCHEMA_PREFIX))
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
        if !workspace.status_available {
            let status_error = workspace.status_error.clone().unwrap_or_else(|| {
                "Git status could not be established for this workspace.".to_string()
            });
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:status-unavailable:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Restore Git status access · {}", workspace.branch),
                summary: format!(
                    "{} cannot be safely classified because Git status is unavailable: {status_error}",
                    workspace.path
                ),
                severity: "status".to_string(),
                priority: "P0".to_string(),
                weight: 3,
                acceptance_criteria: vec![
                    "Restore readable local Git status for the workspace.".to_string(),
                    "A scoped Pronto refresh records branch, cleanliness, upstream, and sync evidence again.".to_string(),
                    "No integration, cleanup, or publication decision is made while status evidence is unavailable.".to_string(),
                ],
                evidence: vec![evidence(
                    "Pronto",
                    &format!("Git status · {}", workspace.branch),
                    "Unavailable",
                    "Fresh",
                    workspace.last_activity_at.as_deref(),
                    None,
                    &status_error,
                )],
                related_finding_ids: Vec::new(),
                source_run_id: None,
            });
        } else if workspace.dirty {
            seeds.push(ActionSeed {
                stable_key: format!("branch_hygiene:dirty:{}", workspace.id),
                domain: "branch_hygiene".to_string(),
                title: format!("Resolve dirty workspace · {}", workspace.branch),
                summary: format!(
                    "{} has uncommitted changes that must be locally checkpointed and handoff-checked before remediation can be verified.",
                    workspace.path
                ),
                severity: "workspace".to_string(),
                priority: "P1".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    "The intended scoped changes are committed on the isolated working branch before this action is verified.".to_string(),
                    "Owner-ambiguous or unrelated changes remain preserved and explicitly reported; they are not stashed, overwritten, or silently folded.".to_string(),
                    "A fresh `pronto remediation handoff-check` receipt reports `ready: true`.".to_string(),
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
        if workspace.status_available && workspace.sync_state != "Synced" {
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
        if workspace.status_available
            && (remote_freshness.contains("not fetched")
                || remote_freshness.contains("stale")
                || remote_freshness.contains("unknown")
                || remote_freshness.contains("unavailable"))
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

pub(crate) fn workspace_activity_requires_coordination(
    activity: &crate::core::WorkspaceActivity,
) -> bool {
    workspace_activity_execution_state(activity) != "clear"
}

pub(crate) fn workspace_activity_execution_state(
    activity: &crate::core::WorkspaceActivity,
) -> &'static str {
    let explicitly_active = activity.state.eq_ignore_ascii_case("active")
        || activity
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.status.as_deref())
            .is_some_and(|status| {
                matches!(
                    status.to_ascii_lowercase().as_str(),
                    "active" | "running" | "started" | "paused" | "interrupted"
                )
            });
    if explicitly_active {
        return "coordination_required";
    }
    if activity
        .signals
        .iter()
        .any(|signal| signal.summary == "Activity state uncertain")
    {
        return "evidence_unavailable";
    }
    "clear"
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

fn add_maturity_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let maturity = &repository.quality.maturity;
    let freshness = maturity.freshness.as_str().to_string();
    let observed_at = maturity.observed_at.as_deref();
    let report_path = maturity.report_path.as_deref();
    match maturity.score {
        None => seeds.push(maturity_seed(
            "maturity:score",
            "Get a current maturity score",
            "No repository maturity score is available in the imported feed.",
            "Missing",
            &freshness,
            observed_at,
            report_path,
            maturity.scanned_branch.as_deref(),
            maturity.scanned_commit.as_deref(),
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
            maturity.scanned_branch.as_deref(),
            maturity.scanned_commit.as_deref(),
            2,
        )),
        Some(score) if score < MATURITY_CLOSURE_TARGET => seeds.push(maturity_seed(
            "maturity:score",
            "Raise the repository maturity score",
            &format!(
                "The current maturity score is {score:.3}/4; the minimum closure target is {MATURITY_CLOSURE_TARGET:.1}/4 and the evidence-backed ideal is {MATURITY_IDEAL_SCORE:.1}/4."
            ),
            "Below target",
            &freshness,
            observed_at,
            report_path,
            maturity.scanned_branch.as_deref(),
            maturity.scanned_commit.as_deref(),
            3,
        )),
        Some(_) => {}
    }
    for (dimension, score) in &maturity.dimension_scores {
        if *score < MATURITY_CLOSURE_TARGET {
            seeds.push(ActionSeed {
                stable_key: format!("maturity:dimension:{dimension}"),
                domain: "maturity".to_string(),
                title: format!("Improve the {dimension} maturity dimension"),
                summary: format!(
                    "The dimension score is {score:.3}/4; the minimum closure target is {MATURITY_CLOSURE_TARGET:.1}/4 and the evidence-backed ideal is {MATURITY_IDEAL_SCORE:.1}/4."
                ),
                severity: "maturity".to_string(),
                priority: "P2".to_string(),
                weight: 2,
                acceptance_criteria: vec![
                    format!("Address the evidence-backed gaps for {dimension}."),
                    format!(
                        "Fresh imported maturity evidence reports {dimension} at or above {MATURITY_CLOSURE_TARGET:.1}/4."
                    ),
                    maturity_improvement_rule(),
                    maturity_integrity_rule(),
                ],
                evidence: vec![evidence_with_provenance(
                    "Quality Runner maturity evidence",
                    &format!("Maturity dimension · {dimension}"),
                    &format!("{score:.3}/4"),
                    &freshness,
                    observed_at,
                    report_path,
                    "Dimension-level score imported from the latest QR maturity evidence.",
                    maturity.scanned_branch.as_deref(),
                    maturity.scanned_commit.as_deref(),
                )],
                related_finding_ids: Vec::new(),
                source_run_id: maturity.audit_id.clone(),
            });
        }
    }
}

fn add_evidence_contract_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    for contract in repository
        .quality
        .evidence_contracts
        .iter()
        .filter(|contract| contract.status != "current")
    {
        let observed = contract.observed_schema.as_deref().unwrap_or("missing");
        seeds.push(ActionSeed {
            stable_key: format!("evidence-contract:{}", contract.contract_id),
            domain: "evidence".to_string(),
            title: format!("Re-audit {} against {}", contract.label, contract.target_schema),
            summary: contract.message.clone(),
            severity: "maturity".to_string(),
            priority: "P1".to_string(),
            weight: 3,
            acceptance_criteria: vec![
                format!(
                    "The owning producer emits {} evidence for this repository.",
                    contract.target_schema
                ),
                "Pronto refreshes the evidence and reports this repository's contract status as current.".to_string(),
                "The audit preserves positive, negative, and ambiguous evidence; do not convert missing evidence into a pass.".to_string(),
            ],
            evidence: vec![evidence(
                "Evidence contract",
                &contract.label,
                observed,
                "Contract audit required",
                None,
                None,
                &contract.message,
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_maturity_gate_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    if !goal
        .maturity_gate_ids
        .iter()
        .any(|gate_id| gate_id == mac_control_maturity::MAC_CONTROL_GATE_ID)
    {
        return;
    }
    if repository
        .quality
        .evidence_contracts
        .iter()
        .any(|contract| {
            contract.contract_id == mac_control_maturity::MAC_CONTROL_TASK_CONTRACT_ID
                && contract.status != "current"
        })
    {
        return;
    }
    let gate = &repository.quality.mac_control_ideal_state;
    if (gate.status == "Passed" && gate.freshness == "Fresh")
        || (gate.status == "Not applicable" && gate.freshness == "Fresh")
    {
        return;
    }
    let summary = if gate.failure_reasons.is_empty() {
        format!(
            "The Mac Control ideal-state gate is {} with {} evidence; a fresh passing gate is required before Pronto can claim the 4.0/4.0 maturity ideal.",
            gate.status, gate.freshness
        )
    } else {
        format!(
            "The Mac Control ideal-state gate is {} with {} evidence: {}",
            gate.status,
            gate.freshness,
            gate.failure_reasons.join("; ")
        )
    };
    seeds.push(ActionSeed {
        stable_key: format!("maturity:gate:{}", mac_control_maturity::MAC_CONTROL_GATE_ID),
        domain: "maturity".to_string(),
        title: "Bring the Mac Control ideal-state gate to a fresh pass".to_string(),
        summary: summary.clone(),
        severity: "maturity".to_string(),
        priority: if gate.status == "Blocked" || gate.status == "Not configured" {
            "P1".to_string()
        } else {
            "P2".to_string()
        },
        weight: 3,
        acceptance_criteria: vec![
            "The canonical report accounts for every repository in Pronto's current maturity scope; no fixed repository count is assumed.".to_string(),
            "Every applicable supported task exposes a stable target, direct semantic action, observable postcondition, meaningful hierarchy, required state observations, explicit change states, and a measured eligible route.".to_string(),
            "The report is fresh and its observed commit matches each current repository commit before the 4.0/4.0 maturity ideal is claimed.".to_string(),
            "Do not add tab stops, Accessibility-only routes, or visual evidence solely to raise the maturity score; route choice must remain evidence-backed and task-appropriate.".to_string(),
            maturity_improvement_rule(),
            maturity_integrity_rule(),
        ],
        evidence: vec![evidence(
            "Mac Control",
            mac_control_maturity::MAC_CONTROL_GATE_LABEL,
            &gate.status,
            &gate.freshness,
            gate.observed_at.as_deref(),
            gate.report_path.as_deref(),
            &summary,
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn maturity_seed(
    stable_key: &str,
    title: &str,
    summary: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
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
            "Quality Runner maturity evidence is refreshed after the relevant work.".to_string(),
            format!(
                "The resulting maturity evidence is fresh and at or above {MATURITY_CLOSURE_TARGET:.1}/4 where applicable."
            ),
            maturity_improvement_rule(),
            maturity_integrity_rule(),
        ],
        evidence: vec![evidence_with_provenance(
            "Quality Runner maturity evidence",
            "Repository maturity",
            status,
            freshness,
            observed_at,
            report_path,
            summary,
            scanned_branch,
            scanned_commit,
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
    let legacy_key = legacy_action_key_for_current(&seed.stable_key);
    let previous_action = previous.get(seed.stable_key.as_str()).copied().or_else(|| {
        legacy_key
            .as_deref()
            .and_then(|key| previous.get(key).copied())
    });
    let preserved_status = previous_action
        .filter(|action| {
            matches!(
                action.status.as_str(),
                "in_progress" | "blocked" | "deferred"
            ) || (action.status == "verified"
                && !action_was_resolved_by_refresh(action)
                && seed
                    .evidence
                    .iter()
                    .any(|item| item.freshness.eq_ignore_ascii_case("fresh")))
        })
        .map(|action| action.status.clone())
        .unwrap_or_else(|| "open".to_string());
    let preserved_status = if seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY {
        "blocked".to_string()
    } else {
        preserved_status
    };
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

fn action_was_resolved_by_refresh(action: &RemediationAction) -> bool {
    action
        .evidence
        .iter()
        .any(|item| item.label == RESOLVED_BY_REFRESH_LABEL)
}

fn normalized_retained_weight(action: &RemediationAction) -> u64 {
    if action.stable_key.starts_with("qr_findings:group:")
        || action.stable_key == "qr_findings:aggregate-report"
    {
        severity_weight(&action.severity)
    } else {
        action.weight
    }
}

fn is_fleet_maturity_qr_action_key(stable_key: &str) -> bool {
    stable_key.starts_with("qr_findings:group:")
        && stable_key.contains(&format!("|{FLEET_MATURITY_FINDING_PACK_PREFIX}"))
}

fn legacy_action_key_for_current(stable_key: &str) -> Option<String> {
    stable_key
        .strip_prefix(DEBLOAT_GROUP_KEY_PREFIX)
        .map(|pack| format!("{LEGACY_DEBLOAT_GROUP_KEY_PREFIX}{pack}"))
}

fn retain_resolved_action_history(
    actions: &mut Vec<RemediationAction>,
    previous: Option<&RemediationPlan>,
    generated_at: &str,
) {
    let Some(previous) = previous else {
        return;
    };
    let current_keys = actions
        .iter()
        .map(|action| action.stable_key.clone())
        .collect::<HashSet<_>>();
    let superseded_legacy_keys = current_keys
        .iter()
        .filter_map(|stable_key| legacy_action_key_for_current(stable_key))
        .collect::<HashSet<_>>();
    let compass_items_are_grouped = current_keys.contains(PROJECT_COMPASS_OPEN_ITEMS_KEY);
    let mut resolved = previous
        .actions
        .iter()
        .filter(|action| {
            action.stable_key != VERIFICATION_ACTION_KEY
                && !current_keys.contains(&action.stable_key)
                && !superseded_legacy_keys.contains(&action.stable_key)
                && !is_fleet_maturity_qr_action_key(&action.stable_key)
                && !(compass_items_are_grouped
                    && LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS.contains(&action.stable_key.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();

    for action in &mut resolved {
        action.weight = normalized_retained_weight(action);
        action.status = "verified".to_string();
        action.updated_at = generated_at.to_string();
        action.completed_at = Some(generated_at.to_string());
        if !action_was_resolved_by_refresh(action) {
            action.evidence.push(evidence(
                "Pronto remediation",
                RESOLVED_BY_REFRESH_LABEL,
                "Resolved",
                "Fresh",
                Some(generated_at),
                None,
                "The refreshed projection no longer emits this action. It is retained as verified history while other remediation remains active.",
            ));
        }
    }
    actions.extend(resolved);
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
                "Lifecycle: {} · candidate: {} · target branch: {} · Git default: {}.",
                repository.lifecycle,
                repository.lifecycle_candidate,
                repository
                    .target_branch
                    .as_deref()
                    .or(repository.default_branch.as_deref())
                    .unwrap_or("Unknown"),
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
                "{} detected finding(s); {} actionable; {} reviewed; {} high-severity.",
                repository.quality.findings.total,
                repository.quality.findings.actionable_total,
                repository.quality.findings.reviewed_total,
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
            &[
                "scope:release-contract",
                "release_evidence:",
                PUBLIC_RELEASE_BOUNDARY_ACTION_KEY,
            ],
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

fn default_remediation_phase_definitions() -> Vec<RemediationPhaseDefinition> {
    vec![
        RemediationPhaseDefinition {
            id: "preserve_and_reconcile".to_string(),
            title: "Preserve and reconcile repository work".to_string(),
            summary: "Protect active or ambiguous work, then make workspaces, branches, operations, and the canonical target intentional.".to_string(),
            domains: ["scope", "repository_health", "branch_hygiene"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            completion_criterion: "Every scoped workspace, branch, operation, and repository-health action is verified or explicitly deferred with evidence.".to_string(),
            after_phase_id: None,
        },
        RemediationPhaseDefinition {
            id: "product_and_provider_truth".to_string(),
            title: "Reconcile product and provider truth".to_string(),
            summary: "Align the intended product outcome with fresh provider-native repository, pull-request, and release evidence.".to_string(),
            domains: ["product_truth", "provider"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            completion_criterion: "Product intent and provider-native branch, pull-request, and release evidence satisfy the repository goal.".to_string(),
            after_phase_id: Some("preserve_and_reconcile".to_string()),
        },
        RemediationPhaseDefinition {
            id: "quality_and_maturity".to_string(),
            title: "Reach quality and maturity threshold".to_string(),
            summary: "Refresh required evidence, clear actionable findings and gate failures, and reach the applicable maturity floor without manufacturing evidence.".to_string(),
            domains: ["evidence_refresh", "ci_ideal", "qr_findings", "maturity"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            completion_criterion: "Required gates and quality evidence are fresh, actionable findings are cleared, and applicable maturity reaches its minimum threshold.".to_string(),
            after_phase_id: Some("product_and_provider_truth".to_string()),
        },
        RemediationPhaseDefinition {
            id: "public_distribution_boundary".to_string(),
            title: "Prove the public distribution boundary".to_string(),
            summary: "Separate public product surfaces from optional adapters and local-only operations, then verify the packaged artifact in an isolated environment.".to_string(),
            domains: vec!["release_boundary".to_string()],
            completion_criterion: "Every release-relevant surface is classified, the artifact allowlist passes, isolated installation succeeds, and optional integrations rely only on sanitized contracts.".to_string(),
            after_phase_id: Some("quality_and_maturity".to_string()),
        },
        RemediationPhaseDefinition {
            id: "verify_and_close".to_string(),
            title: "Refresh and re-evaluate".to_string(),
            summary: "Re-run the scoped evidence sources after the material work and determine whether the current queue still has actionable work.".to_string(),
            domains: vec!["verification".to_string()],
            completion_criterion: "A fresh scoped remediation projection reports the current queue state and records any resolved actions in history without treating the repository as permanently complete.".to_string(),
            after_phase_id: Some("public_distribution_boundary".to_string()),
        },
    ]
}

fn ordered_remediation_phase_definitions(
    goal: &RemediationGoalProfile,
) -> Vec<RemediationPhaseDefinition> {
    let repository_domains = goal
        .remediation_phases
        .iter()
        .flat_map(|phase| phase.domains.iter().cloned())
        .collect::<HashSet<_>>();
    let mut definitions = default_remediation_phase_definitions();
    for definition in &mut definitions {
        definition
            .domains
            .retain(|domain| !repository_domains.contains(domain));
    }

    let mut insertion_tails = HashMap::<String, String>::new();
    for repository_phase in &goal.remediation_phases {
        let insertion_index =
            if let Some(requested_anchor) = repository_phase.after_phase_id.as_ref() {
                let effective_anchor = insertion_tails
                    .get(requested_anchor)
                    .unwrap_or(requested_anchor);
                definitions
                    .iter()
                    .position(|phase| &phase.id == effective_anchor)
                    .map(|index| index + 1)
                    .unwrap_or(definitions.len())
            } else {
                definitions
                    .iter()
                    .position(|phase| phase.id == "verify_and_close")
                    .unwrap_or(definitions.len())
            };
        definitions.insert(insertion_index, repository_phase.clone());
        if let Some(requested_anchor) = repository_phase.after_phase_id.as_ref() {
            insertion_tails.insert(requested_anchor.clone(), repository_phase.id.clone());
        }
    }
    definitions
}

fn explanation_phase(
    definition: &RemediationPhaseDefinition,
    matching: &[&RemediationAction],
) -> RemediationExplanationPhase {
    let status = if matching.iter().any(|action| action.status == "blocked") {
        "blocked"
    } else if matching.iter().any(|action| action.status == "in_progress") {
        "in_progress"
    } else {
        "open"
    };
    RemediationExplanationPhase {
        id: definition.id.clone(),
        title: definition.title.clone(),
        summary: definition.summary.clone(),
        status: status.to_string(),
        steps: matching
            .iter()
            .map(|action| RemediationExplanationStep {
                action_id: action.id.clone(),
                title: action.title.clone(),
                summary: action.summary.clone(),
                status: action.status.clone(),
                priority: action.priority.clone(),
                completion_criteria: action.acceptance_criteria.clone(),
            })
            .collect(),
        completion_criterion: definition.completion_criterion.clone(),
    }
}

fn build_remediation_explanation(
    goal: &RemediationGoalProfile,
    actions: &[RemediationAction],
    coverage: &[RemediationCoverage],
) -> RemediationExplanation {
    let active_actions = actions
        .iter()
        .filter(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"))
        .collect::<Vec<_>>();
    let definitions = ordered_remediation_phase_definitions(goal);
    let verification_phase_id = definitions
        .iter()
        .find(|definition| {
            definition
                .domains
                .iter()
                .any(|domain| domain == "verification")
        })
        .map(|definition| definition.id.clone());
    let mut assigned_action_indices = HashSet::new();
    let mut phases = Vec::new();
    for definition in &definitions {
        let matching_indices = active_actions
            .iter()
            .enumerate()
            .filter_map(|(index, action)| {
                (!assigned_action_indices.contains(&index)
                    && definition.domains.contains(&action.domain))
                .then_some(index)
            })
            .collect::<Vec<_>>();
        if matching_indices.is_empty() {
            continue;
        }
        assigned_action_indices.extend(matching_indices.iter().copied());
        let matching = matching_indices
            .iter()
            .map(|index| active_actions[*index])
            .collect::<Vec<_>>();
        phases.push(explanation_phase(definition, &matching));
    }

    let unmatched = active_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            (!assigned_action_indices.contains(&index)).then_some(*action)
        })
        .collect::<Vec<_>>();
    if !unmatched.is_empty() {
        let fallback = explanation_phase(
            &RemediationPhaseDefinition {
                id: UNCLASSIFIED_REMEDIATION_PHASE_ID.to_string(),
                title: "Classify additional remediation work".to_string(),
                summary: "These active actions use domains that are not yet assigned to a default or repository-defined remediation phase. They remain visible until the work is resolved or the repository goal contract classifies them.".to_string(),
                domains: Vec::new(),
                completion_criterion: "Every listed action is resolved or assigned to an explicit repository remediation phase without being hidden from the active plan.".to_string(),
                after_phase_id: None,
            },
            &unmatched,
        );
        let insertion_index = verification_phase_id
            .as_ref()
            .and_then(|phase_id| phases.iter().position(|phase| &phase.id == phase_id))
            .unwrap_or(phases.len());
        phases.insert(insertion_index, fallback);
    }
    let active_action_count = active_actions.len();
    debug_assert_eq!(
        phases.iter().map(|phase| phase.steps.len()).sum::<usize>(),
        active_action_count,
        "every active remediation action must appear in exactly one explanation phase"
    );
    let summary = if phases.is_empty() {
        "No active remediation phase remains for this refresh. Refresh scoped evidence before treating the queue as current."
            .to_string()
    } else {
        let phase_noun = if phases.len() == 1 { "phase" } else { "phases" };
        let phase_verb = if phases.len() == 1 {
            "remains"
        } else {
            "remain"
        };
        let action_noun = if active_action_count == 1 {
            "action"
        } else {
            "actions"
        };
        format!(
            "{} ordered remediation {phase_noun} {phase_verb} across {active_action_count} active {action_noun}. Work from the first unresolved phase and verify each result before refreshing the queue.",
            phases.len(),
        )
    };
    let healthy_surfaces = coverage
        .iter()
        .filter(|entry| matches!(entry.status.as_str(), "clear" | "verified"))
        .map(|entry| RemediationHealthySurface {
            surface: entry.surface.clone(),
            label: entry.label.clone(),
            status: entry.status.clone(),
            detail: entry.detail.clone(),
        })
        .collect::<Vec<_>>();
    let mut closure_requirements = goal.closure_criteria.clone();
    if let Some(policy) = &goal.maturity_policy {
        let ideal_gate_summary = if policy.ideal_gate_ids.is_empty() {
            "no additional maturity gates".to_string()
        } else {
            format!(
                "configured maturity gates ({})",
                policy.ideal_gate_ids.join(", ")
            )
        };
        closure_requirements.push(format!(
            "Fresh applicable maturity evidence reaches at least {:.1}/4; {:.1}/4 remains the evidence-backed ideal, and the ideal additionally requires {} to be fresh and passing, not a requirement for leaving remediation.",
            policy.minimum_closure_score, policy.ideal_score, ideal_gate_summary
        ));
    }
    closure_requirements.push(
        "A final scoped refresh reports the current open or blocked remediation actions; resolved actions are recorded as history and new evidence may reopen work."
            .to_string(),
    );

    RemediationExplanation {
        authority: "Advisory only: this explanation orders evidence-backed work but does not authorize Git, provider, publication, release, or pruning mutations."
            .to_string(),
        summary,
        phases,
        healthy_surfaces,
        closure_requirements,
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
    evidence_with_provenance(
        source,
        label,
        status,
        freshness,
        observed_at,
        report_path,
        detail,
        None,
        None,
    )
}

fn evidence_with_provenance(
    source: &str,
    label: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    detail: &str,
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
) -> RemediationEvidence {
    RemediationEvidence {
        source: source.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        freshness: freshness.to_string(),
        observed_at: observed_at.map(str::to_string),
        scanned_branch: scanned_branch.map(str::to_string),
        scanned_commit: scanned_commit.map(str::to_string),
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
    let has_active_actions = actions
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"));
    let percentage = if total_weight == 0 {
        100.0
    } else {
        let rounded = (verified_weight as f64 / total_weight as f64 * 100.0).round();
        if has_active_actions && rounded >= 100.0 {
            99.0
        } else {
            rounded
        }
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

fn integration_only_remaining(actions: &[RemediationAction]) -> bool {
    if actions.iter().any(|action| action.status == "blocked") {
        return false;
    }
    let active_material_actions = actions
        .iter()
        .filter(|action| matches!(action.status.as_str(), "open" | "in_progress"))
        .filter(|action| action.stable_key != VERIFICATION_ACTION_KEY)
        .collect::<Vec<_>>();
    !active_material_actions.is_empty()
        && active_material_actions
            .iter()
            .all(|action| action.stable_key.starts_with("branch_hygiene:integrate:"))
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
            let scanned_branch = first_string(
                &manifest,
                &[
                    &["git", "branch"],
                    &["git_provenance", "branch"],
                    &["provenance", "branch"],
                    &["branch"],
                ],
            );
            let scanned_commit = first_string(
                &manifest,
                &[
                    &["git", "head_sha"],
                    &["git_provenance", "head_sha"],
                    &["provenance", "head_sha"],
                    &["head_sha"],
                ],
            );
            let id = first_string(&manifest, &[&["run_id"], &["id"]])
                .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
            Some(QrRunEvidence {
                id,
                run_dir: run_dir.clone(),
                observed_at,
                scanned_branch,
                scanned_commit,
                findings: parse_findings(&run_dir),
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
            let observed_at = first_string(&payload, &[&["as_of"]]).or(summary_observed_at.clone());
            let target_branch = repository_payload
                .get("target_branch")
                .and_then(|value| first_string(value, &[&["branch"]]));
            let checkout = repository_payload
                .get("checkouts")
                .and_then(Value::as_array)
                .and_then(|checkouts| {
                    target_branch
                        .as_deref()
                        .and_then(|branch| {
                            checkouts.iter().find(|checkout| {
                                first_string(checkout, &[&["branch"]]).as_deref() == Some(branch)
                            })
                        })
                        .or_else(|| checkouts.first())
                });
            let scanned_branch = checkout.and_then(|value| first_string(value, &[&["branch"]]));
            let scanned_commit = checkout
                .and_then(|value| first_string(value, &[&["head"], &["fingerprint", "head"]]));
            let id = first_string(&payload, &[&["audit_id"], &["id"]])
                .or(summary_id.clone())
                .unwrap_or_else(|| path.display().to_string());
            Some(QrRunEvidence {
                id,
                run_dir: root.to_path_buf(),
                observed_at,
                scanned_branch,
                scanned_commit,
                findings,
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
        .filter(|(_, item)| fleet_finding_requires_remediation(item))
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
                fingerprint: first_string(item, &[&["fingerprint"]]),
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

fn fleet_finding_requires_remediation(item: &Value) -> bool {
    if item.get("applicable").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    let is_maturity_record = quality::is_fleet_maturity_finding(item);
    if !is_maturity_record {
        return true;
    }
    if let Some(score) = item.get("score").and_then(Value::as_f64) {
        return score < MATURITY_CLOSURE_TARGET;
    }
    !first_string(item, &[&["status"], &["bucket"]]).is_some_and(|status| {
        matches!(
            status.to_ascii_lowercase().as_str(),
            "validated" | "maintained" | "not_applicable"
        )
    })
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
                fingerprint: first_string(item, &[&["fingerprint"]]),
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
Repositories leave the active table when the current evidence produces no \
actionable work or records an explicit deferral. That is a point-in-time queue \
transition, not a permanent repository state; a later refresh may reopen the \
same repository. Git, provider, publication, and pruning actions still require \
their own authorization.\n\n\
For maturity-applicable goals, **{MATURITY_CLOSURE_TARGET:.1}/4 is the minimum \
maturity threshold and {MATURITY_IDEAL_SCORE:.1}/4 is the evidence-backed ideal**. \
Continue only material improvements after the threshold, and never add superficial \
documentation, configuration, tests, or other artifacts solely to raise the \
score.\n\n\
Ranking preserves plan status, the earliest unresolved remediation domain, \
and action priority before fleet leverage. Pronto, AIOS, and Quality Runner \
receive explicit control-plane or evidence-provider precedence before the \
intended repository goal and raw action weight are used as tie-breakers.\n\n\
## Active queue\n\n\
Active repositories: **{}**. Resolved action history entries: **{}**. GitHub-only candidates: **{}**.\n\n\
<!-- prettier-ignore -->\n\
| Rank | Repository | Goal | Goal source | Status | Current stage | Remaining path | Leverage | Tracked gaps | Active actions | First safe action |\n\
| ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- |\n",
        run.schema_version,
        run.generated_at,
        run.plans.len(),
        run.closures.len(),
        run.github_only_candidates.len()
    );
    if run.plans.is_empty() {
        output.push_str("| — | _No active remediation remains_ | — | — | complete | complete | — | — | 0 | 0 | Refresh scoped evidence before treating this as current. |\n");
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
            let remaining_path = plan
                .explanation
                .phases
                .iter()
                .map(|phase| phase.title.as_str())
                .collect::<Vec<_>>()
                .join(" → ");
            let leverage = queue_leverage(&plan.repository_name).1;
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                index + 1,
                markdown_cell(&plan.repository_name),
                markdown_cell(&plan.goal.label),
                markdown_cell(&plan.goal.source),
                markdown_cell(&plan.status),
                markdown_cell(&plan.current_stage),
                markdown_cell(&remaining_path),
                markdown_cell(leverage),
                tracked_gap_count,
                active_action_count,
                markdown_cell(first_action),
            ));
        }
    }
    output.push_str("\n## GitHub-only candidates\n\n");
    if run.github_only_candidates.is_empty() {
        output
            .push_str("No GitHub-only candidates are present in the current provider snapshot.\n");
    } else {
        output.push_str(
            "These provider-backed repositories have no matching local checkout; they remain counted without creating synthetic local plans. The terminal remediation task is **GitHub only**.\n\n\
<!-- prettier-ignore -->\n\
| Candidate | Label | Status | Last remediation task | Observed at |\n\
| --- | --- | --- | --- | --- |\n",
        );
        for candidate in &run.github_only_candidates {
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | `{}` |\n",
                markdown_cell(&candidate.full_name),
                markdown_cell(&candidate.label),
                markdown_cell(&candidate.status),
                markdown_cell(&candidate.last_remediation_task),
                markdown_cell(&candidate.observed_at),
            ));
        }
    }
    output.push_str("\n## Resolved action history\n\n");
    if run.closures.is_empty() {
        output.push_str("No resolved action history is present in this run.\n");
    } else {
        output.push_str(
            "<!-- prettier-ignore -->\n\
| Repository | Goal | Goal source | Disposition | Resolved at | Resolved actions | Evidence observed at | Summary |\n\
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
        ActivitySignal, BranchSummary, Condition, RemoteRepositorySnapshot, SubmoduleSummary,
        WorkspaceActivity, WorkspaceSummary,
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
            target_branch: Some("dev".to_string()),
            target_branch_configured: false,
            workspace: WorkspaceSummary {
                id: format!("workspace-{name}"),
                path: format!("/tmp/{name}"),
                is_primary: true,
                branch: "dev".to_string(),
                status_available: true,
                status_error: None,
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
                sync_detail: None,
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
        let coverage = Vec::new();
        let explanation = build_remediation_explanation(&goal, &actions, &coverage);
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
            integration_only_remaining: integration_only_remaining(&actions),
            progress: calculate_progress(&actions),
            coverage,
            explanation,
            tracks: build_tracks(&actions),
            actions,
        }
    }

    #[test]
    fn project_compass_blockers_and_drift_share_one_remediation_action() {
        let mut repository = fixture_repository("compass-grouping");
        repository.project_compass.status = "Ready".to_string();
        repository.project_compass.contract_path =
            "/tmp/compass-grouping/.project-compass/contract.json".to_string();
        repository.project_compass.updated_at = Some("2026-08-03T18:04:36Z".to_string());
        repository.project_compass.open_blockers = 1;
        repository.project_compass.open_drift = 1;
        let mut seeds = Vec::new();

        add_project_compass_seeds(&repository, &mut seeds);

        assert_eq!(seeds.len(), 1);
        let seed = &seeds[0];
        assert_eq!(seed.stable_key, PROJECT_COMPASS_OPEN_ITEMS_KEY);
        assert_eq!(seed.weight, severity_weight("warning"));
        assert_eq!(seed.evidence.len(), 2);
        assert!(seed.summary.contains("1 open blocker(s)"));
        assert!(seed.summary.contains("1 open drift item(s)"));
    }

    #[test]
    fn does_not_exclude_repositories_without_policy_entries() {
        assert!(!is_excluded_repository(&fixture_repository(
            "soundscape-app"
        )));
        assert!(!is_excluded_repository(&fixture_repository("tenure")));
        assert!(!is_excluded_repository(&fixture_repository("pronto")));
    }

    #[test]
    fn reviewed_non_actionable_findings_do_not_seed_remediation_actions() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-finding-disposition-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("disposition fixture should be writable");
        fs::write(
            contract_dir.join("quality-finding-dispositions.json"),
            r#"{
  "schema_version": "pronto-quality-finding-dispositions/v1",
  "updated_at": "2026-07-29T12:00:00Z",
  "dispositions": [
    {
      "fingerprint": "fp-reviewed",
      "status": "false_positive",
      "reason": "The symbol is referenced by the renderer contract.",
      "reviewer": "fixture-reviewer",
      "reviewed_at": "2026-07-29T12:00:00Z",
      "evidence": ["src/renderer/src/types.ts:1"]
    }
  ]
}"#,
        )
        .expect("disposition contract should be writable");

        let mut repository = fixture_repository("disposition-filter");
        repository.path = root.to_string_lossy().to_string();
        repository.quality.findings.disposition_status = "Ready".to_string();
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: root.join(".quality-runner/runs/qr-run"),
            observed_at: Some("2026-07-29T12:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![ParsedFinding {
                id: "finding-reviewed".to_string(),
                fingerprint: Some("fp-reviewed".to_string()),
                group_key: "dead-code:reviewed".to_string(),
                category: "dead-code".to_string(),
                pack: Some("maintainability".to_string()),
                severity: "warning".to_string(),
                title: "Unused export".to_string(),
                summary: "A detector reported an unused export.".to_string(),
                file: Some("src/renderer/src/types.ts".to_string()),
                line: Some(1),
                verification: None,
                report_path: root
                    .join(".quality-runner/runs/qr-run/findings.json")
                    .to_string_lossy()
                    .to_string(),
            }],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);

        assert!(seeds.is_empty());

        repository.quality.findings.disposition_status = "Unreconcilable".to_string();
        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);
        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].related_finding_ids, vec!["finding-reviewed"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepted_risk_debloat_gate_keeps_a_remediation_link() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("pronto-debloat-risk-{fixture_id}"));
        let contract_dir = root.join(".pronto");
        fs::create_dir_all(&contract_dir).expect("disposition fixture should be writable");
        fs::write(
            contract_dir.join("quality-finding-dispositions.json"),
            r#"{
  "schema_version": "pronto-quality-finding-dispositions/v1",
  "updated_at": "2026-08-04T02:00:00Z",
  "dispositions": [
    {
      "fingerprint": "fp-debloat-risk",
      "status": "accepted_risk",
      "reason": "The owner accepts the current size pending a later redesign.",
      "reviewer": "fixture-reviewer",
      "reviewed_at": "2026-08-04T02:00:00Z",
      "evidence": ["src/router.rs:1"]
    }
  ]
}"#,
        )
        .expect("disposition contract should be writable");

        let mut repository = fixture_repository("debloat-risk");
        repository.path = root.to_string_lossy().to_string();
        repository.quality.findings.disposition_status = "Ready".to_string();
        repository
            .quality
            .findings
            .category_counts
            .insert("debloat".to_string(), 1);
        repository
            .quality
            .findings
            .actionable_category_counts
            .insert("debloat".to_string(), 1);
        repository.quality.gates.push(quality::QualityGate {
            id: "debloat".to_string(),
            label: "Repository debloat review".to_string(),
            status: quality::QualityGateStatus::Blocked,
            freshness: quality::QualityFreshness::Fresh,
            evidence: Vec::new(),
        });
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: root.join(".quality-runner/runs/qr-run"),
            observed_at: Some("2026-08-04T02:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![ParsedFinding {
                id: "finding-debloat-risk".to_string(),
                fingerprint: Some("fp-debloat-risk".to_string()),
                group_key: "debloat|debloat candidate review|unknown".to_string(),
                category: "debloat".to_string(),
                pack: None,
                severity: "warning".to_string(),
                title: "Review oversized source files".to_string(),
                summary: "A source file exceeds the debloat review threshold.".to_string(),
                file: Some("src/router.rs".to_string()),
                line: Some(1),
                verification: None,
                report_path: root
                    .join(".quality-runner/runs/qr-run/findings.json")
                    .to_string_lossy()
                    .to_string(),
            }],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);
        assert!(
            seeds.is_empty(),
            "accepted risk should not remain a leaf action"
        );
        add_debloat_gate_seed(&repository, Some(&run), &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].stable_key, DEBLOAT_GATE_ACTION_KEY);
        assert!(seeds[0].summary.contains("1 unresolved signal(s)"));
        assert!(seeds[0]
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("separately authorized")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn debloat_gate_does_not_duplicate_a_leaf_finding_action() {
        let mut repository = fixture_repository("debloat-leaf");
        repository.quality.findings.disposition_status = "Missing".to_string();
        repository.quality.gates.push(quality::QualityGate {
            id: "debloat".to_string(),
            label: "Repository debloat review".to_string(),
            status: quality::QualityGateStatus::Blocked,
            freshness: quality::QualityFreshness::Fresh,
            evidence: Vec::new(),
        });
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: PathBuf::from("/tmp/qr-run"),
            observed_at: Some("2026-08-04T02:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![ParsedFinding {
                id: "finding-debloat".to_string(),
                fingerprint: None,
                group_key: "debloat|debloat candidate review|unknown".to_string(),
                category: "debloat".to_string(),
                pack: None,
                severity: "warning".to_string(),
                title: "Review oversized source files".to_string(),
                summary: "A source file exceeds the debloat review threshold.".to_string(),
                file: Some("src/router.rs".to_string()),
                line: Some(1),
                verification: None,
                report_path: "/tmp/findings.json".to_string(),
            }],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);
        add_debloat_gate_seed(&repository, Some(&run), &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert!(seeds[0]
            .stable_key
            .starts_with(DEBLOAT_GROUP_CATEGORY_PREFIX));
    }

    #[test]
    fn grouped_qr_occurrences_do_not_multiply_plan_weight() {
        let mut repository = fixture_repository("grouped-findings");
        repository.quality.findings.disposition_status = "Missing".to_string();
        let finding = ParsedFinding {
            id: "finding-one".to_string(),
            fingerprint: None,
            group_key: "developer-experience:setup-path".to_string(),
            category: "developer-experience".to_string(),
            pack: Some("maintainability".to_string()),
            severity: "high".to_string(),
            title: "Remove machine-specific setup paths".to_string(),
            summary: "A setup path is machine-specific.".to_string(),
            file: Some("README.md".to_string()),
            line: Some(10),
            verification: None,
            report_path: "/tmp/findings.json".to_string(),
        };
        let mut second = finding.clone();
        second.id = "finding-two".to_string();
        second.line = Some(20);
        let run = QrRunEvidence {
            id: "qr-run".to_string(),
            run_dir: PathBuf::from("/tmp/qr-run"),
            observed_at: Some("2026-07-29T12:00:00Z".to_string()),
            scanned_branch: None,
            scanned_commit: None,
            findings: vec![finding, second],
        };
        let goal = goal_definition("active_maintained").expect("fixture goal should be supported");
        let mut seeds = Vec::new();

        add_qr_finding_seeds(&repository, Some(&run), &goal, &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].weight, severity_weight("high"));
        assert_eq!(seeds[0].related_finding_ids.len(), 2);
        assert!(seeds[0].summary.contains("2 finding(s)"));
    }

    #[test]
    fn resolved_actions_leave_the_active_queue_and_retain_history() {
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
        let retained_policy = run.closures[0]
            .maturity_policy
            .as_ref()
            .expect("resolved history should retain maturity policy");
        assert_eq!(retained_policy.minimum_closure_score, 3.0);
        assert_eq!(retained_policy.ideal_score, 4.0);
        assert_eq!(
            run.closures[0].last_evidence_at.as_deref(),
            Some("2026-07-29T12:00:00Z")
        );
    }

    #[test]
    fn resolved_history_stays_out_of_queue_until_new_evidence_arrives() {
        let mut repository = fixture_repository("closure-retention");
        repository.last_scan_at = "2026-08-03T12:00:00Z".to_string();
        repository.last_fetch_at = Some("2026-08-03T12:00:00Z".to_string());
        repository.workspace.last_commit_at = Some("2026-08-03T12:00:00Z".to_string());
        let current = build_plan(
            &repository,
            None,
            Some("refresh-1"),
            "2026-08-03T12:30:00Z",
            None,
        );
        let mut previous = empty_run();
        previous.closures = vec![closure_from_plan(
            &current,
            "2026-08-03T13:00:00Z",
            Some("refresh-1"),
        )];

        let retained = rebuild_run(&[repository.clone()], &previous, Some("refresh-1"));

        assert!(retained.plans.is_empty());
        assert_eq!(retained.closures.len(), 1);

        repository.last_scan_at = "2026-08-03T14:00:00Z".to_string();
        let reopened = rebuild_run(&[repository], &retained, Some("refresh-1"));

        assert_eq!(reopened.plans.len(), 1);
        assert_eq!(reopened.plans[0].repository_name, "closure-retention");
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
    fn public_release_goal_requires_an_explicit_distribution_boundary() {
        let repository = fixture_repository("public-distribution-boundary");
        let mut goal = goal_definition("public_release").expect("public release goal");
        goal.source = "repository_contract".to_string();
        let mut seeds = Vec::new();

        add_goal_seeds(&repository, &goal, &mut seeds);

        let boundary = seeds
            .iter()
            .find(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
            .expect("public release should require a distribution boundary action");
        assert_eq!(boundary.domain, "release_boundary");
        assert!(boundary
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("public_core")));
        assert!(boundary
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("isolated temporary home")));
        assert_eq!(
            seeds
                .iter()
                .filter(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
                .count(),
            1
        );
    }

    #[test]
    fn passing_release_boundary_removes_the_remediation_action() {
        let mut repository = fixture_repository("public-boundary-passed");
        repository.quality.release_boundary.status = "Passed".to_string();
        repository.quality.release_boundary.freshness = "Fresh".to_string();
        repository
            .quality
            .release_boundary
            .blocking_check_ids
            .clear();
        repository.quality.release_boundary.checks = [
            "source_provenance",
            "surface_classification",
            "tracked_public_content",
            "public_adapter_fixtures",
            "distribution_archives",
            "clean_room_install",
        ]
        .into_iter()
        .map(|id| crate::release_boundary::ReleaseBoundaryCheck {
            id: id.to_string(),
            status: "passed".to_string(),
            reason: None,
        })
        .collect();
        let goal = goal_definition("public_release").expect("public release goal");
        let mut seeds = Vec::new();

        add_goal_seeds(&repository, &goal, &mut seeds);

        assert!(seeds
            .iter()
            .all(|seed| seed.stable_key != PUBLIC_RELEASE_BOUNDARY_ACTION_KEY));
    }

    #[test]
    fn receipt_blockers_drive_remediation_and_manual_status_cannot_bypass_them() {
        let mut repository = fixture_repository("public-boundary-blocked");
        repository.quality.release_boundary.status = "Blocked".to_string();
        repository.quality.release_boundary.freshness = "Stale".to_string();
        repository.quality.release_boundary.blocking_check_ids = vec![
            "artifact_digest_mismatch".to_string(),
            "matrix_digest_mismatch".to_string(),
        ];
        repository.quality.release_boundary.detail =
            "The receipt no longer matches the release inputs.".to_string();
        let goal = goal_definition("public_release").expect("public release goal");
        let mut seeds = Vec::new();
        add_goal_seeds(&repository, &goal, &mut seeds);
        let seed = seeds
            .into_iter()
            .find(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
            .expect("blocked receipt should produce remediation");
        assert!(seed.summary.contains("artifact_digest_mismatch"));
        assert!(seed.summary.contains("matrix_digest_mismatch"));

        let mut previous_action = fixture_action(
            PUBLIC_RELEASE_BOUNDARY_ACTION_KEY,
            "release_boundary",
            "P1",
            3,
            "verified",
        );
        previous_action.stable_key = PUBLIC_RELEASE_BOUNDARY_ACTION_KEY.to_string();
        let previous = HashMap::from([(previous_action.stable_key.as_str(), &previous_action)]);
        let action = materialize_action(&repository, seed, &previous, "2026-08-11T12:00:00Z");
        assert_eq!(action.status, "blocked");
    }

    #[test]
    fn non_public_and_ambiguous_goals_do_not_inherit_the_distribution_boundary() {
        let repository = fixture_repository("non-public-distribution-boundary");
        for target in [
            "deployed_product",
            "active_maintained",
            "clean_only",
            "prototype",
            "archived",
            "github_only",
        ] {
            let goal = goal_definition(target).expect("supported remediation goal");
            let mut seeds = Vec::new();
            add_goal_seeds(&repository, &goal, &mut seeds);
            assert!(
                seeds
                    .iter()
                    .all(|seed| seed.stable_key != PUBLIC_RELEASE_BOUNDARY_ACTION_KEY),
                "{target} should not inherit public distribution work"
            );
        }

        let inferred = inferred_goal_profile(&repository, None);
        assert_eq!(inferred.target_state, "active_maintained");
        let mut inferred_seeds = Vec::new();
        add_goal_seeds(&repository, &inferred, &mut inferred_seeds);
        assert!(inferred_seeds
            .iter()
            .any(|seed| seed.stable_key == "scope:confirm-remediation-goal"));
        assert!(inferred_seeds
            .iter()
            .all(|seed| seed.stable_key != PUBLIC_RELEASE_BOUNDARY_ACTION_KEY));
    }

    #[test]
    fn public_distribution_boundary_is_a_release_preparation_phase_and_surface() {
        let repository = fixture_repository("public-boundary-coverage");
        let mut goal = goal_definition("public_release").expect("public release goal");
        goal.source = "repository_contract".to_string();
        let mut seeds = Vec::new();
        add_goal_seeds(&repository, &goal, &mut seeds);
        let boundary_seed = seeds
            .into_iter()
            .find(|seed| seed.stable_key == PUBLIC_RELEASE_BOUNDARY_ACTION_KEY)
            .expect("public release boundary seed");
        let action = materialize_action(
            &repository,
            boundary_seed,
            &HashMap::new(),
            "2026-08-11T12:00:00Z",
        );

        let coverage = build_ui_coverage(&repository, &goal, std::slice::from_ref(&action));
        let release_preparation = coverage
            .iter()
            .find(|entry| entry.surface == "release_preparation")
            .expect("release preparation coverage");
        assert_eq!(release_preparation.status, "blocked");
        assert_eq!(release_preparation.action_ids, vec![action.id.clone()]);

        let explanation = build_remediation_explanation(&goal, &[action], &coverage);
        assert_eq!(explanation.phases.len(), 1);
        assert_eq!(explanation.phases[0].id, "public_distribution_boundary");
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
                "evidence_max_age_days": 21,
                "remediation_phases": [
                    {
                        "id": "deployment_validation",
                        "title": "Validate the local deployment",
                        "summary": "Verify repository-specific deployment behavior.",
                        "domains": ["deployment_validation"],
                        "completion_criterion": "The local deployment evidence is current.",
                        "after_phase_id": "quality_and_maturity"
                    }
                ]
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
        assert_eq!(goal.remediation_phases.len(), 1);
        assert_eq!(goal.remediation_phases[0].id, "deployment_validation");
        assert_eq!(
            goal.remediation_phases[0].after_phase_id.as_deref(),
            Some("quality_and_maturity")
        );
        fs::remove_dir_all(root).expect("goal fixture should be removable");
    }

    #[test]
    fn repository_phase_contract_rejects_duplicate_domain_ownership() {
        let phases = vec![
            RemediationPhaseDefinition {
                id: "first".to_string(),
                title: "First".to_string(),
                summary: "First phase.".to_string(),
                domains: vec!["deployment".to_string()],
                completion_criterion: "First phase is complete.".to_string(),
                after_phase_id: None,
            },
            RemediationPhaseDefinition {
                id: "second".to_string(),
                title: "Second".to_string(),
                summary: "Second phase.".to_string(),
                domains: vec!["deployment".to_string()],
                completion_criterion: "Second phase is complete.".to_string(),
                after_phase_id: Some("first".to_string()),
            },
        ];

        let error = normalized_remediation_phases(&phases)
            .expect_err("one action domain cannot belong to two repository phases");

        assert!(error.contains("claimed by more than one repository phase"));
    }

    #[test]
    fn repository_phase_contract_reserves_the_unclassified_fallback_id() {
        let phases = vec![RemediationPhaseDefinition {
            id: UNCLASSIFIED_REMEDIATION_PHASE_ID.to_string(),
            title: "Custom fallback".to_string(),
            summary: "This would collide with the planner fallback.".to_string(),
            domains: vec!["future_domain".to_string()],
            completion_criterion: "The phase is complete.".to_string(),
            after_phase_id: None,
        }];

        let error = normalized_remediation_phases(&phases)
            .expect_err("the planner fallback id cannot be redefined by a repository");

        assert!(error.contains("duplicated or reserved"));
    }

    #[test]
    fn maturity_goals_expose_the_closure_floor_ideal_and_integrity_rule() {
        let goal = goal_definition("active_maintained").expect("maturity goal should be supported");
        let policy = goal
            .maturity_policy
            .as_ref()
            .expect("maturity-applicable goals should expose policy");

        assert_eq!(policy.minimum_closure_score, 3.0);
        assert_eq!(policy.ideal_score, 4.0);
        assert_eq!(
            policy.scoring_owner,
            "Quality Runner canonical maturity feed"
        );
        assert_eq!(
            policy.ideal_gate_ids,
            vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()]
        );
        assert_eq!(
            goal.maturity_gate_ids,
            vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()]
        );
        assert!(policy
            .improvement_rule
            .to_ascii_lowercase()
            .contains("continue material"));
        assert!(policy.integrity_rule.contains("superficial documentation"));

        let seed = maturity_seed(
            "maturity:score",
            "Raise maturity",
            "Below target",
            "Below target",
            "Fresh",
            Some("2026-07-29T12:00:00Z"),
            Some("/tmp/maturity.json"),
            None,
            None,
            3,
        );
        assert!(seed
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("4.0/4")));
        assert!(seed
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("solely to raise the score")));

        let repository = fixture_repository("mac-control-gate");
        let mut gate_seeds = Vec::new();
        add_maturity_gate_seeds(&repository, &goal, &mut gate_seeds);
        assert_eq!(gate_seeds.len(), 1);
        assert!(gate_seeds[0]
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("no fixed repository count")));
        assert!(gate_seeds[0].summary.contains("4.0/4.0 maturity ideal"));

        let mut passing_repository = repository;
        passing_repository.quality.mac_control_ideal_state.status = "Passed".to_string();
        passing_repository.quality.mac_control_ideal_state.freshness = "Fresh".to_string();
        let mut passing_gate_seeds = Vec::new();
        add_maturity_gate_seeds(&passing_repository, &goal, &mut passing_gate_seeds);
        assert!(passing_gate_seeds.is_empty());
    }

    #[test]
    fn stale_evidence_contract_gets_one_generic_remediation_action() {
        let mut repository = fixture_repository("contract-audit");
        let contract = crate::evidence_contract::evaluate_repository_contract(
            mac_control_maturity::MAC_CONTROL_TASK_CONTRACT_ID,
            mac_control_maturity::MAC_CONTROL_TASK_CONTRACT_LABEL,
            mac_control_maturity::MAC_CONTROL_TASK_MANIFEST_SCHEMA,
            Some("mac-control-task-manifest/v2"),
            &repository.id,
            &repository.name,
        );
        repository.quality.evidence_contracts = vec![contract];
        let goal = goal_definition("active_maintained").expect("goal fixture");
        let mut seeds = Vec::new();

        add_evidence_contract_seeds(&repository, &mut seeds);
        add_maturity_gate_seeds(&repository, &goal, &mut seeds);

        assert_eq!(seeds.len(), 1);
        assert_eq!(
            seeds[0].stable_key,
            "evidence-contract:mac-control-task-manifest"
        );
        assert!(seeds[0]
            .title
            .contains(mac_control_maturity::MAC_CONTROL_TASK_MANIFEST_SCHEMA));
        assert!(seeds[0]
            .acceptance_criteria
            .iter()
            .any(|criterion| criterion.contains("ambiguous evidence")));
    }

    #[test]
    fn remediation_explanation_groups_work_into_ordered_phases_and_names_healthy_surfaces() {
        let mut preserve = fixture_action(
            "branch_hygiene:dirty:workspace",
            "branch_hygiene",
            "P1",
            2,
            "open",
        );
        preserve.title = "Preserve the dirty workspace".to_string();
        preserve.summary = "Review and preserve the current coherent slice.".to_string();
        preserve.acceptance_criteria = vec!["The workspace is intentional.".to_string()];
        let mut provider = fixture_action("provider:pull-request:14", "provider", "P2", 2, "open");
        provider.title = "Resolve pull request evidence".to_string();
        let mut maturity = fixture_action(
            "maturity:dimension:quality_commands",
            "maturity",
            "P2",
            2,
            "blocked",
        );
        maturity.title = "Improve quality command maturity".to_string();
        let verification = fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open");
        let verified_history = fixture_action(
            "product_truth:resolved",
            "product_truth",
            "P2",
            1,
            "verified",
        );
        let actions = vec![preserve, provider, maturity, verification, verified_history];
        let coverage = vec![
            RemediationCoverage {
                surface: "quality_evidence".to_string(),
                label: "Quality evidence".to_string(),
                status: "clear".to_string(),
                detail: "Required evidence is fresh.".to_string(),
                action_ids: Vec::new(),
            },
            RemediationCoverage {
                surface: "maturity".to_string(),
                label: "Repository maturity".to_string(),
                status: "blocked".to_string(),
                detail: "Score: 2.5/4.".to_string(),
                action_ids: vec!["maturity:dimension:quality_commands".to_string()],
            },
        ];
        let goal = goal_definition("active_maintained").expect("goal should be supported");

        let explanation = build_remediation_explanation(&goal, &actions, &coverage);

        assert_eq!(
            explanation
                .phases
                .iter()
                .map(|phase| phase.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "preserve_and_reconcile",
                "product_and_provider_truth",
                "quality_and_maturity",
                "verify_and_close"
            ]
        );
        assert_eq!(explanation.phases[0].steps.len(), 1);
        assert_eq!(
            explanation.phases[0].steps[0].title,
            "Preserve the dirty workspace"
        );
        assert_eq!(explanation.phases[2].status, "blocked");
        assert_eq!(
            explanation.summary,
            "4 ordered remediation phases remain across 4 active actions. Work from the first unresolved phase and verify each result before refreshing the queue."
        );
        assert!(!explanation
            .phases
            .iter()
            .flat_map(|phase| &phase.steps)
            .any(|step| step.action_id == "product_truth:resolved"));
        assert_eq!(explanation.healthy_surfaces.len(), 1);
        assert_eq!(explanation.healthy_surfaces[0].surface, "quality_evidence");
        assert!(explanation
            .closure_requirements
            .iter()
            .any(|requirement| requirement.contains("at least 3.0/4")));
        assert!(explanation
            .closure_requirements
            .iter()
            .any(|requirement| requirement.contains("configured maturity gates")));
        assert!(explanation.authority.contains("does not authorize Git"));
    }

    #[test]
    fn remediation_explanation_supports_more_than_four_phases_and_covers_every_action_once() {
        let actions = vec![
            fixture_action("preserve", "branch_hygiene", "P1", 2, "open"),
            fixture_action("provider", "provider", "P2", 2, "open"),
            fixture_action("quality", "maturity", "P1", 3, "blocked"),
            fixture_action("deployment", "deployment_validation", "P2", 2, "open"),
            fixture_action("approval", "approval_rollout", "P2", 2, "in_progress"),
            fixture_action("unknown", "future_domain", "P2", 1, "open"),
            fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open"),
            fixture_action("history", "future_history", "P3", 1, "verified"),
        ];
        let mut goal = goal_definition("active_maintained").expect("goal should be supported");
        goal.remediation_phases = vec![
            RemediationPhaseDefinition {
                id: "deployment_validation".to_string(),
                title: "Validate deployment".to_string(),
                summary: "Verify repository-specific deployment evidence.".to_string(),
                domains: vec!["deployment_validation".to_string()],
                completion_criterion: "Deployment evidence is current.".to_string(),
                after_phase_id: Some("quality_and_maturity".to_string()),
            },
            RemediationPhaseDefinition {
                id: "approval_rollout".to_string(),
                title: "Complete rollout approval".to_string(),
                summary: "Satisfy the repository-specific rollout approval.".to_string(),
                domains: vec!["approval_rollout".to_string()],
                completion_criterion: "Rollout approval is recorded.".to_string(),
                after_phase_id: Some("deployment_validation".to_string()),
            },
        ];

        let explanation = build_remediation_explanation(&goal, &actions, &[]);

        assert_eq!(
            explanation
                .phases
                .iter()
                .map(|phase| phase.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "preserve_and_reconcile",
                "product_and_provider_truth",
                "quality_and_maturity",
                "deployment_validation",
                "approval_rollout",
                "unclassified_remediation",
                "verify_and_close",
            ]
        );
        assert!(explanation
            .summary
            .starts_with("7 ordered remediation phases remain across 7 active actions."));
        let projected_action_ids = explanation
            .phases
            .iter()
            .flat_map(|phase| phase.steps.iter().map(|step| step.action_id.as_str()))
            .collect::<Vec<_>>();
        let active_action_ids = actions
            .iter()
            .filter(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"))
            .map(|action| action.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(projected_action_ids.len(), active_action_ids.len());
        for action_id in active_action_ids {
            assert_eq!(
                projected_action_ids
                    .iter()
                    .filter(|projected_id| **projected_id == action_id)
                    .count(),
                1,
                "active action {action_id} should appear exactly once"
            );
        }
        assert_eq!(
            explanation
                .phases
                .iter()
                .find(|phase| phase.id == "unclassified_remediation")
                .expect("unknown domains must stay visible")
                .steps[0]
                .action_id,
            "unknown"
        );
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
        assert!(plan.goal.maturity_policy.is_none());
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
    fn github_only_candidates_are_counted_with_a_terminal_task_label() {
        let mut run = empty_run();
        let remote_repositories = vec![
            RemoteRepositorySnapshot {
                id: "github:42".to_string(),
                provider: "github".to_string(),
                full_name: "example/kept-online".to_string(),
                name: "kept-online".to_string(),
                owner: "example".to_string(),
                html_url: "https://github.com/example/kept-online".to_string(),
                default_branch: Some("main".to_string()),
                archived: false,
                locality: GITHUB_ONLY_LOCALITY.to_string(),
                identity_id: "github:jakyeamos".to_string(),
                last_refreshed_at: "2026-08-04T12:00:00Z".to_string(),
                pull_requests: Vec::new(),
                releases: Vec::new(),
                ci_checks: Vec::new(),
                ci_branch: None,
                ci_commit: None,
            },
            RemoteRepositorySnapshot {
                id: "github:43".to_string(),
                provider: "github".to_string(),
                full_name: "example/local-match".to_string(),
                name: "local-match".to_string(),
                owner: "example".to_string(),
                html_url: "https://github.com/example/local-match".to_string(),
                default_branch: Some("main".to_string()),
                archived: false,
                locality: "Local and remote".to_string(),
                identity_id: "github:jakyeamos".to_string(),
                last_refreshed_at: "2026-08-04T12:00:00Z".to_string(),
                pull_requests: Vec::new(),
                releases: Vec::new(),
                ci_checks: Vec::new(),
                ci_branch: None,
                ci_commit: None,
            },
        ];

        sync_github_only_candidates(&mut run, &remote_repositories);

        assert_eq!(run.github_only_candidates.len(), 1);
        assert_eq!(
            run.github_only_candidates[0].full_name,
            "example/kept-online"
        );
        assert_eq!(run.github_only_candidates[0].label, GITHUB_ONLY_LOCALITY);
        assert_eq!(
            run.github_only_candidates[0].last_remediation_task,
            GITHUB_ONLY_REMEDIATION_TASK
        );
        assert_eq!(run.github_only_candidates[0].status, "candidate");
    }

    #[test]
    fn github_only_goal_ends_the_local_plan_with_the_github_only_task() {
        let mut repository = fixture_repository("kept-online");
        repository.lifecycle = GITHUB_ONLY_LOCALITY.to_string();
        repository.lifecycle_candidate = GITHUB_ONLY_LOCALITY.to_string();

        let plan = build_plan(
            &repository,
            None,
            Some("refresh-github-only"),
            "2026-08-04T12:00:00Z",
            None,
        );

        assert_eq!(plan.goal.target_state, "github_only");
        assert_eq!(
            plan.actions.last().map(|action| action.title.as_str()),
            Some(GITHUB_ONLY_REMEDIATION_TASK)
        );
        assert_eq!(
            plan.actions.last().map(|action| action.stable_key.as_str()),
            Some(GITHUB_ONLY_VERIFICATION_ACTION_KEY)
        );
        assert_eq!(
            plan.actions.last().map(|action| action.domain.as_str()),
            Some("verification")
        );
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
    fn markdown_export_separates_active_queue_from_resolved_action_history() {
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
        assert!(markdown.contains("| Remaining path |"));
        assert!(markdown.contains("Preserve and reconcile repository work"));
        assert!(markdown.contains("## Resolved action history"));
        assert!(markdown.contains("| Resolved at |"));
        assert!(markdown
            .contains("| `closed-repo` | active_maintained | repository_contract | verified |"));
        assert!(markdown.contains("GitHub-only candidates: **0**"));
        assert!(markdown.contains("## GitHub-only candidates"));
        assert!(!markdown.contains("## Closure ledger"));
        assert!(!markdown.contains("| 2 | `closed-repo` |"));
    }

    #[test]
    fn remediation_export_writes_markdown_and_resolved_action_history_data() {
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
    fn progress_reserves_one_hundred_percent_for_terminal_plans() {
        let actions = vec![
            fixture_action("verified", "qr_findings", "P1", 199, "verified"),
            fixture_action("remaining", "verification", "P2", 1, "open"),
        ];

        let progress = calculate_progress(&actions);

        assert_eq!(progress.verified_weight, 199);
        assert_eq!(progress.total_weight, 200);
        assert_eq!(progress.percentage, 99.0);
        assert_eq!(
            calculate_progress(&[fixture_action(
                "verified-only",
                "verification",
                "P2",
                1,
                "verified",
            )])
            .percentage,
            100.0
        );
    }

    #[test]
    fn refresh_retains_disappeared_actions_as_verified_progress() {
        let previous = fixture_plan(
            "retained-progress",
            "open",
            vec![
                fixture_action("resolved", "product_truth", "P1", 3, "open"),
                fixture_action("remaining", "maturity", "P2", 2, "open"),
                fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open"),
            ],
        );
        let mut actions = vec![
            fixture_action("remaining", "maturity", "P2", 2, "open"),
            fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open"),
        ];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-07-30T12:00:00Z");

        let resolved = actions
            .iter()
            .find(|action| action.stable_key == "resolved")
            .expect("the disappeared action should remain in the plan");
        assert_eq!(resolved.status, "verified");
        assert_eq!(
            resolved.completed_at.as_deref(),
            Some("2026-07-30T12:00:00Z")
        );
        assert!(action_was_resolved_by_refresh(resolved));
        assert_eq!(
            actions
                .iter()
                .filter(|action| action.stable_key == VERIFICATION_ACTION_KEY)
                .count(),
            1
        );
        let progress = calculate_progress(&actions);
        assert_eq!(progress.verified_weight, 3);
        assert_eq!(progress.total_weight, 6);
        assert_eq!(progress.percentage, 50.0);
    }

    #[test]
    fn grouped_project_compass_action_replaces_legacy_duplicate_actions() {
        let previous = fixture_plan(
            "compass-migration",
            "blocked",
            vec![
                fixture_action(
                    LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS[0],
                    "product_truth",
                    "P1",
                    2,
                    "open",
                ),
                fixture_action(
                    LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS[1],
                    "product_truth",
                    "P1",
                    2,
                    "open",
                ),
            ],
        );
        let mut actions = vec![fixture_action(
            PROJECT_COMPASS_OPEN_ITEMS_KEY,
            "product_truth",
            "P1",
            severity_weight("warning"),
            "open",
        )];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-08-03T18:04:36Z");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].stable_key, PROJECT_COMPASS_OPEN_ITEMS_KEY);
        assert!(!actions.iter().any(|action| {
            LEGACY_PROJECT_COMPASS_OPEN_ITEM_KEYS.contains(&action.stable_key.as_str())
        }));
    }

    #[test]
    fn debloat_group_migration_preserves_state_without_legacy_history_churn() {
        let repository = fixture_repository("debloat-key-migration");
        let legacy_key = format!("{LEGACY_DEBLOAT_GROUP_KEY_PREFIX}unknown");
        let current_key = format!("{DEBLOAT_GROUP_KEY_PREFIX}unknown");
        let mut legacy_action = fixture_action(
            &legacy_key,
            "qr_findings",
            "P2",
            severity_weight("warning"),
            "in_progress",
        );
        legacy_action.notes = Some("Owner review has started.".to_string());
        let previous = fixture_plan("debloat-key-migration", "open", vec![legacy_action.clone()]);
        let previous_actions = HashMap::from([(legacy_action.stable_key.as_str(), &legacy_action)]);
        let seed = ActionSeed {
            stable_key: current_key.clone(),
            domain: "qr_findings".to_string(),
            title: "Review oversized source files".to_string(),
            summary: "Review the current debloat candidates.".to_string(),
            severity: "warning".to_string(),
            priority: "P2".to_string(),
            weight: severity_weight("warning"),
            acceptance_criteria: Vec::new(),
            evidence: Vec::new(),
            related_finding_ids: vec!["finding-debloat".to_string()],
            source_run_id: Some("qr-run".to_string()),
        };
        let mut actions = vec![materialize_action(
            &repository,
            seed,
            &previous_actions,
            "2026-08-04T03:00:00Z",
        )];

        assert_eq!(actions[0].stable_key, current_key);
        assert_eq!(actions[0].status, "in_progress");
        assert_eq!(
            actions[0].notes.as_deref(),
            Some("Owner review has started.")
        );

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-08-04T03:00:00Z");

        assert_eq!(actions.len(), 1);
        assert!(!actions.iter().any(|action| action.stable_key == legacy_key));
    }

    #[test]
    fn maturity_actions_replace_legacy_fleet_maturity_qr_history() {
        let legacy_key = format!(
            "qr_findings:group:quality_commands|quality_commands|{FLEET_MATURITY_FINDING_PACK_PREFIX}v0.1"
        );
        let previous = fixture_plan(
            "fleet-maturity-migration",
            "blocked",
            vec![fixture_action(&legacy_key, "qr_findings", "P2", 2, "open")],
        );
        let mut actions = vec![fixture_action(
            "maturity:dimension:quality_commands",
            "maturity",
            "P2",
            2,
            "open",
        )];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-08-03T18:04:36Z");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].stable_key, "maturity:dimension:quality_commands");
        assert!(!actions.iter().any(|action| action.stable_key == legacy_key));
    }

    #[test]
    fn refresh_normalizes_legacy_unbounded_qr_history_weight() {
        let mut legacy = fixture_action(
            "qr_findings:group:developer-experience:setup-path",
            "qr_findings",
            "P3",
            14_831,
            "open",
        );
        legacy.severity = "observation".to_string();
        let previous = fixture_plan("legacy-qr-history", "open", vec![legacy]);
        let mut actions = vec![fixture_action(
            VERIFICATION_ACTION_KEY,
            "verification",
            "P2",
            1,
            "open",
        )];

        retain_resolved_action_history(&mut actions, Some(&previous), "2026-07-30T12:00:00Z");

        let retained = actions
            .iter()
            .find(|action| action.stable_key.starts_with("qr_findings:group:"))
            .expect("the grouped finding should remain as history");
        assert_eq!(retained.status, "verified");
        assert_eq!(retained.weight, severity_weight("observation"));
        assert_eq!(calculate_progress(&actions).percentage, 50.0);
    }

    #[test]
    fn action_reopens_when_a_refresh_resolved_key_reappears() {
        let repository = fixture_repository("recurrence");
        let mut previous_action = fixture_action(
            "project_compass:blockers",
            "product_truth",
            "P1",
            3,
            "verified",
        );
        previous_action.evidence.push(evidence(
            "Pronto remediation",
            RESOLVED_BY_REFRESH_LABEL,
            "Resolved",
            "Fresh",
            Some("2026-07-30T12:00:00Z"),
            None,
            "Previously absent from the refreshed projection.",
        ));
        let previous_actions =
            HashMap::from([(previous_action.stable_key.as_str(), &previous_action)]);
        let seed = ActionSeed {
            stable_key: previous_action.stable_key.clone(),
            domain: previous_action.domain.clone(),
            title: previous_action.title.clone(),
            summary: previous_action.summary.clone(),
            severity: previous_action.severity.clone(),
            priority: previous_action.priority.clone(),
            weight: previous_action.weight,
            acceptance_criteria: previous_action.acceptance_criteria.clone(),
            evidence: vec![evidence(
                "Project Compass",
                "Open blockers",
                "Blocked",
                "Fresh",
                Some("2026-07-31T12:00:00Z"),
                None,
                "The blocker is present again.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        };

        let action =
            materialize_action(&repository, seed, &previous_actions, "2026-07-31T12:00:00Z");

        assert_eq!(action.status, "open");
    }

    #[test]
    fn integration_only_requires_no_other_active_or_blocked_remediation() {
        let integration = fixture_action(
            "branch_hygiene:integrate:feature",
            "branch_hygiene",
            "P2",
            2,
            "open",
        );
        let verification = fixture_action(VERIFICATION_ACTION_KEY, "verification", "P2", 1, "open");
        assert!(integration_only_remaining(&[
            integration.clone(),
            verification.clone(),
        ]));

        let maturity = fixture_action("maturity:minimum", "maturity", "P2", 2, "open");
        assert!(!integration_only_remaining(&[
            integration.clone(),
            verification.clone(),
            maturity,
        ]));

        let mut blocked_integration = integration;
        blocked_integration.status = "blocked".to_string();
        assert!(!integration_only_remaining(&[
            blocked_integration,
            verification,
        ]));
    }

    #[test]
    fn severity_weights_are_deterministic() {
        assert_eq!(severity_weight("critical"), 4);
        assert_eq!(severity_weight("error"), 3);
        assert_eq!(severity_weight("warning"), 2);
        assert_eq!(severity_weight("observation"), 1);
    }

    #[test]
    fn fleet_qr_artifact_supplies_findings_but_not_private_maturity_to_plan() {
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
                    },
                    {
                        "applicable": true,
                        "dimension": "context_routing",
                        "finding_id": "finding-context-routing",
                        "score": 3,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "validated"
                    },
                    {
                        "applicable": true,
                        "dimension": "change_surface_coverage",
                        "finding_id": "finding-change-surface-coverage",
                        "score": 4,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "maintained"
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
        assert!(plan.actions.iter().any(|action| {
            action.stable_key == "maturity:score"
                && action
                    .summary
                    .contains("No repository maturity score is available")
        }));
        assert!(!plan
            .actions
            .iter()
            .any(|action| action.stable_key.starts_with("maturity:dimension:")));
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

    #[test]
    fn canonical_maturity_projection_owns_below_target_fleet_dimension_action() {
        let fixture_id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-remediation-fleet-maturity-owner-{fixture_id}"
        ));
        let repository_path = root.join("repo");
        let findings_dir = root.join("findings");
        fs::create_dir_all(&repository_path).expect("repository fixture should be writable");
        fs::create_dir_all(&findings_dir).expect("findings fixture should be writable");
        let observed_at = Utc::now().to_rfc3339();
        let mut repository = fixture_repository("fleet-maturity-owner");
        repository.path = repository_path.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();
        repository.workspace.last_commit = Some("abc".to_string());
        repository.quality.maturity.score = Some(2.5);
        repository.quality.maturity.score_display = Some("2.5/4".to_string());
        repository.quality.maturity.freshness = crate::quality::QualityFreshness::Fresh;
        repository.quality.maturity.observed_at = Some(observed_at.clone());
        repository
            .quality
            .maturity
            .dimension_scores
            .insert("quality_commands".to_string(), 2.0);
        repository
            .quality
            .maturity
            .dimension_scores
            .insert("matrix_maintenance".to_string(), 0.0);
        fs::write(
            findings_dir.join("repo.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-fleet-maturity-owner",
                "as_of": observed_at,
                "repository": {
                    "primary_path": repository.path.clone(),
                    "checkouts": [{"path": repository.workspace.path.clone(), "head": "abc", "branch": "dev"}]
                },
                "findings": [{
                    "applicable": true,
                    "dimension": "quality_commands",
                    "finding_id": "finding-quality-commands",
                    "label": "quality commands",
                    "message": "Quality commands are not fully legible.",
                    "score": 2,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation"
                }, {
                    "applicable": true,
                    "dimension": "matrix_maintenance",
                    "finding_id": "finding-matrix-maintenance",
                    "label": "change-matrix maintenance",
                    "message": "The repository matrix does not require same-change updates.",
                    "score": 0,
                    "schema": "quality-runner-environment-legibility-finding-v0.1",
                    "severity": "observation",
                    "status": "missing"
                }]
            }))
            .expect("fleet finding should encode"),
        )
        .expect("fleet finding should be writable");

        let run = rebuild_run_with_fleet_root(
            &[repository],
            &empty_run(),
            Some("refresh-fleet-maturity-owner"),
            Some(&root),
        );
        let plan = &run.plans[0];

        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.domain == "qr_findings")
                .count(),
            0
        );
        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.stable_key == "maturity:dimension:quality_commands")
                .count(),
            1
        );
        assert_eq!(
            plan.actions
                .iter()
                .filter(|action| action.stable_key == "maturity:dimension:matrix_maintenance")
                .count(),
            1
        );
        fs::remove_dir_all(root).expect("fleet fixture should be removable");
    }
}
