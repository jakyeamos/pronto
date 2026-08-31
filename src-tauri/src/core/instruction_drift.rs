const INSTRUCTION_DRIFT_SCHEMA: &str = "pronto-instruction-drift/v1";

#[derive(Debug, Clone, Serialize)]
struct InstructionSourceRecord {
    path: String,
    scope: String,
    precedence: String,
    status: String,
    sha256: Option<String>,
    line_count: Option<usize>,
    repository_id: Option<String>,
    repository_name: Option<String>,
    repository_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct InstructionEvidence {
    path: String,
    line: usize,
    sha256: String,
    scope: String,
    precedence: String,
    confidence: String,
    excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyAtom {
    id: String,
    domain: String,
    subject: String,
    trigger: String,
    action: String,
    polarity: String,
    strength: String,
    source: InstructionEvidence,
}

#[derive(Debug, Clone, Serialize)]
struct AuditRepository {
    id: String,
    name: String,
    path: String,
}

#[derive(Debug, Clone, Serialize)]
struct InstructionFinding {
    id: String,
    classification: String,
    severity: String,
    confidence: String,
    comparison_scope: String,
    subject: String,
    rationale: String,
    repository: Option<AuditRepository>,
    baseline: Vec<PolicyAtom>,
    override_atoms: Vec<PolicyAtom>,
    evidence: Vec<InstructionEvidence>,
}

#[derive(Debug, Clone, Serialize)]
struct SkippedInstructionSource {
    path: String,
    scope: String,
    reason: String,
    repository: Option<AuditRepository>,
}

#[derive(Debug, Clone, Serialize)]
struct InstructionSourceInventory {
    status: String,
    global_source_count: usize,
    repository_source_count: usize,
    readable_source_count: usize,
    blocked_source_count: usize,
    global_atom_count: usize,
    repository_atom_count: usize,
    sources: Vec<InstructionSourceRecord>,
}

#[derive(Debug, Clone, Serialize)]
struct CachedRepositoryEvidence {
    repository: AuditRepository,
    branch: String,
    head_sha: Option<String>,
    dirty: bool,
    last_scan_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct CachedProntoEvidence {
    status: String,
    storage_path: String,
    repository_count: usize,
    quality_matched_repository_count: usize,
    quality_audit_status: Option<String>,
    oldest_scan_at: Option<String>,
    repositories: Vec<CachedRepositoryEvidence>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LiveGitRepositoryEvidence {
    repository: AuditRepository,
    status: String,
    branch: Option<String>,
    head_sha: Option<String>,
    dirty: Option<bool>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CachedLiveMismatch {
    repository: AuditRepository,
    fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LiveGitEvidence {
    status: String,
    observed_repository_count: usize,
    unavailable_repository_count: usize,
    dirty_repository_count: usize,
    cached_live_mismatch_count: usize,
    repositories: Vec<LiveGitRepositoryEvidence>,
    mismatches: Vec<CachedLiveMismatch>,
}

#[derive(Debug, Clone, Serialize)]
struct EvidenceFreshness {
    status: String,
    read_only: bool,
    detail: String,
    cached_repository_count: usize,
    live_repository_count: usize,
    cached_live_mismatch_count: usize,
    blocked_repository_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct InstructionDriftSummary {
    compatible: usize,
    local_stricter_override: usize,
    direct_conflict: usize,
    stale_pointer: usize,
    needs_review: usize,
    repository_count: usize,
    finding_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct InstructionDriftReport {
    schema_version: String,
    generated_at: String,
    mode: String,
    status: String,
    authorization: String,
    source_inventory: InstructionSourceInventory,
    cached_pronto: CachedProntoEvidence,
    live_git: LiveGitEvidence,
    evidence_freshness: EvidenceFreshness,
    instruction_findings: Vec<InstructionFinding>,
    skipped: Vec<SkippedInstructionSource>,
    summary: InstructionDriftSummary,
}

struct SourceDocument {
    record: InstructionSourceRecord,
    contents: String,
}

fn policy_atom_key(atom: &PolicyAtom) -> String {
    [
        atom.id.as_str(),
        atom.domain.as_str(),
        atom.subject.as_str(),
        atom.trigger.as_str(),
        atom.action.as_str(),
        atom.polarity.as_str(),
        atom.strength.as_str(),
    ]
    .join("\u{1f}")
}

fn policy_atom_groups(atoms: &[PolicyAtom]) -> Vec<Vec<PolicyAtom>> {
    let mut grouped = BTreeMap::<String, Vec<PolicyAtom>>::new();
    for atom in atoms {
        grouped
            .entry(policy_atom_key(atom))
            .or_default()
            .push(atom.clone());
    }
    grouped.into_values().collect()
}

fn instruction_drift_home_root(override_path: Option<&str>) -> PathBuf {
    override_path
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn instruction_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn instruction_source_record(
    path: &Path,
    scope: &str,
    precedence: &str,
    repository: Option<&AuditRepository>,
) -> InstructionSourceRecord {
    InstructionSourceRecord {
        path: path.display().to_string(),
        scope: scope.to_string(),
        precedence: precedence.to_string(),
        status: "blocked".to_string(),
        sha256: None,
        line_count: None,
        repository_id: repository.map(|item| item.id.clone()),
        repository_name: repository.map(|item| item.name.clone()),
        repository_path: repository.map(|item| item.path.clone()),
    }
}

fn read_instruction_source(
    path: &Path,
    scope: &str,
    precedence: &str,
    repository: Option<&AuditRepository>,
) -> Result<SourceDocument, (InstructionSourceRecord, String)> {
    let mut record = instruction_source_record(path, scope, precedence, repository);
    let bytes = fs::read(path).map_err(|error| {
        (
            record.clone(),
            format!("could not read {}: {error}", path.display()),
        )
    })?;
    let contents = String::from_utf8_lossy(&bytes).into_owned();
    record.status = "read".to_string();
    record.sha256 = Some(instruction_digest(&bytes));
    record.line_count = Some(contents.lines().count());
    Ok(SourceDocument { record, contents })
}

fn instruction_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn instruction_skip_directory(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | "node_modules"
            | "target"
            | ".venv"
            | "venv"
            | "dist"
            | "build"
            | "coverage"
            | ".next"
            | ".turbo"
            | "vendor"
            | "Pods"
            | ".quality-runner"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | "__pycache__"
    )
}

fn collect_supported_instruction_files(
    root: &Path,
    max_depth: usize,
    extensions: &[&str],
    output: &mut Vec<PathBuf>,
) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            if max_depth > 0 && !instruction_skip_directory(&entry.file_name().to_string_lossy()) {
                collect_supported_instruction_files(&path, max_depth - 1, extensions, output);
            }
        } else if metadata.is_file() {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if extensions
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            {
                output.push(path);
            }
        }
    }
}

fn collect_global_instruction_paths(home: &Path) -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    for (relative, precedence) in [
        ("AGENTS.md", "global_contract"),
        (".codex/AGENTS.md", "provider_projection"),
        (".agents/AGENT_PLAYBOOK.md", "global_routing"),
        (".agents/CONTEXT_CONVENTION.md", "global_routing"),
        (".agents/context-manifest.yaml", "global_routing"),
        (
            ".agents/repository-onboarding-change-matrix.json",
            "global_contract",
        ),
    ] {
        let path = home.join(relative);
        if path.is_file() {
            paths.push((path, precedence.to_string()));
        }
    }

    for (directory, precedence) in [
        (home.join(".agents/policies"), "global_policy"),
        (home.join(".agents/skills"), "global_skill"),
    ] {
        let mut found = Vec::new();
        collect_supported_instruction_files(&directory, 5, &["md"], &mut found);
        found.sort();
        paths.extend(found.into_iter().map(|path| (path, precedence.to_string())));
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    paths.dedup_by(|left, right| left.0 == right.0);
    paths
}

fn local_instruction_file(path: &Path, repository_root: &Path) -> bool {
    let name = instruction_file_name(path);
    if matches!(
        name.as_str(),
        "agents.md" | "claude.md" | "codex.md" | "gemini.md"
    ) {
        return true;
    }
    let relative = path.strip_prefix(repository_root).unwrap_or(path);
    let has_context_directory = relative.components().any(|component| {
        matches!(
            component.as_os_str().to_str().unwrap_or_default(),
            ".agents" | ".context" | ".codex"
        )
    });
    if has_context_directory {
        return true;
    }
    let is_markdown = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("md"));
    is_markdown
        && [
            "contract",
            "development",
            "contributing",
            "guidance",
            "instruction",
            "policy",
            "workflow",
            "custody",
            "branch",
        ]
        .iter()
        .any(|marker| name.contains(marker))
}

fn collect_local_instruction_paths(repository_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut all_documents = Vec::new();
    collect_supported_instruction_files(repository_root, 8, &["md"], &mut all_documents);
    for directory in [".agents", ".context", ".codex"] {
        collect_supported_instruction_files(
            &repository_root.join(directory),
            8,
            &["json", "yaml", "yml"],
            &mut all_documents,
        );
    }
    for path in all_documents {
        if local_instruction_file(&path, repository_root) {
            candidates.push(path);
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn instruction_excerpt(line: &str) -> String {
    let compact = line.trim().split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= 240 {
        return compact;
    }
    compact.chars().take(237).collect::<String>() + "..."
}

fn instruction_evidence(
    record: &InstructionSourceRecord,
    line_number: usize,
    line: &str,
    confidence: &str,
) -> InstructionEvidence {
    InstructionEvidence {
        path: record.path.clone(),
        line: line_number,
        sha256: record.sha256.clone().unwrap_or_default(),
        scope: record.scope.clone(),
        precedence: record.precedence.clone(),
        confidence: confidence.to_string(),
        excerpt: instruction_excerpt(line),
    }
}

fn push_policy_atom(
    atoms: &mut Vec<PolicyAtom>,
    record: &InstructionSourceRecord,
    line_number: usize,
    line: &str,
    id: &str,
    subject: &str,
    trigger: &str,
    action: &str,
    polarity: &str,
    strength: &str,
    confidence: &str,
) {
    atoms.push(PolicyAtom {
        id: id.to_string(),
        domain: id.split('.').next().unwrap_or("unknown").to_string(),
        subject: subject.to_string(),
        trigger: trigger.to_string(),
        action: action.to_string(),
        polarity: polarity.to_string(),
        strength: strength.to_string(),
        source: instruction_evidence(record, line_number, line, confidence),
    });
}

fn extract_policy_atoms(document: &SourceDocument) -> Vec<PolicyAtom> {
    let mut atoms = Vec::new();
    for (line_index, line) in document.contents.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let line_number = line_index + 1;
        let direct_dev = lower.contains("direct_dev") || lower.contains("direct dev");
        let short_lived = lower.contains("short_lived") || lower.contains("short-lived");
        let long_lived = lower.contains("long_lived") || lower.contains("long-lived");
        let isolation = lower.contains("isolat") || lower.contains("worktree") || lower.contains("feature branch");
        let strong_requirement = lower.contains("must")
            || lower.contains("required")
            || lower.contains("requires")
            || lower.contains("only");

        if direct_dev && lower.contains("default") {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.default_direct_dev",
                "change_lane",
                "ordinary_bounded_work",
                "direct_dev",
                "default",
                "default",
                "high",
            );
        }
        if direct_dev
            && (lower.contains("bounded")
                || lower.contains("reversible")
                || lower.contains("single-repository"))
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.direct_dev_eligibility",
                "change_lane",
                "bounded_reversible_no_conflict",
                "direct_dev",
                "permit",
                "condition",
                "high",
            );
        }
        if short_lived && (strong_requirement || lower.contains("creates") || lower.contains("exception")) {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.short_lived_criteria",
                "change_lane",
                "explicit_isolation_or_risk",
                "short_lived",
                "require",
                "condition",
                "high",
            );
        }
        if long_lived
            && (lower.contains("explicit")
                || lower.contains("designat")
                || lower.contains("owner")
                || lower.contains("purpose"))
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.long_lived_designation",
                "change_lane",
                "sustained_product_or_release_track",
                "long_lived",
                "require",
                "condition",
                "high",
            );
        }
        if lower.contains("dirty")
            && (lower.contains("alone") || lower.contains("dirty dev"))
            && (lower.contains("never")
                || lower.contains("does not")
                || lower.contains("not select")
                || lower.contains("not trigger"))
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.dirty_does_not_select_isolation",
                "change_lane",
                "dirty_integration_only",
                "keep_current_lane",
                "prohibit",
                "guard",
                "high",
            );
        } else if lower.contains("dirty")
            && isolation
            && (lower.contains("must")
                || lower.contains("requires")
                || lower.contains("branch")
                || lower.contains("worktree"))
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.dirty_selects_isolation",
                "change_lane",
                "dirty_integration_only",
                "short_lived",
                "require",
                "guard",
                "high",
            );
        }
        if (lower.contains("before modifying a non-trivial")
            || lower.contains("every non-trivial")
            || lower.contains("all non-trivial"))
            && isolation
            && (lower.contains("prove") || strong_requirement)
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.isolation_unconditional",
                "change_lane",
                "any_nontrivial_repository_change",
                "isolated_worktree",
                "require",
                "default",
                "high",
            );
        }
        if (lower.contains("feature") || lower.contains("remediation"))
            && isolation
            && strong_requirement
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.feature_isolated",
                "change_lane",
                "feature_or_remediation_work",
                "isolated_worktree",
                "require",
                "condition",
                "high",
            );
        }
        if (lower.contains("sizable change") || lower.contains("substantial change"))
            && short_lived
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.sizable_isolated",
                "change_lane",
                "sizable_change",
                "short_lived",
                "require",
                "condition",
                "medium",
            );
        }
        if (lower.contains("tiny edit") || lower.contains("narrow") || lower.contains("directly on `dev`"))
            && (direct_dev || lower.contains("dev"))
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "lane.narrow_direct_dev",
                "change_lane",
                "narrow_reversible_work",
                "direct_dev",
                "permit",
                "condition",
                "high",
            );
        }
        if lower.contains("integration lane") && lower.contains("dev") {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "target.integration_dev",
                "integration_target",
                "working_integration",
                "dev",
                "define",
                "target",
                "high",
            );
        }
        if (lower.contains("canonical branch") || lower.contains("release branch") || lower.contains("default branch"))
            && lower.contains("main")
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "target.release_main",
                "integration_target",
                "release_or_canonical",
                "main",
                "define",
                "target",
                "high",
            );
        }
        if lower.contains("all git writes")
            && lower.contains("powershell")
            && lower.contains("bash")
        {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "tool.git_write_shell",
                "tool_boundary",
                "repository_git_writes",
                "powershell_not_bash",
                "require",
                "boundary",
                "high",
            );
        }
        if lower.contains("never push `main`") || lower.contains("never push main") {
            push_policy_atom(
                &mut atoms,
                &document.record,
                line_number,
                line,
                "push.main_forbidden",
                "remote_write",
                "main_branch",
                "push_forbidden",
                "prohibit",
                "boundary",
                "high",
            );
        }
    }
    atoms
}

fn auditable_instruction_pointer(token: &str) -> bool {
    if token.starts_with('-')
        || token.contains('$')
        || token.contains('*')
        || token.contains('?')
        || token.contains('{')
        || token.contains('}')
    {
        return false;
    }
    if !token.contains('/') {
        return false;
    }
    let lower = token.to_ascii_lowercase();
    let file_name = Path::new(token)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let has_instruction_directory = lower.contains(".agents/")
        || lower.contains(".context/")
        || lower.contains(".codex/")
        || lower.contains(".pronto/");
    let known_instruction_name = matches!(
        file_name.as_str(),
        "agents.md"
            | "claude.md"
            | "codex.md"
            | "gemini.md"
            | "skill.md"
            | "context.md"
            | "contributing.md"
            | "development.md"
            | "policy.md"
            | "workflow.md"
            | "custody.md"
    );
    let contract_name = [
        "contract",
        "guidance",
        "instruction",
        "policy",
        "workflow",
        "custody",
        "branch",
        "matrix",
    ]
    .iter()
    .any(|marker| file_name.contains(marker));
    match Path::new(token)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "md" => has_instruction_directory || known_instruction_name || contract_name,
        "json" | "yaml" | "yml" => has_instruction_directory || contract_name,
        _ => false,
    }
}

fn instruction_pointer_candidates(line: &str) -> Vec<String> {
    line.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '"' | '\''
            )
    })
    .filter_map(|token| {
        let token = token.trim_matches(|character: char| {
            matches!(character, ':' | '!' | '?' | '<' | '>')
        });
        let token = token.strip_suffix('.').unwrap_or(token);
        let token = token
            .strip_prefix("@~")
            .map(|value| format!("~{value}"))
            .or_else(|| token.strip_prefix("@/").map(|value| format!("/{value}")))
            .unwrap_or_else(|| token.to_string());
        let token = token.as_str();
        if token.is_empty()
            || token.contains("<")
            || token.contains(">")
            || token.starts_with("http://")
            || token.starts_with("https://")
            || !auditable_instruction_pointer(token)
        {
            return None;
        }
        Some(token.to_string())
    })
    .collect()
}

fn unambiguous_instruction_pointer(
    token: &str,
    repository: Option<&AuditRepository>,
) -> bool {
    if repository.is_some() {
        return true;
    }
    token.starts_with('/')
        || token.starts_with("~/")
        || token.starts_with("$HOME/")
        || token.starts_with("./")
        || token.starts_with("../")
}

fn resolve_instruction_pointer(
    token: &str,
    source: &InstructionSourceRecord,
    home: &Path,
    repository: Option<&AuditRepository>,
) -> Option<PathBuf> {
    if token.starts_with("$HOME/") {
        return Some(home.join(token.trim_start_matches("$HOME/")));
    }
    if token.starts_with("~/") {
        return Some(home.join(token.trim_start_matches("~/")));
    }
    let path = PathBuf::from(token);
    if path.is_absolute() {
        Some(path)
    } else if let Some(repository) = repository {
        let repository_root = Path::new(&repository.path);
        if token.starts_with("./") || token.starts_with("../") {
            Path::new(&source.path).parent().map(|parent| parent.join(path))
        } else {
            Some(repository_root.join(path))
        }
    } else if token.starts_with(".agents/")
        || token.starts_with(".context/")
        || token.starts_with(".codex/")
        || matches!(
            path.file_name().and_then(|value| value.to_str()),
            Some("AGENTS.md" | "CLAUDE.md" | "CODEX.md" | "GEMINI.md")
        )
    {
        Some(home.join(path))
    } else {
        Path::new(&source.path).parent().map(|parent| parent.join(path))
    }
}

fn stale_pointer_findings(
    document: &SourceDocument,
    home: &Path,
    repository: Option<&AuditRepository>,
) -> Vec<InstructionFinding> {
    let mut findings = Vec::new();
    for (line_index, line) in document.contents.lines().enumerate() {
        for token in instruction_pointer_candidates(line) {
            if !unambiguous_instruction_pointer(&token, repository) {
                continue;
            }
            let Some(candidate) = resolve_instruction_pointer(
                &token,
                &document.record,
                home,
                repository,
            ) else {
                continue;
            };
            if candidate.exists() {
                continue;
            }
            let evidence = instruction_evidence(&document.record, line_index + 1, line, "high");
            findings.push(InstructionFinding {
                id: String::new(),
                classification: "stale_pointer".to_string(),
                severity: "medium".to_string(),
                confidence: "high".to_string(),
                comparison_scope: if repository.is_some() {
                    "global_vs_repository".to_string()
                } else {
                    "global_source_integrity".to_string()
                },
                subject: token.clone(),
                rationale: format!(
                    "The instruction source points to {}, but that path was not present at audit time.",
                    candidate.display()
                ),
                repository: repository.cloned(),
                baseline: Vec::new(),
                override_atoms: Vec::new(),
                evidence: vec![evidence],
            });
        }
    }
    findings
}

fn atoms_conflict(left: &PolicyAtom, right: &PolicyAtom) -> bool {
    (left.id == "lane.isolation_unconditional" && right.id == "lane.default_direct_dev")
        || (right.id == "lane.isolation_unconditional" && left.id == "lane.default_direct_dev")
        || (left.id == "lane.dirty_selects_isolation"
            && right.id == "lane.dirty_does_not_select_isolation")
        || (right.id == "lane.dirty_selects_isolation"
            && left.id == "lane.dirty_does_not_select_isolation")
        || (left.id == right.id
            && (left.action != right.action || left.polarity != right.polarity))
}

fn finding_for_comparison(
    classification: &str,
    comparison_scope: &str,
    subject: &str,
    rationale: &str,
    repository: Option<&AuditRepository>,
    baseline: Vec<PolicyAtom>,
    override_atoms: Vec<PolicyAtom>,
) -> InstructionFinding {
    let severity = match classification {
        "direct_conflict" => "high",
        "local_stricter_override" | "stale_pointer" => "medium",
        "needs_review" => "low",
        _ => "info",
    };
    let confidence = baseline
        .iter()
        .chain(override_atoms.iter())
        .map(|atom| atom.source.confidence.as_str())
        .all(|value| value == "high")
        .then_some("high")
        .unwrap_or("medium");
    let mut evidence = baseline
        .iter()
        .chain(override_atoms.iter())
        .map(|atom| atom.source.clone())
        .collect::<Vec<_>>();
    evidence.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
    });
    evidence.dedup_by(|left, right| left.path == right.path && left.line == right.line);
    InstructionFinding {
        id: String::new(),
        classification: classification.to_string(),
        severity: severity.to_string(),
        confidence: confidence.to_string(),
        comparison_scope: comparison_scope.to_string(),
        subject: subject.to_string(),
        rationale: rationale.to_string(),
        repository: repository.cloned(),
        baseline,
        override_atoms,
        evidence,
    }
}

fn global_conflict_findings(global_atoms: &[PolicyAtom]) -> Vec<InstructionFinding> {
    let mut findings = Vec::new();
    let groups = policy_atom_groups(global_atoms);
    for (left_index, left_group) in groups.iter().enumerate() {
        let left = &left_group[0];
        for right_group in groups.iter().skip(left_index + 1) {
            let right = &right_group[0];
            if !atoms_conflict(left, right) {
                continue;
            }
            findings.push(finding_for_comparison(
                "direct_conflict",
                "global_vs_global",
                &left.subject,
                &format!(
                    "Global sources prescribe incompatible actions for the same {} trigger: {} versus {}.",
                    left.subject, left.action, right.action
                ),
                None,
                left_group.clone(),
                right_group.clone(),
            ));
        }
    }
    findings
}

fn local_comparison_group(
    local_atoms: &[PolicyAtom],
    global_atoms: &[PolicyAtom],
    repository: &AuditRepository,
) -> InstructionFinding {
    let local = &local_atoms[0];
    let grouped_global_atoms = policy_atom_groups(global_atoms);
    let atoms_with_id = |id: &str| {
        grouped_global_atoms
            .iter()
            .filter(|group| group[0].id == id)
            .flat_map(|group| group.iter().cloned())
            .collect::<Vec<_>>()
    };
    let exact = atoms_with_id(&local.id);
    let defaults = atoms_with_id("lane.default_direct_dev");
    let dirty_guard = atoms_with_id("lane.dirty_does_not_select_isolation");

    if local.id == "lane.isolation_unconditional" && !defaults.is_empty() {
        return finding_for_comparison(
            "direct_conflict",
            "global_vs_repository",
            "change_lane",
            "The repository instruction requires isolation for every non-trivial change, which removes the global direct_dev default and its lane-selection conditions.",
            Some(repository),
            defaults,
            local_atoms.to_vec(),
        );
    }
    if local.id == "lane.dirty_selects_isolation" && !dirty_guard.is_empty() {
        return finding_for_comparison(
            "direct_conflict",
            "global_vs_repository",
            "dirty_integration_only",
            "The repository instruction makes dirty state sufficient for isolation, while the global policy explicitly says dirty integration state alone does not select a branch.",
            Some(repository),
            dirty_guard,
            local_atoms.to_vec(),
        );
    }
    if matches!(
        local.id.as_str(),
        "lane.feature_isolated" | "lane.sizable_isolated"
    ) && !defaults.is_empty()
    {
        return finding_for_comparison(
            "local_stricter_override",
            "global_vs_repository",
            &local.trigger,
            "The repository narrows the global direct_dev default by requiring isolation for a scoped class of work; this is stricter locally, not a fleet-wide default.",
            Some(repository),
            defaults,
            local_atoms.to_vec(),
        );
    }
    if local.id == "lane.narrow_direct_dev" && !defaults.is_empty() {
        return finding_for_comparison(
            "compatible",
            "global_vs_repository",
            &local.trigger,
            "The repository permits direct dev work for a narrower bounded case already covered by the global direct_dev default.",
            Some(repository),
            defaults,
            local_atoms.to_vec(),
        );
    }
    if !exact.is_empty() {
        let compatible = exact.iter().all(|candidate| {
            candidate.action == local.action
                && candidate.polarity == local.polarity
                && candidate.trigger == local.trigger
        });
        return finding_for_comparison(
            if compatible {
                "compatible"
            } else {
                "needs_review"
            },
            "global_vs_repository",
            &local.subject,
            if compatible {
                "The repository atom matches the applicable global policy atom."
            } else {
                "The repository and global sources use the same policy subject with different actions or conditions; inspect the scope before changing either source."
            },
            Some(repository),
            exact,
            local_atoms.to_vec(),
        );
    }
    finding_for_comparison(
        "needs_review",
        "global_vs_repository",
        &local.subject,
        "The repository contains a high-confidence policy atom with no matching global baseline in this audit vocabulary; review whether it is an intentional local contract or a missing global rule.",
        Some(repository),
        Vec::new(),
        local_atoms.to_vec(),
    )
}

fn sort_and_number_findings(findings: &mut Vec<InstructionFinding>) {
    findings.sort_by(|left, right| {
        left.classification
            .cmp(&right.classification)
            .then(left.comparison_scope.cmp(&right.comparison_scope))
            .then(
                left.repository
                    .as_ref()
                    .map(|item| item.path.as_str())
                    .unwrap_or_default()
                    .cmp(
                        right
                            .repository
                            .as_ref()
                            .map(|item| item.path.as_str())
                            .unwrap_or_default(),
                    ),
            )
            .then(left.subject.cmp(&right.subject))
            .then(
                left.evidence
                    .first()
                    .map(|item| item.path.as_str())
                    .unwrap_or_default()
                    .cmp(
                        right
                            .evidence
                            .first()
                            .map(|item| item.path.as_str())
                            .unwrap_or_default(),
                    ),
            )
            .then(
                left.evidence
                    .first()
                    .map(|item| item.line)
                    .unwrap_or_default()
                    .cmp(&right.evidence.first().map(|item| item.line).unwrap_or_default()),
            )
    });
    for (index, finding) in findings.iter_mut().enumerate() {
        finding.id = format!("instruction-drift-{:04}", index + 1);
    }
}

fn instruction_summary(findings: &[InstructionFinding], repository_count: usize) -> InstructionDriftSummary {
    let mut summary = InstructionDriftSummary {
        compatible: 0,
        local_stricter_override: 0,
        direct_conflict: 0,
        stale_pointer: 0,
        needs_review: 0,
        repository_count,
        finding_count: findings.len(),
    };
    for finding in findings {
        match finding.classification.as_str() {
            "compatible" => summary.compatible += 1,
            "local_stricter_override" => summary.local_stricter_override += 1,
            "direct_conflict" => summary.direct_conflict += 1,
            "stale_pointer" => summary.stale_pointer += 1,
            "needs_review" => summary.needs_review += 1,
            _ => {}
        }
    }
    summary
}

fn audit_repository_from_snapshot(repository: &RepositorySnapshot) -> AuditRepository {
    AuditRepository {
        id: repository.id.clone(),
        name: repository.name.clone(),
        path: repository.path.clone(),
    }
}

fn live_git_repository_evidence(repository: &AuditRepository) -> LiveGitRepositoryEvidence {
    let path = Path::new(&repository.path);
    let result = match run_git(path, ["status", "--porcelain=v2", "--branch"].iter()) {
        Ok(result) => result,
        Err(error) => {
            return LiveGitRepositoryEvidence {
                repository: repository.clone(),
                status: "unavailable".to_string(),
                branch: None,
                head_sha: None,
                dirty: None,
                error: Some(error),
            };
        }
    };
    if !result.success {
        return LiveGitRepositoryEvidence {
            repository: repository.clone(),
            status: "unavailable".to_string(),
            branch: None,
            head_sha: None,
            dirty: None,
            error: Some(if result.stderr.trim().is_empty() {
                format!("Git status exited with code {:?}", result.exit_code)
            } else {
                result.stderr.trim().to_string()
            }),
        };
    }
    let parsed = parse_status(&result.stdout);
    let head_sha = git_static(path, &["rev-parse", "HEAD"]);
    LiveGitRepositoryEvidence {
        repository: repository.clone(),
        status: "observed".to_string(),
        branch: Some(parsed.branch),
        head_sha,
        dirty: Some(parsed.dirty),
        error: None,
    }
}

fn cached_live_mismatch(
    cached: &CachedRepositoryEvidence,
    live: &LiveGitRepositoryEvidence,
) -> Option<CachedLiveMismatch> {
    if live.status != "observed" {
        return None;
    }
    let mut fields = Vec::new();
    if live.branch.as_deref() != Some(cached.branch.as_str()) {
        fields.push("branch".to_string());
    }
    if cached.head_sha.is_some() && cached.head_sha != live.head_sha {
        fields.push("head_sha".to_string());
    }
    if live.dirty != Some(cached.dirty) {
        fields.push("dirty".to_string());
    }
    (!fields.is_empty()).then(|| CachedLiveMismatch {
        repository: cached.repository.clone(),
        fields,
    })
}

fn audit_instruction_sources(
    home: &Path,
    repositories: &[AuditRepository],
    skipped: &mut Vec<SkippedInstructionSource>,
) -> (
    Vec<SourceDocument>,
    Vec<(AuditRepository, Vec<SourceDocument>)>,
) {
    let mut global_documents = Vec::new();
    for (path, precedence) in collect_global_instruction_paths(home) {
        match read_instruction_source(&path, "global", &precedence, None) {
            Ok(document) => global_documents.push(document),
            Err((record, reason)) => {
                skipped.push(SkippedInstructionSource {
                    path: record.path,
                    scope: record.scope,
                    reason,
                    repository: None,
                });
            }
        }
    }
    let mut repository_documents = Vec::new();
    for repository in repositories {
        let mut documents = Vec::new();
        let root = Path::new(&repository.path);
        let paths = if root.is_dir() {
            collect_local_instruction_paths(root)
        } else {
            skipped.push(SkippedInstructionSource {
                path: repository.path.clone(),
                scope: "repository".to_string(),
                reason: "repository path is unavailable; local instructions were not scanned".to_string(),
                repository: Some(repository.clone()),
            });
            Vec::new()
        };
        for path in paths {
            match read_instruction_source(
                &path,
                "repository",
                if path
                    .strip_prefix(root)
                    .ok()
                    .and_then(|relative| relative.components().next())
                    .is_some_and(|component| component.as_os_str() == ".agents")
                {
                    "repository_context"
                } else {
                    "repository_documentation"
                },
                Some(repository),
            ) {
                Ok(document) => documents.push(document),
                Err((record, reason)) => skipped.push(SkippedInstructionSource {
                    path: record.path,
                    scope: record.scope,
                    reason,
                    repository: Some(repository.clone()),
                }),
            }
        }
        repository_documents.push((repository.clone(), documents));
    }
    (global_documents, repository_documents)
}

fn build_instruction_drift_report(
    storage_path: &Path,
    home: &Path,
    query: Option<&str>,
) -> InstructionDriftReport {
    let cached_result = load_store_read_only(storage_path);
    let mut skipped = Vec::new();
    let mut repositories = Vec::new();
    let mut cached_repositories = Vec::new();
    let (cached_status, cached_error, quality_count, quality_status, oldest_scan_at) =
        match cached_result.as_ref() {
            Ok(state) => {
                let selected = state
                    .repositories
                    .iter()
                    .filter(|repository| {
                        query.map_or(true, |value| {
                            repository.id == value
                                || repository.path == value
                                || repository.name.eq_ignore_ascii_case(value)
                        })
                    })
                    .collect::<Vec<_>>();
                if let Some(value) = query {
                    if selected.is_empty() {
                        skipped.push(SkippedInstructionSource {
                            path: value.to_string(),
                            scope: "repository".to_string(),
                            reason: "repository query was not found in the read-only Pronto registry".to_string(),
                            repository: None,
                        });
                    }
                }
                for repository in selected {
                    let audit_repository = audit_repository_from_snapshot(repository);
                    repositories.push(audit_repository.clone());
                    cached_repositories.push(CachedRepositoryEvidence {
                        repository: audit_repository,
                        branch: repository.workspace.branch.clone(),
                        head_sha: repository.workspace.last_commit.clone(),
                        dirty: repository.workspace.dirty,
                        last_scan_at: repository.last_scan_at.clone(),
                    });
                }
                let oldest = cached_repositories
                    .iter()
                    .map(|repository| repository.last_scan_at.clone())
                    .min();
                (
                    "available".to_string(),
                    None,
                    state.quality.matched_repository_count,
                    Some(state.quality.audit_status.clone()),
                    oldest,
                )
            }
            Err(error) => (
                "blocked".to_string(),
                Some(error.clone()),
                0,
                None,
                None,
            ),
        };

    let (global_documents, repository_documents) =
        audit_instruction_sources(home, &repositories, &mut skipped);
    let global_atoms = global_documents
        .iter()
        .flat_map(extract_policy_atoms)
        .collect::<Vec<_>>();
    let mut repository_atoms = Vec::new();
    let mut findings = global_conflict_findings(&global_atoms);
    for document in &global_documents {
        findings.extend(stale_pointer_findings(document, home, None));
    }
    for (repository, documents) in &repository_documents {
        for document in documents {
            let atoms = extract_policy_atoms(document);
            repository_atoms.extend(atoms.iter().cloned());
            findings.extend(stale_pointer_findings(document, home, Some(repository)));
            for group in policy_atom_groups(&atoms) {
                findings.push(local_comparison_group(&group, &global_atoms, repository));
            }
        }
    }

    let live_repositories = repositories
        .iter()
        .map(live_git_repository_evidence)
        .collect::<Vec<_>>();
    let mismatches = cached_repositories
        .iter()
        .filter_map(|cached| {
            live_repositories
                .iter()
                .find(|live| live.repository.id == cached.repository.id)
                .and_then(|live| cached_live_mismatch(cached, live))
        })
        .collect::<Vec<_>>();
    let unavailable_repository_count = live_repositories
        .iter()
        .filter(|repository| repository.status != "observed")
        .count();
    let dirty_repository_count = live_repositories
        .iter()
        .filter(|repository| repository.dirty == Some(true))
        .count();
    let live_status = if unavailable_repository_count > 0 {
        "partial"
    } else {
        "observed"
    };
    let live_git = LiveGitEvidence {
        status: live_status.to_string(),
        observed_repository_count: live_repositories
            .iter()
            .filter(|repository| repository.status == "observed")
            .count(),
        unavailable_repository_count,
        dirty_repository_count,
        cached_live_mismatch_count: mismatches.len(),
        repositories: live_repositories,
        mismatches,
    };
    let source_inventory = InstructionSourceInventory {
        status: if skipped
            .iter()
            .any(|item| item.scope == "global" || item.repository.is_some())
        {
            "partial".to_string()
        } else {
            "observed".to_string()
        },
        global_source_count: global_documents.len(),
        repository_source_count: repository_documents
            .iter()
            .map(|(_, documents)| documents.len())
            .sum(),
        readable_source_count: global_documents.len()
            + repository_documents
                .iter()
                .map(|(_, documents)| documents.len())
                .sum::<usize>(),
        blocked_source_count: skipped
            .iter()
            .filter(|item| item.scope == "global" || item.repository.is_some())
            .count(),
        global_atom_count: global_atoms.len(),
        repository_atom_count: repository_atoms.len(),
        sources: global_documents
            .iter()
            .map(|document| document.record.clone())
            .chain(
                repository_documents
                    .iter()
                    .flat_map(|(_, documents)| documents.iter())
                    .map(|document| document.record.clone()),
            )
            .collect(),
    };
    let cached_pronto = CachedProntoEvidence {
        status: cached_status.clone(),
        storage_path: storage_path.display().to_string(),
        repository_count: cached_repositories.len(),
        quality_matched_repository_count: quality_count,
        quality_audit_status: quality_status,
        oldest_scan_at,
        repositories: cached_repositories,
        error: cached_error,
    };
    let evidence_status = if cached_status == "blocked" {
        "blocked"
    } else if live_git.status == "partial" || !skipped.is_empty() || !live_git.mismatches.is_empty() {
        "partial"
    } else {
        "observed"
    };
    let evidence_freshness = EvidenceFreshness {
        status: evidence_status.to_string(),
        read_only: true,
        detail: if cached_status == "blocked" {
            "The Pronto registry could not be read; instruction and live-Git coverage is incomplete.".to_string()
        } else if !live_git.mismatches.is_empty() {
            "Live Git differs from cached Pronto rows; keep those evidence lanes separate and refresh only through the explicit refresh command.".to_string()
        } else if live_git.status == "partial" {
            "Some registered repositories could not be inspected through live Git.".to_string()
        } else {
            "Cached Pronto rows, live Git observations, and instruction findings were collected independently without refresh or mutation.".to_string()
        },
        cached_repository_count: cached_pronto.repository_count,
        live_repository_count: live_git.observed_repository_count,
        cached_live_mismatch_count: live_git.cached_live_mismatch_count,
        blocked_repository_count: live_git.unavailable_repository_count,
    };
    sort_and_number_findings(&mut findings);
    let status = if cached_status == "blocked" {
        "blocked"
    } else if evidence_freshness.status == "partial" {
        "partial"
    } else {
        "complete"
    };
    let summary = instruction_summary(&findings, repositories.len());
    InstructionDriftReport {
        schema_version: INSTRUCTION_DRIFT_SCHEMA.to_string(),
        generated_at: iso_now(),
        mode: "read_only".to_string(),
        status: status.to_string(),
        authorization: "Read-only local audit; no Pronto refresh, SQLite write, Git mutation, provider call, branch, worktree, or repository-file edit was performed.".to_string(),
        source_inventory,
        cached_pronto,
        live_git,
        evidence_freshness,
        instruction_findings: findings,
        skipped,
        summary,
    }
}

fn print_human_instruction_drift(report: &InstructionDriftReport) {
    println!(
        "Instruction drift: {} · {} repositories · {} findings",
        report.status, report.summary.repository_count, report.summary.finding_count
    );
    println!(
        "  compatible={} local_stricter_override={} direct_conflict={} stale_pointer={} needs_review={}",
        report.summary.compatible,
        report.summary.local_stricter_override,
        report.summary.direct_conflict,
        report.summary.stale_pointer,
        report.summary.needs_review
    );
    println!(
        "  evidence={} cached_repositories={} live_repositories={} cached_live_mismatches={}",
        report.evidence_freshness.status,
        report.evidence_freshness.cached_repository_count,
        report.evidence_freshness.live_repository_count,
        report.evidence_freshness.cached_live_mismatch_count
    );
    println!("Authorization: {}", report.authorization);
}

#[allow(unused_variables)]
fn run_cli_arm_39(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let home = cli_option(arguments, "--home").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let positionals = cli_positionals(arguments, &["--home"]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto instruction-drift [<repository>] [--home <agent-root>] [--json]"
        );
        std::process::exit(2);
    }
    let report = build_instruction_drift_report(
        path,
        &instruction_drift_home_root(home.as_deref()),
        positionals.first().map(String::as_str),
    );
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        print_human_instruction_drift(&report);
    }
}

#[cfg(test)]
mod instruction_drift_tests {
    use super::*;

    fn document(scope: &str, precedence: &str, contents: &str) -> SourceDocument {
        let path = PathBuf::from(format!("/{scope}/{precedence}.md"));
        let bytes = contents.as_bytes();
        SourceDocument {
            record: InstructionSourceRecord {
                path: path.display().to_string(),
                scope: scope.to_string(),
                precedence: precedence.to_string(),
                status: "read".to_string(),
                sha256: Some(instruction_digest(bytes)),
                line_count: Some(contents.lines().count()),
                repository_id: None,
                repository_name: None,
                repository_path: None,
            },
            contents: contents.to_string(),
        }
    }

    fn repository() -> AuditRepository {
        AuditRepository {
            id: "repo-1".to_string(),
            name: "fixture".to_string(),
            path: "/repo/fixture".to_string(),
        }
    }

    #[test]
    fn extracts_lane_atoms_with_source_hash_and_line() {
        let parsed = extract_policy_atoms(&document(
            "global",
            "global_policy",
            "`direct_dev` is the default for bounded, reversible, single-repository work.\nLong-lived lines require explicit designation, an owner, and an exit condition.\n",
        ));
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].id, "lane.default_direct_dev");
        assert_eq!(parsed[0].source.line, 1);
        assert_eq!(parsed[0].source.sha256.len(), 64);
        assert_eq!(parsed[2].id, "lane.long_lived_designation");
    }

    #[test]
    fn classifies_unconditional_repository_isolation_as_direct_conflict() {
        let global = extract_policy_atoms(&document(
            "global",
            "global_policy",
            "`direct_dev` is the default for bounded, reversible work.\n",
        ));
        let local = extract_policy_atoms(&document(
            "repository",
            "repository_context",
            "Before modifying a non-trivial repository, prove that the current directory is a task-owned isolated worktree.\n",
        ));
        let finding = local_comparison_group(std::slice::from_ref(&local[0]), &global, &repository());
        assert_eq!(finding.classification, "direct_conflict");
        assert_eq!(finding.comparison_scope, "global_vs_repository");
        assert_eq!(finding.evidence.len(), 2);
    }

    #[test]
    fn classifies_dirty_only_isolation_as_direct_conflict() {
        let global = extract_policy_atoms(&document(
            "global",
            "global_policy",
            "A dirty integration branch alone never selects `short_lived`.\n",
        ));
        let local = extract_policy_atoms(&document(
            "repository",
            "repository_context",
            "A dirty dev branch requires an isolated worktree before any change.\n",
        ));
        let finding = local_comparison_group(std::slice::from_ref(&local[0]), &global, &repository());
        assert_eq!(finding.classification, "direct_conflict");
    }

    #[test]
    fn classifies_scoped_feature_isolation_as_local_stricter_override() {
        let global = extract_policy_atoms(&document(
            "global",
            "global_policy",
            "`direct_dev` is the default for bounded, reversible work.\n",
        ));
        let local = extract_policy_atoms(&document(
            "repository",
            "repository_context",
            "Feature and remediation work must use an isolated branch and worktree.\n",
        ));
        let finding = local_comparison_group(std::slice::from_ref(&local[0]), &global, &repository());
        assert_eq!(finding.classification, "local_stricter_override");
    }

    #[test]
    fn global_conflict_finds_the_dirty_guard_pair() {
        let atoms = extract_policy_atoms(&document(
            "global",
            "mixed",
            "A dirty integration branch alone never selects isolation.\nA dirty dev branch requires an isolated worktree.\n",
        ));
        let findings = global_conflict_findings(&atoms);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].classification, "direct_conflict");
    }

    #[test]
    fn equivalent_global_atoms_are_aggregated_with_all_source_evidence() {
        let first = extract_policy_atoms(&document(
            "global",
            "first",
            "A dirty integration branch alone never selects isolation.\n",
        ));
        let second = extract_policy_atoms(&document(
            "global",
            "second",
            "A dirty integration branch alone never selects isolation.\n",
        ));
        let opposing = extract_policy_atoms(&document(
            "global",
            "opposing",
            "A dirty dev branch requires an isolated worktree.\n",
        ));
        let atoms = first
            .into_iter()
            .chain(second)
            .chain(opposing)
            .collect::<Vec<_>>();
        let findings = global_conflict_findings(&atoms);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].baseline.len() + findings[0].override_atoms.len(), 3);
        assert_eq!(findings[0].evidence.len(), 3);
    }

    #[test]
    fn stale_pointer_is_reported_without_reading_or_writing_the_target() {
        let root = std::env::temp_dir().join(format!("pronto-drift-pointer-{}", std::process::id()));
        let source_path = root.join(".agents/context/README.md");
        let source = document(
            "repository",
            "repository_context",
            "See `docs/missing-contract.md` for the active contract.\n",
        );
        let findings = stale_pointer_findings(
            &SourceDocument {
                record: InstructionSourceRecord {
                    path: source_path.display().to_string(),
                    ..source.record
                },
                contents: source.contents,
            },
            Path::new("/home/test"),
            Some(&repository()),
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].classification, "stale_pointer");
        assert_eq!(findings[0].subject, "docs/missing-contract.md");
    }

    #[test]
    fn pointer_scan_ignores_bare_names_and_shell_variables() {
        assert!(instruction_pointer_candidates("See CLAUDE.md and SKILL.md.").is_empty());
        assert!(instruction_pointer_candidates("Write $API_RUN_DIR/research.json.").is_empty());
        assert_eq!(
            instruction_pointer_candidates("See ../../docs/repository-contract.md."),
            vec!["../../docs/repository-contract.md"]
        );
    }

    #[test]
    fn global_relative_repository_paths_are_not_called_stale_when_ambiguous() {
        let source = document(
            "global",
            "global_policy",
            "In a repository, read .agents/context/README.md when present.\n",
        );
        assert!(stale_pointer_findings(&source, Path::new("/home/test"), None).is_empty());
    }
}
