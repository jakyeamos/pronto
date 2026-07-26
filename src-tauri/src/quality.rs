use crate::core::{CheckSnapshot, RemoteRepositorySnapshot, RepositorySnapshot};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_EVIDENCE_AGE_DAYS: i64 = 7;

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

const CONDITIONAL_GATE_DEFINITIONS: [(&str, &str); 1] = [("dependency_audit", "Dependency audit")];

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
    pub severity_counts: BTreeMap<String, u64>,
    pub high_severity_total: u64,
    pub source: Option<QualitySource>,
    pub observed_at: Option<String>,
    pub scanned_commit: Option<String>,
    pub scanned_branch: Option<String>,
    pub freshness: QualityFreshness,
    pub report_path: Option<String>,
}

impl Default for QualityFindings {
    fn default() -> Self {
        Self {
            total: 0,
            severity_counts: BTreeMap::new(),
            high_severity_total: 0,
            source: None,
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
            freshness: QualityFreshness::Unknown,
            report_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMaturity {
    pub score: Option<f64>,
    pub score_display: Option<String>,
    pub scored_dimension_count: Option<u64>,
    pub audit_id: Option<String>,
    pub observed_at: Option<String>,
    pub freshness: QualityFreshness,
    pub report_path: Option<String>,
}

impl Default for QualityMaturity {
    fn default() -> Self {
        Self {
            score: None,
            score_display: None,
            scored_dimension_count: None,
            audit_id: None,
            observed_at: None,
            freshness: QualityFreshness::Unknown,
            report_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySnapshot {
    pub gates: Vec<QualityGate>,
    pub findings: QualityFindings,
    pub maturity: QualityMaturity,
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
            last_ingested_at: None,
            ingestion_status: "No evidence".to_string(),
            ingestion_message: None,
        }
    }
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
    pub audit_status: String,
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
            audit_status: "Not configured".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditImport {
    pub portfolio: QualityPortfolioSnapshot,
    pub maturities: HashMap<String, QualityMaturity>,
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
        "dependency_audit"
        | "dependency_scan"
        | "dependency_check"
        | "security_dependency_audit"
        | "software_composition_analysis" => "dependency_audit".to_string(),
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
    let commit_matches = scanned_commit
        .zip(current_commit)
        .is_some_and(|(scanned, current)| scanned == current);
    let branch_matches = scanned_branch
        .zip(current_branch)
        .is_some_and(|(scanned, current)| scanned == current);
    if commit_matches || branch_matches {
        QualityFreshness::Fresh
    } else if scanned_commit.is_some() || scanned_branch.is_some() {
        QualityFreshness::Stale
    } else {
        QualityFreshness::Unknown
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

pub fn ingest_repository_quality(
    repository: &RepositorySnapshot,
    remote: Option<&RemoteRepositorySnapshot>,
    maturity: Option<QualityMaturity>,
) -> QualitySnapshot {
    let mut gates = default_quality_gates();
    let mut findings = QualityFindings::default();
    let mut last_ingested_at = None;

    if let Some(run) = latest_qr_run(Path::new(&repository.path)) {
        last_ingested_at = run.observed_at.clone();
        for evidence in run.gate_evidence(repository) {
            add_evidence(&mut gates, evidence);
        }
        findings = run.findings(repository);
    }

    for evidence in ci_evidence(repository, remote) {
        add_evidence(&mut gates, evidence);
    }

    for gate in &mut gates {
        let (status, freshness) = aggregate_gate_status(&gate.evidence);
        gate.status = status;
        gate.freshness = freshness;
    }
    let maturity_available = maturity.is_some();
    let evidence_available =
        gates.iter().any(|gate| !gate.evidence.is_empty()) || findings.total > 0;
    QualitySnapshot {
        gates,
        findings,
        maturity: maturity.unwrap_or_default(),
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
    if gate.freshness == QualityFreshness::Conflicted {
        return (
            QualityGateStatus::Blocked,
            QualityFreshness::Conflicted,
            format!(
                "{} has conflicting evidence across imported sources",
                gate.label
            ),
        );
    }
    let evidence = gate
        .evidence
        .iter()
        .filter(|item| item.source == requirement.source)
        .collect::<Vec<_>>();
    if evidence.is_empty() {
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
                audit_id: run.audit_id.clone(),
                observed_at: run.as_of.clone(),
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
    }
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
    observed_at: Option<String>,
}

impl QrRun {
    fn gate_evidence(&self, repository: &RepositorySnapshot) -> Vec<QualityEvidence> {
        let branch = self.branch();
        let commit = self.commit();
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
                if failure_type.is_some() || skip_type.is_some() {
                    status = QualityGateStatus::Blocked;
                }
                let gate_observed_at = json_string_at(&gate, &["completed_at"])
                    .or_else(|| json_string_at(&gate, &["observed_at"]))
                    .or_else(|| observed_at.clone());
                let freshness = evaluate_freshness_at(
                    gate_observed_at.as_deref(),
                    commit.as_deref(),
                    branch.as_deref(),
                    repository.workspace.last_commit.as_deref(),
                    Some(repository.branch.as_str()),
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
        let report_names = [
            "quality-audit.json",
            "completed-report.json",
            "code-quality-scan.json",
            "repo-scan.json",
            "run-summary.json",
        ];
        let Some((report_path, payload)) = report_names
            .iter()
            .map(|name| self.run_dir.join(name))
            .find_map(|path| read_json(&path).map(|payload| (path, payload)))
        else {
            return QualityFindings::default();
        };
        let severity_counts = severity_counts(&payload);
        let total = json_u64_at(&payload, &["finding_count"])
            .or_else(|| json_u64_at(&payload, &["summary", "finding_count"]))
            .or_else(|| json_u64_at(&payload, &["finding_counts", "total"]))
            .or_else(|| json_u64_at(&payload, &["summary", "finding_counts", "total"]))
            .unwrap_or_else(|| {
                if severity_counts.is_empty() {
                    payload
                        .get("findings")
                        .and_then(Value::as_array)
                        .map_or(0, |items| items.len() as u64)
                } else {
                    severity_counts.values().sum()
                }
            });
        let high_severity_total = severity_counts
            .iter()
            .filter(|(severity, _)| matches!(severity.as_str(), "critical" | "high"))
            .map(|(_, count)| *count)
            .sum();
        let branch = self.branch();
        let commit = self.commit();
        QualityFindings {
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
                repository.workspace.last_commit.as_deref(),
                Some(repository.branch.as_str()),
                Utc::now(),
            ),
            report_path: Some(report_path.to_string_lossy().to_string()),
        }
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
    fn freshness_requires_current_ref_and_seven_day_window() {
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
                Some("feature"),
                Some("new"),
                Some("main"),
                now,
            ),
            QualityFreshness::Stale
        );
    }

    #[test]
    fn ingests_qr_gates_and_severity_breakdown_without_running_commands() {
        let root = fixture_root();
        let repository_path = root.join("repo");
        let run = repository_path
            .join(".quality-runner")
            .join("runs")
            .join("run-1");
        fs::create_dir_all(&run).expect("QR run should be writable");
        fs::write(
            run.join("run-manifest.json"),
            r#"{"created_at":"2026-07-26T11:00:00Z","git":{"branch":"main","head_sha":"abc"}}"#,
        )
        .expect("run manifest should be writable");
        fs::write(
            run.join("gate-verification.json"),
            r#"{"gates":[
                {"id":"runtime_smoke","status":"passed","capability_kind":"local_command","command":"pnpm smoke","completed_at":"2026-07-26T11:00:00Z"},
                {"id":"security scan","status":"failed","capability_kind":"qr","reason":"finding threshold exceeded"},
                {"id":"review","status":"skipped","skip_type":"missing evidence"}
            ]}"#,
        )
        .expect("gate verification should be writable");
        fs::write(
            run.join("quality-audit.json"),
            r#"{"findings":[
                {"severity":"critical"}, {"severity":"high"},
                {"severity":"warning"}, {"severity":"low"}
            ]}"#,
        )
        .expect("QR report should be writable");

        let repository = fixture_repository(&repository_path);
        let snapshot = ingest_repository_quality(&repository, None, None);
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
        assert_eq!(security.evidence[0].detail, "failed");
        assert!(snapshot
            .gates
            .iter()
            .any(|gate| gate.id == "custom:review" && gate.status == QualityGateStatus::Blocked));
        assert_eq!(snapshot.findings.total, 4);
        assert_eq!(snapshot.findings.high_severity_total, 2);
        assert_eq!(snapshot.findings.severity_counts.get("medium"), Some(&1));
        assert!(snapshot.findings.report_path.is_some());
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
        let snapshot = ingest_repository_quality(&repository, None, None);
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
            },
        );
        assert_eq!(status, QualityGateStatus::NotConfigured);
        assert_eq!(freshness, QualityFreshness::Unknown);
        assert!(detail.contains("CI evidence"));
        fs::remove_dir_all(root).expect("fixture root should be removable");
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
