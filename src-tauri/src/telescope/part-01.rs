pub const SCHEMA_VERSION: &str = "pronto-telescope/v2";
pub const NARRATIVE_MANIFEST_PATH: &str = ".pronto/telescope-map.json";
pub const VISUAL_MODEL_VERSION: &str = "pronto-telescope-city/v2";
const MAX_SOURCE_FILES: usize = 2_500;
const MAX_SOURCE_BYTES: u64 = 512 * 1024;
const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py", "go", "java", "kt", "swift", "rb", "php",
    "cs", "cpp", "cc", "c", "h", "vue", "svelte",
];
static ACTIVE_REFRESHES: LazyLock<Mutex<BTreeMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeProjection {
    pub schema_version: String,
    pub repository_id: String,
    pub repository_name: String,
    pub binding: TelescopeBinding,
    pub freshness: TelescopeFreshness,
    pub coverage: TelescopeCoverage,
    pub groups: Vec<TelescopeGroup>,
    pub nodes: Vec<TelescopeNode>,
    pub edges: Vec<TelescopeEdge>,
    pub flows: Vec<TelescopeFlow>,
    #[serde(default)]
    pub actions: Vec<TelescopeAction>,
    #[serde(default)]
    pub action_coverage: TelescopeActionCoverage,
    pub warnings: Vec<TelescopeWarning>,
    pub enrichment: TelescopeEnrichment,
    #[serde(default)]
    pub narrative: TelescopeNarrative,
    #[serde(default)]
    pub map_readiness: TelescopeMapReadiness,
    #[serde(default)]
    pub blocking_gaps: Vec<TelescopeKnowledgeGap>,
    #[serde(default)]
    pub enhancement_gaps: Vec<TelescopeKnowledgeGap>,
    #[serde(default)]
    pub knowledge_tasks: Vec<TelescopeKnowledgeTask>,
    #[serde(default)]
    pub actors: Vec<TelescopeActor>,
    #[serde(default)]
    pub payloads: Vec<TelescopePayload>,
    #[serde(default)]
    pub scopes: Vec<TelescopeScope>,
    #[serde(default)]
    pub readiness_receipt: TelescopeReadinessReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeBinding {
    pub workspace_id: String,
    pub branch: String,
    pub commit: Option<String>,
    pub dirty: bool,
    pub dirty_state_fingerprint: String,
    pub workspace_fingerprint: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeFreshness {
    pub state: String,
    pub cache: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeCoverage {
    pub discovered_source_files: usize,
    pub examined_source_files: usize,
    pub supported_source_files: usize,
    pub partial_source_files: usize,
    pub skipped_large_files: usize,
    pub truncated: bool,
    pub resolved_relationships: usize,
    pub inferred_relationships: usize,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeGroup {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub summary: String,
    pub confidence: String,
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub source_file_count: usize,
    #[serde(default)]
    pub measured_lines: usize,
    #[serde(default)]
    pub visual_archetype: String,
    #[serde(default)]
    pub visual_override_provenance: String,
    #[serde(default)]
    pub narrative_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeAnchor {
    pub path: String,
    pub line: Option<usize>,
    pub symbol: Option<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeNode {
    pub id: String,
    pub group_id: String,
    pub label: String,
    pub kind: String,
    pub technology: String,
    pub semantic_summary: String,
    pub implementation_summary: String,
    pub summary_status: String,
    pub confidence: String,
    pub provenance: Vec<String>,
    pub source_anchors: Vec<TelescopeAnchor>,
    pub symbols: Vec<String>,
    pub data_shapes: Vec<String>,
    #[serde(default)]
    pub source_paths: Vec<String>,
    #[serde(default)]
    pub measured_lines: usize,
    #[serde(default)]
    pub source_file_count: usize,
    #[serde(default)]
    pub visual_building_id: Option<String>,
    #[serde(default)]
    pub visual_archetype: String,
    #[serde(default)]
    pub visual_override_provenance: String,
    #[serde(default)]
    pub narrative_status: String,
    #[serde(default)]
    pub city_role: String,
    #[serde(default)]
    pub explanation: TelescopeStructuredExplanation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    pub direction: String,
    pub label: String,
    pub confidence: String,
    pub provenance: String,
    pub inferred: bool,
    pub source_anchor: Option<TelescopeAnchor>,
    #[serde(default)]
    pub rail_kind: String,
    #[serde(default)]
    pub visual_override_provenance: String,
    #[serde(default)]
    pub narrative_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeFlow {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub data_shape: Option<String>,
    pub confidence: String,
    pub provenance: String,
    #[serde(default)]
    pub narrative_status: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeAction {
    pub id: String,
    pub label: String,
    pub verb: String,
    pub category: String,
    pub what_it_does: String,
    pub how_its_built: String,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub flow_id: Option<String>,
    #[serde(default)]
    pub behavior_id: Option<String>,
    #[serde(default)]
    pub scenario_ids: Vec<String>,
    #[serde(default)]
    pub behavior_state: String,
    #[serde(default)]
    pub behavior_verification: String,
    pub source_anchors: Vec<TelescopeAnchor>,
    pub status: String,
    pub confidence: String,
    pub provenance: String,
    pub read_only: bool,
    pub guarded: bool,
    #[serde(default)]
    pub explanation: TelescopeStructuredExplanation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeActionCoverage {
    #[serde(default)]
    pub inventory_status: String,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub authored: usize,
    #[serde(default)]
    pub inferred: usize,
    #[serde(default)]
    pub partial: usize,
    #[serde(default)]
    pub mapped: usize,
    #[serde(default)]
    pub unmapped: usize,
    #[serde(default)]
    pub behavior_backed: usize,
    #[serde(default)]
    pub unprofiled: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeWarning {
    pub code: String,
    pub message: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelescopeEnrichment {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub source_content_transmitted: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrative {
    #[serde(default)]
    pub manifest_path: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub manifest_fingerprint: Option<String>,
    #[serde(default)]
    pub measured_fingerprint: Option<String>,
    #[serde(default)]
    pub visual_model_version: String,
    #[serde(default)]
    pub primary_flow_id: Option<String>,
    #[serde(default)]
    pub authored_groups: Vec<TelescopeNarrativeGroup>,
    #[serde(default)]
    pub authored_nodes: Vec<TelescopeNarrativeNode>,
    #[serde(default)]
    pub authored_edges: Vec<TelescopeNarrativeEdge>,
    #[serde(default)]
    pub authored_flows: Vec<TelescopeNarrativeFlow>,
    #[serde(default)]
    pub authored_actions: Vec<TelescopeNarrativeAction>,
    #[serde(default)]
    pub coverage: TelescopeNarrativeCoverage,
    #[serde(default)]
    pub drift_warnings: Vec<TelescopeWarning>,
    #[serde(default)]
    pub identity: TelescopeNarrativeIdentity,
    #[serde(default)]
    pub actors: Vec<TelescopeNarrativeActor>,
    #[serde(default)]
    pub payloads: Vec<TelescopeNarrativePayload>,
    #[serde(default)]
    pub decisions: Vec<TelescopeNarrativeDecision>,
    #[serde(default)]
    pub failures: Vec<TelescopeNarrativeFailure>,
    #[serde(default)]
    pub applicability: Vec<TelescopeApplicabilityDecision>,
    #[serde(default)]
    pub review: TelescopeNarrativeReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeCoverage {
    #[serde(default)]
    pub authored_source_files: usize,
    #[serde(default)]
    pub mapped_source_files: usize,
    #[serde(default)]
    pub unmapped_source_files: Vec<String>,
    #[serde(default)]
    pub coverage_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeGroup {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, rename = "pathPrefixes")]
    pub path_prefixes: Vec<String>,
    #[serde(default, rename = "visualArchetype")]
    pub visual_archetype: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeNode {
    pub id: String,
    pub label: String,
    #[serde(default, rename = "groupId")]
    pub group_id: String,
    #[serde(default, rename = "whatItDoes")]
    pub what_it_does: String,
    #[serde(default, rename = "howItsBuilt")]
    pub how_its_built: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default, rename = "visualArchetype")]
    pub visual_archetype: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "cityRole")]
    pub city_role: String,
    #[serde(default)]
    pub explanation: TelescopeStructuredExplanation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeEdge {
    pub id: String,
    #[serde(rename = "sourceFile")]
    pub source_file: String,
    #[serde(rename = "targetFile")]
    pub target_file: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default, rename = "railKind")]
    pub rail_kind: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeFlow {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default, rename = "nodeIds")]
    pub node_ids: Vec<String>,
    #[serde(default, rename = "edgeIds")]
    pub edge_ids: Vec<String>,
    #[serde(default, rename = "dataShape")]
    pub data_shape: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TelescopeNarrativeAction {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub verb: String,
    #[serde(default)]
    pub category: String,
    #[serde(default, rename = "whatItDoes")]
    pub what_it_does: String,
    #[serde(default, rename = "howItsBuilt")]
    pub how_its_built: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default, rename = "nodeIds")]
    pub node_ids: Vec<String>,
    #[serde(default, rename = "edgeIds")]
    pub edge_ids: Vec<String>,
    #[serde(default, rename = "flowId")]
    pub flow_id: Option<String>,
    #[serde(default, rename = "behaviorId")]
    pub behavior_id: Option<String>,
    #[serde(default, rename = "scenarioIds")]
    pub scenario_ids: Vec<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "readOnly")]
    pub read_only: bool,
    #[serde(default)]
    pub guarded: bool,
    #[serde(default)]
    pub explanation: TelescopeStructuredExplanation,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TelescopeManifest {
    #[serde(default, rename = "schemaVersion")]
    schema_version: String,
    #[serde(default)]
    status: String,
    #[serde(default, rename = "topologyFingerprint")]
    topology_fingerprint: Option<String>,
    #[serde(default, rename = "visualModelVersion")]
    visual_model_version: Option<String>,
    #[serde(default, rename = "primaryFlowId")]
    primary_flow_id: Option<String>,
    #[serde(default)]
    groups: Vec<TelescopeNarrativeGroup>,
    #[serde(default)]
    nodes: Vec<TelescopeNarrativeNode>,
    #[serde(default)]
    edges: Vec<TelescopeNarrativeEdge>,
    #[serde(default)]
    flows: Vec<TelescopeNarrativeFlow>,
    #[serde(default)]
    actions: Vec<TelescopeNarrativeAction>,
    #[serde(default)]
    identity: TelescopeNarrativeIdentity,
    #[serde(default)]
    actors: Vec<TelescopeNarrativeActor>,
    #[serde(default)]
    payloads: Vec<TelescopeNarrativePayload>,
    #[serde(default)]
    decisions: Vec<TelescopeNarrativeDecision>,
    #[serde(default)]
    failures: Vec<TelescopeNarrativeFailure>,
    #[serde(default)]
    applicability: Vec<TelescopeApplicabilityDecision>,
    #[serde(default)]
    review: TelescopeNarrativeReview,
}

#[derive(Debug, Clone)]
pub struct TelescopeRequest<'a> {
    pub repository_id: &'a str,
    pub repository_name: &'a str,
    pub workspace_id: &'a str,
    pub workspace_path: &'a Path,
    pub branch: &'a str,
    pub known_commit: Option<&'a str>,
    pub known_dirty: bool,
}

#[derive(Debug, Clone)]
struct SourceFile {
    relative_path: String,
    absolute_path: PathBuf,
    language: String,
    supported: bool,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct ImportRecord {
    source_path: String,
    line: usize,
    specifier: String,
    kind: String,
    confidence: String,
}
