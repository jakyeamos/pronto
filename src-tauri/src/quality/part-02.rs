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
pub struct CacheDesignTotals {
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
    pub exclusive_allocated_bytes: u64,
    pub shared_allocated_bytes: u64,
    pub file_count: u64,
    pub shared_file_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CacheDesignCategory {
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
    pub exclusive_allocated_bytes: u64,
    pub shared_allocated_bytes: u64,
    pub file_count: u64,
    pub shared_file_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CacheDesignAssessment {
    pub schema: String,
    pub status: String,
    pub score: Option<u8>,
    pub measurement_complete: bool,
    pub totals: CacheDesignTotals,
    pub categories: BTreeMap<String, CacheDesignCategory>,
    pub risk_flags: Vec<String>,
    pub growth: Value,
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
    pub not_applicable_capabilities: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct FoundationReadinessReason {
    pub id: String,
    pub severity: String,
    pub summary: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FoundationReadinessGate {
    pub schema: String,
    pub label: String,
    pub disposition: String,
    pub confidence: String,
    pub freshness: QualityFreshness,
    pub advisory_only: bool,
    pub execution_authority: bool,
    pub blocks_urgent_fixes: bool,
    pub summary: String,
    pub reasons: Vec<FoundationReadinessReason>,
    pub unknowns: Vec<String>,
    pub next_step: String,
    pub observed_at: Option<String>,
    pub scanned_commit: Option<String>,
    pub scanned_branch: Option<String>,
}

impl Default for FoundationReadinessGate {
    fn default() -> Self {
        Self {
            schema: "pronto-foundation-readiness/v1".to_string(),
            label: "Modernization readiness".to_string(),
            disposition: "unknown".to_string(),
            confidence: "low".to_string(),
            freshness: QualityFreshness::Unknown,
            advisory_only: true,
            execution_authority: false,
            blocks_urgent_fixes: false,
            summary: "Repository modernization readiness is unknown because current foundation evidence is unavailable.".to_string(),
            reasons: Vec::new(),
            unknowns: vec!["repository_maturity_evidence".to_string()],
            next_step: "Refresh repository maturity evidence before choosing whether to extend or modernize the foundation.".to_string(),
            observed_at: None,
            scanned_commit: None,
            scanned_branch: None,
        }
    }
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateAuditRepository {
    pub name: String,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateAuditPolicy {
    pub authority: String,
    pub implementation_allowed: bool,
    pub promotion_requirement: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateCandidateEvidence {
    pub kind: Option<String>,
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateCheckContext {
    pub context: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateExistingCheck {
    pub status: String,
    pub contexts: Vec<CiGateCheckContext>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateSuggestedTrigger {
    pub event: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateAdmission {
    pub state: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateCandidate {
    pub id: String,
    pub label: String,
    pub recommendation: String,
    pub confidence: String,
    pub invariant: String,
    pub failure_mode: String,
    pub evidence: Vec<CiGateCandidateEvidence>,
    pub suggested_trigger: CiGateSuggestedTrigger,
    pub suggested_check_context: String,
    pub existing_check: CiGateExistingCheck,
    pub negative_controls: Vec<CiGateCandidateEvidence>,
    pub admission: CiGateAdmission,
    pub next_step: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CiGateCandidateAudit {
    pub schema: String,
    pub status: String,
    pub generated_at: String,
    pub repository: CiGateAuditRepository,
    pub policy: CiGateAuditPolicy,
    pub candidate_count: usize,
    pub candidates: Vec<CiGateCandidate>,
    pub provenance_hash: String,
    pub error: Option<String>,
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
    #[serde(default)]
    pub cache_design: Option<CacheDesignAssessment>,
    #[serde(default)]
    pub ci_gate_audit: Option<CiGateCandidateAudit>,
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
            cache_design: None,
            ci_gate_audit: None,
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
    #[serde(default = "default_ci_profile_source")]
    pub profile_source: String,
    #[serde(default)]
    pub profile_contract_path: Option<String>,
    #[serde(default)]
    pub profile_reason: Option<String>,
    #[serde(default)]
    pub profile_error: Option<String>,
    #[serde(default)]
    pub optional_gate_ids: Vec<String>,
    #[serde(default)]
    pub not_applicable_gate_ids: Vec<String>,
    #[serde(default)]
    pub gate_labels: BTreeMap<String, String>,
    #[serde(default)]
    pub gate_reasons: BTreeMap<String, String>,
}

fn default_ci_profile_source() -> String {
    "unavailable".to_string()
}
