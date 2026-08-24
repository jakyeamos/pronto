#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorContract {
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub applicability: String,
    #[serde(default)]
    pub behaviors: Vec<BehaviorContractBehavior>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorContractBehavior {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tier: u8,
    #[serde(default)]
    pub automation: String,
    #[serde(default, rename = "change_triggers")]
    pub change_triggers: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub scenarios: Vec<BehaviorContractScenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorContractScenario {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub verification_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorAssuranceGap {
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub behavior_id: Option<String>,
    #[serde(default)]
    pub scenario_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorAssuranceVerification {
    pub behavior_id: String,
    pub scenario_id: String,
    pub status: String,
    pub verification_level: String,
    pub receipt_id: String,
    pub receipt_commit: String,
    pub carried_forward: bool,
    #[serde(default)]
    pub accepted_defect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorCoverageCounts {
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub profiled: usize,
    #[serde(default)]
    pub verified: usize,
    #[serde(default)]
    pub stale: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub blocked: usize,
    #[serde(default)]
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorScenarioCoverage {
    pub behavior_id: String,
    pub scenario_id: String,
    pub tier: usize,
    #[serde(default)]
    pub profiled: bool,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub side_effects: Option<String>,
    pub status: String,
    #[serde(default)]
    pub verification_level: Option<String>,
    #[serde(default)]
    pub receipt_id: Option<String>,
    pub freshness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorCategoryGap {
    pub category: String,
    pub scenario_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorCoverage {
    #[serde(flatten)]
    pub counts: BehaviorCoverageCounts,
    #[serde(default)]
    pub profile_status: String,
    #[serde(default)]
    pub per_tier: BTreeMap<String, BehaviorCoverageCounts>,
    #[serde(default)]
    pub per_edge_category: BTreeMap<String, BehaviorCoverageCounts>,
    #[serde(default)]
    pub category_gaps: Vec<BehaviorCategoryGap>,
    #[serde(default)]
    pub scenarios: Vec<BehaviorScenarioCoverage>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorAssuranceRepositoryState {
    pub schema: String,
    pub applicability: String,
    #[serde(default = "default_behavior_assurance_state")]
    pub state: String,
    pub contract_status: String,
    #[serde(default)]
    pub contract_schema: Option<String>,
    #[serde(default)]
    pub edge_profile_status: String,
    pub result_status: String,
    pub freshness: String,
    pub release_ready: bool,
    pub score: Option<f64>,
    pub contract_path: String,
    pub receipt_directory: String,
    #[serde(default)]
    pub contract_digest: Option<String>,
    #[serde(default)]
    pub target_branch: Option<String>,
    #[serde(default)]
    pub target_commit: Option<String>,
    #[serde(default)]
    pub observed_at: Option<String>,
    #[serde(default)]
    pub required_scenario_count: usize,
    #[serde(default)]
    pub passed_scenario_count: usize,
    #[serde(default)]
    pub accepted_defect_count: usize,
    #[serde(default)]
    pub receipt_count: usize,
    #[serde(default)]
    pub verified: Vec<BehaviorAssuranceVerification>,
    #[serde(default)]
    pub coverage: BehaviorCoverage,
    #[serde(default)]
    pub gaps: Vec<BehaviorAssuranceGap>,
    #[serde(default)]
    pub detail: Option<String>,
    pub next_step: String,
}
