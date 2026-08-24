#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeMapReadiness {
    pub state: String,
    pub reason: String,
    #[serde(default)]
    pub requirements: Vec<TelescopeReadinessRequirement>,
    #[serde(default)]
    pub blocking_gap_keys: Vec<String>,
    #[serde(default)]
    pub enhancement_gap_keys: Vec<String>,
    pub reviewed_fingerprint: Option<String>,
    pub current_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeReadinessRequirement {
    pub key: String,
    pub label: String,
    pub applicability: String,
    pub status: String,
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<TelescopeAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeKnowledgeGap {
    pub key: String,
    pub category: String,
    pub question: String,
    pub why_source_cannot_answer: String,
    #[serde(default)]
    pub unlocks: Vec<String>,
    #[serde(default)]
    pub candidate_answers: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<TelescopeAnchor>,
    #[serde(default)]
    pub allowed_responses: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub completion_criteria: Vec<String>,
    #[serde(default)]
    pub manifest_fields: Vec<String>,
    pub blocking: bool,
    pub freshness: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeKnowledgeTask {
    pub id: String,
    pub stable_gap_key: String,
    pub domain: String,
    pub status: String,
    pub title: String,
    pub question: String,
    pub summary: String,
    pub priority: String,
    pub dependency_order: usize,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub unlocks: Vec<String>,
    #[serde(default)]
    pub candidate_answers: Vec<String>,
    #[serde(default)]
    pub allowed_responses: Vec<String>,
    #[serde(default)]
    pub completion_criteria: Vec<String>,
    #[serde(default)]
    pub manifest_fields: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<TelescopeAnchor>,
    pub freshness: String,
    pub provenance: String,
    pub read_only: bool,
    pub guarded_handoff: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeActor {
    pub id: String,
    pub label: String,
    pub role: String,
    pub metaphor: String,
    pub description: String,
    #[serde(default)]
    pub action_ids: Vec<String>,
    #[serde(default)]
    pub node_ids: Vec<String>,
    pub status: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopePayload {
    pub id: String,
    pub label: String,
    pub metaphor: String,
    pub description: String,
    #[serde(default)]
    pub flow_ids: Vec<String>,
    #[serde(default)]
    pub data_shapes: Vec<String>,
    pub status: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeScope {
    pub id: String,
    pub level: String,
    pub label: String,
    pub purpose: String,
    #[serde(default)]
    pub group_ids: Vec<String>,
    #[serde(default)]
    pub node_ids: Vec<String>,
    #[serde(default)]
    pub edge_ids: Vec<String>,
    #[serde(default)]
    pub flow_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeReadinessReceipt {
    pub schema_version: String,
    pub lane: String,
    pub state: String,
    pub applicability: String,
    pub workspace_fingerprint: String,
    pub generated_at: String,
    pub architecture_visibility_ready: bool,
    #[serde(default)]
    pub blocking_gap_keys: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeIdentity {
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub audience: Vec<String>,
    #[serde(default)]
    pub outcomes: Vec<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeActor {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub metaphor: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "actionIds")]
    pub action_ids: Vec<String>,
    #[serde(default, rename = "nodeIds")]
    pub node_ids: Vec<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativePayload {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub metaphor: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "flowIds")]
    pub flow_ids: Vec<String>,
    #[serde(default, rename = "dataShapes")]
    pub data_shapes: Vec<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeDecision {
    pub id: String,
    pub label: String,
    pub explanation: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeFailure {
    pub id: String,
    pub label: String,
    pub behavior: String,
    #[serde(default, rename = "actionIds")]
    pub action_ids: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeApplicabilityDecision {
    pub requirement: String,
    pub state: String,
    pub reason: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeReview {
    #[serde(default, rename = "reviewedFingerprint")]
    pub reviewed_fingerprint: Option<String>,
    #[serde(default, rename = "reviewedAt")]
    pub reviewed_at: Option<String>,
    #[serde(default, rename = "reviewerProvenance")]
    pub reviewer_provenance: String,
    #[serde(default, rename = "highImpactClaimIds")]
    pub high_impact_claim_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeStructuredExplanation {
    #[serde(default)]
    pub purpose: String,
    #[serde(default, rename = "userOutcome")]
    pub user_outcome: String,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default, rename = "stateChanges")]
    pub state_changes: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub boundaries: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub failures: Vec<String>,
    #[serde(default)]
    pub security: Vec<String>,
    #[serde(default)]
    pub performance: Vec<String>,
    #[serde(default)]
    pub testing: Vec<String>,
}
