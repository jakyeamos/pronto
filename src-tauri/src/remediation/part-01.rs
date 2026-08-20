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
