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
            profile_source: default_ci_profile_source(),
            profile_contract_path: None,
            profile_reason: None,
            profile_error: None,
            optional_gate_ids: Vec::new(),
            not_applicable_gate_ids: Vec::new(),
            gate_labels: BTreeMap::new(),
            gate_reasons: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CiGateProfile {
    pub source: String,
    pub contract_path: Option<String>,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub required_gate_ids: Vec<String>,
    pub optional_gate_ids: Vec<String>,
    pub not_applicable_gate_ids: Vec<String>,
    pub gate_labels: BTreeMap<String, String>,
    pub gate_reasons: BTreeMap<String, String>,
}

impl Default for CiGateProfile {
    fn default() -> Self {
        Self {
            source: default_ci_profile_source(),
            contract_path: None,
            reason: None,
            error: None,
            required_gate_ids: Vec::new(),
            optional_gate_ids: Vec::new(),
            not_applicable_gate_ids: Vec::new(),
            gate_labels: BTreeMap::new(),
            gate_reasons: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RepositoryCiGateProfileContract {
    schema_version: String,
    reason: String,
    gates: Vec<RepositoryCiGateDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
struct RepositoryCiGateDefinition {
    id: String,
    #[serde(default)]
    label: Option<String>,
    classification: CiGateClassification,
    reason: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CiGateClassification {
    Required,
    Optional,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySnapshot {
    pub gates: Vec<QualityGate>,
    pub findings: QualityFindings,
    pub maturity: QualityMaturity,
    #[serde(default)]
    pub foundation_readiness: FoundationReadinessGate,
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
            foundation_readiness: FoundationReadinessGate::default(),
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
    pub ci_profile_repository_contract_count: usize,
    #[serde(default)]
    pub ci_profile_compatibility_count: usize,
    #[serde(default)]
    pub ci_profile_invalid_count: usize,
    #[serde(default)]
    pub ci_profile_unavailable_count: usize,
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
    #[serde(default)]
    pub maturity_checkpoint: MaturityCheckpointSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaturityCheckpointSnapshot {
    pub status: String,
    pub publication_status: String,
    pub quality_status: String,
    pub freshness: QualityFreshness,
    #[serde(default)]
    pub checkpoint_id: Option<String>,
    #[serde(default)]
    pub observed_at: Option<String>,
    #[serde(default)]
    pub qr_audit_id: Option<String>,
    #[serde(default)]
    pub mac_control_audit_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

impl Default for MaturityCheckpointSnapshot {
    fn default() -> Self {
        Self {
            status: "Not configured".to_string(),
            publication_status: "Unknown".to_string(),
            quality_status: "Unknown".to_string(),
            freshness: QualityFreshness::Unknown,
            checkpoint_id: None,
            observed_at: None,
            qr_audit_id: None,
            mac_control_audit_id: None,
            path: None,
            reason: None,
        }
    }
}

impl MaturityCheckpointSnapshot {
    pub fn legacy() -> Self {
        Self {
            status: "Legacy separate".to_string(),
            publication_status: "Legacy".to_string(),
            quality_status: "Unknown".to_string(),
            freshness: QualityFreshness::Unknown,
            reason: Some(
                "QR maturity and Mac Control evidence are available as separate legacy feeds."
                    .to_string(),
            ),
            ..Self::default()
        }
    }
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
            ci_profile_repository_contract_count: 0,
            ci_profile_compatibility_count: 0,
            ci_profile_invalid_count: 0,
            ci_profile_unavailable_count: 0,
            feed_schema: None,
            provenance_hash: None,
            quality_outcome_counts: BTreeMap::new(),
            quality_outcome_taxonomy: BTreeMap::new(),
            mac_control_ideal_state: MacControlPortfolioSnapshot::default(),
            behavior_assurance: BehaviorAssurancePortfolioState::default(),
            evidence_contracts: Vec::new(),
            maturity_checkpoint: MaturityCheckpointSnapshot::default(),
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
pub struct CoordinatedMaturityImport {
    pub audit: AuditImport,
    pub mac_control: MacControlEvaluation,
    pub checkpoint: MaturityCheckpointSnapshot,
}

pub fn maturity_checkpoint_import(
    checkpoint_path: Option<&Path>,
    repositories: &[RepositorySnapshot],
) -> Option<CoordinatedMaturityImport> {
    let checkpoint_path = checkpoint_path?;
    if checkpoint_path.is_symlink() {
        return Some(blocked_maturity_checkpoint_import(
            checkpoint_path,
            repositories,
            "The canonical maturity checkpoint must not be a symlink.",
        ));
    }
    if !checkpoint_path.exists() {
        return None;
    }
    if !checkpoint_path.is_file() {
        return Some(blocked_maturity_checkpoint_import(
            checkpoint_path,
            repositories,
            "The canonical maturity checkpoint is not a regular file.",
        ));
    }
    let result = read_json(checkpoint_path)
        .ok_or_else(|| "The canonical maturity checkpoint is invalid JSON.".to_string())
        .and_then(|checkpoint| {
            load_coordinated_maturity_checkpoint(checkpoint_path, repositories, &checkpoint)
        });
    Some(match result {
        Ok(import) => import,
        Err(reason) => blocked_maturity_checkpoint_import(checkpoint_path, repositories, &reason),
    })
}
