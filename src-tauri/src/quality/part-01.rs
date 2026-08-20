use crate::behavior_assurance::{
    BehaviorAssurancePortfolioState, BehaviorAssuranceRepositoryState,
};

use crate::core::{CheckSnapshot, RemoteRepositorySnapshot, RepositorySnapshot};

use crate::evidence_contract::{EvidenceContractFleetCoverage, EvidenceContractRepositoryStatus};

use crate::installed_runtime::{self, InstalledRuntimeSnapshot};

use crate::mac_control_maturity::{
    MacControlEvaluation, MacControlPortfolioSnapshot, MacControlRepositoryState,
};

use crate::release_boundary::{self, ReleaseBoundarySnapshot};

use chrono::{DateTime, Duration, Utc};

use serde::{Deserialize, Serialize};

use serde_json::Value;

use sha2::{Digest, Sha256};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use std::fs;

use std::path::{Component, Path, PathBuf};

use std::process::{Command, Stdio};

use std::thread;

use std::time::{Duration as StdDuration, Instant};

pub const MAX_EVIDENCE_AGE_DAYS: i64 = 7;

pub const FINDING_DISPOSITIONS_SCHEMA: &str = "pronto-quality-finding-dispositions/v1";

pub const FINDING_DISPOSITIONS_RELATIVE_PATH: &str = ".pronto/quality-finding-dispositions.json";

pub const CI_GATE_PROFILE_SCHEMA: &str = "pronto-ci-gate-profile/v1";

pub const CI_GATE_PROFILE_RELATIVE_PATH: &str = ".pronto/ci-gate-profile.json";

pub const CANONICAL_MATURITY_FEED_RELATIVE_PATH: &str =
    ".quality-runner/fleet-audit/current/maturity.json";

pub const CANONICAL_MATURITY_CHECKPOINT_RELATIVE_PATH: &str =
    ".quality-runner/fleet-audit/current/maturity-checkpoint.json";

const MATURITY_CHECKPOINT_SCHEMA: &str = "quality-runner-maturity-checkpoint/v1";

const MATURITY_FEED_SCHEMAS: [&str; 2] = [
    "quality-runner-maturity-feed/v1",
    "quality-runner-maturity-feed/v2",
];

const CI_GATE_AUDIT_SCHEMA: &str = "quality-runner-ci-gate-candidates/v1";

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

pub const CI_STANDARD_GATE_IDS: [&str; 9] = [
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

const RECOMMENDATION_MATRIX_MARKDOWN: &str =
    include_str!("../../../docs/quality-gate-recommendation-matrix.md");

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
