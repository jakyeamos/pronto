pub const SCHEMA_VERSION: &str = "pronto-telescope/v1";
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
    pub warnings: Vec<TelescopeWarning>,
    pub enrichment: TelescopeEnrichment,
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
