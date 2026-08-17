use crate::behavior_assurance::{
    BehaviorAssurancePortfolioState, BehaviorAssuranceRepositoryState,
};
use crate::core::{CheckSnapshot, RemoteRepositorySnapshot, RepositorySnapshot};
use crate::evidence_contract::{EvidenceContractFleetCoverage, EvidenceContractRepositoryStatus};
use crate::installed_runtime::{self, InstalledRuntimeSnapshot};
use crate::mac_control_maturity::{MacControlPortfolioSnapshot, MacControlRepositoryState};
use crate::release_boundary::{self, ReleaseBoundarySnapshot};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration as StdDuration, Instant};

pub const MAX_EVIDENCE_AGE_DAYS: i64 = 7;
pub const FINDING_DISPOSITIONS_SCHEMA: &str = "pronto-quality-finding-dispositions/v1";
pub const FINDING_DISPOSITIONS_RELATIVE_PATH: &str = ".pronto/quality-finding-dispositions.json";
pub const CANONICAL_MATURITY_FEED_RELATIVE_PATH: &str =
    ".quality-runner/fleet-audit/current/maturity.json";
const MATURITY_FEED_SCHEMAS: [&str; 2] = [
    "quality-runner-maturity-feed/v1",
    "quality-runner-maturity-feed/v2",
];
pub(crate) const FLEET_MATURITY_FINDING_SCHEMA_PREFIX: &str =
    "quality-runner-environment-legibility-finding-";
const MATURITY_FEED_STATUS: [&str; 2] = ["completed", "complete_with_blockers"];
const MAX_MATURITY_GAPS: usize = 64;
const QUALITY_GIT_TIMEOUT: StdDuration = StdDuration::from_secs(2);
const MATURITY_FEED_FORBIDDEN_KEYS: [&str; 15] = [
    "prompt",
    "prompts",
    "raw_prompt",
    "raw_prompts",
    "code",
    "diff",
    "diffs",
    "raw_code",
    "raw_diff",
    "raw_diffs",
    "transcript",
    "transcripts",
    "raw_transcript",
    "raw_transcripts",
    "credential",
];

const CANONICAL_GATE_DEFINITIONS: [(&str, &str); 8] = [
    ("build", "Build"),
    ("runtime_smoke", "Smoke"),
    ("tests", "Tests"),
    ("lint", "Lint"),
    ("formatter", "Formatter"),
    ("typecheck", "Typecheck"),
    ("dead_code", "Dead-code"),
    ("secrets_scan", "Secrets scan"),
];

const CONDITIONAL_GATE_DEFINITIONS: [(&str, &str); 3] = [
    ("dependency_audit", "Dependency audit"),
    ("debloat", "Repository debloat review"),
    ("web_readiness", "Web readiness"),
];
const CI_READINESS_BASELINE_GATE_IDS: [&str; 6] = [
    "build",
    "tests",
    "lint",
    "formatter",
    "typecheck",
    "secrets_scan",
];
const CI_READINESS_CONDITIONAL_GATE_IDS: [&str; 4] = [
    "runtime_smoke",
    "dead_code",
    "dependency_audit",
    "web_readiness",
];
const WEB_READINESS_SCHEMA: &str = "quality-runner-web-readiness/v1";
const WEB_READINESS_RELATIVE_PATH: &str = ".quality-runner/web-readiness.json";
const RECOMMENDATION_MATRIX_MARKDOWN: &str =
    include_str!("../../docs/quality-gate-recommendation-matrix.md");
const RECOMMENDATION_MATRIX_GATE_COLUMNS: [(&str, usize); 9] = [
    ("build", 2),
    ("tests", 3),
    ("runtime_smoke", 4),
    ("lint", 5),
    ("formatter", 6),
    ("typecheck", 7),
    ("dead_code", 8),
    ("secrets_scan", 9),
    ("dependency_audit", 10),
];

fn default_disposition_status() -> String {
    "Unavailable".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityGateStatus {
    Passed,
    Failed,
    Blocked,
    #[serde(rename = "Not configured")]
    NotConfigured,
}

impl Default for QualityGateStatus {
    fn default() -> Self {
        Self::NotConfigured
    }
}

impl QualityGateStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::Failed => "Failed",
            Self::Blocked => "Blocked",
            Self::NotConfigured => "Not configured",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualitySource {
    #[serde(rename = "CI")]
    Ci,
    Local,
    #[serde(rename = "QR")]
    Qr,
}

impl QualitySource {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ci" | "github" | "github checks" => Some(Self::Ci),
            "local" | "command" | "local command" => Some(Self::Local),
            "qr" | "quality runner" | "quality-runner" | "report" => Some(Self::Qr),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ci => "CI",
            Self::Local => "Local",
            Self::Qr => "QR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityFreshness {
    Fresh,
    Stale,
    Unknown,
    Conflicted,
}

impl Default for QualityFreshness {
    fn default() -> Self {
        Self::Unknown
    }
}

impl QualityFreshness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fresh => "Fresh",
            Self::Stale => "Stale",
            Self::Unknown => "Unknown",
            Self::Conflicted => "Conflicted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateRequirement {
    pub gate_id: String,
    pub source: QualitySource,
    #[serde(default)]
    pub minimum_verification_level: Option<QualityVerificationLevel>,
    #[serde(default)]
    pub policy: QualityRequirementPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityVerificationLevel {
    #[default]
    Unknown,
    SourceInferred,
    ArtifactInspected,
    BrowserRendered,
    DeploymentVerified,
}

impl QualityVerificationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::SourceInferred => "source_inferred",
            Self::ArtifactInspected => "artifact_inspected",
            Self::BrowserRendered => "browser_rendered",
            Self::DeploymentVerified => "deployment_verified",
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::SourceInferred => 1,
            Self::ArtifactInspected => 2,
            Self::BrowserRendered => 3,
            Self::DeploymentVerified => 4,
        }
    }

    fn satisfies(&self, minimum: &Self) -> bool {
        self.rank() >= minimum.rank()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityRequirementPolicy {
    #[default]
    Block,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityEvidence {
    pub id: String,
    pub source: QualitySource,
    pub status: QualityGateStatus,
    pub freshness: QualityFreshness,
    pub observed_at: Option<String>,
    pub scanned_commit: Option<String>,
    pub scanned_branch: Option<String>,
    pub command: Option<String>,
    pub source_label: String,
    pub report_path: Option<String>,
    pub report_url: Option<String>,
    pub report_kind: Option<String>,
    pub detail: String,
    #[serde(default)]
    pub verification_level: QualityVerificationLevel,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub target_url: Option<String>,
    #[serde(default)]
    pub target_provider: Option<String>,
    #[serde(default)]
    pub deployment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebReadinessCheck {
    pub id: String,
    pub label: String,
    pub category: String,
    pub policy: String,
    pub status: String,
    pub verification_level: QualityVerificationLevel,
    pub detail: String,
    #[serde(default)]
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebReadinessTarget {
    pub kind: String,
    pub commit: Option<String>,
    pub url: Option<String>,
    pub provider: Option<String>,
    pub deployment_id: Option<String>,
    pub artifact_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebReadinessSnapshot {
    pub status: String,
    pub applicability: String,
    pub applicability_reason: Option<String>,
    pub freshness: QualityFreshness,
    pub observed_at: Option<String>,
    pub scanned_commit: Option<String>,
    pub scanned_branch: Option<String>,
    pub report_path: Option<String>,
    pub target: WebReadinessTarget,
    pub checks: Vec<WebReadinessCheck>,
    pub passed_count: u64,
    pub failed_count: u64,
    pub blocked_count: u64,
    pub unknown_count: u64,
    pub warning_count: u64,
}

impl Default for WebReadinessSnapshot {
    fn default() -> Self {
        Self {
            status: "Unknown".to_string(),
            applicability: "unknown".to_string(),
            applicability_reason: None,
            freshness: QualityFreshness::Unknown,
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
            report_path: None,
            target: WebReadinessTarget::default(),
            checks: Vec::new(),
            passed_count: 0,
            failed_count: 0,
            blocked_count: 0,
            unknown_count: 0,
            warning_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    pub id: String,
    pub label: String,
    pub status: QualityGateStatus,
    pub freshness: QualityFreshness,
    pub evidence: Vec<QualityEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFindings {
    pub total: u64,
    /// Compatibility count for the complete detector report. New consumers
    /// should use the explicit detector fields below instead of treating this
    /// value as a generic quality or maturity count.
    #[serde(default)]
    pub detector_findings_total: u64,
    #[serde(default)]
    pub category_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub actionable_category_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub actionable_total: u64,
    #[serde(default)]
    pub detector_actionable_total: u64,
    #[serde(default)]
    pub reviewed_total: u64,
    #[serde(default)]
    pub unreviewed_total: u64,
    #[serde(default)]
    pub detector_unreviewed_total: u64,
    #[serde(default)]
    pub disposition_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub stale_disposition_total: u64,
    #[serde(default = "default_disposition_status")]
    pub disposition_status: String,
    #[serde(default)]
    pub disposition_contract_path: Option<String>,
    #[serde(default)]
    pub disposition_message: Option<String>,
    pub severity_counts: BTreeMap<String, u64>,
    pub high_severity_total: u64,
    pub source: Option<QualitySource>,
    pub observed_at: Option<String>,
    pub scanned_commit: Option<String>,
    pub scanned_branch: Option<String>,
    pub freshness: QualityFreshness,
    pub report_path: Option<String>,
    #[serde(default)]
    pub report_paths: Vec<String>,
    #[serde(default)]
    pub enabled_detector_count: u64,
    #[serde(default)]
    pub enabled_rule_count: u64,
    #[serde(default)]
    pub producer_versions: BTreeMap<String, String>,
    #[serde(default)]
    pub producer_source_shas: BTreeMap<String, String>,
    #[serde(default)]
    pub ruleset_fingerprints: BTreeMap<String, String>,
    #[serde(default)]
    pub configuration_fingerprints: BTreeMap<String, String>,
    #[serde(default)]
    pub qr_version: Option<String>,
    #[serde(default)]
    pub target_sha: Option<String>,
    #[serde(default)]
    pub refresh_time: Option<String>,
    #[serde(default)]
    pub delta_total: Option<i64>,
    #[serde(default)]
    pub refresh_required: bool,
    #[serde(default)]
    pub refresh_required_reason: Option<String>,
    #[serde(default)]
    pub detector_status: Option<String>,
}

impl Default for QualityFindings {
    fn default() -> Self {
        Self {
            total: 0,
            detector_findings_total: 0,
            category_counts: BTreeMap::new(),
            actionable_category_counts: BTreeMap::new(),
            actionable_total: 0,
            detector_actionable_total: 0,
            reviewed_total: 0,
            unreviewed_total: 0,
            detector_unreviewed_total: 0,
            disposition_counts: BTreeMap::new(),
            stale_disposition_total: 0,
            disposition_status: default_disposition_status(),
            disposition_contract_path: None,
            disposition_message: None,
            severity_counts: BTreeMap::new(),
            high_severity_total: 0,
            source: None,
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
            freshness: QualityFreshness::Unknown,
            report_path: None,
            report_paths: Vec::new(),
            enabled_detector_count: 0,
            enabled_rule_count: 0,
            producer_versions: BTreeMap::new(),
            producer_source_shas: BTreeMap::new(),
            ruleset_fingerprints: BTreeMap::new(),
            configuration_fingerprints: BTreeMap::new(),
            qr_version: None,
            target_sha: None,
            refresh_time: None,
            delta_total: None,
            refresh_required: false,
            refresh_required_reason: None,
            detector_status: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFindingDisposition {
    pub fingerprint: String,
    pub status: String,
    pub reason: String,
    pub reviewer: String,
    pub reviewed_at: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFindingDispositionsContract {
    pub schema_version: String,
    pub updated_at: String,
    #[serde(default)]
    pub dispositions: Vec<QualityFindingDisposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMaturityGap {
    pub dimension: String,
    pub status: String,
    pub score: Option<f64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RepositoryMaturityPillar {
    pub id: String,
    pub label: String,
    pub weight: f64,
    pub applicability: String,
    pub status: String,
    pub score: Option<f64>,
    pub dimension_scores: BTreeMap<String, f64>,
    pub missing_capabilities: Vec<String>,
    pub critical_dimensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RepositoryMaturityEvidence {
    pub applicable_pillar_count: u64,
    pub assessed_pillar_count: u64,
    #[serde(default)]
    pub applicable_dimension_count: u64,
    #[serde(default)]
    pub assessed_dimension_count: u64,
    pub applicable_weight: f64,
    pub assessed_weight: f64,
    pub evidence_coverage: f64,
    pub fresh_evidence_coverage: f64,
    pub unknown_applicability: Vec<String>,
    pub unmapped_dimensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RepositoryMaturityCriticalCap {
    pub applied: bool,
    pub maximum_score: Option<f64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RepositoryMaturityModel {
    pub schema: String,
    pub score: Option<f64>,
    pub uncapped_score: Option<f64>,
    pub status: String,
    pub pillars: Vec<RepositoryMaturityPillar>,
    pub evidence: RepositoryMaturityEvidence,
    pub critical_cap: RepositoryMaturityCriticalCap,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PortfolioMaturityPillar {
    pub id: String,
    pub label: String,
    pub score: Option<f64>,
    pub assessed_repository_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRepositoryOutcome {
    pub state: String,
    pub label: String,
    #[serde(default)]
    pub disposition: Option<String>,
    #[serde(default)]
    pub next_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentUsabilityLane {
    pub id: String,
    pub label: String,
    pub applicable: bool,
    pub score: Option<f64>,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AgentUsabilityGrowthHealth {
    pub status: String,
    pub score: Option<f64>,
    pub message: String,
    pub document_count: u64,
    pub agent_document_count: u64,
    pub routed_agent_document_count: u64,
    pub unrouted_agent_document_count: u64,
    pub oversized_document_count: u64,
    pub skill_count: u64,
    pub family_count: u64,
    pub largest_family_size: u64,
    pub unclassified_skill_count: u64,
    pub oversized_skill_count: u64,
    pub tool_count: u64,
    pub documented_tool_count: u64,
    pub skill_covered_tool_count: u64,
    pub behavior_declared_tool_count: u64,
    pub behavior_verified_tool_count: u64,
    pub inventory_truncated: bool,
}

impl Default for AgentUsabilityGrowthHealth {
    fn default() -> Self {
        Self {
            status: "unavailable".to_string(),
            score: None,
            message: "Agent-usability growth evidence is unavailable.".to_string(),
            document_count: 0,
            agent_document_count: 0,
            routed_agent_document_count: 0,
            unrouted_agent_document_count: 0,
            oversized_document_count: 0,
            skill_count: 0,
            family_count: 0,
            largest_family_size: 0,
            unclassified_skill_count: 0,
            oversized_skill_count: 0,
            tool_count: 0,
            documented_tool_count: 0,
            skill_covered_tool_count: 0,
            behavior_declared_tool_count: 0,
            behavior_verified_tool_count: 0,
            inventory_truncated: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AgentUsabilityMaturity {
    pub schema: String,
    pub status: String,
    pub applicability: String,
    pub manifest_status: String,
    pub manifest_path: String,
    pub applicable_lane_count: u64,
    pub covered_lane_count: u64,
    pub lanes: Vec<AgentUsabilityLane>,
    pub growth_health: AgentUsabilityGrowthHealth,
}

impl Default for AgentUsabilityMaturity {
    fn default() -> Self {
        Self {
            schema: "quality-runner-agent-usability/v1".to_string(),
            status: "unavailable".to_string(),
            applicability: "applicable".to_string(),
            manifest_status: "missing".to_string(),
            manifest_path: ".agents/agent-usability.json".to_string(),
            applicable_lane_count: 0,
            covered_lane_count: 0,
            lanes: Vec::new(),
            growth_health: AgentUsabilityGrowthHealth::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMaturity {
    pub score: Option<f64>,
    pub score_display: Option<String>,
    pub scored_dimension_count: Option<u64>,
    #[serde(default)]
    pub dimension_scores: BTreeMap<String, f64>,
    #[serde(default)]
    pub gaps: Vec<QualityMaturityGap>,
    #[serde(default)]
    pub quality_outcome: Option<QualityRepositoryOutcome>,
    #[serde(default)]
    pub agent_usability: Option<AgentUsabilityMaturity>,
    #[serde(default)]
    pub repository_maturity: Option<RepositoryMaturityModel>,
    pub audit_id: Option<String>,
    pub observed_at: Option<String>,
    #[serde(default)]
    pub scanned_commit: Option<String>,
    #[serde(default)]
    pub scanned_branch: Option<String>,
    pub freshness: QualityFreshness,
    pub report_path: Option<String>,
}

impl Default for QualityMaturity {
    fn default() -> Self {
        Self {
            score: None,
            score_display: None,
            scored_dimension_count: None,
            dimension_scores: BTreeMap::new(),
            gaps: Vec::new(),
            quality_outcome: None,
            agent_usability: None,
            repository_maturity: None,
            audit_id: None,
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
            freshness: QualityFreshness::Unknown,
            report_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QualityReadiness {
    pub score: Option<f64>,
    pub score_display: Option<String>,
    #[serde(default)]
    pub evidence_coverage_score: Option<f64>,
    #[serde(default)]
    pub evidence_coverage_score_display: Option<String>,
    #[serde(default)]
    pub configuration_score: Option<f64>,
    #[serde(default)]
    pub configuration_score_display: Option<String>,
    pub applicable_gate_ids: Vec<String>,
    #[serde(default)]
    pub configured_gate_ids: Vec<String>,
    #[serde(default)]
    pub unconfigured_gate_ids: Vec<String>,
    pub covered_gate_ids: Vec<String>,
    #[serde(default)]
    pub fresh_passing_gate_ids: Vec<String>,
    pub missing_gate_ids: Vec<String>,
    pub stale_gate_ids: Vec<String>,
    pub failed_gate_ids: Vec<String>,
    pub blocked_gate_ids: Vec<String>,
}

impl Default for QualityReadiness {
    fn default() -> Self {
        Self {
            score: None,
            score_display: None,
            evidence_coverage_score: None,
            evidence_coverage_score_display: None,
            configuration_score: None,
            configuration_score_display: None,
            applicable_gate_ids: Vec::new(),
            configured_gate_ids: Vec::new(),
            unconfigured_gate_ids: Vec::new(),
            covered_gate_ids: Vec::new(),
            fresh_passing_gate_ids: Vec::new(),
            missing_gate_ids: Vec::new(),
            stale_gate_ids: Vec::new(),
            failed_gate_ids: Vec::new(),
            blocked_gate_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySnapshot {
    pub gates: Vec<QualityGate>,
    pub findings: QualityFindings,
    pub maturity: QualityMaturity,
    #[serde(default)]
    pub target_fleet_audit_root: Option<String>,
    #[serde(default)]
    pub ci_readiness: QualityReadiness,
    #[serde(default)]
    pub mac_control_ideal_state: MacControlRepositoryState,
    #[serde(default)]
    pub behavior_assurance: BehaviorAssuranceRepositoryState,
    #[serde(default)]
    pub evidence_contracts: Vec<EvidenceContractRepositoryStatus>,
    #[serde(default)]
    pub web_readiness: WebReadinessSnapshot,
    #[serde(default)]
    pub release_boundary: ReleaseBoundarySnapshot,
    #[serde(default)]
    pub installed_runtime: InstalledRuntimeSnapshot,
    pub last_ingested_at: Option<String>,
    pub ingestion_status: String,
    pub ingestion_message: Option<String>,
}

impl Default for QualitySnapshot {
    fn default() -> Self {
        Self {
            gates: default_quality_gates(),
            findings: QualityFindings::default(),
            maturity: QualityMaturity::default(),
            target_fleet_audit_root: None,
            ci_readiness: QualityReadiness::default(),
            mac_control_ideal_state: MacControlRepositoryState::default(),
            behavior_assurance: BehaviorAssuranceRepositoryState::default(),
            evidence_contracts: Vec::new(),
            web_readiness: WebReadinessSnapshot::default(),
            release_boundary: ReleaseBoundarySnapshot::default(),
            installed_runtime: InstalledRuntimeSnapshot::default(),
            last_ingested_at: None,
            ingestion_status: "No evidence".to_string(),
            ingestion_message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMeasurementConfidence {
    pub level: String,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub population_status: String,
    pub expected_repository_count: u64,
    pub observed_repository_count: u64,
    pub excluded_repository_count: u64,
    pub unresolved_measurement_gap_count: u64,
    pub deterministic_replay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityOutcomeDefinition {
    pub label: String,
    pub meaning: String,
    #[serde(default)]
    pub next_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPortfolioSnapshot {
    pub audit_root: Option<String>,
    pub latest_audit_id: Option<String>,
    pub latest_audit_at: Option<String>,
    pub latest_audit_path: Option<String>,
    pub matched_repository_count: usize,
    pub maturity_score: Option<f64>,
    pub maturity_score_display: Option<String>,
    pub scored_dimension_count: Option<u64>,
    #[serde(default)]
    pub measurement_confidence: Option<QualityMeasurementConfidence>,
    #[serde(default)]
    pub source_maturity_score: Option<f64>,
    #[serde(default)]
    pub source_maturity_score_display: Option<String>,
    #[serde(default)]
    pub source_scored_dimension_count: Option<u64>,
    #[serde(default)]
    pub maturity_pillars: Vec<PortfolioMaturityPillar>,
    #[serde(default)]
    pub maturity_evidence_coverage: Option<f64>,
    #[serde(default)]
    pub maturity_fresh_evidence_coverage: Option<f64>,
    #[serde(default)]
    pub maturity_provisional_repository_count: usize,
    #[serde(default)]
    pub maturity_capped_repository_count: usize,
    pub audit_status: String,
    #[serde(default)]
    pub ci_readiness_score: Option<f64>,
    #[serde(default)]
    pub ci_readiness_score_display: Option<String>,
    #[serde(default)]
    pub ci_evidence_coverage_score: Option<f64>,
    #[serde(default)]
    pub ci_evidence_coverage_score_display: Option<String>,
    #[serde(default)]
    pub ci_configuration_score: Option<f64>,
    #[serde(default)]
    pub ci_configuration_score_display: Option<String>,
    #[serde(default)]
    pub ci_readiness_full_repository_count: usize,
    #[serde(default)]
    pub ci_readiness_repository_count: usize,
    #[serde(default)]
    pub ci_readiness_unscored_repository_count: usize,
    #[serde(default)]
    pub ci_readiness_open_gate_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub ci_evidence_fresh_passing_gate_count: usize,
    #[serde(default)]
    pub ci_evidence_ideal_gate_count: usize,
    #[serde(default)]
    pub ci_configuration_configured_gate_count: usize,
    #[serde(default)]
    pub ci_configuration_ideal_gate_count: usize,
    #[serde(default)]
    pub ci_configuration_full_repository_count: usize,
    #[serde(default)]
    pub ci_configuration_repository_count: usize,
    #[serde(default)]
    pub ci_configuration_unscored_repository_count: usize,
    #[serde(default)]
    pub feed_schema: Option<String>,
    #[serde(default)]
    pub provenance_hash: Option<String>,
    #[serde(default)]
    pub quality_outcome_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub quality_outcome_taxonomy: BTreeMap<String, QualityOutcomeDefinition>,
    #[serde(default)]
    pub mac_control_ideal_state: MacControlPortfolioSnapshot,
    #[serde(default)]
    pub behavior_assurance: BehaviorAssurancePortfolioState,
    #[serde(default)]
    pub evidence_contracts: Vec<EvidenceContractFleetCoverage>,
}

impl Default for QualityPortfolioSnapshot {
    fn default() -> Self {
        Self {
            audit_root: None,
            latest_audit_id: None,
            latest_audit_at: None,
            latest_audit_path: None,
            matched_repository_count: 0,
            maturity_score: None,
            maturity_score_display: None,
            scored_dimension_count: None,
            measurement_confidence: None,
            source_maturity_score: None,
            source_maturity_score_display: None,
            source_scored_dimension_count: None,
            maturity_pillars: Vec::new(),
            maturity_evidence_coverage: None,
            maturity_fresh_evidence_coverage: None,
            maturity_provisional_repository_count: 0,
            maturity_capped_repository_count: 0,
            audit_status: "Not configured".to_string(),
            ci_readiness_score: None,
            ci_readiness_score_display: None,
            ci_evidence_coverage_score: None,
            ci_evidence_coverage_score_display: None,
            ci_configuration_score: None,
            ci_configuration_score_display: None,
            ci_readiness_full_repository_count: 0,
            ci_readiness_repository_count: 0,
            ci_readiness_unscored_repository_count: 0,
            ci_readiness_open_gate_counts: BTreeMap::new(),
            ci_evidence_fresh_passing_gate_count: 0,
            ci_evidence_ideal_gate_count: 0,
            ci_configuration_configured_gate_count: 0,
            ci_configuration_ideal_gate_count: 0,
            ci_configuration_full_repository_count: 0,
            ci_configuration_repository_count: 0,
            ci_configuration_unscored_repository_count: 0,
            feed_schema: None,
            provenance_hash: None,
            quality_outcome_counts: BTreeMap::new(),
            quality_outcome_taxonomy: BTreeMap::new(),
            mac_control_ideal_state: MacControlPortfolioSnapshot::default(),
            behavior_assurance: BehaviorAssurancePortfolioState::default(),
            evidence_contracts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditImport {
    pub portfolio: QualityPortfolioSnapshot,
    pub maturities: HashMap<String, QualityMaturity>,
    pub behavior_assurance: HashMap<String, BehaviorAssuranceRepositoryState>,
}

#[derive(Debug, Clone)]
pub struct FleetAuditEvidence {
    pub maturity: QualityMaturity,
    pub findings: QualityFindings,
}

pub fn target_provenance_matches(
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
    target_branch: &str,
    target_commit: &str,
) -> bool {
    scanned_branch == Some(target_branch) && scanned_commit == Some(target_commit)
}

pub fn evaluate_target_freshness(
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
    target_branch: &str,
    target_commit: &str,
) -> QualityFreshness {
    if target_provenance_matches(scanned_branch, scanned_commit, target_branch, target_commit) {
        return QualityFreshness::Fresh;
    }
    if scanned_branch.is_none() || scanned_commit.is_none() {
        QualityFreshness::Unknown
    } else {
        QualityFreshness::Stale
    }
}

pub fn target_evidence_is_current(
    snapshot: &QualitySnapshot,
    target_branch: &str,
    target_commit: &str,
) -> bool {
    snapshot.gates.iter().any(|gate| {
        gate.evidence.iter().any(|evidence| {
            target_provenance_matches(
                evidence.scanned_branch.as_deref(),
                evidence.scanned_commit.as_deref(),
                target_branch,
                target_commit,
            )
        })
    }) || target_provenance_matches(
        snapshot.findings.scanned_branch.as_deref(),
        snapshot.findings.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    ) || target_provenance_matches(
        snapshot.maturity.scanned_branch.as_deref(),
        snapshot.maturity.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    )
}

pub fn scope_fleet_audit_evidence_to_target(
    evidence: &mut FleetAuditEvidence,
    target_branch: &str,
    target_commit: &str,
) {
    evidence.maturity.scanned_branch = Some(target_branch.to_string());
    evidence.maturity.scanned_commit = Some(target_commit.to_string());
    evidence.maturity.freshness = evaluate_target_freshness(
        evidence.maturity.scanned_branch.as_deref(),
        evidence.maturity.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
    evidence.findings.scanned_branch = Some(target_branch.to_string());
    evidence.findings.scanned_commit = Some(target_commit.to_string());
    evidence.findings.freshness = evaluate_target_freshness(
        evidence.findings.scanned_branch.as_deref(),
        evidence.findings.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
}

#[derive(Debug, Clone, Default)]
pub struct FleetAuditImport {
    pub audit_id: Option<String>,
    pub observed_at: Option<String>,
    pub evidence: HashMap<String, FleetAuditEvidence>,
}

pub fn canonical_maturity_feed_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(CANONICAL_MATURITY_FEED_RELATIVE_PATH))
}

pub fn is_stable_detector_report(report_path: Option<&str>) -> bool {
    report_path
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "code-quality-scan.json")
}

fn sync_detector_counts(findings: &mut QualityFindings) {
    findings.detector_findings_total = findings.total;
    findings.detector_actionable_total = findings.actionable_total;
    findings.detector_unreviewed_total = findings.unreviewed_total;
}

fn apply_detector_evidence(findings: &mut QualityFindings, payload: &Value) {
    let receipts = payload
        .get("detector_evidence")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            payload
                .get("detector")
                .is_some()
                .then(|| vec![payload.clone()])
        })
        .unwrap_or_default();
    if receipts.is_empty() {
        sync_detector_counts(findings);
        return;
    }

    let mut detectors = BTreeSet::new();
    let mut rules = BTreeSet::new();
    let mut statuses = BTreeSet::new();
    for receipt in receipts.iter().filter(|receipt| receipt.is_object()) {
        let detector = json_string_at(receipt, &["detector"]);
        let status = json_string_at(receipt, &["status"]);
        if let Some(status) = status.as_deref() {
            statuses.insert(status.to_string());
        }
        let applicable = receipt
            .get("applicable")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if applicable && status.as_deref() != Some("not_applicable") {
            if let Some(detector) = detector.as_deref() {
                detectors.insert(detector.to_string());
            }
            if let Some(enabled_rules) = receipt.get("enabled_rules").and_then(Value::as_array) {
                rules.extend(
                    enabled_rules
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string),
                );
            }
        }
        if let Some(detector) = detector.as_deref() {
            if let Some(version) = json_string_at(receipt, &["producer", "version"]) {
                findings
                    .producer_versions
                    .insert(detector.to_string(), version);
            }
            if let Some(source_sha) = json_string_at(receipt, &["producer", "source_sha"]) {
                findings
                    .producer_source_shas
                    .insert(detector.to_string(), source_sha);
            }
            if let Some(ruleset_hash) = json_string_at(receipt, &["ruleset_hash"]) {
                findings
                    .ruleset_fingerprints
                    .insert(detector.to_string(), ruleset_hash);
            }
            if let Some(configuration_hash) = json_string_at(receipt, &["configuration_hash"]) {
                findings
                    .configuration_fingerprints
                    .insert(detector.to_string(), configuration_hash);
            }
        }
        if findings.qr_version.is_none() {
            findings.qr_version = json_string_at(receipt, &["qr_version"]);
        }
        if findings.target_sha.is_none() {
            findings.target_sha = json_string_at(receipt, &["target_sha"]);
        }
        if let Some(scan_time) = json_string_at(receipt, &["scan_time"]) {
            if findings
                .refresh_time
                .as_deref()
                .is_none_or(|current| current < scan_time.as_str())
            {
                findings.refresh_time = Some(scan_time);
            }
        }
        if receipt
            .get("refresh_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || status.as_deref() == Some("blocked")
        {
            findings.refresh_required = true;
            if findings.refresh_required_reason.is_none() {
                findings.refresh_required_reason = json_string_at(receipt, &["reason"]);
            }
        }
    }

    findings.enabled_detector_count = detectors.len() as u64;
    findings.enabled_rule_count = rules.len() as u64;
    findings.detector_status = if statuses.contains("blocked") {
        Some("blocked".to_string())
    } else if statuses.contains("passed") {
        Some("passed".to_string())
    } else if statuses.contains("not_applicable") {
        Some("not_applicable".to_string())
    } else {
        None
    };
    if findings.target_sha.is_none() {
        findings.target_sha = findings.scanned_commit.clone();
    }
    sync_detector_counts(findings);
}

pub fn preserve_detector_evidence_on_refresh_failure(
    prior: &QualityFindings,
    current: &mut QualityFindings,
) {
    if !current.refresh_required
        || current.detector_status.as_deref() != Some("blocked")
        || prior.source.is_none()
    {
        return;
    }
    let mut preserved = prior.clone();
    preserved.refresh_required = true;
    preserved.refresh_required_reason = current
        .refresh_required_reason
        .clone()
        .or_else(|| Some("The latest detector refresh was blocked.".to_string()));
    preserved.detector_status = Some("blocked".to_string());
    preserved.refresh_time = current.refresh_time.clone().or(prior.refresh_time.clone());
    preserved.target_sha = current.target_sha.clone().or(prior.target_sha.clone());
    preserved.qr_version = current.qr_version.clone().or(prior.qr_version.clone());
    if current.report_path.is_some() && !is_stable_detector_report(current.report_path.as_deref()) {
        preserved.refresh_required_reason = preserved.refresh_required_reason.or_else(|| {
            Some(
                "The detector receipt is blocked; the prior valid scan is retained for review."
                    .to_string(),
            )
        });
    }
    *current = preserved;
}

pub fn update_detector_delta(prior: &QualityFindings, current: &mut QualityFindings) {
    current.delta_total = None;
    if current.refresh_required
        || prior.refresh_required
        || prior.source.is_none()
        || current.source.is_none()
        || prior.target_sha.is_none()
        || current.target_sha.is_none()
    {
        return;
    }
    let same_identity = prior.target_sha == current.target_sha
        && prior.qr_version == current.qr_version
        && prior.producer_versions == current.producer_versions
        && prior.producer_source_shas == current.producer_source_shas
        && prior.ruleset_fingerprints == current.ruleset_fingerprints
        && prior.configuration_fingerprints == current.configuration_fingerprints;
    if same_identity {
        current.delta_total =
            Some(current.detector_findings_total as i64 - prior.detector_findings_total as i64);
    }
}

pub fn fleet_audit_import(
    root: Option<&Path>,
    repositories: &[RepositorySnapshot],
) -> FleetAuditImport {
    let Some(root) = root else {
        return FleetAuditImport::default();
    };
    let summary = read_json(&root.join("summary.json"));
    let audit_id = summary
        .as_ref()
        .and_then(|value| json_string_at(value, &["audit_id"]));
    let observed_at = summary
        .as_ref()
        .and_then(|value| json_string_at(value, &["as_of"]));
    let Some(entries) = fs::read_dir(root.join("findings")).ok() else {
        return FleetAuditImport {
            audit_id,
            observed_at,
            evidence: HashMap::new(),
        };
    };
    let mut evidence = HashMap::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(payload) = read_json(&path) else {
            continue;
        };
        let repository_payload = payload.get("repository");
        let candidate_path = repository_payload
            .and_then(|value| json_string_at(value, &["primary_path"]))
            .or_else(|| {
                repository_payload
                    .and_then(|value| value.get("checkouts"))
                    .and_then(Value::as_array)
                    .and_then(|checkouts| checkouts.first())
                    .and_then(|checkout| json_string_at(checkout, &["path"]))
            });
        let candidate_remote = repository_payload.and_then(|value| {
            ["identity_key", "remote_url", "remote_identity"]
                .iter()
                .find_map(|key| json_string_at(value, &[key]))
        });
        let Some(repository) = repositories.iter().find(|repository| {
            canonical_path_matches(candidate_path.as_deref(), &repository.path)
                || candidate_remote
                    .as_deref()
                    .and_then(remote_identity)
                    .zip(repository.remote_url.as_deref().and_then(remote_identity))
                    .is_some_and(|(candidate, local)| candidate == local)
        }) else {
            continue;
        };
        let run_id = payload
            .get("audit_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| audit_id.clone())
            .unwrap_or_else(|| path.display().to_string());
        let run_observed_at = payload
            .get("as_of")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| observed_at.clone());
        let checkouts = repository_payload
            .and_then(|value| value.get("checkouts"))
            .and_then(Value::as_array);
        let target_branch = repository_payload
            .and_then(|value| value.get("target_branch"))
            .and_then(|value| json_string_at(value, &["branch"]));
        let checkout = checkouts.and_then(|items| {
            target_branch
                .as_deref()
                .and_then(|branch| {
                    items.iter().find(|checkout| {
                        json_string_at(checkout, &["branch"]).as_deref() == Some(branch)
                    })
                })
                .or_else(|| items.first())
        });
        let scanned_commit = checkout
            .and_then(|value| json_string_at(value, &["head"]))
            .or_else(|| checkout.and_then(|value| json_string_at(value, &["fingerprint", "head"])));
        let scanned_branch = checkout.and_then(|value| json_string_at(value, &["branch"]));
        let findings = payload
            .get("findings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let agent_usability = payload
            .get("agent_usability")
            .filter(|value| value.is_object())
            .and_then(|value| serde_json::from_value(value.clone()).ok());
        let (mut dimension_scores, _) = fleet_dimension_scores(&findings);
        let mut gaps = fleet_maturity_gaps(&findings);
        if let Some(assessment) = agent_usability.as_ref() {
            merge_agent_usability_dimensions(&mut dimension_scores, &mut gaps, assessment);
        }
        gaps.sort_by(|left, right| left.dimension.cmp(&right.dimension));
        gaps.truncate(MAX_MATURITY_GAPS);
        let score = fleet_score(&dimension_scores)
            .or_else(|| payload.get("mean_maturity").and_then(Value::as_f64));
        let maturity = QualityMaturity {
            score,
            score_display: score.map(|value| format!("{value:.3}")),
            scored_dimension_count: Some(dimension_scores.len() as u64),
            dimension_scores,
            gaps,
            quality_outcome: payload
                .get("quality_outcome")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            agent_usability,
            repository_maturity: None,
            audit_id: Some(run_id.clone()),
            observed_at: run_observed_at.clone(),
            scanned_commit: scanned_commit.clone(),
            scanned_branch: scanned_branch.clone(),
            freshness: evaluate_audit_freshness_at(run_observed_at.as_deref(), Utc::now()),
            report_path: Some(path.to_string_lossy().to_string()),
        };
        // Fleet maturity rows are evidence for the maturity projection, not code
        // findings. Counting the raw array here duplicates maturity remediation and
        // can manufacture an aggregate findings action when every dimension passes.
        let quality_findings = findings
            .iter()
            .filter(|finding| !is_fleet_maturity_finding(finding))
            .cloned()
            .collect::<Vec<_>>();
        let severity_counts = fleet_severity_counts(&quality_findings);
        let high_severity_total = severity_counts
            .iter()
            .filter(|(severity, _)| matches!(severity.as_str(), "critical" | "high"))
            .map(|(_, count)| *count)
            .sum();
        let findings_freshness = evaluate_freshness_at(
            run_observed_at.as_deref(),
            scanned_commit.as_deref(),
            scanned_branch.as_deref(),
            repository.workspace.last_commit.as_deref(),
            Some(repository.branch.as_str()),
            Utc::now(),
        );
        let mut imported_findings = QualityFindings {
            total: quality_findings.len() as u64,
            severity_counts,
            high_severity_total,
            source: Some(QualitySource::Qr),
            observed_at: run_observed_at.clone(),
            scanned_commit,
            scanned_branch,
            freshness: findings_freshness,
            report_path: Some(path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };
        apply_detector_evidence(&mut imported_findings, &payload);
        evidence.insert(
            repository.id.clone(),
            FleetAuditEvidence {
                maturity,
                findings: imported_findings,
            },
        );
    }
    FleetAuditImport {
        audit_id,
        observed_at,
        evidence,
    }
}

pub(crate) fn is_fleet_maturity_finding(finding: &Value) -> bool {
    ["schema", "pack", "pack_id"].iter().any(|key| {
        finding
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with(FLEET_MATURITY_FINDING_SCHEMA_PREFIX))
    })
}

pub fn default_quality_gates() -> Vec<QualityGate> {
    CANONICAL_GATE_DEFINITIONS
        .iter()
        .map(|(id, label)| QualityGate {
            id: (*id).to_string(),
            label: (*label).to_string(),
            status: QualityGateStatus::NotConfigured,
            freshness: QualityFreshness::Unknown,
            evidence: Vec::new(),
        })
        .collect()
}

pub fn evaluate_ci_readiness(gates: &[QualityGate]) -> QualityReadiness {
    let mut applicable_gate_ids = CI_READINESS_BASELINE_GATE_IDS
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    applicable_gate_ids.extend(
        CI_READINESS_CONDITIONAL_GATE_IDS
            .iter()
            .filter(|id| {
                gates
                    .iter()
                    .find(|gate| gate.id == **id)
                    .is_some_and(|gate| !gate.evidence.is_empty())
            })
            .map(|id| (*id).to_string()),
    );

    let configured_gate_ids = gates
        .iter()
        .filter(|gate| !gate.evidence.is_empty())
        .map(|gate| gate.id.clone())
        .collect::<Vec<_>>();
    let mut readiness = evaluate_ci_readiness_for_ideal_with_configuration(
        gates,
        &applicable_gate_ids,
        &configured_gate_ids,
    );
    let passing_gate_count = applicable_gate_ids
        .iter()
        .filter(|gate_id| {
            gates.iter().any(|gate| {
                gate.id == **gate_id
                    && gate.status == QualityGateStatus::Passed
                    && gate.freshness == QualityFreshness::Fresh
            })
        })
        .count();
    readiness.score = (applicable_gate_ids.len() > 0).then(|| {
        let score = (passing_gate_count as f64 / applicable_gate_ids.len() as f64) * 4.0;
        (score * 100.0).round() / 100.0
    });
    readiness.score_display = readiness.score.map(format_quality_score);
    readiness
}

pub fn evaluate_ci_readiness_for_ideal(
    gates: &[QualityGate],
    ideal_gate_ids: &[String],
) -> QualityReadiness {
    let configured_gate_ids = gates
        .iter()
        .filter(|gate| !gate.evidence.is_empty())
        .map(|gate| gate.id.clone())
        .collect::<Vec<_>>();
    evaluate_ci_readiness_for_ideal_with_configuration(gates, ideal_gate_ids, &configured_gate_ids)
}

pub fn evaluate_ci_readiness_for_ideal_with_configuration(
    gates: &[QualityGate],
    ideal_gate_ids: &[String],
    configured_gate_ids: &[String],
) -> QualityReadiness {
    let mut applicable_gate_ids = Vec::new();
    let mut seen_gate_ids = HashSet::new();
    for gate_id in ideal_gate_ids {
        let normalized_gate_id = normalize_gate_id(gate_id);
        if seen_gate_ids.insert(normalized_gate_id.clone()) {
            applicable_gate_ids.push(normalized_gate_id);
        }
    }
    let mut readiness = QualityReadiness {
        applicable_gate_ids: applicable_gate_ids.clone(),
        ..QualityReadiness::default()
    };
    let configured_gate_ids = configured_gate_ids
        .iter()
        .map(|gate_id| normalize_gate_id(gate_id))
        .collect::<HashSet<_>>();

    for gate_id in &applicable_gate_ids {
        if configured_gate_ids.contains(gate_id) {
            readiness.configured_gate_ids.push(gate_id.clone());
        } else {
            readiness.unconfigured_gate_ids.push(gate_id.clone());
        }
        let Some(gate) = gates.iter().find(|gate| gate.id == *gate_id) else {
            readiness.missing_gate_ids.push(gate_id.clone());
            continue;
        };
        if gate.status != QualityGateStatus::NotConfigured && !gate.evidence.is_empty() {
            readiness.covered_gate_ids.push(gate_id.clone());
        }
        if gate.status == QualityGateStatus::Passed && gate.freshness == QualityFreshness::Fresh {
            readiness.fresh_passing_gate_ids.push(gate_id.clone());
        }
        match gate.status {
            QualityGateStatus::Passed if gate.freshness == QualityFreshness::Fresh => {}
            QualityGateStatus::Passed => readiness.stale_gate_ids.push(gate_id.clone()),
            QualityGateStatus::Failed => readiness.failed_gate_ids.push(gate_id.clone()),
            QualityGateStatus::Blocked => readiness.blocked_gate_ids.push(gate_id.clone()),
            QualityGateStatus::NotConfigured => readiness.missing_gate_ids.push(gate_id.clone()),
        }
    }

    let applicable_gate_count = applicable_gate_ids.len();
    if applicable_gate_count > 0 {
        let configuration_score =
            (readiness.configured_gate_ids.len() as f64 / applicable_gate_count as f64) * 4.0;
        let configuration_score = (configuration_score * 100.0).round() / 100.0;
        readiness.configuration_score = Some(configuration_score);
        readiness.configuration_score_display = Some(format_quality_score(configuration_score));
        let evidence_coverage_score =
            (readiness.covered_gate_ids.len() as f64 / applicable_gate_count as f64) * 4.0;
        let evidence_coverage_score = (evidence_coverage_score * 100.0).round() / 100.0;
        readiness.evidence_coverage_score = Some(evidence_coverage_score);
        readiness.evidence_coverage_score_display =
            Some(format_quality_score(evidence_coverage_score));
        let fresh_passing_score =
            (readiness.fresh_passing_gate_ids.len() as f64 / applicable_gate_count as f64) * 4.0;
        let fresh_passing_score = (fresh_passing_score * 100.0).round() / 100.0;
        readiness.score = Some(fresh_passing_score);
        readiness.score_display = Some(format_quality_score(fresh_passing_score));
    }
    readiness
}

fn normalize_repository_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn recommendation_matrix_profiles() -> Vec<(String, Vec<String>)> {
    RECOMMENDATION_MATRIX_MARKDOWN
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('|') || !line.ends_with('|') {
                return None;
            }
            let cells = line[1..line.len() - 1]
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.len() < 11
                || cells[0] == "Project"
                || cells[0].chars().all(|character| character == '-')
            {
                return None;
            }
            let project = normalize_repository_key(cells[0]);
            if project.is_empty() || matches!(cells[1], "PENDING" | "FIXTURE?") {
                return Some((project, Vec::new()));
            }
            let ideal_gate_ids = RECOMMENDATION_MATRIX_GATE_COLUMNS
                .iter()
                .filter_map(|(gate_id, column)| {
                    matches!(cells[*column], "K" | "N" | "A" | "C").then(|| (*gate_id).to_string())
                })
                .collect::<Vec<_>>();
            Some((project, ideal_gate_ids))
        })
        .collect()
}

pub fn ideal_gate_ids_for_repository(repository: &RepositorySnapshot) -> Option<Vec<String>> {
    let key = normalize_repository_key(&repository.name);
    let matches = recommendation_matrix_profiles()
        .into_iter()
        .filter(|(project, _)| project == &key)
        .collect::<Vec<_>>();
    let ideal_gate_ids = (matches.len() == 1)
        .then(|| matches.into_iter().next().map(|(_, gates)| gates))
        .flatten()
        .filter(|gates| !gates.is_empty())
        .unwrap_or_else(|| {
            CI_READINESS_BASELINE_GATE_IDS
                .iter()
                .map(|gate_id| (*gate_id).to_string())
                .collect()
        });
    Some(ideal_gate_ids)
}

pub fn update_ci_readiness_summary(
    portfolio: &mut QualityPortfolioSnapshot,
    repositories: &[RepositorySnapshot],
) {
    let scores = repositories
        .iter()
        .filter_map(|repository| repository.quality.ci_readiness.score)
        .collect::<Vec<_>>();
    portfolio.ci_readiness_repository_count = scores.len();
    portfolio.ci_readiness_unscored_repository_count =
        repositories.len().saturating_sub(scores.len());
    portfolio.ci_readiness_full_repository_count =
        scores.iter().filter(|score| **score >= 4.0).count();
    portfolio.ci_readiness_score = if scores.is_empty() {
        None
    } else {
        let score = scores.iter().sum::<f64>() / scores.len() as f64;
        Some((score * 100.0).round() / 100.0)
    };
    portfolio.ci_readiness_score_display = portfolio.ci_readiness_score.map(format_quality_score);
    let evidence_coverage_scores = repositories
        .iter()
        .filter_map(|repository| repository.quality.ci_readiness.evidence_coverage_score)
        .collect::<Vec<_>>();
    portfolio.ci_evidence_coverage_score = average_quality_scores(&evidence_coverage_scores);
    portfolio.ci_evidence_coverage_score_display = portfolio
        .ci_evidence_coverage_score
        .map(format_quality_score);
    let configuration_scores = repositories
        .iter()
        .filter_map(|repository| repository.quality.ci_readiness.configuration_score)
        .collect::<Vec<_>>();
    portfolio.ci_configuration_score = average_quality_scores(&configuration_scores);
    portfolio.ci_configuration_score_display =
        portfolio.ci_configuration_score.map(format_quality_score);
    portfolio.ci_evidence_fresh_passing_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.fresh_passing_gate_ids.len())
        .sum();
    portfolio.ci_evidence_ideal_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.applicable_gate_ids.len())
        .sum();
    portfolio.ci_configuration_configured_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.configured_gate_ids.len())
        .sum();
    portfolio.ci_configuration_ideal_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.applicable_gate_ids.len())
        .sum();
    let configured_profiles = repositories
        .iter()
        .filter(|repository| {
            !repository
                .quality
                .ci_readiness
                .applicable_gate_ids
                .is_empty()
        })
        .collect::<Vec<_>>();
    portfolio.ci_configuration_repository_count = configured_profiles.len();
    portfolio.ci_configuration_unscored_repository_count =
        repositories.len().saturating_sub(configured_profiles.len());
    portfolio.ci_configuration_full_repository_count = configured_profiles
        .iter()
        .filter(|repository| {
            repository
                .quality
                .ci_readiness
                .unconfigured_gate_ids
                .is_empty()
        })
        .count();
    portfolio.ci_readiness_open_gate_counts = BTreeMap::new();
    for repository in repositories {
        let readiness = &repository.quality.ci_readiness;
        for gate_id in readiness
            .missing_gate_ids
            .iter()
            .chain(readiness.stale_gate_ids.iter())
            .chain(readiness.failed_gate_ids.iter())
            .chain(readiness.blocked_gate_ids.iter())
        {
            *portfolio
                .ci_readiness_open_gate_counts
                .entry(gate_id.clone())
                .or_default() += 1;
        }
    }
}

const LEGACY_COMPOSITE_MATURITY_DIMENSIONS: [&str; 10] = [
    "ci.configuration",
    "ci.evidence_coverage",
    "ci.fresh_passing",
    "project_compass.mvp_progress",
    "project_compass.complete_product_progress",
    "mac_control.implementation_contract",
    "mac_control.live_task_evidence",
    "mac_control.task_usability",
    "web_readiness.user_journey",
    "product_readiness",
];

const REPOSITORY_MATURITY_PILLARS: [(&str, &str, f64, bool); 7] = [
    (
        "correctness_reliability",
        "Correctness and reliability",
        0.22,
        true,
    ),
    (
        "security_privacy_supply_chain",
        "Security, privacy, and supply chain",
        0.22,
        true,
    ),
    (
        "maintainability_evolvability",
        "Maintainability and evolvability",
        0.16,
        true,
    ),
    (
        "operability_release_safety",
        "Operability and release safety",
        0.14,
        true,
    ),
    ("user_facing_quality", "User-facing quality", 0.10, false),
    (
        "human_agent_usability",
        "Human and agent usability",
        0.10,
        false,
    ),
    (
        "governance_sustainability",
        "Governance and sustainability",
        0.06,
        false,
    ),
];

pub fn update_composite_maturity_summary(
    portfolio: &mut QualityPortfolioSnapshot,
    repositories: &mut [RepositorySnapshot],
) {
    portfolio.source_maturity_score = portfolio.maturity_score;
    portfolio.source_maturity_score_display = portfolio.maturity_score_display.clone();
    portfolio.source_scored_dimension_count = portfolio.scored_dimension_count;

    let mut implementation_scores = Vec::new();
    let mut live_scores = Vec::new();
    for repository in repositories.iter_mut() {
        let maturity = &mut repository.quality.maturity;
        for dimension in LEGACY_COMPOSITE_MATURITY_DIMENSIONS {
            maturity.dimension_scores.remove(dimension);
        }
        maturity
            .gaps
            .retain(|gap| !LEGACY_COMPOSITE_MATURITY_DIMENSIONS.contains(&gap.dimension.as_str()));

        merge_composite_dimension(
            maturity,
            "ci.fresh_passing",
            repository.quality.ci_readiness.score,
            "Fresh passing CI evidence",
        );
        let mac_control = &repository.quality.mac_control_ideal_state;
        if mac_control.applicability != "Not applicable" {
            let implementation_score = mac_control_implementation_score(mac_control);
            let live_score = mac_control_live_score(mac_control);
            merge_composite_dimension(
                maturity,
                "mac_control.task_usability",
                average_quality_scores(&[implementation_score, live_score]),
                "Mac Control implementation and live task usability",
            );
            implementation_scores.push(implementation_score);
            live_scores.push(live_score);
        }

        merge_composite_dimension(
            maturity,
            "web_readiness.user_journey",
            web_readiness_maturity_score(&repository.quality.web_readiness),
            "User-facing route readiness",
        );

        let model = build_repository_maturity_model(maturity);
        maturity.score = model.score;
        maturity.score_display = model.score.map(|score| format!("{score:.3}"));
        maturity.scored_dimension_count = Some(model.evidence.assessed_pillar_count);
        maturity.repository_maturity = Some(model);
        maturity
            .gaps
            .sort_by(|left, right| left.dimension.cmp(&right.dimension));
    }

    portfolio.mac_control_ideal_state.implementation_score =
        average_quality_scores(&implementation_scores);
    portfolio
        .mac_control_ideal_state
        .implementation_score_display = portfolio
        .mac_control_ideal_state
        .implementation_score
        .map(format_quality_score);
    portfolio.mac_control_ideal_state.live_score = average_quality_scores(&live_scores);
    portfolio.mac_control_ideal_state.live_score_display = portfolio
        .mac_control_ideal_state
        .live_score
        .map(format_quality_score);

    let repository_scores = repositories
        .iter()
        .filter_map(|repository| {
            repository
                .quality
                .maturity
                .repository_maturity
                .as_ref()
                .and_then(|model| model.score)
        })
        .collect::<Vec<_>>();
    portfolio.maturity_score = average_quality_scores(&repository_scores);
    portfolio.maturity_score_display = portfolio.maturity_score.map(|score| format!("{score:.3}"));
    portfolio.scored_dimension_count = Some(
        repositories
            .iter()
            .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
            .map(|model| model.evidence.assessed_pillar_count)
            .sum(),
    );
    portfolio.maturity_pillars = REPOSITORY_MATURITY_PILLARS
        .iter()
        .map(|(id, label, _, _)| {
            let scores = repositories
                .iter()
                .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
                .flat_map(|model| model.pillars.iter())
                .filter(|pillar| pillar.id == *id)
                .filter_map(|pillar| pillar.score)
                .collect::<Vec<_>>();
            PortfolioMaturityPillar {
                id: (*id).to_string(),
                label: (*label).to_string(),
                score: average_quality_scores(&scores),
                assessed_repository_count: scores.len(),
            }
        })
        .collect();
    let evidence_coverages = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .map(|model| model.evidence.evidence_coverage * 4.0)
        .collect::<Vec<_>>();
    portfolio.maturity_evidence_coverage =
        average_quality_scores(&evidence_coverages).map(|score| score / 4.0);
    let fresh_coverages = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .map(|model| model.evidence.fresh_evidence_coverage * 4.0)
        .collect::<Vec<_>>();
    portfolio.maturity_fresh_evidence_coverage =
        average_quality_scores(&fresh_coverages).map(|score| score / 4.0);
    portfolio.maturity_provisional_repository_count = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .filter(|model| model.status == "provisional")
        .count();
    portfolio.maturity_capped_repository_count = repositories
        .iter()
        .filter_map(|repository| repository.quality.maturity.repository_maturity.as_ref())
        .filter(|model| model.critical_cap.applied)
        .count();
}

fn web_readiness_maturity_score(readiness: &WebReadinessSnapshot) -> Option<f64> {
    if readiness.applicability == "not_applicable" {
        return None;
    }
    let total = readiness.passed_count
        + readiness.failed_count
        + readiness.blocked_count
        + readiness.unknown_count;
    (total > 0)
        .then(|| ((readiness.passed_count as f64 / total as f64) * 4.0 * 1000.0).round() / 1000.0)
}

fn build_repository_maturity_model(maturity: &QualityMaturity) -> RepositoryMaturityModel {
    let prior_critical_dimensions = maturity
        .repository_maturity
        .as_ref()
        .map(|model| {
            model
                .pillars
                .iter()
                .flat_map(|pillar| pillar.critical_dimensions.iter().cloned())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let statuses = maturity
        .gaps
        .iter()
        .map(|gap| (gap.dimension.as_str(), gap.status.as_str()))
        .collect::<BTreeMap<_, _>>();
    let agent_applicability = maturity
        .agent_usability
        .as_ref()
        .map(|assessment| assessment.applicability.as_str())
        .unwrap_or("unknown");
    let mut assigned = BTreeSet::new();
    let mut critical_reasons = Vec::new();
    let mut pillars = Vec::new();

    for (id, label, weight, required) in REPOSITORY_MATURITY_PILLARS {
        let dimension_scores = maturity
            .dimension_scores
            .iter()
            .filter(|(dimension, _)| maturity_dimension_pillar(dimension) == Some(id))
            .map(|(dimension, score)| {
                assigned.insert(dimension.clone());
                (dimension.clone(), *score)
            })
            .collect::<BTreeMap<_, _>>();
        let applicability = if required {
            "applicable"
        } else if id == "human_agent_usability" && agent_applicability == "not_applicable" {
            "not_applicable"
        } else if !dimension_scores.is_empty()
            || (id == "human_agent_usability" && agent_applicability == "applicable")
        {
            "applicable"
        } else {
            "unknown"
        };
        let score = (applicability == "applicable")
            .then(|| {
                average_quality_scores(&dimension_scores.values().copied().collect::<Vec<_>>())
            })
            .flatten();
        let critical_dimensions = dimension_scores
            .keys()
            .filter(|dimension| {
                is_critical_maturity_pillar(id)
                    && (prior_critical_dimensions.contains(*dimension)
                        || statuses.get(dimension.as_str()) == Some(&"blocked"))
            })
            .cloned()
            .collect::<Vec<_>>();
        critical_reasons.extend(
            critical_dimensions
                .iter()
                .map(|dimension| format!("{id}:{dimension}")),
        );
        let missing_capabilities = maturity_pillar_capabilities(id)
            .iter()
            .filter(|(_, patterns)| {
                !dimension_scores
                    .keys()
                    .any(|dimension| maturity_dimension_matches(dimension, patterns))
            })
            .map(|(capability, _)| (*capability).to_string())
            .collect::<Vec<_>>();
        let pillar_status = if applicability == "not_applicable" {
            "not_applicable".to_string()
        } else if applicability == "unknown" || score.is_none() {
            "unknown".to_string()
        } else if !critical_dimensions.is_empty() {
            "blocked".to_string()
        } else if dimension_scores
            .keys()
            .any(|dimension| statuses.get(dimension.as_str()) == Some(&"blocked"))
        {
            "blocked".to_string()
        } else if dimension_scores
            .keys()
            .any(|dimension| statuses.get(dimension.as_str()) == Some(&"stale"))
        {
            "stale".to_string()
        } else if dimension_scores
            .keys()
            .any(|dimension| statuses.get(dimension.as_str()) == Some(&"unknown"))
        {
            "unknown".to_string()
        } else if score == Some(4.0) {
            "maintained".to_string()
        } else {
            "attention".to_string()
        };
        pillars.push(RepositoryMaturityPillar {
            id: id.to_string(),
            label: label.to_string(),
            weight,
            applicability: applicability.to_string(),
            status: pillar_status,
            score,
            dimension_scores,
            missing_capabilities,
            critical_dimensions,
        });
    }

    let applicable = pillars
        .iter()
        .filter(|pillar| pillar.applicability == "applicable")
        .collect::<Vec<_>>();
    let assessed = applicable
        .iter()
        .copied()
        .filter(|pillar| pillar.score.is_some())
        .collect::<Vec<_>>();
    let applicable_weight = applicable.iter().map(|pillar| pillar.weight).sum::<f64>();
    let scored_weight = assessed.iter().map(|pillar| pillar.weight).sum::<f64>();
    let assessed_weight = applicable
        .iter()
        .map(|pillar| pillar.weight * maturity_pillar_capability_coverage(pillar))
        .sum::<f64>();
    let uncapped_score = (scored_weight > 0.0).then(|| {
        round_quality_score(
            assessed
                .iter()
                .map(|pillar| pillar.score.unwrap_or_default() * pillar.weight)
                .sum::<f64>()
                / scored_weight,
        )
    });
    let maximum_score = (!critical_reasons.is_empty()).then_some(2.0);
    let score = uncapped_score.map(|score| maximum_score.map_or(score, |cap| score.min(cap)));
    let unknown_applicability = pillars
        .iter()
        .filter(|pillar| pillar.applicability == "unknown")
        .map(|pillar| pillar.id.clone())
        .collect::<Vec<_>>();
    let evidence_coverage = if applicable_weight > 0.0 {
        round_ratio(assessed_weight / applicable_weight)
    } else {
        0.0
    };
    let fresh_weight = assessed
        .iter()
        .filter(|pillar| !matches!(pillar.status.as_str(), "blocked" | "stale" | "unknown"))
        .map(|pillar| pillar.weight * maturity_pillar_capability_coverage(pillar))
        .sum::<f64>();
    let fresh_evidence_coverage = if applicable_weight > 0.0 {
        round_ratio(fresh_weight / applicable_weight)
    } else {
        0.0
    };
    let certified = score == Some(4.0)
        && evidence_coverage == 1.0
        && critical_reasons.is_empty()
        && unknown_applicability.is_empty()
        && applicable
            .iter()
            .all(|pillar| pillar.status == "maintained");
    let status = if score.is_none() {
        "unknown"
    } else if !critical_reasons.is_empty() {
        "blocked"
    } else if certified {
        "certified"
    } else if evidence_coverage < 1.0 || !unknown_applicability.is_empty() {
        "provisional"
    } else {
        "measured"
    };
    let applicable_pillar_count = applicable.len() as u64;
    let assessed_pillar_count = assessed.len() as u64;
    let assessed_dimension_count = maturity.dimension_scores.len() as u64;
    let applicable_dimensions = maturity
        .dimension_scores
        .keys()
        .cloned()
        .chain(
            maturity
                .gaps
                .iter()
                .filter(|gap| gap.status != "not_applicable")
                .map(|gap| gap.dimension.clone()),
        )
        .collect::<BTreeSet<_>>();
    let applicable_dimension_count = applicable_dimensions.len() as u64;

    critical_reasons.sort();
    RepositoryMaturityModel {
        schema: "quality-runner-repository-maturity/v2".to_string(),
        score,
        uncapped_score,
        status: status.to_string(),
        pillars,
        evidence: RepositoryMaturityEvidence {
            applicable_pillar_count,
            assessed_pillar_count,
            applicable_dimension_count,
            assessed_dimension_count,
            applicable_weight: round_ratio(applicable_weight),
            assessed_weight: round_ratio(assessed_weight),
            evidence_coverage,
            fresh_evidence_coverage,
            unknown_applicability,
            unmapped_dimensions: maturity
                .dimension_scores
                .keys()
                .filter(|dimension| !assigned.contains(*dimension))
                .cloned()
                .collect(),
        },
        critical_cap: RepositoryMaturityCriticalCap {
            applied: maximum_score.is_some(),
            maximum_score,
            reasons: critical_reasons,
        },
    }
}

fn maturity_dimension_pillar(dimension: &str) -> Option<&'static str> {
    REPOSITORY_MATURITY_PILLARS
        .iter()
        .map(|(id, _, _, _)| *id)
        .find(|pillar| {
            maturity_pillar_patterns(pillar)
                .iter()
                .any(|pattern| dimension == *pattern || dimension.starts_with(pattern))
        })
}

fn maturity_dimension_matches(dimension: &str, patterns: &[&str]) -> bool {
    patterns
        .iter()
        .any(|pattern| dimension == *pattern || dimension.starts_with(pattern))
}

fn maturity_pillar_patterns(pillar: &str) -> &'static [&'static str] {
    match pillar {
        "correctness_reliability" => &[
            "behavior_assurance",
            "behavior_",
            "ci.",
            "dynamic_verification",
            "quality_commands",
            "reliability",
            "test_",
        ],
        "security_privacy_supply_chain" => &[
            "approval_gated_paths",
            "dependency_",
            "privacy",
            "security_",
            "secret_",
            "slsa",
            "supply_chain",
            "vulnerability",
        ],
        "maintainability_evolvability" => &[
            "architecture_boundaries",
            "change_surface_coverage",
            "coding_conventions",
            "maintainability",
            "strict_type_debt",
        ],
        "operability_release_safety" => &[
            "deployment_rollback",
            "diagnosability.",
            "failure_modes",
            "observability",
            "operability",
            "release_",
        ],
        "user_facing_quality" => &[
            "accessibility",
            "performance",
            "user_journey",
            "web_readiness",
        ],
        "human_agent_usability" => &[
            "agent_usability.",
            "context_routing",
            "implementation_examples",
            "mac_control.",
            "skill_contract_quality",
        ],
        "governance_sustainability" => &[
            "contributor_",
            "definition_of_done",
            "governance",
            "license",
            "maintained",
            "matrix_maintenance",
            "ownership_",
        ],
        _ => &[],
    }
}

fn maturity_pillar_capabilities(
    pillar: &str,
) -> &'static [(&'static str, &'static [&'static str])] {
    match pillar {
        "correctness_reliability" => &[
            (
                "automated_quality_gates",
                &["quality_commands", "dynamic_verification", "ci."],
            ),
            ("behavior_outcomes", &["behavior_assurance", "behavior_"]),
            ("reliability_evidence", &["reliability", "test_"]),
        ],
        "security_privacy_supply_chain" => &[
            (
                "security_constraints",
                &["security_constraints", "approval_gated_paths"],
            ),
            (
                "dependency_and_vulnerability_risk",
                &["dependency_", "vulnerability"],
            ),
            ("secret_and_privacy_controls", &["secret_", "privacy"]),
            ("artifact_provenance", &["slsa", "supply_chain"]),
        ],
        "maintainability_evolvability" => &[
            ("architecture_boundaries", &["architecture_boundaries"]),
            ("change_impact_contract", &["change_surface_coverage"]),
            (
                "coding_and_type_health",
                &["coding_conventions", "strict_type_debt"],
            ),
        ],
        "operability_release_safety" => &[
            (
                "failure_and_recovery_contract",
                &["failure_modes", "deployment_rollback"],
            ),
            ("diagnosability", &["diagnosability."]),
            (
                "operational_observability",
                &["observability", "operability"],
            ),
        ],
        "user_facing_quality" => &[
            ("accessible_experience", &["accessibility"]),
            ("performance_evidence", &["performance"]),
            ("user_journey_evidence", &["user_journey", "web_readiness"]),
        ],
        "human_agent_usability" => &[
            (
                "documentation_contract",
                &["agent_usability.documentation_contract"],
            ),
            (
                "tool_skill_coverage",
                &["agent_usability.tool_skill_coverage"],
            ),
            ("behavior_evidence", &["agent_usability.behavior_evidence"]),
            (
                "routing_and_examples",
                &["context_routing", "implementation_examples"],
            ),
        ],
        "governance_sustainability" => &[
            ("ownership_and_governance", &["ownership_", "governance"]),
            ("maintenance_continuity", &["maintained", "contributor_"]),
            ("license_and_contribution", &["license"]),
            (
                "completion_and_matrix_discipline",
                &["definition_of_done", "matrix_maintenance"],
            ),
        ],
        _ => &[],
    }
}

fn is_critical_maturity_pillar(pillar: &str) -> bool {
    matches!(
        pillar,
        "correctness_reliability" | "security_privacy_supply_chain" | "operability_release_safety"
    )
}

fn maturity_pillar_capability_coverage(pillar: &RepositoryMaturityPillar) -> f64 {
    let capability_count = maturity_pillar_capabilities(&pillar.id).len();
    if capability_count == 0 {
        return 0.0;
    }
    (capability_count.saturating_sub(pillar.missing_capabilities.len())) as f64
        / capability_count as f64
}

fn round_quality_score(score: f64) -> f64 {
    (score * 1000.0).round() / 1000.0
}

fn round_ratio(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn merge_composite_dimension(
    maturity: &mut QualityMaturity,
    dimension: &str,
    score: Option<f64>,
    label: &str,
) {
    let Some(score) = score.filter(|score| score.is_finite() && (0.0..=4.0).contains(score)) else {
        return;
    };
    maturity
        .dimension_scores
        .insert(dimension.to_string(), score);
    if score < 4.0 {
        maturity.gaps.push(QualityMaturityGap {
            dimension: dimension.to_string(),
            status: if score == 0.0 { "blocked" } else { "attention" }.to_string(),
            score: Some(score),
            message: format!("{label} is {score:.2}/4.00 and remains below the fleet ideal."),
        });
    }
}

fn mac_control_implementation_score(state: &MacControlRepositoryState) -> f64 {
    if state.implementation_status == "Blocked" || state.implementation_criteria_total == 0 {
        return 0.0;
    }
    let score = (state.implementation_criteria_passed_count as f64
        / state.implementation_criteria_total as f64)
        * 4.0;
    cap_stale_maturity_score(score, &state.freshness)
}

fn mac_control_live_score(state: &MacControlRepositoryState) -> f64 {
    if state.live_status == "Blocked" || state.live_task_count == 0 {
        return 0.0;
    }
    let score = (state.measured_route_count as f64 / state.live_task_count as f64) * 4.0;
    cap_stale_maturity_score(score, &state.freshness)
}

fn cap_stale_maturity_score(score: f64, freshness: &str) -> f64 {
    let score = if freshness == "Fresh" {
        score
    } else {
        score.min(3.0)
    };
    (score * 100.0).round() / 100.0
}

fn average_quality_scores(scores: &[f64]) -> Option<f64> {
    (!scores.is_empty()).then(|| {
        let score = scores.iter().sum::<f64>() / scores.len() as f64;
        (score * 100.0).round() / 100.0
    })
}

fn format_quality_score(score: f64) -> String {
    if score.fract() == 0.0 {
        format!("{score:.1}")
    } else {
        format!("{score:.2}")
    }
}

pub fn normalize_gate_id(value: &str) -> String {
    let slug = slug(value);
    match slug.as_str() {
        "build" | "compile" | "bundle" => "build".to_string(),
        "verify_and_build" => "build".to_string(),
        "smoke" | "smoke_test" | "runtime_smoke" | "runtime_smoke_test" | "runtime_test" => {
            "runtime_smoke".to_string()
        }
        "test" | "tests" | "test_suite" | "unit_test" | "unit_tests" | "integration_test"
        | "integration_tests" | "e2e_test" | "e2e_tests" | "full_suite" => "tests".to_string(),
        "lint" | "linting" => "lint".to_string(),
        "format" | "formatter" | "formatting" | "fmt" => "formatter".to_string(),
        "typecheck" | "type_check" | "typechecking" | "check_types" => "typecheck".to_string(),
        "dead_code" | "deadcode" | "unused_code" => "dead_code".to_string(),
        "secrets_scan"
        | "secret_scan"
        | "secret_scanning"
        | "security_secrets_scan"
        | "secret_scanning_gitleaks"
        | "gitleaks" => "secrets_scan".to_string(),
        "debloat" | "repository_debloat" | "repository_debloat_maturity" => "debloat".to_string(),
        "dependency_audit"
        | "dependency_scan"
        | "dependency_check"
        | "security_dependency_audit"
        | "software_composition_analysis" => "dependency_audit".to_string(),
        "web_readiness" | "web_production_readiness" | "production_web_readiness" => {
            "web_readiness".to_string()
        }
        value if value.starts_with("unit_tests_") => "tests".to_string(),
        value if value.starts_with("integration_tests_") => "tests".to_string(),
        value if value.starts_with("e2e_tests_") => "tests".to_string(),
        value if value.starts_with("full_suite_") => "tests".to_string(),
        _ => format!("custom:{slug}"),
    }
}

pub fn gate_label(id: &str) -> String {
    CANONICAL_GATE_DEFINITIONS
        .iter()
        .chain(CONDITIONAL_GATE_DEFINITIONS.iter())
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| {
            let value = id.strip_prefix("custom:").unwrap_or(id);
            value
                .split('_')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    chars
                        .next()
                        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
}

pub fn normalize_requirement_source(value: &str) -> Option<QualitySource> {
    QualitySource::parse(value)
}

pub fn evaluate_freshness_at(
    observed_at: Option<&str>,
    scanned_commit: Option<&str>,
    scanned_branch: Option<&str>,
    current_commit: Option<&str>,
    current_branch: Option<&str>,
    now: DateTime<Utc>,
) -> QualityFreshness {
    let Some(observed_at) = observed_at else {
        return QualityFreshness::Unknown;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(observed_at) else {
        return QualityFreshness::Unknown;
    };
    let age = now.signed_duration_since(parsed.with_timezone(&Utc));
    if age > Duration::days(MAX_EVIDENCE_AGE_DAYS) {
        return QualityFreshness::Stale;
    }
    match (scanned_commit, current_commit) {
        (Some(scanned), Some(current)) => {
            if scanned == current {
                QualityFreshness::Fresh
            } else {
                QualityFreshness::Stale
            }
        }
        (Some(_), None) | (None, Some(_)) => QualityFreshness::Unknown,
        (None, None) => match (scanned_branch, current_branch) {
            (Some(scanned), Some(current)) if scanned != current => QualityFreshness::Stale,
            _ => QualityFreshness::Unknown,
        },
    }
}

pub fn evaluate_audit_freshness_at(
    observed_at: Option<&str>,
    now: DateTime<Utc>,
) -> QualityFreshness {
    let Some(observed_at) = observed_at else {
        return QualityFreshness::Unknown;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(observed_at) else {
        return QualityFreshness::Unknown;
    };
    if now.signed_duration_since(parsed.with_timezone(&Utc)) > Duration::days(MAX_EVIDENCE_AGE_DAYS)
    {
        QualityFreshness::Stale
    } else {
        QualityFreshness::Fresh
    }
}

pub fn aggregate_gate_status(
    evidence: &[QualityEvidence],
) -> (QualityGateStatus, QualityFreshness) {
    if evidence.is_empty() {
        return (QualityGateStatus::NotConfigured, QualityFreshness::Unknown);
    }
    let has_passed = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Passed);
    let has_failed = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Failed);
    let has_blocked = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Blocked);
    let conflict = has_passed && (has_failed || has_blocked);
    let status = if conflict {
        QualityGateStatus::Blocked
    } else if has_blocked {
        QualityGateStatus::Blocked
    } else if has_failed {
        QualityGateStatus::Failed
    } else if has_passed {
        QualityGateStatus::Passed
    } else {
        QualityGateStatus::NotConfigured
    };
    let freshness = if conflict {
        QualityFreshness::Conflicted
    } else if status == QualityGateStatus::Passed
        && evidence.iter().any(|item| {
            item.status == QualityGateStatus::Passed && item.freshness == QualityFreshness::Fresh
        })
    {
        QualityFreshness::Fresh
    } else if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Stale)
    {
        QualityFreshness::Stale
    } else if evidence
        .iter()
        .all(|item| item.freshness == QualityFreshness::Unknown)
    {
        QualityFreshness::Unknown
    } else {
        QualityFreshness::Unknown
    };
    (status, freshness)
}

fn parse_verification_level(value: Option<&str>) -> QualityVerificationLevel {
    match value.unwrap_or_default() {
        "source_inferred" => QualityVerificationLevel::SourceInferred,
        "artifact_inspected" => QualityVerificationLevel::ArtifactInspected,
        "browser_rendered" => QualityVerificationLevel::BrowserRendered,
        "deployment_verified" => QualityVerificationLevel::DeploymentVerified,
        _ => QualityVerificationLevel::Unknown,
    }
}

fn web_readiness_gate_status(status: &str) -> QualityGateStatus {
    match status {
        "ready" | "warnings" => QualityGateStatus::Passed,
        "blocked" => QualityGateStatus::Failed,
        "not_applicable" => QualityGateStatus::NotConfigured,
        _ => QualityGateStatus::Blocked,
    }
}

fn web_readiness_display_status(status: &str) -> String {
    match status {
        "ready" => "Ready",
        "warnings" => "Warnings",
        "blocked" => "Blocked",
        "not_applicable" => "Not applicable",
        _ => "Unknown",
    }
    .to_string()
}

fn web_readiness_target_level(kind: &str) -> QualityVerificationLevel {
    match kind {
        "deployment" => QualityVerificationLevel::DeploymentVerified,
        "browser" => QualityVerificationLevel::BrowserRendered,
        "artifact" => QualityVerificationLevel::ArtifactInspected,
        "source" => QualityVerificationLevel::SourceInferred,
        _ => QualityVerificationLevel::Unknown,
    }
}

fn invalid_web_readiness_evidence(report_path: &Path, detail: String) -> QualityEvidence {
    QualityEvidence {
        id: "web_readiness".to_string(),
        source: QualitySource::Qr,
        status: QualityGateStatus::Blocked,
        freshness: QualityFreshness::Unknown,
        observed_at: None,
        scanned_commit: None,
        scanned_branch: None,
        command: Some("qr web-readiness . --json".to_string()),
        source_label: "Quality Runner web readiness".to_string(),
        report_path: Some(report_path.to_string_lossy().to_string()),
        report_url: None,
        report_kind: Some("Quality Runner web readiness".to_string()),
        detail,
        verification_level: QualityVerificationLevel::Unknown,
        target_kind: None,
        target_url: None,
        target_provider: None,
        deployment_id: None,
    }
}

fn import_web_readiness(
    repository: &RepositorySnapshot,
) -> (WebReadinessSnapshot, Option<QualityEvidence>) {
    let report_path = Path::new(&repository.path).join(WEB_READINESS_RELATIVE_PATH);
    if !report_path.is_file() {
        return (WebReadinessSnapshot::default(), None);
    }
    let invalid = |detail: String| {
        (
            WebReadinessSnapshot {
                report_path: Some(report_path.to_string_lossy().to_string()),
                applicability_reason: Some(detail.clone()),
                ..WebReadinessSnapshot::default()
            },
            Some(invalid_web_readiness_evidence(&report_path, detail)),
        )
    };
    let contents = match fs::read_to_string(&report_path) {
        Ok(contents) => contents,
        Err(error) => return invalid(format!("Web-readiness report could not be read: {error}")),
    };
    let payload = match serde_json::from_str::<Value>(&contents) {
        Ok(payload) => payload,
        Err(error) => return invalid(format!("Web-readiness report is not valid JSON: {error}")),
    };
    if json_string_at(&payload, &["schema"]).as_deref() != Some(WEB_READINESS_SCHEMA) {
        return invalid(format!(
            "Web-readiness report must use schema {WEB_READINESS_SCHEMA}"
        ));
    }

    let status = json_string_at(&payload, &["status"]).unwrap_or_else(|| "unknown".to_string());
    let observed_at = json_string_at(&payload, &["generated_at"]);
    let scanned_commit = json_string_at(&payload, &["repository", "head_sha"]);
    let scanned_branch = json_string_at(&payload, &["repository", "branch"]);
    let applicability = json_string_at(&payload, &["applicability", "status"])
        .unwrap_or_else(|| "unknown".to_string());
    let target_kind = json_string_at(&payload, &["target", "kind"]).unwrap_or_default();
    let verification_level = web_readiness_target_level(&target_kind);
    let freshness = evaluate_freshness_at(
        observed_at.as_deref(),
        scanned_commit.as_deref(),
        scanned_branch.as_deref(),
        repository.workspace.last_commit.as_deref(),
        Some(repository.branch.as_str()),
        Utc::now(),
    );
    let checks = payload
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|check| {
            let id = json_string_at(check, &["id"])?;
            let routes = check
                .get("evidence")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| json_string_at(item, &["route"]))
                .collect::<Vec<_>>();
            Some(WebReadinessCheck {
                id,
                label: json_string_at(check, &["label"]).unwrap_or_else(|| "Web check".to_string()),
                category: json_string_at(check, &["category"])
                    .unwrap_or_else(|| "baseline".to_string()),
                policy: json_string_at(check, &["policy"]).unwrap_or_else(|| "block".to_string()),
                status: json_string_at(check, &["status"]).unwrap_or_else(|| "unknown".to_string()),
                verification_level: parse_verification_level(
                    json_string_at(check, &["verification_level"]).as_deref(),
                ),
                detail: json_string_at(check, &["detail"]).unwrap_or_default(),
                routes,
            })
        })
        .collect::<Vec<_>>();
    let warning_count = checks
        .iter()
        .filter(|check| check.policy == "warn" && check.status != "passed")
        .count() as u64;
    let target = WebReadinessTarget {
        kind: target_kind.clone(),
        commit: json_string_at(&payload, &["target", "commit"]),
        url: json_string_at(&payload, &["target", "url"]),
        provider: json_string_at(&payload, &["target", "provider"]),
        deployment_id: json_string_at(&payload, &["target", "deployment_id"]),
        artifact_digest: json_string_at(&payload, &["target", "artifact_digest"]),
    };
    let snapshot = WebReadinessSnapshot {
        status: web_readiness_display_status(&status),
        applicability,
        applicability_reason: json_string_at(&payload, &["applicability", "reason"]),
        freshness: freshness.clone(),
        observed_at: observed_at.clone(),
        scanned_commit: scanned_commit.clone(),
        scanned_branch: scanned_branch.clone(),
        report_path: Some(report_path.to_string_lossy().to_string()),
        target: target.clone(),
        passed_count: json_u64_at(&payload, &["summary", "passed"]).unwrap_or(0),
        failed_count: json_u64_at(&payload, &["summary", "failed"]).unwrap_or(0),
        blocked_count: json_u64_at(&payload, &["summary", "blocked"]).unwrap_or(0),
        unknown_count: json_u64_at(&payload, &["summary", "unknown"]).unwrap_or(0),
        warning_count,
        checks,
    };
    let detail = format!(
        "{} web readiness: {} passed, {} failed, {} blocked, {} unknown, {} warning",
        snapshot.status,
        snapshot.passed_count,
        snapshot.failed_count,
        snapshot.blocked_count,
        snapshot.unknown_count,
        snapshot.warning_count
    );
    let evidence = QualityEvidence {
        id: "web_readiness".to_string(),
        source: QualitySource::Qr,
        status: web_readiness_gate_status(&status),
        freshness,
        observed_at,
        scanned_commit,
        scanned_branch,
        command: Some("qr web-readiness . --json".to_string()),
        source_label: "Quality Runner web readiness".to_string(),
        report_path: snapshot.report_path.clone(),
        report_url: target.url.clone(),
        report_kind: Some("Quality Runner web readiness".to_string()),
        detail,
        verification_level,
        target_kind: (!target.kind.is_empty()).then_some(target.kind),
        target_url: target.url,
        target_provider: target.provider,
        deployment_id: target.deployment_id,
    };
    (snapshot, Some(evidence))
}

pub fn ingest_repository_quality(
    repository: &RepositorySnapshot,
    remote: Option<&RemoteRepositorySnapshot>,
    maturity: Option<QualityMaturity>,
    ideal_gate_ids: Option<&[String]>,
) -> QualitySnapshot {
    let mut gates = default_quality_gates();
    let mut findings = QualityFindings::default();
    let mut last_ingested_at = None;
    let mut configured_gate_ids = Vec::new();
    let (web_readiness, web_readiness_evidence) = import_web_readiness(repository);
    let release_boundary = release_boundary::import_release_boundary(repository);

    if let Some(run) = latest_qr_run(Path::new(&repository.path)) {
        last_ingested_at = run.observed_at.clone();
        configured_gate_ids.extend(run.configured_gate_ids());
        for evidence in run.gate_evidence(repository) {
            configured_gate_ids.push(evidence.id.clone());
            add_evidence(&mut gates, evidence);
        }
        findings = run.findings(repository);
    }
    reconcile_finding_dispositions(Path::new(&repository.path), &mut findings);
    if let Some(evidence) = debloat_maturity_evidence(&findings) {
        configured_gate_ids.push(evidence.id.clone());
        add_evidence(&mut gates, evidence);
    }

    for evidence in ci_evidence(repository, remote) {
        configured_gate_ids.push(evidence.id.clone());
        add_evidence(&mut gates, evidence);
    }

    if let Some(evidence) = web_readiness_evidence {
        if web_readiness.applicability != "not_applicable" {
            configured_gate_ids.push(evidence.id.clone());
        }
        add_evidence(&mut gates, evidence);
        if last_ingested_at.as_deref() < web_readiness.observed_at.as_deref() {
            last_ingested_at = web_readiness.observed_at.clone();
        }
    }

    for gate in &mut gates {
        let (status, freshness) = aggregate_gate_status(&gate.evidence);
        gate.status = status;
        gate.freshness = freshness;
    }
    let effective_ideal_gate_ids = ideal_gate_ids.map(|gate_ids| {
        let mut gate_ids = gate_ids.to_vec();
        if matches!(
            web_readiness.applicability.as_str(),
            "public_web" | "internal_web"
        ) && !gate_ids.iter().any(|gate_id| gate_id == "web_readiness")
        {
            gate_ids.push("web_readiness".to_string());
        }
        gate_ids
    });
    let ci_readiness = effective_ideal_gate_ids
        .as_deref()
        .map(|gate_ids| {
            evaluate_ci_readiness_for_ideal_with_configuration(
                &gates,
                gate_ids,
                &configured_gate_ids,
            )
        })
        .unwrap_or_default();
    let maturity_available = maturity.is_some();
    let installed_runtime = installed_runtime::evaluate(
        Path::new(&repository.path),
        repository.workspace.last_commit.as_deref(),
    );
    let runtime_available = installed_runtime.applicability == "applicable";
    let evidence_available = gates.iter().any(|gate| !gate.evidence.is_empty())
        || findings.total > 0
        || runtime_available;
    QualitySnapshot {
        gates,
        findings,
        maturity: maturity.unwrap_or_default(),
        target_fleet_audit_root: None,
        ci_readiness,
        mac_control_ideal_state: MacControlRepositoryState::default(),
        behavior_assurance: BehaviorAssuranceRepositoryState::default(),
        evidence_contracts: Vec::new(),
        web_readiness,
        release_boundary,
        installed_runtime,
        last_ingested_at,
        ingestion_status: if evidence_available || maturity_available {
            "Available".to_string()
        } else {
            "No evidence".to_string()
        },
        ingestion_message: if evidence_available || maturity_available {
            None
        } else {
            Some("No QR artifacts or CI check runs were found for this repository.".to_string())
        },
    }
}

pub fn project_quality_snapshot_for_target(
    snapshot: &mut QualitySnapshot,
    target_branch: &str,
    target_commit: &str,
) {
    for gate in &mut snapshot.gates {
        gate.evidence.retain(|evidence| {
            target_provenance_matches(
                evidence.scanned_branch.as_deref(),
                evidence.scanned_commit.as_deref(),
                target_branch,
                target_commit,
            )
        });
        for evidence in &mut gate.evidence {
            evidence.freshness = evaluate_target_freshness(
                evidence.scanned_branch.as_deref(),
                evidence.scanned_commit.as_deref(),
                target_branch,
                target_commit,
            );
        }
        let (status, freshness) = aggregate_gate_status(&gate.evidence);
        gate.status = status;
        gate.freshness = freshness;
    }

    let findings_match = target_provenance_matches(
        snapshot.findings.scanned_branch.as_deref(),
        snapshot.findings.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
    if findings_match {
        snapshot.findings.freshness = evaluate_target_freshness(
            snapshot.findings.scanned_branch.as_deref(),
            snapshot.findings.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    } else if snapshot.findings.refresh_required
        && snapshot.findings.detector_status.as_deref() == Some("blocked")
        && snapshot.findings.detector_findings_total > 0
    {
        // Keep the prior valid report available as stale/raw evidence when a
        // replacement detector run is blocked. The target UI still refuses to
        // present it as a verified result because refresh_required remains set.
        snapshot.findings.freshness = QualityFreshness::Stale;
    } else {
        snapshot.findings = QualityFindings::default();
    }

    let maturity_match = target_provenance_matches(
        snapshot.maturity.scanned_branch.as_deref(),
        snapshot.maturity.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
    if maturity_match {
        snapshot.maturity.freshness = evaluate_target_freshness(
            snapshot.maturity.scanned_branch.as_deref(),
            snapshot.maturity.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    } else {
        snapshot.maturity = QualityMaturity::default();
    }

    if target_provenance_matches(
        snapshot.web_readiness.scanned_branch.as_deref(),
        snapshot.web_readiness.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    ) {
        snapshot.web_readiness.freshness = QualityFreshness::Fresh;
    } else if snapshot.web_readiness.report_path.is_some() {
        snapshot.web_readiness.freshness = evaluate_target_freshness(
            snapshot.web_readiness.scanned_branch.as_deref(),
            snapshot.web_readiness.scanned_commit.as_deref(),
            target_branch,
            target_commit,
        );
    }

    release_boundary::project_for_target(
        &mut snapshot.release_boundary,
        target_branch,
        target_commit,
    );

    let configured_gate_ids = snapshot
        .gates
        .iter()
        .filter(|gate| !gate.evidence.is_empty())
        .map(|gate| gate.id.clone())
        .collect::<Vec<_>>();
    snapshot.ci_readiness = evaluate_ci_readiness_for_ideal_with_configuration(
        &snapshot.gates,
        &snapshot.ci_readiness.applicable_gate_ids,
        &configured_gate_ids,
    );
    snapshot.ingestion_status = if snapshot.maturity.score.is_some()
        || snapshot.findings.source.is_some()
        || snapshot.gates.iter().any(|gate| !gate.evidence.is_empty())
    {
        "Available".to_string()
    } else {
        "No evidence".to_string()
    };
    snapshot.ingestion_message = (snapshot.ingestion_status == "No evidence").then(|| {
        "No target-scoped QR or fleet evidence was found for this branch and commit.".to_string()
    });
}

fn normalize_disposition_status(value: &str) -> Option<&'static str> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' '], "_")
        .as_str()
    {
        "confirmed" => Some("confirmed"),
        "false_positive" => Some("false_positive"),
        "accepted_intentional" => Some("accepted_intentional"),
        "accepted_risk" => Some("accepted_risk"),
        "deferred" => Some("deferred"),
        "fixed" => Some("fixed"),
        "superseded" => Some("superseded"),
        _ => None,
    }
}

fn validate_finding_dispositions_contract(
    contract: &mut QualityFindingDispositionsContract,
) -> Result<(), String> {
    if contract.schema_version != FINDING_DISPOSITIONS_SCHEMA {
        return Err(format!(
            "Expected schema_version {FINDING_DISPOSITIONS_SCHEMA}, found {}",
            contract.schema_version
        ));
    }
    DateTime::parse_from_rfc3339(&contract.updated_at)
        .map_err(|error| format!("updated_at is not RFC 3339: {error}"))?;
    let mut fingerprints = HashSet::new();
    for disposition in &mut contract.dispositions {
        disposition.fingerprint = disposition.fingerprint.trim().to_string();
        if disposition.fingerprint.is_empty() {
            return Err("A finding disposition has an empty fingerprint".to_string());
        }
        if !fingerprints.insert(disposition.fingerprint.clone()) {
            return Err(format!(
                "Finding fingerprint {} is dispositioned more than once",
                disposition.fingerprint
            ));
        }
        disposition.status = normalize_disposition_status(&disposition.status)
            .ok_or_else(|| {
                format!(
                    "Finding {} has unsupported disposition status '{}'",
                    disposition.fingerprint, disposition.status
                )
            })?
            .to_string();
        if disposition.reason.trim().is_empty() {
            return Err(format!(
                "Finding {} is missing a disposition reason",
                disposition.fingerprint
            ));
        }
        if disposition.reviewer.trim().is_empty() {
            return Err(format!(
                "Finding {} is missing a reviewer",
                disposition.fingerprint
            ));
        }
        DateTime::parse_from_rfc3339(&disposition.reviewed_at).map_err(|error| {
            format!(
                "Finding {} reviewed_at is not RFC 3339: {error}",
                disposition.fingerprint
            )
        })?;
        if let Some(expires_at) = disposition.expires_at.as_deref() {
            DateTime::parse_from_rfc3339(expires_at).map_err(|error| {
                format!(
                    "Finding {} expires_at is not RFC 3339: {error}",
                    disposition.fingerprint
                )
            })?;
        }
    }
    contract
        .dispositions
        .sort_by(|left, right| left.fingerprint.cmp(&right.fingerprint));
    Ok(())
}

fn load_finding_dispositions_contract(
    repository_path: &Path,
) -> Result<Option<QualityFindingDispositionsContract>, String> {
    let path = repository_path.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
    if !path.is_file() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let mut contract = serde_json::from_str::<QualityFindingDispositionsContract>(&contents)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))?;
    validate_finding_dispositions_contract(&mut contract)?;
    Ok(Some(contract))
}

struct ReportFindingInventory {
    fingerprint_counts: HashMap<String, u64>,
    fingerprint_category_counts: HashMap<String, BTreeMap<String, u64>>,
    category_counts: BTreeMap<String, u64>,
}

fn report_finding_inventory(report_paths: &[String]) -> Option<ReportFindingInventory> {
    let mut fingerprint_counts = HashMap::new();
    let mut fingerprint_category_counts: HashMap<String, BTreeMap<String, u64>> = HashMap::new();
    let mut derived_category_counts = BTreeMap::new();
    let mut prior_report_fingerprints: HashSet<String> = HashSet::new();
    let mut saw_findings = false;
    for payload in report_paths
        .iter()
        .filter_map(|path| read_json(Path::new(path)))
    {
        let Some(findings) = payload.get("findings").and_then(Value::as_array) else {
            continue;
        };
        saw_findings = true;
        let mut current_report_fingerprints = HashSet::new();
        for finding in findings {
            let fingerprint = finding
                .get("fingerprint")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if fingerprint.is_some_and(|value| prior_report_fingerprints.contains(value)) {
                continue;
            }
            if let Some(fingerprint) = fingerprint {
                current_report_fingerprints.insert(fingerprint.to_string());
            }
            let category = finding
                .get("category")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(category) = category {
                *derived_category_counts
                    .entry(category.to_string())
                    .or_insert(0) += 1;
            }
            let Some(fingerprint) = fingerprint else {
                continue;
            };
            *fingerprint_counts
                .entry(fingerprint.to_string())
                .or_insert(0) += 1;
            if let Some(category) = category {
                *fingerprint_category_counts
                    .entry(fingerprint.to_string())
                    .or_default()
                    .entry(category.to_string())
                    .or_insert(0) += 1;
            }
        }
        prior_report_fingerprints.extend(current_report_fingerprints);
    }
    if !saw_findings {
        return None;
    }
    Some(ReportFindingInventory {
        fingerprint_counts,
        fingerprint_category_counts,
        category_counts: derived_category_counts,
    })
}

fn disposition_is_expired(disposition: &QualityFindingDisposition, now: DateTime<Utc>) -> bool {
    disposition
        .expires_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|expires_at| expires_at.with_timezone(&Utc) <= now)
}

pub fn non_actionable_finding_fingerprints(
    repository_path: &Path,
) -> Result<HashSet<String>, String> {
    let Some(contract) = load_finding_dispositions_contract(repository_path)? else {
        return Ok(HashSet::new());
    };
    let now = Utc::now();
    Ok(contract
        .dispositions
        .into_iter()
        .filter(|disposition| !disposition_is_expired(disposition, now))
        .filter(|disposition| {
            matches!(
                disposition.status.as_str(),
                "false_positive" | "accepted_intentional" | "accepted_risk"
            )
        })
        .map(|disposition| disposition.fingerprint)
        .collect())
}

pub fn reconcile_finding_dispositions(repository_path: &Path, findings: &mut QualityFindings) {
    sync_detector_counts(findings);
    findings.actionable_total = findings.total;
    findings.reviewed_total = 0;
    findings.unreviewed_total = findings.total;
    findings.disposition_counts.clear();
    findings.stale_disposition_total = 0;
    findings.disposition_contract_path = Some(
        repository_path
            .join(FINDING_DISPOSITIONS_RELATIVE_PATH)
            .to_string_lossy()
            .to_string(),
    );
    findings.disposition_message = None;
    let report_paths = if findings.report_paths.is_empty() {
        findings.report_path.iter().cloned().collect::<Vec<_>>()
    } else {
        findings.report_paths.clone()
    };
    let report_inventory = report_finding_inventory(&report_paths);
    if let Some(inventory) = report_inventory.as_ref() {
        findings.category_counts = inventory.category_counts.clone();
        findings.actionable_category_counts = inventory.category_counts.clone();
    } else {
        findings.category_counts.clear();
        findings.actionable_category_counts.clear();
    }

    let contract = match load_finding_dispositions_contract(repository_path) {
        Ok(Some(contract)) => contract,
        Ok(None) => {
            findings.disposition_status = "Missing".to_string();
            findings.disposition_message = Some(
                "No repository-owned finding disposition contract was found; every detected finding remains unreviewed and actionable."
                    .to_string(),
            );
            sync_detector_counts(findings);
            return;
        }
        Err(error) => {
            findings.disposition_status = "Invalid".to_string();
            findings.disposition_message = Some(error);
            sync_detector_counts(findings);
            return;
        }
    };

    let Some(report_inventory) = report_inventory else {
        findings.disposition_status = "Unreconcilable".to_string();
        findings.disposition_message = Some(
            "The current finding report does not expose stable fingerprints; saved dispositions were not applied."
                .to_string(),
        );
        findings.stale_disposition_total = contract.dispositions.len() as u64;
        sync_detector_counts(findings);
        return;
    };
    let identified_total = report_inventory.fingerprint_counts.values().sum::<u64>();
    if identified_total != findings.total {
        findings.disposition_status = "Unreconcilable".to_string();
        findings.disposition_message = Some(format!(
            "The report declares {} findings but only {} expose stable fingerprints in this scope; saved dispositions were not applied.",
            findings.total, identified_total
        ));
        findings.stale_disposition_total = contract.dispositions.len() as u64;
        sync_detector_counts(findings);
        return;
    }

    let now = Utc::now();
    let mut actionable_reviewed = 0_u64;
    for disposition in &contract.dispositions {
        let current_count = report_inventory
            .fingerprint_counts
            .get(&disposition.fingerprint)
            .copied()
            .unwrap_or(0);
        let expired = disposition_is_expired(disposition, now);
        let applies_to_current = current_count > 0
            && !expired
            && !matches!(disposition.status.as_str(), "fixed" | "superseded");
        if !applies_to_current {
            findings.stale_disposition_total += 1;
            continue;
        }
        *findings
            .disposition_counts
            .entry(disposition.status.clone())
            .or_insert(0) += current_count;
        findings.reviewed_total += current_count;
        if matches!(disposition.status.as_str(), "confirmed" | "deferred") {
            actionable_reviewed += current_count;
        }
        if matches!(
            disposition.status.as_str(),
            "false_positive" | "accepted_intentional" | "accepted_risk"
        ) {
            if let Some(categories) = report_inventory
                .fingerprint_category_counts
                .get(&disposition.fingerprint)
            {
                for (category, count) in categories {
                    if let Some(actionable) = findings.actionable_category_counts.get_mut(category)
                    {
                        *actionable = actionable.saturating_sub(*count);
                    }
                }
            }
        }
    }
    findings.unreviewed_total = findings.total.saturating_sub(findings.reviewed_total);
    findings.actionable_total = findings.unreviewed_total + actionable_reviewed;
    findings.disposition_status = "Ready".to_string();
    sync_detector_counts(findings);
}

fn debloat_maturity_evidence(findings: &QualityFindings) -> Option<QualityEvidence> {
    let detected = findings.category_counts.get("debloat").copied()?;
    let actionable = findings
        .actionable_category_counts
        .get("debloat")
        .copied()
        .unwrap_or(detected);
    let status = if actionable == 0 && findings.freshness == QualityFreshness::Fresh {
        QualityGateStatus::Passed
    } else {
        QualityGateStatus::Blocked
    };
    let detail = if actionable > 0 {
        format!(
            "QR's structural scan reported {detected} debloat signal(s), with {actionable} unresolved. Each signal requires a broader ownership-pressure audit with confidence assessed separately from implementation readiness; no signal authorizes deletion."
        )
    } else if findings.freshness != QualityFreshness::Fresh {
        format!(
            "QR reported no unresolved structural debloat signals, but the evidence is {}; refresh the QR scan before treating this review gate as passed.",
            findings.freshness.as_str()
        )
    } else {
        format!(
            "QR's structural debloat review has no unresolved signals ({detected} detected). This clears the candidate-review gate only; it does not prove that an ownership-pressure audit found no architectural opportunity, establish deletion readiness, or authorize deletion."
        )
    };
    Some(QualityEvidence {
        id: "debloat".to_string(),
        source: QualitySource::Qr,
        status,
        freshness: findings.freshness.clone(),
        observed_at: findings.observed_at.clone(),
        scanned_commit: findings.scanned_commit.clone(),
        scanned_branch: findings.scanned_branch.clone(),
        command: None,
        source_label: "Quality Runner structural debloat signals".to_string(),
        report_path: findings.report_path.clone(),
        report_url: None,
        report_kind: Some("code-quality-scan".to_string()),
        detail,
        verification_level: QualityVerificationLevel::SourceInferred,
        target_kind: Some("source".to_string()),
        target_url: None,
        target_provider: None,
        deployment_id: None,
    })
}

pub fn set_finding_disposition(
    repository_path: &Path,
    fingerprint: &str,
    status: &str,
    reason: &str,
    reviewer: &str,
    evidence: Vec<String>,
    expires_at: Option<String>,
) -> Result<QualityFindingDispositionsContract, String> {
    let fingerprint = fingerprint.trim();
    let reason = reason.trim();
    let reviewer = reviewer.trim();
    if fingerprint.is_empty() {
        return Err("Finding fingerprint must not be empty".to_string());
    }
    let status = normalize_disposition_status(status).ok_or_else(|| {
        "Disposition status must be confirmed, false_positive, accepted_intentional, accepted_risk, deferred, fixed, or superseded"
            .to_string()
    })?;
    if reason.is_empty() {
        return Err("A disposition reason is required".to_string());
    }
    if reviewer.is_empty() {
        return Err("A reviewer is required".to_string());
    }
    if let Some(value) = expires_at.as_deref() {
        DateTime::parse_from_rfc3339(value)
            .map_err(|error| format!("expires_at must be RFC 3339: {error}"))?;
    }
    let now = Utc::now().to_rfc3339();
    let mut contract = load_finding_dispositions_contract(repository_path)?.unwrap_or(
        QualityFindingDispositionsContract {
            schema_version: FINDING_DISPOSITIONS_SCHEMA.to_string(),
            updated_at: now.clone(),
            dispositions: Vec::new(),
        },
    );
    contract
        .dispositions
        .retain(|item| item.fingerprint != fingerprint);
    contract.dispositions.push(QualityFindingDisposition {
        fingerprint: fingerprint.to_string(),
        status: status.to_string(),
        reason: reason.to_string(),
        reviewer: reviewer.to_string(),
        reviewed_at: now.clone(),
        evidence: evidence
            .into_iter()
            .map(|item| item.trim().chars().take(512).collect::<String>())
            .filter(|item| !item.is_empty())
            .take(16)
            .collect(),
        expires_at,
    });
    contract.updated_at = now;
    validate_finding_dispositions_contract(&mut contract)?;
    let path = repository_path.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
    let parent = path
        .parent()
        .ok_or_else(|| format!("Could not resolve parent directory for {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    let payload = serde_json::to_string_pretty(&contract)
        .map_err(|error| format!("Could not encode finding dispositions: {error}"))?;
    let temporary_path = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&temporary_path, format!("{payload}\n")).map_err(|error| {
        format!(
            "Could not write temporary finding dispositions {}: {error}",
            temporary_path.display()
        )
    })?;
    fs::rename(&temporary_path, &path)
        .map_err(|error| format!("Could not replace {} atomically: {error}", path.display()))?;
    Ok(contract)
}

pub fn evaluate_requirement(
    repository: &RepositorySnapshot,
    requirement: &QualityGateRequirement,
) -> (QualityGateStatus, QualityFreshness, String) {
    let gate_id = normalize_gate_id(&requirement.gate_id);
    let Some(gate) = repository
        .quality
        .gates
        .iter()
        .find(|gate| gate.id == gate_id)
    else {
        return (
            QualityGateStatus::NotConfigured,
            QualityFreshness::Unknown,
            format!("{} has no imported evidence", gate_label(&gate_id)),
        );
    };
    let source_evidence = gate
        .evidence
        .iter()
        .filter(|item| item.source == requirement.source)
        .collect::<Vec<_>>();
    if source_evidence.is_empty() {
        return (
            QualityGateStatus::NotConfigured,
            QualityFreshness::Unknown,
            format!(
                "{} has no {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    let evidence = source_evidence
        .into_iter()
        .filter(|item| {
            requirement
                .minimum_verification_level
                .as_ref()
                .is_none_or(|minimum| item.verification_level.satisfies(minimum))
        })
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        let minimum = requirement
            .minimum_verification_level
            .as_ref()
            .map(QualityVerificationLevel::as_str)
            .unwrap_or("unknown");
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Unknown,
            format!(
                "{} has no {} evidence at or above {} verification",
                gate.label,
                requirement.source.as_str(),
                minimum
            ),
        );
    }
    let has_conflict = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Passed)
        && evidence.iter().any(|item| {
            item.status == QualityGateStatus::Failed || item.status == QualityGateStatus::Blocked
        });
    if has_conflict {
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Conflicted,
            format!(
                "{} has conflicting {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    if evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Blocked)
    {
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Unknown,
            format!(
                "{} is blocked by {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    if evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Failed)
    {
        return (
            QualityGateStatus::Failed,
            evidence_freshness(&evidence),
            format!(
                "{} failed in {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        );
    }
    let freshness = evidence_freshness(&evidence);
    if evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Passed)
    {
        (
            QualityGateStatus::Passed,
            freshness,
            format!(
                "{} passed in {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        )
    } else {
        (
            QualityGateStatus::NotConfigured,
            freshness,
            format!(
                "{} has no passing {} evidence",
                gate.label,
                requirement.source.as_str()
            ),
        )
    }
}

pub fn audit_import(root: Option<&Path>, repositories: &[RepositorySnapshot]) -> AuditImport {
    let Some(root) = root else {
        return AuditImport::default();
    };
    let mut portfolio = QualityPortfolioSnapshot {
        audit_root: Some(root.to_string_lossy().to_string()),
        ..QualityPortfolioSnapshot::default()
    };
    let Some(run) = latest_audit_run(root) else {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    };
    portfolio.latest_audit_id = run.audit_id.clone();
    portfolio.latest_audit_at = run.as_of.clone();
    portfolio.latest_audit_path = Some(run.summary_path.to_string_lossy().to_string());
    portfolio.maturity_score = run.mean_maturity;
    portfolio.maturity_score_display = run.mean_maturity_display.clone();
    portfolio.scored_dimension_count = run.scored_dimension_count;
    portfolio.audit_status = "Ready".to_string();

    let mut matches = HashMap::new();
    for repository in repositories {
        let candidates = run
            .findings
            .iter()
            .filter(|finding| {
                canonical_path_matches(finding.canonical_path.as_deref(), &repository.path)
            })
            .collect::<Vec<_>>();
        let selected = if candidates.len() == 1 {
            candidates.first().copied()
        } else if candidates.is_empty() {
            let remote_key = repository.remote_url.as_deref().and_then(remote_identity);
            let remote_matches = remote_key.as_deref().map_or_else(Vec::new, |key| {
                run.findings
                    .iter()
                    .filter(|finding| finding.remote_key.as_deref() == Some(key))
                    .collect::<Vec<_>>()
            });
            (remote_matches.len() == 1)
                .then(|| remote_matches.first().copied())
                .flatten()
        } else {
            None
        };
        if let Some(finding) = selected {
            let maturity = QualityMaturity {
                score: finding.mean_maturity,
                score_display: finding.mean_maturity_display.clone(),
                scored_dimension_count: finding.scored_dimension_count,
                dimension_scores: finding.dimension_scores.clone(),
                gaps: Vec::new(),
                quality_outcome: None,
                agent_usability: None,
                repository_maturity: None,
                audit_id: run.audit_id.clone(),
                observed_at: run.as_of.clone(),
                scanned_commit: None,
                scanned_branch: None,
                freshness: evaluate_audit_freshness_at(run.as_of.as_deref(), Utc::now()),
                report_path: Some(finding.path.to_string_lossy().to_string()),
            };
            matches.insert(repository.id.clone(), maturity);
        }
    }
    portfolio.matched_repository_count = matches.len();
    AuditImport {
        portfolio,
        maturities: matches,
        behavior_assurance: HashMap::new(),
    }
}

pub fn maturity_feed_import(
    feed_path: Option<&Path>,
    repositories: &[RepositorySnapshot],
) -> AuditImport {
    let Some(feed_path) = feed_path else {
        return AuditImport::default();
    };
    let mut portfolio = QualityPortfolioSnapshot {
        audit_root: Some(feed_path.to_string_lossy().to_string()),
        ..QualityPortfolioSnapshot::default()
    };
    if !feed_path.is_file() || feed_path.is_symlink() {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    }
    let Some(feed) = read_json(feed_path) else {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    };
    if !validate_maturity_feed(&feed) {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    }

    let source = feed.get("source").and_then(Value::as_object);
    let audit_id = source
        .and_then(|value| value.get("audit_id"))
        .and_then(Value::as_str);
    let as_of = source
        .and_then(|value| value.get("as_of"))
        .and_then(Value::as_str);
    let freshness = evaluate_audit_freshness_at(as_of, Utc::now());
    let feed_status = feed
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    portfolio.feed_schema = feed
        .get("schema")
        .and_then(Value::as_str)
        .map(str::to_string);
    portfolio.provenance_hash = feed
        .get("provenance_hash")
        .and_then(Value::as_str)
        .map(str::to_string);
    portfolio.quality_outcome_counts = feed
        .get("quality_outcome_counts")
        .filter(|value| value.is_object())
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    portfolio.quality_outcome_taxonomy = feed
        .get("quality_outcome_taxonomy")
        .filter(|value| value.is_object())
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    portfolio.behavior_assurance = feed
        .get("behavior_assurance")
        .filter(|value| value.is_object())
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    portfolio.latest_audit_id = audit_id.map(str::to_string);
    portfolio.latest_audit_at = as_of.map(str::to_string);
    portfolio.latest_audit_path = Some(feed_path.to_string_lossy().to_string());
    portfolio.maturity_score = feed.get("mean_maturity").and_then(Value::as_f64);
    portfolio.maturity_score_display = portfolio.maturity_score.map(|score| format!("{score:.3}"));
    portfolio.scored_dimension_count = Some(feed_scored_dimension_count(&feed));
    portfolio.measurement_confidence = feed_measurement_confidence(&feed);
    portfolio.audit_status = match freshness {
        QualityFreshness::Fresh if feed_status == "complete_with_blockers" => {
            "Ready with blockers".to_string()
        }
        QualityFreshness::Fresh => "Ready".to_string(),
        QualityFreshness::Stale => "Stale".to_string(),
        QualityFreshness::Unknown | QualityFreshness::Conflicted => "Unknown".to_string(),
    };

    let projections = feed
        .get("repositories")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_object())
        .collect::<Vec<_>>();
    let mut matches = HashMap::new();
    let mut behavior_assurance_matches = HashMap::new();
    for repository in repositories {
        let stable_id = repository_feed_id(repository);
        let projection = projections
            .iter()
            .find(|projection| {
                projection.get("repo_id").and_then(Value::as_str) == Some(stable_id.as_str())
            })
            .or_else(|| {
                projections.iter().find(|projection| {
                    projection
                        .get("local_identity")
                        .and_then(|value| value.get("primary_path"))
                        .and_then(Value::as_str)
                        .is_some_and(|path| canonical_path_matches(Some(path), &repository.path))
                })
            });
        let Some(projection) = projection else {
            continue;
        };
        if let Some(assurance) = projection
            .get("behavior_assurance")
            .filter(|value| value.is_object())
            .and_then(|value| {
                let mut assurance: BehaviorAssuranceRepositoryState =
                    serde_json::from_value(value.clone()).ok()?;
                assurance.normalize_state();
                Some(assurance)
            })
        {
            behavior_assurance_matches.insert(repository.id.clone(), assurance);
        }
        let score = projection.get("maturity_score").and_then(Value::as_f64);
        let projection_freshness = if score.is_some() {
            freshness.clone()
        } else {
            QualityFreshness::Unknown
        };
        let maturity = QualityMaturity {
            score,
            score_display: score.map(|value| format!("{value:.3}")),
            scored_dimension_count: projection
                .get("dimension_scores")
                .and_then(Value::as_object)
                .map(|scores| scores.values().filter(|value| value.is_number()).count() as u64),
            dimension_scores: projection
                .get("dimension_scores")
                .and_then(Value::as_object)
                .map(|scores| {
                    scores
                        .iter()
                        .filter_map(|(dimension, score)| {
                            score.as_f64().map(|value| (dimension.clone(), value))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            gaps: projection
                .get("dimension_gaps")
                .and_then(Value::as_array)
                .map(|gaps| {
                    gaps.iter()
                        .filter_map(|gap| {
                            let gap = gap.as_object()?;
                            Some(QualityMaturityGap {
                                dimension: gap.get("dimension")?.as_str()?.to_string(),
                                status: gap
                                    .get("status")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unknown")
                                    .to_string(),
                                score: gap.get("score").and_then(Value::as_f64),
                                message: gap
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Evidence is incomplete.")
                                    .chars()
                                    .take(240)
                                    .collect(),
                            })
                        })
                        .take(MAX_MATURITY_GAPS)
                        .collect()
                })
                .unwrap_or_default(),
            quality_outcome: projection
                .get("quality_outcome")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            agent_usability: projection
                .get("agent_usability")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            repository_maturity: projection
                .get("repository_maturity")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            audit_id: audit_id.map(str::to_string),
            observed_at: as_of.map(str::to_string),
            scanned_commit: projection
                .get("target_head")
                .and_then(Value::as_str)
                .map(str::to_string),
            scanned_branch: projection
                .get("target_branch")
                .and_then(Value::as_str)
                .map(str::to_string),
            freshness: projection_freshness,
            report_path: Some(feed_path.to_string_lossy().to_string()),
        };
        matches.insert(repository.id.clone(), maturity);
    }
    portfolio.matched_repository_count = matches.len();
    portfolio.behavior_assurance.state_counts.clear();
    for assurance in behavior_assurance_matches.values() {
        *portfolio
            .behavior_assurance
            .state_counts
            .entry(assurance.state.clone())
            .or_insert(0) += 1;
    }
    AuditImport {
        portfolio,
        maturities: matches,
        behavior_assurance: behavior_assurance_matches,
    }
}

fn validate_maturity_feed(feed: &Value) -> bool {
    let Some(feed) = feed.as_object() else {
        return false;
    };
    let feed_schema = feed.get("schema").and_then(Value::as_str);
    if !feed_schema.is_some_and(|schema| MATURITY_FEED_SCHEMAS.contains(&schema))
        || !feed
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| MATURITY_FEED_STATUS.contains(&status))
        || feed.get("feed_timestamp").and_then(Value::as_str).is_none()
    {
        return false;
    }
    let Some(source) = feed.get("source").and_then(Value::as_object) else {
        return false;
    };
    if !has_non_empty_string(source, "audit_id")
        || !has_non_empty_string(source, "as_of")
        || !has_non_empty_string(source, "projects_root")
    {
        return false;
    }
    let Some(replay) = feed.get("replay").and_then(Value::as_object) else {
        return false;
    };
    if replay.get("status").and_then(Value::as_str) != Some("passed")
        || replay.get("deterministic") != Some(&Value::Bool(true))
        || replay.get("source_summary_hash") != source.get("summary_hash")
        || replay.get("replayed_summary_hash") != source.get("summary_hash")
    {
        return false;
    }
    if !valid_measurement_confidence(feed.get("measurement_confidence")) {
        return false;
    }
    let Some(repositories) = feed.get("repositories").and_then(Value::as_array) else {
        return false;
    };
    if repositories.is_empty()
        || feed.get("repository_count").and_then(Value::as_u64) != Some(repositories.len() as u64)
    {
        return false;
    }
    let mut repository_ids = HashSet::new();
    for repository in repositories {
        let Some(repo_id) = repository.get("repo_id").and_then(Value::as_str) else {
            return false;
        };
        if repo_id.is_empty() || !repository_ids.insert(repo_id) {
            return false;
        }
        if feed_schema == Some("quality-runner-maturity-feed/v2")
            && !valid_repository_maturity_projection(repository)
        {
            return false;
        }
    }
    let Some(provenance_hash) = feed.get("provenance_hash").and_then(Value::as_str) else {
        return false;
    };
    if provenance_hash.len() != 64
        || !provenance_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        || maturity_feed_hash(&Value::Object(feed.clone())).as_deref() != Some(provenance_hash)
    {
        return false;
    }
    let Some(privacy) = feed.get("privacy").and_then(Value::as_object) else {
        return false;
    };
    if privacy.get("private_local_feed") != Some(&Value::Bool(true))
        || [
            "raw_paths",
            "raw_prompts",
            "raw_code",
            "raw_diffs",
            "raw_transcripts",
            "credentials",
        ]
        .iter()
        .any(|key| privacy.get(*key) != Some(&Value::Bool(false)))
    {
        return false;
    }
    feed_tree_is_safe(&Value::Object(feed.clone()), None, None)
}

fn valid_measurement_confidence(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let Some(confidence) = value.as_object() else {
        return false;
    };
    let level = confidence.get("level").and_then(Value::as_str);
    if !matches!(level, Some("low" | "medium" | "high"))
        || confidence.get("deterministic_replay") != Some(&Value::Bool(true))
        || !json_string_array(confidence.get("basis"))
        || !json_string_array(confidence.get("limitations"))
    {
        return false;
    }
    let Some(population) = confidence
        .get("population_coverage")
        .and_then(Value::as_object)
    else {
        return false;
    };
    let expected = population
        .get("expected_repository_count")
        .and_then(Value::as_u64);
    let observed = population
        .get("observed_repository_count")
        .and_then(Value::as_u64);
    let gaps = confidence
        .get("unresolved_measurement_gap_count")
        .and_then(Value::as_u64);
    if expected.is_none()
        || observed.is_none()
        || gaps.is_none()
        || population
            .get("excluded_repository_count")
            .and_then(Value::as_u64)
            .is_none()
    {
        return false;
    }
    level != Some("high")
        || (population.get("status").and_then(Value::as_str) == Some("complete")
            && expected == observed
            && gaps == Some(0)
            && confidence
                .get("limitations")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty))
}

fn feed_measurement_confidence(feed: &Value) -> Option<QualityMeasurementConfidence> {
    let confidence = feed.get("measurement_confidence")?.as_object()?;
    let population = confidence.get("population_coverage")?.as_object()?;
    Some(QualityMeasurementConfidence {
        level: confidence.get("level")?.as_str()?.to_string(),
        basis: json_string_values(confidence.get("basis")),
        limitations: json_string_values(confidence.get("limitations")),
        population_status: population.get("status")?.as_str()?.to_string(),
        expected_repository_count: population.get("expected_repository_count")?.as_u64()?,
        observed_repository_count: population.get("observed_repository_count")?.as_u64()?,
        excluded_repository_count: population.get("excluded_repository_count")?.as_u64()?,
        unresolved_measurement_gap_count: confidence
            .get("unresolved_measurement_gap_count")?
            .as_u64()?,
        deterministic_replay: confidence.get("deterministic_replay")?.as_bool()?,
    })
}

fn json_string_array(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().all(Value::is_string))
}

fn json_string_values(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn valid_repository_maturity_projection(repository: &Value) -> bool {
    let Some(model) = repository
        .get("repository_maturity")
        .and_then(Value::as_object)
    else {
        return false;
    };
    if model.get("schema").and_then(Value::as_str) != Some("quality-runner-repository-maturity/v2")
        || model.get("score").and_then(Value::as_f64)
            != repository.get("maturity_score").and_then(Value::as_f64)
    {
        return false;
    }
    let Some(pillars) = model.get("pillars").and_then(Value::as_array) else {
        return false;
    };
    let expected = [
        "correctness_reliability",
        "security_privacy_supply_chain",
        "maintainability_evolvability",
        "operability_release_safety",
        "user_facing_quality",
        "human_agent_usability",
        "governance_sustainability",
    ];
    pillars.len() == expected.len()
        && pillars.iter().zip(expected).all(|(pillar, expected_id)| {
            pillar.get("id").and_then(Value::as_str) == Some(expected_id)
                && pillar
                    .get("weight")
                    .and_then(Value::as_f64)
                    .is_some_and(|weight| weight.is_finite() && weight > 0.0)
                && pillar.get("score").is_some_and(|score| {
                    score.is_null()
                        || score
                            .as_f64()
                            .is_some_and(|value| value.is_finite() && (0.0..=4.0).contains(&value))
                })
        })
        && (pillars
            .iter()
            .filter_map(|pillar| pillar.get("weight").and_then(Value::as_f64))
            .sum::<f64>()
            - 1.0)
            .abs()
            < 0.000_001
}

fn maturity_feed_hash(feed: &Value) -> Option<String> {
    let mut content = feed.clone();
    content.as_object_mut()?.remove("provenance_hash");
    let payload = serde_json::to_string(&content).ok()?;
    let digest = Sha256::digest(payload.as_bytes());
    Some(format!("{digest:x}"))
}

fn has_non_empty_string(value: &serde_json::Map<String, Value>, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|item| !item.trim().is_empty())
}

fn feed_tree_is_safe(value: &Value, key: Option<&str>, parent_key: Option<&str>) -> bool {
    if let Some(key) = key {
        let normalized = key.to_ascii_lowercase();
        if MATURITY_FEED_FORBIDDEN_KEYS.contains(&normalized.as_str())
            || normalized == "raw_credentials"
            || normalized == "raw_output"
            || normalized == "command_output"
            || normalized == "stdout"
            || normalized == "stderr"
        {
            let privacy_flag = parent_key == Some("privacy")
                && [
                    "raw_paths",
                    "raw_prompts",
                    "raw_code",
                    "raw_diffs",
                    "raw_transcripts",
                    "credentials",
                ]
                .contains(&normalized.as_str());
            if !privacy_flag {
                return false;
            }
        }
    }
    match value {
        Value::Object(object) => object
            .iter()
            .all(|(child_key, child_value)| feed_tree_is_safe(child_value, Some(child_key), key)),
        Value::Array(array) => array
            .iter()
            .all(|child| feed_tree_is_safe(child, None, key)),
        _ => true,
    }
}

fn feed_scored_dimension_count(feed: &Value) -> u64 {
    feed.get("repositories")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|repository| repository.get("dimension_scores"))
        .filter_map(Value::as_object)
        .map(|scores| scores.values().filter(|value| value.is_number()).count() as u64)
        .sum()
}

fn repository_feed_id(repository: &RepositorySnapshot) -> String {
    let identity = repository_identity_key(repository);
    let payload = serde_json::to_string(&[identity]).unwrap_or_else(|_| "[]".to_string());
    let digest = Sha256::digest(payload.as_bytes());
    let hex = format!("{digest:x}");
    format!("repo-{}", &hex[..16])
}

fn repository_identity_key(repository: &RepositorySnapshot) -> String {
    if let Some(origin) = repository.remote_url.as_deref().and_then(normalized_origin) {
        return format!("origin:{origin}");
    }
    if let Some(common) = common_git_dir(Path::new(&repository.path)) {
        return format!("common:{common}");
    }
    format!("path:{}", identity_path(&repository.path))
}

fn normalized_origin(value: &str) -> Option<String> {
    let mut value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    if let Some(stripped) = value.strip_prefix("git@") {
        let (host, path) = stripped.split_once(':')?;
        return Some(format!("{host}/{}", strip_git_suffix(path)));
    }
    for scheme in ["https://", "http://", "ssh://", "git://"] {
        if let Some(stripped) = value.strip_prefix(scheme) {
            value = stripped.to_string();
            break;
        }
    }
    if let Some(at) = value.find('@') {
        value = value[at + 1..].to_string();
    }
    let value = value.trim_start_matches('/');
    let (host, path) = value.split_once('/')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{host}/{}", strip_git_suffix(path)))
}

fn strip_git_suffix(value: &str) -> String {
    value
        .trim_matches('/')
        .strip_suffix(".git")
        .unwrap_or(value.trim_matches('/'))
        .trim_matches('/')
        .to_string()
}

fn common_git_dir(path: &Path) -> Option<String> {
    let mut child = Command::new("git")
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + QUALITY_GIT_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(StdDuration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    Some(
        fs::canonicalize(&path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string(),
    )
}

fn identity_path(path: &str) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .to_string()
}

pub fn safe_report_path(candidate: &Path, allowed_roots: &[PathBuf]) -> Result<PathBuf, String> {
    let canonical_candidate = fs::canonicalize(candidate)
        .map_err(|error| format!("Quality report is not available: {error}"))?;
    let allowed = allowed_roots
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .collect::<Vec<_>>();
    if allowed
        .iter()
        .any(|root| canonical_candidate.starts_with(root))
    {
        Ok(canonical_candidate)
    } else {
        Err("Quality reports can only be opened from configured QR or audit roots".to_string())
    }
}

fn add_evidence(gates: &mut Vec<QualityGate>, evidence: QualityEvidence) {
    let id = evidence.id.clone();
    let label = gate_label(&id);
    if let Some(gate) = gates.iter_mut().find(|gate| gate.id == id) {
        if gate.label == gate_label(&id) && label != gate.label {
            gate.label = label;
        }
        gate.evidence.push(evidence);
    } else {
        gates.push(QualityGate {
            id,
            label,
            status: QualityGateStatus::NotConfigured,
            freshness: QualityFreshness::Unknown,
            evidence: vec![evidence],
        });
    }
    gates.sort_by_key(|gate| gate_sort_key(&gate.id));
}

fn gate_sort_key(id: &str) -> (usize, String) {
    let canonical_index = CANONICAL_GATE_DEFINITIONS
        .iter()
        .position(|(candidate, _)| *candidate == id);
    if let Some(index) = canonical_index {
        return (index, id.to_string());
    }
    let conditional_index = CONDITIONAL_GATE_DEFINITIONS
        .iter()
        .position(|(candidate, _)| *candidate == id);
    (
        CANONICAL_GATE_DEFINITIONS.len()
            + conditional_index.unwrap_or(CONDITIONAL_GATE_DEFINITIONS.len()),
        id.to_string(),
    )
}

fn evidence_freshness(evidence: &[&QualityEvidence]) -> QualityFreshness {
    if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Conflicted)
    {
        QualityFreshness::Conflicted
    } else if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Fresh)
    {
        QualityFreshness::Fresh
    } else if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Stale)
    {
        QualityFreshness::Stale
    } else {
        QualityFreshness::Unknown
    }
}

fn ci_evidence(
    repository: &RepositorySnapshot,
    remote: Option<&RemoteRepositorySnapshot>,
) -> Vec<QualityEvidence> {
    let mut checks = Vec::<(&CheckSnapshot, Option<&str>, Option<&str>)>::new();
    if let Some(pull_request) = repository
        .pull_requests
        .iter()
        .filter(|pull_request| pull_request.head_branch == repository.branch)
        .find(|pull_request| !pull_request.checks.is_empty())
    {
        checks.extend(pull_request.checks.iter().map(|check| {
            (
                check,
                Some(pull_request.head_branch.as_str()),
                pull_request.head_commit.as_deref(),
            )
        }));
    } else if let Some(remote) = remote {
        checks.extend(remote.ci_checks.iter().map(|check| {
            (
                check,
                remote.ci_branch.as_deref(),
                check.head_sha.as_deref(),
            )
        }));
    }
    checks
        .into_iter()
        .map(|(check, branch, commit)| {
            let id = normalize_gate_id(&check.context);
            let status = github_check_status(check);
            let freshness = evaluate_freshness_at(
                Some(&check.last_refreshed_at),
                commit,
                branch,
                repository.workspace.last_commit.as_deref(),
                Some(repository.branch.as_str()),
                Utc::now(),
            );
            QualityEvidence {
                id,
                source: QualitySource::Ci,
                status,
                freshness,
                observed_at: Some(check.last_refreshed_at.clone()),
                scanned_commit: commit.map(str::to_string),
                scanned_branch: branch.map(str::to_string),
                command: None,
                source_label: format!("GitHub check · {}", check.context),
                report_path: None,
                report_url: check.html_url.clone(),
                report_kind: Some("GitHub check run".to_string()),
                detail: check
                    .conclusion
                    .clone()
                    .unwrap_or_else(|| check.state.clone()),
                verification_level: QualityVerificationLevel::SourceInferred,
                target_kind: Some("source".to_string()),
                target_url: None,
                target_provider: Some("github".to_string()),
                deployment_id: None,
            }
        })
        .collect()
}

fn github_check_status(check: &CheckSnapshot) -> QualityGateStatus {
    let state = check.state.to_ascii_lowercase();
    let conclusion = check
        .conclusion
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        conclusion.as_str(),
        "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure"
    ) {
        QualityGateStatus::Failed
    } else if matches!(conclusion.as_str(), "success" | "neutral") {
        QualityGateStatus::Passed
    } else if state == "completed" && conclusion == "skipped" {
        QualityGateStatus::Blocked
    } else {
        QualityGateStatus::Blocked
    }
}

struct QrRun {
    run_dir: PathBuf,
    manifest: Value,
    verification: Value,
    execution_plan: Value,
    capability_matrix: Value,
    repo_scan: Value,
    observed_at: Option<String>,
}

fn repository_provenance_for_branch<'a>(
    repository: &'a RepositorySnapshot,
    scanned_branch: Option<&str>,
) -> (Option<&'a str>, Option<&'a str>) {
    if let Some(scanned_branch) = scanned_branch {
        if let Some(branch) = repository
            .branches
            .iter()
            .find(|branch| branch.name == scanned_branch)
        {
            return (branch.last_commit.as_deref(), Some(branch.name.as_str()));
        }
    }
    (
        repository.workspace.last_commit.as_deref(),
        Some(repository.branch.as_str()),
    )
}

impl QrRun {
    fn finding_reports(&self) -> Vec<(PathBuf, Value)> {
        let report_names = [
            "code-quality-scan.json",
            "quality-audit.json",
            "completed-report.json",
            "repo-scan.json",
            "run-summary.json",
        ];
        let mut run_dirs = vec![self.run_dir.clone()];

        let publication = read_json(&self.run_dir.join("fleet-detector-publication.json"));
        let is_fleet_detector_run = publication.as_ref().is_some_and(|payload| {
            json_string_at(payload, &["schema"]).as_deref()
                == Some("quality-runner-fleet-detector-publication/v1")
        });
        if is_fleet_detector_run {
            let run_name = self.run_dir.file_name().and_then(|name| name.to_str());
            let group_name = run_name.and_then(|name| {
                ["-inspect", "-run", "-verify"]
                    .iter()
                    .find_map(|suffix| name.strip_suffix(suffix))
            });
            if let (Some(parent), Some(group_name)) = (self.run_dir.parent(), group_name) {
                let mut siblings = fs::read_dir(parent)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path != &self.run_dir)
                    .filter(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| {
                                ["-inspect", "-run", "-verify"]
                                    .iter()
                                    .any(|suffix| name == format!("{group_name}{suffix}"))
                            })
                    })
                    .filter(|path| {
                        read_json(&path.join("fleet-detector-publication.json")).is_some_and(
                            |payload| {
                                json_string_at(&payload, &["schema"]).as_deref()
                                    == Some("quality-runner-fleet-detector-publication/v1")
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                siblings.sort();
                run_dirs.extend(siblings);
            }
        }

        let mut seen = HashSet::new();
        report_names
            .iter()
            .flat_map(|name| {
                run_dirs.iter().filter_map(move |run_dir| {
                    let path = run_dir.join(name);
                    read_json(&path).map(|payload| (path, payload))
                })
            })
            .filter(|(path, _)| seen.insert(path.clone()))
            .collect()
    }

    fn configured_gate_ids(&self) -> Vec<String> {
        let mut configured_gate_ids = Vec::new();
        let mut seen_gate_ids = HashSet::new();
        let mut append_entries = |entries: Option<&Vec<Value>>| {
            if let Some(entries) = entries {
                for entry in entries {
                    let Some(raw_id) =
                        json_string_at(entry, &["id"]).or_else(|| json_string_at(entry, &["name"]))
                    else {
                        continue;
                    };
                    let id = normalize_gate_id(&raw_id);
                    if seen_gate_ids.insert(id.clone()) {
                        configured_gate_ids.push(id);
                    }
                }
            }
        };
        append_entries(
            self.capability_matrix
                .get("available")
                .and_then(Value::as_array),
        );
        append_entries(
            self.repo_scan
                .get("quality_commands")
                .and_then(Value::as_array),
        );
        append_entries(self.verification.get("gates").and_then(Value::as_array));
        append_entries(
            self.verification
                .get("execution_plan")
                .and_then(Value::as_array),
        );
        append_entries(self.execution_plan.as_array());
        configured_gate_ids
    }

    fn gate_evidence(&self, repository: &RepositorySnapshot) -> Vec<QualityEvidence> {
        let branch = self.branch();
        let commit = self.commit();
        let (current_commit, current_branch) =
            repository_provenance_for_branch(repository, branch.as_deref());
        let report_path = artifact_path(&self.run_dir, "gate-verification.json")
            .or_else(|| artifact_path(&self.run_dir, "gate-execution-plan.json"));
        let observed_at = self.observed_at.clone();
        let mut entries = self
            .verification
            .get("gates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if entries.is_empty() {
            entries = self
                .verification
                .get("execution_plan")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        if entries.is_empty() {
            entries = self.execution_plan.as_array().cloned().unwrap_or_default();
        }
        entries
            .into_iter()
            .filter_map(|gate| {
                let raw_id =
                    json_string_at(&gate, &["id"]).or_else(|| json_string_at(&gate, &["name"]))?;
                let id = normalize_gate_id(&raw_id);
                let capability_kind = json_string_at(&gate, &["capability_kind"])
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let source = QualitySource::parse(
                    json_string_at(&gate, &["source"])
                        .as_deref()
                        .unwrap_or_default(),
                )
                .or_else(|| (capability_kind == "ci_only").then_some(QualitySource::Ci))
                .unwrap_or_else(|| {
                    if capability_kind == "local_command"
                        || capability_kind == "command"
                        || json_string_at(&gate, &["command"]).is_some()
                    {
                        QualitySource::Local
                    } else {
                        QualitySource::Qr
                    }
                });
                let mut status = parse_qr_status(json_string_at(&gate, &["status"]).as_deref());
                if matches!(
                    capability_kind.as_str(),
                    "evidence" | "evidence_file" | "agent_review" | "ci_only"
                ) {
                    status = QualityGateStatus::Blocked;
                }
                let failure_type = json_string_at(&gate, &["failure_type"]);
                let skip_type = json_string_at(&gate, &["skip_type"]);
                if skip_type.is_some() {
                    status = QualityGateStatus::Blocked;
                }
                let gate_observed_at = json_string_at(&gate, &["completed_at"])
                    .or_else(|| json_string_at(&gate, &["observed_at"]))
                    .or_else(|| observed_at.clone());
                let freshness = evaluate_freshness_at(
                    gate_observed_at.as_deref(),
                    commit.as_deref(),
                    branch.as_deref(),
                    current_commit,
                    current_branch,
                    Utc::now(),
                );
                let command = json_string_at(&gate, &["command"]);
                let source_name = json_string_at(&gate, &["source"])
                    .or_else(|| command.clone())
                    .unwrap_or_else(|| "QR gate-verification".to_string());
                let detail = failure_type.or(skip_type).unwrap_or_else(|| {
                    json_string_at(&gate, &["status"]).unwrap_or_else(|| "No result".to_string())
                });
                Some(QualityEvidence {
                    id,
                    source,
                    status,
                    freshness,
                    observed_at: gate_observed_at,
                    scanned_commit: commit.clone(),
                    scanned_branch: branch.clone(),
                    command,
                    source_label: format!(
                        "{} · {}",
                        gate_label(&normalize_gate_id(&raw_id)),
                        source_name
                    ),
                    report_path: report_path.clone(),
                    report_url: None,
                    report_kind: Some("QR gate verification".to_string()),
                    detail,
                    verification_level: QualityVerificationLevel::SourceInferred,
                    target_kind: Some("source".to_string()),
                    target_url: None,
                    target_provider: None,
                    deployment_id: None,
                })
            })
            .collect()
    }

    fn branch(&self) -> Option<String> {
        json_string_at(&self.manifest, &["git", "branch"])
            .or_else(|| json_string_at(&self.manifest, &["git_provenance", "branch"]))
            .or_else(|| json_string_at(&self.manifest, &["provenance", "branch"]))
            .or_else(|| json_string_at(&self.manifest, &["branch"]))
            .or_else(|| json_string_at(&self.verification, &["provenance", "branch"]))
    }

    fn commit(&self) -> Option<String> {
        json_string_at(&self.manifest, &["git", "head_sha"])
            .or_else(|| json_string_at(&self.manifest, &["git_provenance", "head_sha"]))
            .or_else(|| json_string_at(&self.manifest, &["provenance", "head_sha"]))
            .or_else(|| json_string_at(&self.manifest, &["head_sha"]))
            .or_else(|| json_string_at(&self.verification, &["provenance", "head_sha"]))
    }

    fn findings(&self, repository: &RepositorySnapshot) -> QualityFindings {
        let reports = self.finding_reports();
        let Some((report_path, payload)) = reports.first() else {
            let detector_path = self.run_dir.join("anti-slop-detector.json");
            if let Some(payload) = read_json(&detector_path) {
                let branch = self.branch();
                let commit = self.commit();
                let mut findings = QualityFindings {
                    source: Some(QualitySource::Qr),
                    observed_at: self.observed_at.clone(),
                    scanned_commit: commit.clone(),
                    scanned_branch: branch.clone(),
                    target_sha: json_string_at(&payload, &["target_sha"]),
                    freshness: evaluate_freshness_at(
                        self.observed_at.as_deref(),
                        commit.as_deref(),
                        branch.as_deref(),
                        repository.workspace.last_commit.as_deref(),
                        Some(repository.branch.as_str()),
                        Utc::now(),
                    ),
                    report_path: Some(detector_path.to_string_lossy().to_string()),
                    ..QualityFindings::default()
                };
                apply_detector_evidence(&mut findings, &payload);
                return findings;
            }
            return QualityFindings::default();
        };
        let mut merged_findings = Vec::new();
        let mut prior_report_fingerprints = HashSet::new();
        for (_, report) in &reports {
            let mut current_report_fingerprints = HashSet::new();
            for finding in report
                .get("findings")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let fingerprint = finding
                    .get("fingerprint")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                if fingerprint.is_some_and(|value| prior_report_fingerprints.contains(value)) {
                    continue;
                }
                if let Some(fingerprint) = fingerprint {
                    current_report_fingerprints.insert(fingerprint.to_string());
                }
                merged_findings.push(finding.clone());
            }
            prior_report_fingerprints.extend(current_report_fingerprints);
        }
        let (total, severity_counts) = if merged_findings.is_empty() {
            let severity_counts = severity_counts(payload);
            let total = json_u64_at(payload, &["finding_count"])
                .or_else(|| json_u64_at(payload, &["summary", "finding_count"]))
                .or_else(|| json_u64_at(payload, &["finding_counts", "total"]))
                .or_else(|| json_u64_at(payload, &["summary", "finding_counts", "total"]))
                .unwrap_or_else(|| severity_counts.values().sum());
            (total, severity_counts)
        } else {
            (
                merged_findings.len() as u64,
                fleet_severity_counts(&merged_findings),
            )
        };
        let high_severity_total = severity_counts
            .iter()
            .filter(|(severity, _)| matches!(severity.as_str(), "critical" | "high"))
            .map(|(_, count)| *count)
            .sum();
        let branch = self.branch();
        let commit = self.commit();
        let (current_commit, current_branch) =
            repository_provenance_for_branch(repository, branch.as_deref());
        let mut findings = QualityFindings {
            total,
            severity_counts,
            high_severity_total,
            source: Some(QualitySource::Qr),
            observed_at: self.observed_at.clone(),
            scanned_commit: commit.clone(),
            scanned_branch: branch.clone(),
            freshness: evaluate_freshness_at(
                self.observed_at.as_deref(),
                commit.as_deref(),
                branch.as_deref(),
                current_commit,
                current_branch,
                Utc::now(),
            ),
            report_path: Some(report_path.to_string_lossy().to_string()),
            report_paths: reports
                .iter()
                .map(|(path, _)| path.to_string_lossy().to_string())
                .collect(),
            ..QualityFindings::default()
        };
        apply_detector_evidence(&mut findings, &payload);
        findings
    }
}

fn latest_qr_run(repository_path: &Path) -> Option<QrRun> {
    let runs = repository_path.join(".quality-runner").join("runs");
    let entries = fs::read_dir(runs).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let run_dir = entry.path();
            let manifest = read_json(&run_dir.join("run-manifest.json"))?;
            let verification =
                read_json(&run_dir.join("gate-verification.json")).unwrap_or(Value::Null);
            let execution_plan =
                read_json(&run_dir.join("gate-execution-plan.json")).unwrap_or(Value::Null);
            let capability_matrix =
                read_json(&run_dir.join("capability-matrix.json")).unwrap_or(Value::Null);
            let repo_scan = read_json(&run_dir.join("repo-scan.json")).unwrap_or(Value::Null);
            let observed_at = json_string_at(&manifest, &["created_at"])
                .or_else(|| json_string_at(&manifest, &["started_at"]))
                .or_else(|| json_string_at(&manifest, &["completed_at"]))
                .or_else(|| json_string_at(&manifest, &["finished_at"]))
                .or_else(|| json_string_at(&manifest, &["generated_at"]))
                .or_else(|| json_string_at(&manifest, &["as_of"]))
                .or_else(|| json_string_at(&verification, &["provenance", "captured_at"]));
            Some(QrRun {
                run_dir,
                manifest,
                verification,
                execution_plan,
                capability_matrix,
                repo_scan,
                observed_at,
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

#[derive(Debug, Clone)]
struct AuditFinding {
    path: PathBuf,
    canonical_path: Option<String>,
    remote_key: Option<String>,
    mean_maturity: Option<f64>,
    mean_maturity_display: Option<String>,
    scored_dimension_count: Option<u64>,
    dimension_scores: BTreeMap<String, f64>,
}

#[derive(Debug)]
struct AuditRun {
    audit_id: Option<String>,
    as_of: Option<String>,
    summary_path: PathBuf,
    mean_maturity: Option<f64>,
    mean_maturity_display: Option<String>,
    scored_dimension_count: Option<u64>,
    findings: Vec<AuditFinding>,
}

fn latest_audit_run(root: &Path) -> Option<AuditRun> {
    let mut directories = Vec::new();
    if root.join("summary.json").is_file() {
        directories.push(root.to_path_buf());
    } else {
        directories = fs::read_dir(root)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect();
    }
    let mut runs = directories
        .into_iter()
        .filter_map(|directory| parse_audit_run(&directory))
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| {
        left.as_of
            .cmp(&right.as_of)
            .then_with(|| left.summary_path.cmp(&right.summary_path))
    });
    runs.pop()
}

fn parse_audit_run(directory: &Path) -> Option<AuditRun> {
    let summary_path = directory.join("summary.json");
    let summary = read_json(&summary_path)?;
    let as_of = json_string_at(&summary, &["as_of"]);
    if as_of
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_none()
    {
        return None;
    }
    let findings = fs::read_dir(directory.join("findings"))
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .filter_map(|entry| {
            let path = entry.path();
            let payload = read_json(&path)?;
            Some(AuditFinding {
                path,
                canonical_path: json_string_at(&payload, &["canonical_path"])
                    .or_else(|| json_string_at(&payload, &["path"])),
                remote_key: [
                    "remote_key",
                    "remote_url",
                    "remote_identity",
                    "identity_key",
                ]
                .iter()
                .find_map(|key| json_string_at(&payload, &[*key]))
                .and_then(|value| remote_identity(&value)),
                mean_maturity: json_number_at(&payload, &["mean_maturity"]),
                mean_maturity_display: json_display_at(&payload, &["mean_maturity"]),
                scored_dimension_count: json_u64_at(&payload, &["scored_dimension_count"]).or_else(
                    || {
                        payload
                            .get("dimension_results")
                            .and_then(Value::as_array)
                            .map(|values| values.len() as u64)
                    },
                ),
                dimension_scores: payload
                    .get("dimension_scores")
                    .and_then(Value::as_object)
                    .map(|scores| {
                        scores
                            .iter()
                            .filter_map(|(dimension, score)| {
                                score.as_f64().map(|value| (dimension.clone(), value))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    Some(AuditRun {
        audit_id: json_string_at(&summary, &["audit_id"]),
        as_of,
        summary_path,
        mean_maturity: json_number_at(&summary, &["mean_maturity"]),
        mean_maturity_display: json_display_at(&summary, &["mean_maturity"]),
        scored_dimension_count: json_u64_at(&summary, &["scored_dimension_count"]),
        findings,
    })
}

fn canonical_path_matches(candidate: Option<&str>, repository_path: &str) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    canonical_path(candidate) == canonical_path(repository_path)
}

fn canonical_path(path: &str) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn remote_identity(value: &str) -> Option<String> {
    let mut normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if let Some(stripped) = normalized.strip_prefix("git@") {
        normalized = stripped.replacen(':', "/", 1);
    } else {
        for prefix in ["https://", "http://"] {
            if let Some(stripped) = normalized.strip_prefix(prefix) {
                normalized = stripped.to_string();
                break;
            }
        }
    }
    if let Some(stripped) = normalized.strip_prefix("github.com/") {
        normalized = stripped.to_string();
    }
    Some(
        normalized
            .trim_end_matches(".git")
            .trim_end_matches('/')
            .to_string(),
    )
}

fn parse_qr_status(value: Option<&str>) -> QualityGateStatus {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "passed" | "pass" | "success" => QualityGateStatus::Passed,
        "failed" | "fail" | "failure" => QualityGateStatus::Failed,
        "not configured" | "not_configured" | "unconfigured" => QualityGateStatus::NotConfigured,
        _ => QualityGateStatus::Blocked,
    }
}

fn severity_counts(payload: &Value) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    merge_severity_counts(&mut counts, payload.get("severity_counts"));
    merge_severity_counts(
        &mut counts,
        payload
            .get("finding_counts")
            .and_then(|value| value.get("severity_counts")),
    );
    if counts.is_empty() {
        merge_severity_counts(
            &mut counts,
            payload
                .get("summary")
                .and_then(|summary| summary.get("severity_counts")),
        );
        merge_severity_counts(
            &mut counts,
            payload
                .get("summary")
                .and_then(|summary| summary.get("finding_counts"))
                .and_then(|value| value.get("severity_counts")),
        );
    }
    if counts.is_empty() {
        for finding in payload
            .get("findings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let severity = finding
                .get("severity")
                .or_else(|| finding.get("priority"))
                .and_then(Value::as_str)
                .map(normalize_severity);
            if let Some(severity) = severity {
                *counts.entry(severity).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn fleet_dimension_scores(findings: &[Value]) -> (BTreeMap<String, f64>, Option<f64>) {
    let mut scores = BTreeMap::new();
    for finding in findings {
        if finding
            .get("applicable")
            .and_then(Value::as_bool)
            .is_some_and(|applicable| !applicable)
        {
            continue;
        }
        let Some(dimension) = finding.get("dimension").and_then(Value::as_str) else {
            continue;
        };
        let Some(score) = finding.get("score").and_then(Value::as_f64) else {
            continue;
        };
        scores.insert(dimension.to_string(), score);
    }
    let mean = (!scores.is_empty()).then(|| {
        let total = scores.values().sum::<f64>();
        total / scores.len() as f64
    });
    (scores, mean)
}

fn fleet_score(scores: &BTreeMap<String, f64>) -> Option<f64> {
    (!scores.is_empty()).then(|| scores.values().sum::<f64>() / scores.len() as f64)
}

fn merge_agent_usability_dimensions(
    scores: &mut BTreeMap<String, f64>,
    gaps: &mut Vec<QualityMaturityGap>,
    assessment: &AgentUsabilityMaturity,
) {
    if assessment.applicability == "not_applicable" || assessment.status == "not_applicable" {
        return;
    }
    for lane in assessment.lanes.iter().filter(|lane| lane.applicable) {
        let Some(score) = lane
            .score
            .filter(|score| score.is_finite() && (0.0..=4.0).contains(score))
        else {
            continue;
        };
        merge_agent_usability_dimension(
            scores,
            gaps,
            format!("agent_usability.{}", lane.id),
            score,
            &lane.status,
            &lane.message,
        );
    }
    let growth = &assessment.growth_health;
    let growth_score = growth.score.or(match growth.status.as_str() {
        "blocked" => Some(0.0),
        "attention" => Some(2.0),
        "healthy" => Some(4.0),
        _ => None,
    });
    if let Some(score) = growth_score.filter(|score| score.is_finite()) {
        merge_agent_usability_dimension(
            scores,
            gaps,
            "agent_usability.growth_health".to_string(),
            score,
            &growth.status,
            &growth.message,
        );
    }
}

fn merge_agent_usability_dimension(
    scores: &mut BTreeMap<String, f64>,
    gaps: &mut Vec<QualityMaturityGap>,
    dimension: String,
    score: f64,
    status: &str,
    message: &str,
) {
    scores.insert(dimension.clone(), score);
    if score < 4.0 {
        gaps.push(QualityMaturityGap {
            dimension,
            status: status.to_string(),
            score: Some(score),
            message: message.chars().take(240).collect(),
        });
    }
}

fn fleet_maturity_gaps(findings: &[Value]) -> Vec<QualityMaturityGap> {
    findings
        .iter()
        .filter(|finding| {
            finding.get("status").and_then(Value::as_str) != Some("not_applicable")
                && finding
                    .get("score")
                    .and_then(Value::as_f64)
                    .map_or(true, |score| score < 4.0)
        })
        .filter_map(|finding| {
            Some(QualityMaturityGap {
                dimension: finding.get("dimension")?.as_str()?.to_string(),
                status: finding
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                score: finding.get("score").and_then(Value::as_f64),
                message: finding
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Evidence is incomplete.")
                    .chars()
                    .take(240)
                    .collect(),
            })
        })
        .collect()
}

fn fleet_severity_counts(findings: &[Value]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        let severity = finding
            .get("severity")
            .or_else(|| finding.get("priority"))
            .and_then(Value::as_str)
            .map(normalize_severity)
            .unwrap_or_else(|| "unknown".to_string());
        *counts.entry(severity).or_insert(0) += 1;
    }
    counts
}

fn merge_severity_counts(counts: &mut BTreeMap<String, u64>, value: Option<&Value>) {
    let Some(object) = value.and_then(Value::as_object) else {
        return;
    };
    for (severity, count) in object {
        if let Some(count) = count.as_u64() {
            counts.insert(normalize_severity(severity), count);
        }
    }
}

fn normalize_severity(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "blocker" | "critical" | "crit" => "critical".to_string(),
        "high" | "error" => "high".to_string(),
        "medium" | "warning" | "warn" => "medium".to_string(),
        "low" | "info" | "informational" => "low".to_string(),
        other => other.to_string(),
    }
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    for character in value.trim().to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_string()
}

fn read_json(path: &Path) -> Option<Value> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn artifact_path(run_dir: &Path, name: &str) -> Option<String> {
    let path = run_dir.join(name);
    path.is_file().then(|| path.to_string_lossy().to_string())
}

fn json_string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_str().map(str::to_string)
}

fn json_number_at(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_f64()
}

fn json_display_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    match current {
        Value::Number(number) => Some(number.to_string()),
        Value::String(string) => Some(string.clone()),
        _ => None,
    }
}

fn json_u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_u64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn fixture_root() -> PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("pronto-quality-{id}"));
        fs::create_dir_all(&path).expect("fixture root should be writable");
        path
    }

    fn evidence(
        source: QualitySource,
        status: QualityGateStatus,
        freshness: QualityFreshness,
    ) -> QualityEvidence {
        QualityEvidence {
            id: "lint".to_string(),
            source,
            status,
            freshness,
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
            command: None,
            source_label: "fixture".to_string(),
            report_path: None,
            report_url: None,
            report_kind: None,
            detail: String::new(),
            verification_level: QualityVerificationLevel::Unknown,
            target_kind: None,
            target_url: None,
            target_provider: None,
            deployment_id: None,
        }
    }

    fn fixture_repository(path: &Path) -> RepositorySnapshot {
        serde_json::from_value(serde_json::json!({
            "id": "repo-1",
            "name": "repo",
            "path": path.to_string_lossy(),
            "locality": "Local",
            "lifecycle": "Active",
            "lifecycle_candidate": "Active",
            "provider_state": "Unknown",
            "branch": "main",
            "workspace": {
                "id": "w",
                "path": path.to_string_lossy(),
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
                "last_commit": "abc",
                "integration_state": "Unknown",
                "target_branch": null,
                "target_confidence": "Unknown",
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
            "last_scan_at": "2026-07-26T11:00:00Z",
            "last_fetch_at": null,
            "last_activity_at": null
        }))
        .expect("repository fixture should decode")
    }

    fn fixture_maturity_feed(repository: &RepositorySnapshot, as_of: &str) -> Value {
        let summary_hash = "b".repeat(64);
        let mut feed = serde_json::json!({
            "schema": MATURITY_FEED_SCHEMAS[0],
            "status": "completed",
            "feed_timestamp": as_of,
            "generated_at": as_of,
            "source": {
                "audit_id": "audit-fixture",
                "as_of": as_of,
                "projects_root": "/tmp/projects",
                "artifact_schema": "quality-runner-fleet-audit-v0.1",
                "summary_hash": summary_hash,
            },
            "replay": {
                "status": "passed",
                "deterministic": true,
                "source_summary_hash": "b".repeat(64),
                "replayed_summary_hash": "b".repeat(64),
            },
            "repository_count": 1,
            "checkout_count": 1,
            "mean_maturity": 3.5,
            "measurement_confidence": {
                "level": "high",
                "basis": ["population_complete", "dynamic_verification_conclusive"],
                "limitations": [],
                "population_coverage": {
                    "status": "complete",
                    "expected_repository_count": 1,
                    "observed_repository_count": 1,
                    "excluded_repository_count": 2,
                },
                "unresolved_measurement_gap_count": 0,
                "deterministic_replay": true,
            },
            "dimension_means": {
                "agent_usability.documentation_contract": 3.0,
                "agent_usability.growth_health": 4.0,
                "architecture_boundaries": 3.5
            },
            "maturity_certified_repository_count": 0,
            "maturity_status_counts": {"not_certified": 1},
            "quality_outcome_counts": {"healthy": 1},
            "quality_outcome_taxonomy": {
                "healthy": {
                    "label": "Quality healthy",
                    "meaning": "Verification passed or was safely reused and all applicable dimensions are maintained.",
                    "next_step": "Keep the evidence checkpoint current."
                }
            },
            "finding_counts": {},
            "unresolved_measurement_gaps": [],
            "repositories": [{
                "repo_id": repository_feed_id(repository),
                "display_name": repository.name,
                "target_branch": "dev",
                "target_branch_status": "ready",
                "target_head": "abc",
                "maturity_score": 3.5,
                "maturity_status": "not_certified",
                "dimension_scores": {
                    "agent_usability.documentation_contract": 3.0,
                    "agent_usability.growth_health": 4.0,
                    "architecture_boundaries": 3.5
                },
                "dimension_gaps": [{
                    "dimension": "change_surface_coverage",
                    "status": "missing",
                    "score": 0,
                    "message": "No repository-owned change-surface matrix was found."
                }],
                "quality_status": "healthy",
                "quality_outcome": {
                    "state": "healthy",
                    "label": "Quality healthy",
                    "disposition": "Applicable dimensions are maintained with current evidence.",
                    "next_step": "Keep the evidence checkpoint current."
                },
                "finding_count": 0,
                "blocker_count": 0,
                "dynamic_status": "reused"
            }],
            "privacy": {
                "private_local_feed": true,
                "raw_paths": false,
                "raw_prompts": false,
                "raw_code": false,
                "raw_diffs": false,
                "raw_transcripts": false,
                "credentials": false,
            },
            "provenance_hash": "",
        });
        feed["behavior_assurance"] = serde_json::json!({
            "schema": "quality-runner-behavior-assurance-summary/v1",
            "status": "gaps_present",
            "repository_count": 1,
            "ready_repository_count": 0,
            "applicability_counts": {"applicable": 1},
            "result_status_counts": {"unknown": 1},
            "required_scenario_count": 2,
            "passed_scenario_count": 1,
            "gap_count": 1
        });
        feed["repositories"][0]["behavior_assurance"] = serde_json::json!({
            "schema": "quality-runner-behavior-assurance/v1",
            "applicability": "applicable",
            "contract_status": "current",
            "result_status": "unknown",
            "freshness": "stale",
            "release_ready": false,
            "score": 2,
            "contract_path": ".pronto/behavior-assurance.json",
            "receipt_directory": ".quality-runner/behavior-assurance/receipts",
            "contract_digest": "contract-fixture",
            "target_branch": "dev",
            "target_commit": "abc",
            "observed_at": as_of,
            "required_scenario_count": 2,
            "passed_scenario_count": 1,
            "accepted_defect_count": 0,
            "receipt_count": 1,
            "verified": [],
            "gaps": [{
                "kind": "receipt_stale",
                "message": "One required scenario needs a current receipt.",
                "behavior_id": "save-state",
                "scenario_id": "reload-restores-value"
            }],
            "detail": "1/2 required Tier-0 scenarios have current trusted receipts.",
            "next_step": "Resolve the listed contract or receipt gaps, then rerun the Quality Runner fleet audit."
        });
        feed["repositories"][0]["agent_usability"] = serde_json::json!({
            "applicability": "applicable",
            "schema": "quality-runner-agent-usability/v1",
            "status": "attention",
            "manifest_status": "present",
            "manifest_path": ".agents/agent-usability.json",
            "applicable_lane_count": 4,
            "covered_lane_count": 3,
            "lanes": [{
                "id": "documentation_contract",
                "label": "Documentation contract",
                "applicable": true,
                "score": 3,
                "status": "maintained",
                "message": "Every declared tool has fresh, routed documentation."
            }],
            "growth_health": {
                "status": "healthy",
                "score": 4,
                "message": "Documentation and skill structure remains proportionate and routed.",
                "document_count": 12,
                "agent_document_count": 3,
                "routed_agent_document_count": 3,
                "unrouted_agent_document_count": 0,
                "oversized_document_count": 0,
                "skill_count": 4,
                "family_count": 2,
                "largest_family_size": 2,
                "unclassified_skill_count": 0,
                "oversized_skill_count": 0,
                "tool_count": 2,
                "documented_tool_count": 2,
                "skill_covered_tool_count": 2,
                "behavior_declared_tool_count": 0,
                "behavior_verified_tool_count": 0,
                "inventory_truncated": false
            }
        });
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("fixture feed should hash"));
        feed
    }

    #[test]
    fn reconciles_current_finding_dispositions_without_hiding_raw_detector_totals() {
        let root = fixture_root();
        let report_path = root.join("code-quality-scan.json");
        fs::write(
            &report_path,
            r#"{"summary":{"findings_by_category":{"debloat":3,"simplify":1}},"findings":[{"fingerprint":"fp-1","category":"debloat"},{"fingerprint":"fp-1","category":"debloat"},{"fingerprint":"fp-2","category":"debloat"},{"fingerprint":"fp-3","category":"simplify"}]}"#,
        )
        .expect("finding report should be writable");
        let contract_path = root.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
        fs::create_dir_all(
            contract_path
                .parent()
                .expect("contract should have a parent"),
        )
        .expect("contract directory should be writable");
        fs::write(
            &contract_path,
            format!(
                r#"{{
  "schema_version": "{FINDING_DISPOSITIONS_SCHEMA}",
  "updated_at": "2026-07-29T18:00:00Z",
  "dispositions": [
    {{
      "fingerprint": "fp-1",
      "status": "false_positive",
      "reason": "The symbol has a verified caller.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-07-29T18:00:00Z",
      "evidence": ["src/example.ts:10"]
    }},
    {{
      "fingerprint": "fp-2",
      "status": "confirmed",
      "reason": "The finding reproduces.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-07-29T18:00:00Z"
    }},
    {{
      "fingerprint": "no-longer-detected",
      "status": "fixed",
      "reason": "The finding disappeared after the fix.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-07-29T18:00:00Z"
    }}
  ]
}}"#
            ),
        )
        .expect("disposition contract should be writable");
        let mut findings = QualityFindings {
            total: 4,
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };

        reconcile_finding_dispositions(&root, &mut findings);

        assert_eq!(findings.total, 4);
        assert_eq!(findings.reviewed_total, 3);
        assert_eq!(findings.unreviewed_total, 1);
        assert_eq!(findings.actionable_total, 2);
        assert_eq!(findings.category_counts.get("debloat"), Some(&3));
        assert_eq!(findings.actionable_category_counts.get("debloat"), Some(&1));
        assert_eq!(
            findings.actionable_category_counts.get("simplify"),
            Some(&1)
        );
        assert_eq!(findings.disposition_counts.get("false_positive"), Some(&2));
        assert_eq!(findings.disposition_counts.get("confirmed"), Some(&1));
        assert_eq!(findings.stale_disposition_total, 1);
        assert_eq!(findings.disposition_status, "Ready");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn preserves_open_finding_categories_and_reconciles_accepted_risk() {
        let root = fixture_root();
        let report_path = root.join("code-quality-scan.json");
        fs::write(
            &report_path,
            r#"{"summary":{"findings_by_category":{"maintenance-surface":0,"speed":1,"skill:future-review":1,"future-category":1}},"findings":[{"fingerprint":"surface-1","category":"maintenance-surface"},{"fingerprint":"speed-1","category":"speed"},{"fingerprint":"skill-1","category":"skill:future-review"},{"fingerprint":"future-1","category":"future-category"}]}"#,
        )
        .expect("finding report should be writable");
        let contract_path = root.join(FINDING_DISPOSITIONS_RELATIVE_PATH);
        fs::create_dir_all(
            contract_path
                .parent()
                .expect("contract should have a parent"),
        )
        .expect("contract directory should be writable");
        fs::write(
            &contract_path,
            format!(
                r#"{{
  "schema_version": "{FINDING_DISPOSITIONS_SCHEMA}",
  "updated_at": "2026-08-11T13:00:00Z",
  "dispositions": [
    {{
      "fingerprint": "surface-1",
      "status": "accepted_risk",
      "reason": "The maintenance burden is documented and explicitly accepted.",
      "reviewer": "test-reviewer",
      "reviewed_at": "2026-08-11T13:00:00Z"
    }}
  ]
}}"#
            ),
        )
        .expect("disposition contract should be writable");
        let mut findings = QualityFindings {
            total: 4,
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };

        reconcile_finding_dispositions(&root, &mut findings);

        assert_eq!(
            findings.category_counts.get("maintenance-surface"),
            Some(&1)
        );
        assert_eq!(findings.category_counts.get("speed"), Some(&1));
        assert_eq!(
            findings.category_counts.get("skill:future-review"),
            Some(&1)
        );
        assert_eq!(findings.category_counts.get("future-category"), Some(&1));
        assert_eq!(
            findings
                .actionable_category_counts
                .get("maintenance-surface"),
            Some(&0)
        );
        assert_eq!(findings.actionable_total, 3);
        assert_eq!(findings.disposition_counts.get("accepted_risk"), Some(&1));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn debloat_review_gate_requires_explicit_fresh_category_evidence() {
        let mut findings = QualityFindings::default();
        assert!(debloat_maturity_evidence(&findings).is_none());

        findings.category_counts.insert("debloat".to_string(), 0);
        findings
            .actionable_category_counts
            .insert("debloat".to_string(), 0);
        findings.freshness = QualityFreshness::Fresh;
        let clear = debloat_maturity_evidence(&findings)
            .expect("declared debloat coverage should create maturity evidence");
        assert_eq!(clear.status, QualityGateStatus::Passed);
        assert_eq!(clear.freshness, QualityFreshness::Fresh);

        findings.category_counts.insert("debloat".to_string(), 1);
        findings
            .actionable_category_counts
            .insert("debloat".to_string(), 1);
        let unresolved = debloat_maturity_evidence(&findings)
            .expect("an unresolved candidate should create maturity evidence");
        assert_eq!(unresolved.status, QualityGateStatus::Blocked);
        assert!(unresolved.detail.contains("ownership-pressure audit"));

        findings
            .actionable_category_counts
            .insert("debloat".to_string(), 0);
        findings.freshness = QualityFreshness::Stale;
        let stale =
            debloat_maturity_evidence(&findings).expect("stale coverage should remain visible");
        assert_eq!(stale.status, QualityGateStatus::Blocked);
        assert_eq!(stale.freshness, QualityFreshness::Stale);
    }

    #[test]
    fn missing_disposition_contract_keeps_every_finding_actionable() {
        let root = fixture_root();
        let report_path = root.join("code-quality-scan.json");
        fs::write(
            &report_path,
            r#"{"findings":[{"fingerprint":"fp-1"},{"fingerprint":"fp-2"}]}"#,
        )
        .expect("finding report should be writable");
        let mut findings = QualityFindings {
            total: 2,
            report_path: Some(report_path.to_string_lossy().to_string()),
            ..QualityFindings::default()
        };

        reconcile_finding_dispositions(&root, &mut findings);

        assert_eq!(findings.actionable_total, 2);
        assert_eq!(findings.unreviewed_total, 2);
        assert_eq!(findings.reviewed_total, 0);
        assert_eq!(findings.disposition_status, "Missing");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn writes_auditable_finding_dispositions_and_replaces_prior_review() {
        let root = fixture_root();
        let first = set_finding_disposition(
            &root,
            "fp-1",
            "confirmed",
            "The finding reproduces.",
            "test-reviewer",
            vec!["test:quality".to_string()],
            None,
        )
        .expect("first disposition should be written");
        assert_eq!(first.dispositions.len(), 1);
        assert_eq!(first.dispositions[0].status, "confirmed");

        let replaced = set_finding_disposition(
            &root,
            "fp-1",
            "false-positive",
            "A caller was verified.",
            "second-reviewer",
            vec!["src/example.ts:10".to_string()],
            Some("2099-01-01T00:00:00Z".to_string()),
        )
        .expect("replacement disposition should be written");

        assert_eq!(replaced.dispositions.len(), 1);
        assert_eq!(replaced.dispositions[0].status, "false_positive");
        assert_eq!(replaced.dispositions[0].reviewer, "second-reviewer");
        assert_eq!(replaced.dispositions[0].evidence, vec!["src/example.ts:10"]);
        assert!(root.join(FINDING_DISPOSITIONS_RELATIVE_PATH).is_file());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn imports_canonical_maturity_feed_by_stable_repository_id() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let as_of = Utc::now().to_rfc3339();
        fs::write(
            &feed_path,
            serde_json::to_string(&fixture_maturity_feed(&repository, &as_of))
                .expect("feed should serialize"),
        )
        .expect("feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(imported.portfolio.audit_status, "Ready");
        assert_eq!(
            imported.portfolio.latest_audit_id.as_deref(),
            Some("audit-fixture")
        );
        let expected_provenance = imported
            .portfolio
            .provenance_hash
            .clone()
            .expect("feed should expose provenance");
        assert_eq!(
            imported.portfolio.provenance_hash.as_deref(),
            Some(expected_provenance.as_str())
        );
        assert_eq!(imported.portfolio.matched_repository_count, 1);
        let confidence = imported
            .portfolio
            .measurement_confidence
            .as_ref()
            .expect("feed should expose measurement confidence");
        assert_eq!(confidence.level, "high");
        assert_eq!(confidence.expected_repository_count, 1);
        assert_eq!(confidence.observed_repository_count, 1);
        assert_eq!(confidence.excluded_repository_count, 2);
        assert_eq!(confidence.unresolved_measurement_gap_count, 0);
        assert_eq!(imported.portfolio.behavior_assurance.status, "gaps_present");
        assert_eq!(
            imported.portfolio.behavior_assurance.ready_repository_count,
            0
        );
        assert_eq!(
            imported.portfolio.quality_outcome_counts.get("healthy"),
            Some(&1)
        );
        assert_eq!(
            imported.portfolio.quality_outcome_taxonomy["healthy"].label,
            "Quality healthy"
        );
        assert_eq!(
            imported.maturities[&repository.id]
                .quality_outcome
                .as_ref()
                .and_then(|outcome| outcome.disposition.as_deref()),
            Some("Applicable dimensions are maintained with current evidence.")
        );
        assert_eq!(imported.maturities[&repository.id].score, Some(3.5));
        assert_eq!(
            imported.maturities[&repository.id]
                .dimension_scores
                .get("agent_usability.growth_health"),
            Some(&4.0)
        );
        assert_eq!(imported.maturities[&repository.id].gaps.len(), 1);
        assert_eq!(
            imported.maturities[&repository.id].gaps[0].dimension,
            "change_surface_coverage"
        );
        let agent_usability = imported.maturities[&repository.id]
            .agent_usability
            .as_ref()
            .expect("feed should preserve agent-usability evidence");
        assert_eq!(agent_usability.covered_lane_count, 3);
        assert_eq!(agent_usability.lanes[0].id, "documentation_contract");
        assert_eq!(agent_usability.growth_health.skill_count, 4);
        assert_eq!(agent_usability.growth_health.family_count, 2);
        let behavior_assurance = &imported.behavior_assurance[&repository.id];
        assert_eq!(behavior_assurance.contract_status, "current");
        assert_eq!(behavior_assurance.state, "legacy_v1");
        assert_eq!(behavior_assurance.result_status, "unknown");
        assert!(!behavior_assurance.release_ready);
        assert_eq!(behavior_assurance.gaps[0].kind, "receipt_stale");
        assert_eq!(
            imported.maturities[&repository.id].freshness,
            QualityFreshness::Fresh
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn imports_legacy_v1_maturity_feed() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let mut feed = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        feed["schema"] = Value::String(MATURITY_FEED_SCHEMAS[0].to_string());
        feed.as_object_mut()
            .expect("fixture feed should be an object")
            .remove("measurement_confidence");
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("fixture feed should hash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&feed).expect("feed should serialize"),
        )
        .expect("feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), &[repository]);
        assert_eq!(imported.portfolio.audit_status, "Ready");
        assert_eq!(
            imported.portfolio.feed_schema.as_deref(),
            Some(MATURITY_FEED_SCHEMAS[0])
        );
        assert!(imported.portfolio.measurement_confidence.is_none());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn imports_holistic_v2_maturity_projection() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity-v2.json");
        let as_of = Utc::now().to_rfc3339();
        let mut feed = fixture_maturity_feed(&repository, &as_of);
        feed["schema"] = Value::String("quality-runner-maturity-feed/v2".to_string());
        feed["repositories"][0]["repository_maturity"] = serde_json::json!({
            "schema": "quality-runner-repository-maturity/v2",
            "score": 3.5,
            "uncapped_score": 3.5,
            "status": "provisional",
            "pillars": [
                {"id": "correctness_reliability", "label": "Correctness and reliability", "weight": 0.22, "applicability": "applicable", "status": "attention", "score": 3.5},
                {"id": "security_privacy_supply_chain", "label": "Security, privacy, and supply chain", "weight": 0.22, "applicability": "applicable", "status": "unknown", "score": null},
                {"id": "maintainability_evolvability", "label": "Maintainability and evolvability", "weight": 0.16, "applicability": "applicable", "status": "attention", "score": 3.5},
                {"id": "operability_release_safety", "label": "Operability and release safety", "weight": 0.14, "applicability": "applicable", "status": "unknown", "score": null},
                {"id": "user_facing_quality", "label": "User-facing quality", "weight": 0.10, "applicability": "unknown", "status": "unknown", "score": null},
                {"id": "human_agent_usability", "label": "Human and agent usability", "weight": 0.10, "applicability": "applicable", "status": "attention", "score": 3.5},
                {"id": "governance_sustainability", "label": "Governance and sustainability", "weight": 0.06, "applicability": "unknown", "status": "unknown", "score": null}
            ],
            "evidence": {
                "applicable_pillar_count": 5,
                "assessed_pillar_count": 3,
                "applicable_weight": 0.84,
                "assessed_weight": 0.48,
                "evidence_coverage": 0.571,
                "fresh_evidence_coverage": 0.571,
                "unknown_applicability": ["user_facing_quality", "governance_sustainability"],
                "unmapped_dimensions": []
            },
            "critical_cap": {"applied": false, "maximum_score": null, "reasons": []}
        });
        feed["provenance_hash"] =
            Value::String(maturity_feed_hash(&feed).expect("v2 fixture feed should hash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&feed).expect("v2 feed should serialize"),
        )
        .expect("v2 feed should be writable");

        let imported = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));

        assert_eq!(imported.portfolio.audit_status, "Ready");
        let model = imported.maturities[&repository.id]
            .repository_maturity
            .as_ref()
            .expect("v2 feed should preserve the repository model");
        assert_eq!(model.status, "provisional");
        assert_eq!(model.pillars.len(), 7);
        assert_eq!(model.evidence.evidence_coverage, 0.571);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn rejects_invalid_and_stale_maturity_feeds() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let feed_path = root.join("maturity.json");
        let stale_as_of = (Utc::now() - Duration::days(MAX_EVIDENCE_AGE_DAYS + 1)).to_rfc3339();
        fs::write(
            &feed_path,
            serde_json::to_string(&fixture_maturity_feed(&repository, &stale_as_of))
                .expect("feed should serialize"),
        )
        .expect("feed should be writable");
        let stale = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(stale.portfolio.audit_status, "Stale");
        assert_eq!(
            stale.maturities[&repository.id].freshness,
            QualityFreshness::Stale
        );

        let mut invalid_feed = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        invalid_feed["replay"]["status"] = Value::String("failed".to_string());
        fs::write(
            &feed_path,
            serde_json::to_string(&invalid_feed).expect("feed should serialize"),
        )
        .expect("feed should be writable");
        let invalid = maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(invalid.portfolio.audit_status, "Unavailable");
        assert!(invalid.maturities.is_empty());

        let mut contradictory = fixture_maturity_feed(&repository, &Utc::now().to_rfc3339());
        contradictory["measurement_confidence"]["limitations"] =
            serde_json::json!(["dynamic_verification_disabled"]);
        contradictory["provenance_hash"] =
            Value::String(maturity_feed_hash(&contradictory).expect("fixture feed should hash"));
        fs::write(
            &feed_path,
            serde_json::to_string(&contradictory).expect("feed should serialize"),
        )
        .expect("feed should be writable");
        let contradictory =
            maturity_feed_import(Some(&feed_path), std::slice::from_ref(&repository));
        assert_eq!(contradictory.portfolio.audit_status, "Unavailable");
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn normalizes_qr_repository_identity_inputs() {
        assert_eq!(
            normalized_origin("git@github.com:Example/Repo.git").as_deref(),
            Some("github.com/example/repo")
        );
        assert_eq!(
            normalized_origin("https://github.com/Example/Repo.git").as_deref(),
            Some("github.com/example/repo")
        );
        assert_eq!(
            normalized_origin("ssh://git@github.com/Example/Repo.git").as_deref(),
            Some("github.com/example/repo")
        );
    }

    #[test]
    fn normalizes_qr_aliases_and_discovers_custom_gates() {
        assert_eq!(normalize_gate_id("runtime_smoke"), "runtime_smoke");
        assert_eq!(normalize_gate_id("smoke-test"), "runtime_smoke");
        assert_eq!(normalize_gate_id("dead_code"), "dead_code");
        assert_eq!(normalize_gate_id("unit_tests_vitest"), "tests");
        assert_eq!(
            normalize_gate_id("secret_scanning_gitleaks"),
            "secrets_scan"
        );
        assert_eq!(
            normalize_gate_id("security_dependency_audit"),
            "dependency_audit"
        );
        assert_eq!(normalize_gate_id("verify_and_build"), "build");
        assert_eq!(normalize_gate_id("security scan"), "custom:security_scan");
        assert_eq!(
            default_quality_gates()
                .iter()
                .map(|gate| gate.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "build",
                "runtime_smoke",
                "tests",
                "lint",
                "formatter",
                "typecheck",
                "dead_code",
                "secrets_scan"
            ]
        );
    }

    #[test]
    fn recommendation_matrix_provides_repository_specific_ideal_gates() {
        let root = fixture_root();
        let mut repository = fixture_repository(&root.join("repo"));
        repository.name = "BIP-Console".to_string();
        assert_eq!(
            ideal_gate_ids_for_repository(&repository),
            Some(vec![
                "build".to_string(),
                "tests".to_string(),
                "runtime_smoke".to_string(),
                "lint".to_string(),
                "formatter".to_string(),
                "typecheck".to_string(),
                "dead_code".to_string(),
                "secrets_scan".to_string(),
                "dependency_audit".to_string(),
            ])
        );

        repository.name = "dotfiles".to_string();
        assert_eq!(
            ideal_gate_ids_for_repository(&repository),
            Some(vec!["secrets_scan".to_string()])
        );

        repository.name = "Book-documents-github".to_string();
        assert_eq!(
            ideal_gate_ids_for_repository(&repository),
            Some(
                CI_READINESS_BASELINE_GATE_IDS
                    .iter()
                    .map(|gate_id| (*gate_id).to_string())
                    .collect()
            )
        );
        repository.name = "newly-registered-repository".to_string();
        assert_eq!(
            ideal_gate_ids_for_repository(&repository),
            Some(
                CI_READINESS_BASELINE_GATE_IDS
                    .iter()
                    .map(|gate_id| (*gate_id).to_string())
                    .collect()
            )
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    fn readiness_gate(
        id: &str,
        status: QualityGateStatus,
        freshness: QualityFreshness,
        with_evidence: bool,
    ) -> QualityGate {
        QualityGate {
            id: id.to_string(),
            label: gate_label(id),
            status: status.clone(),
            freshness: freshness.clone(),
            evidence: with_evidence
                .then(|| QualityEvidence {
                    id: id.to_string(),
                    source: QualitySource::Ci,
                    status,
                    freshness,
                    observed_at: None,
                    scanned_commit: None,
                    scanned_branch: None,
                    command: None,
                    source_label: "fixture".to_string(),
                    report_path: None,
                    report_url: None,
                    report_kind: None,
                    detail: "fixture".to_string(),
                    verification_level: QualityVerificationLevel::SourceInferred,
                    target_kind: Some("source".to_string()),
                    target_url: None,
                    target_provider: None,
                    deployment_id: None,
                })
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn ci_readiness_reaches_four_only_for_fresh_baseline_and_applicable_gates() {
        let baseline = [
            "build",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "secrets_scan",
        ]
        .into_iter()
        .map(|id| readiness_gate(id, QualityGateStatus::Passed, QualityFreshness::Fresh, true))
        .collect::<Vec<_>>();
        let complete = evaluate_ci_readiness(&baseline);
        assert_eq!(complete.score, Some(4.0));
        assert_eq!(complete.score_display.as_deref(), Some("4.0"));
        assert_eq!(complete.applicable_gate_ids.len(), 6);
        assert!(complete.missing_gate_ids.is_empty());

        let mut incomplete = baseline;
        let tests = incomplete
            .iter_mut()
            .find(|gate| gate.id == "tests")
            .unwrap();
        tests.status = QualityGateStatus::NotConfigured;
        tests.evidence.clear();
        let lint = incomplete
            .iter_mut()
            .find(|gate| gate.id == "lint")
            .unwrap();
        lint.freshness = QualityFreshness::Stale;
        lint.evidence[0].freshness = QualityFreshness::Stale;
        incomplete.push(readiness_gate(
            "dependency_audit",
            QualityGateStatus::Passed,
            QualityFreshness::Fresh,
            true,
        ));
        let result = evaluate_ci_readiness(&incomplete);
        assert_eq!(result.score, Some(2.86));
        assert_eq!(result.missing_gate_ids, vec!["tests"]);
        assert_eq!(result.stale_gate_ids, vec!["lint"]);
        assert!(result
            .applicable_gate_ids
            .contains(&"dependency_audit".to_string()));
    }

    #[test]
    fn ci_maturity_scores_ideal_gate_coverage() {
        let ideal_gate_ids = [
            "build",
            "tests",
            "lint",
            "formatter",
            "typecheck",
            "secrets_scan",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let gates = ideal_gate_ids
            .iter()
            .enumerate()
            .map(|(index, gate_id)| {
                if index < 4 {
                    readiness_gate(
                        gate_id,
                        QualityGateStatus::Passed,
                        QualityFreshness::Fresh,
                        true,
                    )
                } else if index == 4 {
                    readiness_gate(
                        gate_id,
                        QualityGateStatus::Blocked,
                        QualityFreshness::Unknown,
                        true,
                    )
                } else {
                    readiness_gate(
                        gate_id,
                        QualityGateStatus::NotConfigured,
                        QualityFreshness::Unknown,
                        false,
                    )
                }
            })
            .collect::<Vec<_>>();

        let readiness = evaluate_ci_readiness_for_ideal(&gates, &ideal_gate_ids);
        assert_eq!(readiness.score, Some(2.67));
        assert_eq!(readiness.evidence_coverage_score, Some(3.33));
        assert_eq!(readiness.applicable_gate_ids, ideal_gate_ids);
        assert_eq!(
            readiness.covered_gate_ids,
            vec!["build", "tests", "lint", "formatter", "typecheck"]
        );
        assert_eq!(readiness.blocked_gate_ids, vec!["typecheck"]);
        assert_eq!(readiness.missing_gate_ids, vec!["secrets_scan"]);
    }

    #[test]
    fn ci_configuration_separates_discovered_gates_from_imported_results() {
        let ideal_gate_ids = [
            "build",
            "tests",
            "runtime_smoke",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
            "secrets_scan",
            "dependency_audit",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let configured_gate_ids = [
            "build",
            "tests",
            "runtime_smoke",
            "lint",
            "formatter",
            "typecheck",
            "dead_code",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let readiness = evaluate_ci_readiness_for_ideal_with_configuration(
            &default_quality_gates(),
            &ideal_gate_ids,
            &configured_gate_ids,
        );

        assert_eq!(
            readiness.configuration_score_display.as_deref(),
            Some("3.11")
        );
        assert_eq!(readiness.configured_gate_ids, configured_gate_ids);
        assert_eq!(
            readiness.unconfigured_gate_ids,
            vec!["secrets_scan", "dependency_audit"]
        );
        assert_eq!(readiness.score, Some(0.0));
        assert!(readiness.covered_gate_ids.is_empty());
        assert!(readiness.fresh_passing_gate_ids.is_empty());
    }

    #[test]
    fn ci_maturity_summary_excludes_unscored_repositories() {
        let root = fixture_root();
        let mut scored = fixture_repository(&root.join("scored"));
        scored.quality.ci_readiness.score = Some(2.0);
        let unscored = fixture_repository(&root.join("unscored"));
        let repositories = vec![scored, unscored];
        let mut portfolio = QualityPortfolioSnapshot::default();

        update_ci_readiness_summary(&mut portfolio, &repositories);

        assert_eq!(portfolio.ci_readiness_score, Some(2.0));
        assert_eq!(portfolio.ci_readiness_repository_count, 1);
        assert_eq!(portfolio.ci_readiness_unscored_repository_count, 1);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn conflicting_evidence_is_blocked() {
        let items = vec![
            evidence(
                QualitySource::Ci,
                QualityGateStatus::Passed,
                QualityFreshness::Fresh,
            ),
            evidence(
                QualitySource::Local,
                QualityGateStatus::Failed,
                QualityFreshness::Fresh,
            ),
        ];
        assert_eq!(
            aggregate_gate_status(&items),
            (QualityGateStatus::Blocked, QualityFreshness::Conflicted)
        );
    }

    #[test]
    fn freshness_requires_current_commit_and_seven_day_window() {
        let now = DateTime::parse_from_rfc3339("2026-07-26T12:00:00Z")
            .expect("now should parse")
            .with_timezone(&Utc);
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                Some("abc"),
                Some("main"),
                Some("abc"),
                Some("main"),
                now,
            ),
            QualityFreshness::Fresh
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-10T12:00:00Z"),
                Some("abc"),
                Some("main"),
                Some("abc"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                Some("old"),
                Some("main"),
                Some("new"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                Some("old"),
                Some("feature"),
                Some("new"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                None,
                Some("main"),
                None,
                Some("main"),
                now,
            ),
            QualityFreshness::Unknown
        );
        assert_eq!(
            evaluate_freshness_at(
                Some("2026-07-25T12:00:00Z"),
                None,
                Some("feature"),
                None,
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
    }

    #[test]
    fn target_freshness_is_commit_based_instead_of_time_based() {
        assert_eq!(
            evaluate_target_freshness(Some("dev"), Some("target-commit"), "dev", "target-commit"),
            QualityFreshness::Fresh
        );
        assert_eq!(
            evaluate_target_freshness(Some("dev"), Some("old-commit"), "dev", "target-commit"),
            QualityFreshness::Stale
        );
        assert_eq!(
            evaluate_target_freshness(Some("dev"), None, "dev", "target-commit"),
            QualityFreshness::Unknown
        );
        assert_eq!(
            evaluate_target_freshness(None, None, "dev", "target-commit"),
            QualityFreshness::Unknown
        );
    }

    #[test]
    fn target_projection_keeps_exact_branch_failure_and_drops_mismatched_evidence() {
        let observed_at = (Utc::now() - Duration::days(MAX_EVIDENCE_AGE_DAYS + 1)).to_rfc3339();
        let mut snapshot = QualitySnapshot::default();
        snapshot.ci_readiness.applicable_gate_ids = vec!["build".to_string(), "tests".to_string()];
        let build = snapshot
            .gates
            .iter_mut()
            .find(|gate| gate.id == "build")
            .expect("build gate should exist");
        build.evidence.push(QualityEvidence {
            id: "build".to_string(),
            source: QualitySource::Qr,
            status: QualityGateStatus::Failed,
            freshness: QualityFreshness::Stale,
            observed_at: Some(observed_at.clone()),
            scanned_commit: Some("target-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            command: Some("pnpm build".to_string()),
            source_label: "target QR".to_string(),
            report_path: None,
            report_url: None,
            report_kind: None,
            detail: "fixture failure".to_string(),
            verification_level: QualityVerificationLevel::SourceInferred,
            target_kind: Some("source".to_string()),
            target_url: None,
            target_provider: None,
            deployment_id: None,
        });
        let lint = snapshot
            .gates
            .iter_mut()
            .find(|gate| gate.id == "lint")
            .expect("lint gate should exist");
        lint.evidence.push(QualityEvidence {
            id: "lint".to_string(),
            source: QualitySource::Qr,
            status: QualityGateStatus::Passed,
            freshness: QualityFreshness::Fresh,
            observed_at: Some(observed_at.clone()),
            scanned_commit: Some("old-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            command: Some("pnpm lint".to_string()),
            source_label: "old QR".to_string(),
            report_path: None,
            report_url: None,
            report_kind: None,
            detail: "mismatched fixture".to_string(),
            verification_level: QualityVerificationLevel::SourceInferred,
            target_kind: Some("source".to_string()),
            target_url: None,
            target_provider: None,
            deployment_id: None,
        });
        snapshot.findings = QualityFindings {
            total: 3,
            source: Some(QualitySource::Qr),
            observed_at: Some(observed_at.clone()),
            scanned_commit: Some("target-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            freshness: QualityFreshness::Stale,
            ..QualityFindings::default()
        };
        snapshot.maturity = QualityMaturity {
            score: Some(2.5),
            score_display: Some("2.500".to_string()),
            audit_id: Some("target-audit".to_string()),
            observed_at: Some(observed_at),
            scanned_commit: Some("target-commit".to_string()),
            scanned_branch: Some("dev".to_string()),
            freshness: QualityFreshness::Stale,
            ..QualityMaturity::default()
        };

        project_quality_snapshot_for_target(&mut snapshot, "dev", "target-commit");

        let build = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "build")
            .expect("build gate should remain configured");
        assert_eq!(build.status, QualityGateStatus::Failed);
        assert_eq!(build.evidence.len(), 1);
        assert_eq!(build.evidence[0].freshness, QualityFreshness::Fresh);
        assert_eq!(build.freshness, QualityFreshness::Unknown);
        let lint = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "lint")
            .expect("lint gate should remain in the canonical list");
        assert!(lint.evidence.is_empty());
        assert_eq!(lint.status, QualityGateStatus::NotConfigured);
        assert_eq!(snapshot.findings.total, 3);
        assert_eq!(snapshot.findings.freshness, QualityFreshness::Fresh);
        assert_eq!(snapshot.maturity.score, Some(2.5));
        assert_eq!(snapshot.maturity.freshness, QualityFreshness::Fresh);
        assert_eq!(snapshot.ingestion_status, "Available");
        assert!(snapshot.ingestion_message.is_none());
        assert!(snapshot
            .ci_readiness
            .failed_gate_ids
            .contains(&"build".to_string()));
        assert!(!snapshot
            .ci_readiness
            .configured_gate_ids
            .contains(&"lint".to_string()));
    }

    #[test]
    fn ingests_qr_gates_and_severity_breakdown_without_running_commands() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let observed_at = Utc::now().to_rfc3339();
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-1");
        fs::create_dir_all(&run).expect("QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            serde_json::json!({
                "created_at": observed_at,
                "git": { "branch": "main", "head_sha": "abc" }
            })
            .to_string(),
        )
        .expect("run manifest should be writable");
        fs::write(
            run.join("gate-verification.json"),
            serde_json::json!({
                "gates": [
                    {
                        "id": "runtime_smoke",
                        "status": "passed",
                        "capability_kind": "local_command",
                        "command": "pnpm smoke",
                        "completed_at": observed_at
                    },
                    {
                        "id": "security scan",
                        "status": "failed",
                        "capability_kind": "qr",
                        "failure_type": "command-failed",
                        "reason": "finding threshold exceeded"
                    },
                    {
                        "id": "review",
                        "status": "skipped",
                        "skip_type": "missing evidence"
                    }
                ]
            })
            .to_string(),
        )
        .expect("gate verification should be writable");
        fs::write(
            run.join("quality-audit.json"),
            r#"{"findings":[
                {"fingerprint":"audit-1","severity":"critical"},
                {"fingerprint":"audit-2","severity":"high"},
                {"fingerprint":"audit-3","severity":"warning"},
                {"fingerprint":"audit-4","severity":"low"}
            ]}"#,
        )
        .expect("QR report should be writable");
        fs::write(
            run.join("code-quality-scan.json"),
            r#"{"summary":{"findings_by_category":{"debloat":1,"simplify":1}},"findings":[
                {"fingerprint":"stable-1","category":"debloat","severity":"warning"},
                {"fingerprint":"stable-2","category":"simplify","severity":"observation"}
            ]}"#,
        )
        .expect("fingerprinted QR report should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        let smoke = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "runtime_smoke")
            .expect("smoke gate should be imported");
        assert_eq!(smoke.status, QualityGateStatus::Passed);
        assert_eq!(smoke.evidence[0].source, QualitySource::Local);
        assert_eq!(smoke.evidence[0].command.as_deref(), Some("pnpm smoke"));
        assert_eq!(smoke.freshness, QualityFreshness::Fresh);
        let security = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "custom:security_scan")
            .expect("security gate should be imported");
        assert_eq!(security.label, "Security Scan");
        assert_eq!(security.status, QualityGateStatus::Failed);
        assert_eq!(security.evidence[0].detail, "command-failed");
        assert!(snapshot
            .gates
            .iter()
            .any(|gate| gate.id == "custom:review" && gate.status == QualityGateStatus::Blocked));
        assert_eq!(snapshot.findings.total, 6);
        assert_eq!(snapshot.findings.category_counts.get("debloat"), Some(&1));
        let debloat = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "debloat")
            .expect("debloat review gate should be imported");
        assert_eq!(debloat.status, QualityGateStatus::Blocked);
        assert_eq!(debloat.freshness, QualityFreshness::Unknown);
        assert!(debloat.evidence[0]
            .detail
            .contains("no signal authorizes deletion"));
        assert_eq!(snapshot.findings.high_severity_total, 2);
        assert_eq!(snapshot.findings.severity_counts.get("critical"), Some(&1));
        assert_eq!(snapshot.findings.severity_counts.get("high"), Some(&1));
        assert_eq!(snapshot.findings.severity_counts.get("medium"), Some(&2));
        assert_eq!(snapshot.findings.report_paths.len(), 2);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("code-quality-scan.json")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn reads_execution_plan_and_run_summary_when_completed_artifacts_are_partial() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-partial");
        fs::create_dir_all(&run).expect("QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            r#"{"created_at":"2026-07-26T11:00:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("run manifest should be writable");
        fs::write(
            run.join("gate-execution-plan.json"),
            r#"[{"id":"formatter","command":"pnpm format:check","capability_kind":"local_command","source":"package.json","local_execution_status":"consent-required"}]"#,
        )
        .expect("execution plan should be writable");
        fs::write(
            run.join("run-summary.json"),
            r#"{"finding_counts":{"total":3}}"#,
        )
        .expect("run summary should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        let formatter = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "formatter")
            .expect("formatter gate should be imported from the plan");
        assert_eq!(formatter.status, QualityGateStatus::Blocked);
        assert_eq!(formatter.evidence[0].source, QualitySource::Local);
        assert_eq!(
            formatter.evidence[0].command.as_deref(),
            Some("pnpm format:check")
        );
        assert_eq!(snapshot.findings.total, 3);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("run-summary.json")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn fleet_detector_run_uses_sibling_inspect_findings_with_latest_verify_evidence() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let runs = repository_path.join(".quality-runner").join("runs");
        let inspect = runs.join("fleet-detector-transaction-repo-inspect");
        let verify = runs.join("fleet-detector-transaction-repo-verify");
        fs::create_dir_all(&inspect).expect("inspect run should be writable");
        fs::create_dir_all(&verify).expect("verify run should be writable");
        let publication = r#"{"schema":"quality-runner-fleet-detector-publication/v1"}"#;
        fs::write(inspect.join("fleet-detector-publication.json"), publication)
            .expect("inspect publication should be writable");
        fs::write(verify.join("fleet-detector-publication.json"), publication)
            .expect("verify publication should be writable");
        fs::write(
            inspect.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:00:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("inspect manifest should be writable");
        fs::write(
            inspect.join("code-quality-scan.json"),
            r#"{"findings":[{"severity":"high"},{"severity":"medium"}]}"#,
        )
        .expect("detector report should be writable");
        fs::write(
            verify.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:02:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("verify manifest should be writable");
        fs::write(
            verify.join("run-summary.json"),
            r#"{"finding_counts":{"total":0}}"#,
        )
        .expect("verify summary should be writable");
        fs::write(
            verify.join("gate-verification.json"),
            r#"{"gates":[{"id":"lint","status":"passed","command":"pnpm lint"}]}"#,
        )
        .expect("verification should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        assert_eq!(snapshot.findings.total, 2);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("-inspect/code-quality-scan.json")));
        assert!(snapshot.gates.iter().any(|gate| gate.id == "lint"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn unmarked_similarly_named_runs_do_not_share_finding_reports() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let runs = repository_path.join(".quality-runner").join("runs");
        let inspect = runs.join("ordinary-run-inspect");
        let verify = runs.join("ordinary-run-verify");
        fs::create_dir_all(&inspect).expect("inspect run should be writable");
        fs::create_dir_all(&verify).expect("verify run should be writable");
        fs::write(
            inspect.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:00:00Z"}"#,
        )
        .expect("inspect manifest should be writable");
        fs::write(
            inspect.join("code-quality-scan.json"),
            r#"{"findings":[{"severity":"high"}]}"#,
        )
        .expect("detector report should be writable");
        fs::write(
            verify.join("run-manifest.json"),
            r#"{"created_at":"2026-08-14T11:02:00Z"}"#,
        )
        .expect("verify manifest should be writable");
        fs::write(
            verify.join("run-summary.json"),
            r#"{"finding_counts":{"total":0}}"#,
        )
        .expect("verify summary should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None, None);
        assert_eq!(snapshot.findings.total, 0);
        assert!(snapshot
            .findings
            .report_path
            .as_deref()
            .is_some_and(|path| path.ends_with("-verify/run-summary.json")));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn source_requirements_do_not_fall_back_to_another_source() {
        let root = fixture_root();
        let repository = fixture_repository(&root.join("repo"));
        let mut repository = repository;
        repository.quality.gates = vec![QualityGate {
            id: "lint".to_string(),
            label: "Lint".to_string(),
            status: QualityGateStatus::Passed,
            freshness: QualityFreshness::Fresh,
            evidence: vec![evidence(
                QualitySource::Local,
                QualityGateStatus::Passed,
                QualityFreshness::Fresh,
            )],
        }];
        let (status, freshness, detail) = evaluate_requirement(
            &repository,
            &QualityGateRequirement {
                gate_id: "lint".to_string(),
                source: QualitySource::Ci,
                minimum_verification_level: None,
                policy: QualityRequirementPolicy::Block,
            },
        );
        assert_eq!(status, QualityGateStatus::NotConfigured);
        assert_eq!(freshness, QualityFreshness::Unknown);
        assert!(detail.contains("CI evidence"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn verification_level_requirement_rejects_weaker_passing_evidence() {
        let root = fixture_root();
        let mut repository = fixture_repository(&root.join("repo"));
        let mut source = evidence(
            QualitySource::Qr,
            QualityGateStatus::Passed,
            QualityFreshness::Fresh,
        );
        source.id = "web_readiness".to_string();
        source.verification_level = QualityVerificationLevel::SourceInferred;
        repository.quality.gates = vec![QualityGate {
            id: "web_readiness".to_string(),
            label: "Web readiness".to_string(),
            status: QualityGateStatus::Passed,
            freshness: QualityFreshness::Fresh,
            evidence: vec![source],
        }];
        let requirement = QualityGateRequirement {
            gate_id: "web_readiness".to_string(),
            source: QualitySource::Qr,
            minimum_verification_level: Some(QualityVerificationLevel::DeploymentVerified),
            policy: QualityRequirementPolicy::Block,
        };

        let (status, freshness, detail) = evaluate_requirement(&repository, &requirement);
        assert_eq!(status, QualityGateStatus::Blocked);
        assert_eq!(freshness, QualityFreshness::Unknown);
        assert!(detail.contains("deployment_verified"));

        let mut deployment = repository.quality.gates[0].evidence[0].clone();
        deployment.verification_level = QualityVerificationLevel::DeploymentVerified;
        deployment.target_kind = Some("deployment".to_string());
        repository.quality.gates[0].evidence.push(deployment);
        let (status, freshness, _) = evaluate_requirement(&repository, &requirement);
        assert_eq!(status, QualityGateStatus::Passed);
        assert_eq!(freshness, QualityFreshness::Fresh);
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn web_readiness_import_preserves_target_identity_and_categorical_status() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(repository_path.join(".quality-runner"))
            .expect("web-readiness directory should be writable");
        let repository = fixture_repository(&repository_path);
        fs::write(
            repository_path.join(WEB_READINESS_RELATIVE_PATH),
            serde_json::to_string(&serde_json::json!({
                "schema": WEB_READINESS_SCHEMA,
                "status": "warnings",
                "generated_at": Utc::now().to_rfc3339(),
                "repository": {"path": repository.path, "branch": "main", "head_sha": "abc"},
                "applicability": {"status": "public_web", "reason": "Public product"},
                "summary": {"passed": 2, "failed": 1, "blocked": 0, "unknown": 0},
                "target": {
                    "kind": "deployment",
                    "commit": "abc",
                    "url": "https://preview.example.test",
                    "provider": "fixture",
                    "deployment_id": "dep-123"
                },
                "checks": [
                    {
                        "id": "route_titles", "label": "Route titles", "category": "baseline",
                        "policy": "block", "status": "passed", "verification_level": "deployment_verified",
                        "detail": "Unique titles", "evidence": [{"route": "/"}]
                    },
                    {
                        "id": "social_image", "label": "Social image", "category": "polish",
                        "policy": "warn", "status": "failed", "verification_level": "deployment_verified",
                        "detail": "Missing image", "evidence": [{"route": "/"}]
                    }
                ]
            }))
            .expect("web-readiness fixture should encode"),
        )
        .expect("web-readiness fixture should be writable");

        let snapshot = ingest_repository_quality(
            &repository,
            None,
            None,
            Some(&ideal_gate_ids_for_repository(&repository).expect("gate profile")),
        );
        assert_eq!(snapshot.web_readiness.status, "Warnings");
        assert_eq!(snapshot.web_readiness.applicability, "public_web");
        assert_eq!(
            snapshot.web_readiness.target.deployment_id.as_deref(),
            Some("dep-123")
        );
        assert_eq!(snapshot.web_readiness.warning_count, 1);
        assert!(snapshot
            .ci_readiness
            .applicable_gate_ids
            .contains(&"web_readiness".to_string()));
        let evidence = snapshot
            .gates
            .iter()
            .find(|gate| gate.id == "web_readiness")
            .and_then(|gate| gate.evidence.first())
            .expect("web-readiness evidence should be projected");
        assert_eq!(evidence.status, QualityGateStatus::Passed);
        assert_eq!(
            evidence.verification_level,
            QualityVerificationLevel::DeploymentVerified
        );
        assert_eq!(
            evidence.target_url.as_deref(),
            Some("https://preview.example.test")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn fleet_import_keeps_maturity_rows_out_of_quality_findings() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit");
        fs::create_dir_all(audit_root.join("findings")).expect("audit should be writable");
        fs::write(
            audit_root.join("summary.json"),
            r#"{"audit_id":"audit-fleet","as_of":"2026-08-03T20:05:55Z"}"#,
        )
        .expect("summary should be writable");
        fs::write(
            audit_root.join("findings").join("repo.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-fleet-repo",
                "as_of": "2026-08-03T20:05:55Z",
                "repository": {
                    "primary_path": repository_path.to_string_lossy(),
                    "checkouts": [{"path": repository_path.to_string_lossy(), "head": "abc", "branch": "main"}]
                },
                "detector_evidence": [{
                    "detector": "anti-slop",
                    "status": "passed",
                    "applicable": true,
                    "enabled_rules": ["anti-slop/no-known-value-widening", "anti-slop/no-widen-then-assert"],
                    "producer": {"version": "0.8.0", "source_sha": "producer-sha"},
                    "ruleset_hash": "ruleset-sha",
                    "configuration_hash": "configuration-sha",
                    "qr_version": "0.7.0",
                    "target_sha": "abc",
                    "scan_time": "2026-08-03T20:05:54Z"
                }],
                "agent_usability": {
                    "applicability": "applicable",
                    "schema": "quality-runner-agent-usability/v1",
                    "status": "attention",
                    "manifest_status": "present",
                    "manifest_path": ".agents/agent-usability.json",
                    "applicable_lane_count": 4,
                    "covered_lane_count": 3,
                    "lanes": [
                        {
                            "id": "documentation_contract",
                            "label": "Documentation contract",
                            "applicable": true,
                            "score": 3,
                            "status": "validated",
                            "message": "Documentation exists."
                        },
                        {
                            "id": "tool_skill_coverage",
                            "label": "Tool-to-skill coverage",
                            "applicable": true,
                            "score": 3,
                            "status": "static_validated",
                            "message": "Skill mapping exists."
                        },
                        {
                            "id": "behavior_evidence",
                            "label": "Behavior evidence",
                            "applicable": true,
                            "score": 1,
                            "status": "missing",
                            "message": "Behavior evidence remains unverified."
                        },
                        {
                            "id": "freshness_portability",
                            "label": "Freshness and portability",
                            "applicable": true,
                            "score": 3,
                            "status": "static_validated",
                            "message": "References are portable."
                        }
                    ],
                    "growth_health": {
                        "status": "healthy",
                        "score": 4,
                        "message": "Documentation and skill structure remains proportionate and routed.",
                        "document_count": 4,
                        "agent_document_count": 2,
                        "routed_agent_document_count": 2,
                        "skill_count": 1,
                        "family_count": 1,
                        "tool_count": 1
                    }
                },
                "findings": [
                    {
                        "applicable": true,
                        "dimension": "quality_commands",
                        "score": 4,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "validated"
                    },
                    {
                        "applicable": true,
                        "dimension": "security_constraints",
                        "score": 4,
                        "pack": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "maintained"
                    },
                    {
                        "applicable": true,
                        "dimension": "matrix_maintenance",
                        "score": 2,
                        "schema": "quality-runner-environment-legibility-finding-v0.1",
                        "severity": "observation",
                        "status": "incomplete"
                    },
                    {
                        "finding_id": "finding-real",
                        "schema": "quality-runner-code-finding-v1",
                        "severity": "high"
                    }
                ]
            }))
            .expect("fleet finding should encode"),
        )
        .expect("fleet finding should be writable");

        let imported =
            fleet_audit_import(Some(&audit_root), &[fixture_repository(&repository_path)]);
        let evidence = imported
            .evidence
            .get("repo-1")
            .expect("repository evidence should be imported");

        assert_eq!(evidence.maturity.score_display.as_deref(), Some("3.000"));
        assert_eq!(evidence.maturity.dimension_scores.len(), 8);
        assert_eq!(
            evidence.maturity.dimension_scores.get("matrix_maintenance"),
            Some(&2.0)
        );
        assert_eq!(
            evidence
                .maturity
                .dimension_scores
                .get("agent_usability.behavior_evidence"),
            Some(&1.0)
        );
        let agent_usability = evidence
            .maturity
            .agent_usability
            .as_ref()
            .expect("per-repository audits should retain agent-usability evidence");
        assert_eq!(agent_usability.covered_lane_count, 3);
        assert_eq!(agent_usability.lanes[2].id, "behavior_evidence");
        assert_eq!(evidence.findings.total, 1);
        assert_eq!(evidence.findings.detector_findings_total, 1);
        assert_eq!(evidence.findings.enabled_detector_count, 1);
        assert_eq!(evidence.findings.enabled_rule_count, 2);
        assert_eq!(
            evidence.findings.producer_versions.get("anti-slop"),
            Some(&"0.8.0".to_string())
        );
        assert_eq!(
            evidence.findings.ruleset_fingerprints.get("anti-slop"),
            Some(&"ruleset-sha".to_string())
        );
        assert_eq!(evidence.findings.target_sha.as_deref(), Some("abc"));
        assert_eq!(evidence.findings.qr_version.as_deref(), Some("0.7.0"));
        assert_eq!(evidence.findings.high_severity_total, 1);
        assert_eq!(evidence.findings.severity_counts.get("high"), Some(&1));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn fleet_import_uses_the_selected_target_checkout_for_branch_provenance() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit");
        fs::create_dir_all(audit_root.join("findings")).expect("audit should be writable");
        fs::write(
            audit_root.join("summary.json"),
            r#"{"audit_id":"audit-target","as_of":"2026-08-08T12:00:00Z"}"#,
        )
        .expect("summary should be writable");
        fs::write(
            audit_root.join("findings").join("repo.json"),
            serde_json::to_string(&serde_json::json!({
                "audit_id": "audit-target-repo",
                "as_of": "2026-08-08T12:00:00Z",
                "repository": {
                    "primary_path": repository_path.to_string_lossy(),
                    "target_branch": {"branch": "dev"},
                    "checkouts": [
                        {"path": repository_path.to_string_lossy(), "head": "stale-main", "branch": "main"},
                        {"path": "/private/tmp/target-worktree", "head": "target-dev", "branch": "dev"}
                    ]
                },
                "findings": []
            }))
            .expect("target fleet finding should encode"),
        )
        .expect("target fleet finding should be writable");

        let imported =
            fleet_audit_import(Some(&audit_root), &[fixture_repository(&repository_path)]);
        let evidence = imported
            .evidence
            .get("repo-1")
            .expect("repository evidence should be imported");
        assert_eq!(evidence.maturity.scanned_branch.as_deref(), Some("dev"));
        assert_eq!(
            evidence.maturity.scanned_commit.as_deref(),
            Some("target-dev")
        );
        assert_eq!(evidence.findings.scanned_branch.as_deref(), Some("dev"));
        assert_eq!(
            evidence.findings.scanned_commit.as_deref(),
            Some("target-dev")
        );
        fs::remove_dir_all(root).expect("target fleet fixture should be removable");
    }

    #[test]
    fn stable_detector_reports_are_distinguished_from_aggregate_reports() {
        assert!(is_stable_detector_report(Some(
            "/tmp/pronto/.quality-runner/runs/run-1/code-quality-scan.json"
        )));
        assert!(!is_stable_detector_report(Some(
            "/tmp/pronto/.quality-runner/fleet-audit/findings/repo.json"
        )));
        assert!(!is_stable_detector_report(None));
    }

    #[test]
    fn blocked_detector_refresh_retains_prior_evidence_and_suppresses_delta() {
        let mut prior = QualityFindings {
            total: 2,
            detector_findings_total: 2,
            source: Some(QualitySource::Qr),
            target_sha: Some("target-sha".to_string()),
            qr_version: Some("0.7.0".to_string()),
            producer_versions: BTreeMap::from([("anti-slop".to_string(), "0.8.0".to_string())]),
            producer_source_shas: BTreeMap::from([(
                "anti-slop".to_string(),
                "producer-sha".to_string(),
            )]),
            ruleset_fingerprints: BTreeMap::from([(
                "anti-slop".to_string(),
                "ruleset-sha".to_string(),
            )]),
            configuration_fingerprints: BTreeMap::from([(
                "anti-slop".to_string(),
                "configuration-sha".to_string(),
            )]),
            ..QualityFindings::default()
        };
        prior.delta_total = Some(1);
        let mut blocked = QualityFindings {
            source: Some(QualitySource::Qr),
            target_sha: Some("target-sha".to_string()),
            qr_version: Some("0.7.0".to_string()),
            refresh_required: true,
            refresh_required_reason: Some("Detector execution failed".to_string()),
            detector_status: Some("blocked".to_string()),
            report_path: Some("/tmp/anti-slop-detector.json".to_string()),
            ..QualityFindings::default()
        };

        preserve_detector_evidence_on_refresh_failure(&prior, &mut blocked);
        assert_eq!(blocked.detector_findings_total, 2);
        assert_eq!(blocked.total, 2);
        assert_eq!(blocked.ruleset_fingerprints, prior.ruleset_fingerprints);
        assert!(blocked.refresh_required);
        assert_eq!(blocked.detector_status.as_deref(), Some("blocked"));

        update_detector_delta(&prior, &mut blocked);
        assert_eq!(blocked.delta_total, None);

        let mut comparable = prior.clone();
        comparable.total = 3;
        comparable.detector_findings_total = 3;
        update_detector_delta(&prior, &mut comparable);
        assert_eq!(comparable.delta_total, Some(1));
    }

    #[test]
    fn audit_import_matches_canonical_path_before_remote_identity() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit").join("audit-1");
        fs::create_dir_all(audit_root.join("findings")).expect("audit should be writable");
        fs::write(
            audit_root.join("summary.json"),
            r#"{"audit_id":"audit-1","as_of":"2026-07-26T11:00:00Z","mean_maturity":1.933,"scored_dimension_count":10}"#,
        )
        .expect("summary should be writable");
        fs::write(
            audit_root.join("findings").join("repo.json"),
            format!(
                r#"{{"canonical_path":"{}","mean_maturity":4.0,"scored_dimension_count":10}}"#,
                repository_path.display()
            ),
        )
        .expect("finding should be writable");
        let repository = RepositorySnapshot {
            id: "repo-1".to_string(),
            name: "repo".to_string(),
            path: repository_path.to_string_lossy().to_string(),
            remote_url: Some("git@github.com:example/repo.git".to_string()),
            ..serde_json::from_value(serde_json::json!({
                "id":"repo-1","name":"repo","path":repository_path.to_string_lossy(),
                "locality":"Local","lifecycle":"Active","lifecycle_candidate":"Active",
                "provider_state":"Unknown","branch":"main","workspace":{
                    "id":"w","path":repository_path.to_string_lossy(),"is_primary":true,"branch":"main",
                    "dirty":false,"added":0,"removed":0,"line_totals_partial":false,"sync_state":"Synced",
                    "remote_freshness":"Unknown","ahead":0,"behind":0,"integration_state":"Unknown",
                    "target_branch":null,"target_confidence":"Unknown","role":"Primary","role_confidence":"High",
                    "activity":{"state":"Unknown","confidence":"Low","signals":[]}
                },"workspaces":[],"branches":[],"conditions":[],"last_scan_at":"2026-07-26T11:00:00Z",
                "last_fetch_at":null,"last_activity_at":null
            })).expect("repository fixture should decode")
        };
        let imported = audit_import(Some(&root.join("audit")), &[repository]);
        assert_eq!(
            imported.portfolio.maturity_score_display.as_deref(),
            Some("1.933")
        );
        assert_eq!(imported.portfolio.matched_repository_count, 1);
        assert_eq!(
            imported
                .maturities
                .values()
                .next()
                .and_then(|maturity| maturity.score_display.as_deref()),
            Some("4.0")
        );
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn audit_import_selects_latest_valid_run_and_does_not_guess_unmatched_repositories() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        fs::create_dir_all(&repository_path).expect("repository should be writable");
        let audit_root = root.join("audit");
        let older = audit_root.join("older");
        let latest = audit_root.join("latest");
        let invalid = audit_root.join("invalid");
        for directory in [&older, &latest, &invalid] {
            fs::create_dir_all(directory.join("findings")).expect("audit should be writable");
        }
        fs::write(
            older.join("summary.json"),
            r#"{"audit_id":"older","as_of":"2026-07-20T11:00:00Z","mean_maturity":1.2}"#,
        )
        .expect("older summary should be writable");
        fs::write(
            latest.join("summary.json"),
            r#"{"audit_id":"latest","as_of":"2026-07-26T11:00:00Z","mean_maturity":1.933}"#,
        )
        .expect("latest summary should be writable");
        fs::write(
            invalid.join("summary.json"),
            r#"{"audit_id":"invalid","as_of":"not-a-timestamp","mean_maturity":4.0}"#,
        )
        .expect("invalid summary should be writable");
        fs::write(
            older.join("findings").join("repository.json"),
            format!(
                r#"{{"canonical_path":"{}","mean_maturity":1.2}}"#,
                repository_path.display()
            ),
        )
        .expect("older finding should be writable");

        let repository = fixture_repository(&repository_path);
        let imported = audit_import(Some(&audit_root), &[repository]);
        assert_eq!(
            imported.portfolio.latest_audit_id.as_deref(),
            Some("latest")
        );
        assert_eq!(
            imported.portfolio.maturity_score_display.as_deref(),
            Some("1.933")
        );
        assert_eq!(imported.portfolio.matched_repository_count, 0);
        assert!(imported.maturities.is_empty());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn composite_maturity_uses_pillars_and_excludes_product_progress() {
        let root = fixture_root();
        let mut repository = fixture_repository(&root.join("repo"));
        repository.quality.maturity.dimension_scores =
            BTreeMap::from([("source.one".to_string(), 4.0)]);
        repository.quality.ci_readiness.configuration_score = Some(4.0);
        repository.quality.ci_readiness.evidence_coverage_score = Some(2.0);
        repository.quality.ci_readiness.score = Some(0.0);
        repository.quality.mac_control_ideal_state.applicability = "Not applicable".to_string();
        let mut repositories = vec![repository];
        let mut portfolio = QualityPortfolioSnapshot {
            maturity_score: Some(2.0),
            maturity_score_display: Some("2.000".to_string()),
            scored_dimension_count: Some(10),
            ..QualityPortfolioSnapshot::default()
        };

        update_composite_maturity_summary(&mut portfolio, &mut repositories);

        assert_eq!(portfolio.source_maturity_score, Some(2.0));
        assert_eq!(portfolio.source_scored_dimension_count, Some(10));
        assert_eq!(portfolio.scored_dimension_count, Some(1));
        assert_eq!(portfolio.maturity_score, Some(0.0));
        assert_eq!(
            repositories[0]
                .quality
                .maturity
                .dimension_scores
                .get("ci.fresh_passing"),
            Some(&0.0)
        );
        assert!(!repositories[0]
            .quality
            .maturity
            .dimension_scores
            .contains_key("project_compass.mvp_progress"));
        let model = repositories[0]
            .quality
            .maturity
            .repository_maturity
            .as_ref()
            .expect("composite summary should expose the holistic model");
        assert_eq!(model.status, "blocked");
        assert!(model.critical_cap.applied);
        assert_eq!(model.evidence.assessed_pillar_count, 1);
        assert!(model
            .evidence
            .unmapped_dimensions
            .contains(&"source.one".to_string()));
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }

    #[test]
    fn repository_maturity_caps_critical_blockers() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([
                ("quality_commands".to_string(), 4.0),
                ("security_constraints".to_string(), 4.0),
                ("architecture_boundaries".to_string(), 4.0),
                ("observability".to_string(), 4.0),
            ]),
            gaps: vec![QualityMaturityGap {
                dimension: "security_constraints".to_string(),
                status: "blocked".to_string(),
                score: Some(4.0),
                message: "Security verification is blocked.".to_string(),
            }],
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);

        assert_eq!(model.uncapped_score, Some(4.0));
        assert_eq!(model.score, Some(2.0));
        assert_eq!(model.status, "blocked");
        assert!(model.critical_cap.applied);
    }

    #[test]
    fn repository_maturity_keeps_conditional_applicability_explicit() {
        let maturity = QualityMaturity {
            dimension_scores: BTreeMap::from([
                ("quality_commands".to_string(), 4.0),
                ("security_constraints".to_string(), 4.0),
                ("architecture_boundaries".to_string(), 4.0),
                ("observability".to_string(), 4.0),
            ]),
            agent_usability: Some(AgentUsabilityMaturity {
                applicability: "not_applicable".to_string(),
                ..AgentUsabilityMaturity::default()
            }),
            ..QualityMaturity::default()
        };

        let model = build_repository_maturity_model(&maturity);

        assert_eq!(model.score, Some(4.0));
        assert_eq!(model.status, "provisional");
        assert_eq!(model.pillars[4].applicability, "unknown");
        assert_eq!(model.pillars[5].applicability, "not_applicable");
        assert_eq!(model.pillars[6].applicability, "unknown");
        assert_eq!(model.evidence.assessed_pillar_count, 4);
        assert_eq!(model.evidence.assessed_weight, 0.228);
        assert_eq!(model.evidence.evidence_coverage, 0.309);
    }

    #[test]
    fn safe_report_paths_reject_escape() {
        let root = fixture_root();
        let allowed = root.join("allowed");
        fs::create_dir_all(&allowed).expect("allowed root should be writable");
        let report = allowed.join("report.json");
        fs::write(&report, "{}").expect("report should be writable");
        assert!(safe_report_path(&report, std::slice::from_ref(&allowed)).is_ok());
        let sibling = root.join("allowed-escape");
        fs::create_dir_all(&sibling).expect("sibling should be writable");
        let sibling_report = sibling.join("report.json");
        fs::write(&sibling_report, "{}").expect("sibling report should be writable");
        assert!(safe_report_path(&sibling_report, std::slice::from_ref(&allowed)).is_err());
        fs::remove_dir_all(root).expect("fixture root should be removable");
    }
}
