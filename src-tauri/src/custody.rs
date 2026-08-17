//! Read-only projection of isolated-change-workflow custody evidence.
//!
//! The workflow owns receipt mutation and HMAC verification. Pronto only
//! projects live Git state plus structurally recognizable receipts; it never
//! treats a cached label as permission to adopt, integrate, delete, or push.

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CUSTODY_PROJECTION_SCHEMA: &str = "pronto-custody-projection/v1";
pub const WORKSPACE_POLICY_SCHEMA: &str = "workspace-policy/v1";
const TASK_SCHEMA: &str = "isolated-change-task/v2";
const LEGACY_TASK_SCHEMA: &str = "isolated-change-task/v1";
const DEFAULT_LEASE_SECONDS: i64 = 24 * 60 * 60;
const DEFAULT_ADOPTION_SECONDS: i64 = 72 * 60 * 60;
const DEFAULT_WORKSPACE_POLICY_RELATIVE_PATH: &str = ".agents/workspace-policy.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustodyLane {
    pub task_id: String,
    #[serde(default)]
    pub work_item_id: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub worktree: Option<String>,
    #[serde(default)]
    pub base_ref: Option<String>,
    #[serde(default)]
    pub base_sha: Option<String>,
    #[serde(default)]
    pub head_sha: Option<String>,
    #[serde(default = "default_temporary_workspace_class")]
    pub workspace_class: String,
    #[serde(default = "default_true")]
    pub lease_required: bool,
    #[serde(default)]
    pub recorded_state: Option<String>,
    pub state: String,
    #[serde(default)]
    pub disposition: String,
    #[serde(default)]
    pub dispositions: Vec<String>,
    #[serde(default)]
    pub next_action: String,
    #[serde(default)]
    pub custodian: Option<String>,
    #[serde(default)]
    pub declared_scope: Vec<String>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub last_activity_at: Option<String>,
    #[serde(default)]
    pub lease_expires_at: Option<String>,
    #[serde(default)]
    pub provider_review: Option<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub receipt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustodyCounts {
    #[serde(flatten)]
    pub states: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustodyOverlap {
    pub left_task_id: String,
    pub right_task_id: String,
    pub paths: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustodySnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub repository: String,
    pub receipt_root: String,
    pub source: String,
    pub read_only: bool,
    pub implementation_allowed: bool,
    pub mutation_risk: String,
    pub status: String,
    pub next_safe_step: String,
    #[serde(default)]
    pub lanes: Vec<CustodyLane>,
    #[serde(default)]
    pub unleased_worktrees: Vec<String>,
    #[serde(default)]
    pub counts: CustodyCounts,
    #[serde(default)]
    pub disposition_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub overlaps: Vec<CustodyOverlap>,
    #[serde(default)]
    pub workspace_policy: WorkspacePolicyProjection,
    #[serde(default)]
    pub integrity: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonicalWorkspace {
    pub id: String,
    pub role: String,
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub path: Option<String>,
    pub protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspacePolicyProjection {
    pub schema_version: String,
    #[serde(default)]
    pub repository_id: Option<String>,
    pub repository_role: String,
    pub status: String,
    pub disposition: String,
    #[serde(default)]
    pub baseline_target: Option<u64>,
    #[serde(default)]
    pub canonical_target: Option<u64>,
    pub canonical_observed: u64,
    pub temporary_observed: u64,
    pub active_temporary_lanes: u64,
    pub retained_lane_count: u64,
    #[serde(default)]
    pub managed_target_total: Option<u64>,
    pub canonical_workspaces: Vec<CanonicalWorkspace>,
    pub protected_refs: Vec<String>,
    pub lease_required_for: String,
    pub canonical_protection: String,
    #[serde(default)]
    pub policy_path: Option<String>,
    pub drift: Vec<String>,
}

#[derive(Debug, Clone)]
struct LiveWorktree {
    path: PathBuf,
    branch: Option<String>,
    head_sha: Option<String>,
    clean: Option<bool>,
    operations: Vec<String>,
    open_files: Option<bool>,
}

#[derive(Debug, Clone)]
struct ParsedWorkspacePolicy {
    repository_id: Option<String>,
    repository_role: String,
    canonical_workspaces: Vec<CanonicalWorkspace>,
    retained_lane_count: u64,
    path: PathBuf,
}

fn default_temporary_workspace_class() -> String {
    "temporary".to_string()
}

fn default_true() -> bool {
    true
}

fn nonempty_policy_string(value: Option<&Value>, field: &str) -> Result<String, String> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Err(format!("{field} must be a non-empty string"));
    };
    if value.trim().is_empty() {
        return Err(format!("{field} must be a non-empty string"));
    }
    Ok(value.trim().to_string())
}

fn workspace_policy_path(repository: &Path) -> PathBuf {
    repository.join(DEFAULT_WORKSPACE_POLICY_RELATIVE_PATH)
}

fn policy_workspace_path(repository: &Path, value: Option<&String>) -> Option<PathBuf> {
    let value = value.filter(|value| !value.trim().is_empty())?;
    let path = Path::new(value);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository.join(path)
    };
    Some(canonical(&resolved))
}

fn parse_workspace_policy(repository: &Path) -> Result<Option<ParsedWorkspacePolicy>, String> {
    let path = workspace_policy_path(repository);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&content)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("{} must contain an object", path.display()))?;
    if object.get("schema_version").and_then(Value::as_str) != Some(WORKSPACE_POLICY_SCHEMA) {
        return Err(format!("schema_version must be {WORKSPACE_POLICY_SCHEMA}"));
    }
    let repository_role = nonempty_policy_string(object.get("repository_role"), "repository_role")?;
    if !matches!(
        repository_role.as_str(),
        "production_product" | "supporting_project" | "role_unresolved"
    ) {
        return Err(
            "repository_role must be production_product, supporting_project, or role_unresolved"
                .to_string(),
        );
    }

    let canonical_values = object
        .get("canonical_workspaces")
        .and_then(Value::as_array)
        .ok_or_else(|| "canonical_workspaces must be an array".to_string())?;
    let mut ids = BTreeSet::new();
    let mut refs = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut canonical_workspaces = Vec::new();
    for (index, item) in canonical_values.iter().enumerate() {
        let entry = item
            .as_object()
            .ok_or_else(|| format!("canonical_workspaces[{index}] must be an object"))?;
        let id = nonempty_policy_string(
            entry.get("id"),
            &format!("canonical_workspaces[{index}].id"),
        )?;
        let role = nonempty_policy_string(
            entry.get("role"),
            &format!("canonical_workspaces[{index}].role"),
        )?;
        let reference = nonempty_policy_string(
            entry.get("ref"),
            &format!("canonical_workspaces[{index}].ref"),
        )?;
        if !matches!(role.as_str(), "release" | "integration" | "working") {
            return Err(format!(
                "canonical_workspaces[{index}].role is not supported"
            ));
        }
        if entry.get("protected").and_then(Value::as_bool) != Some(true) {
            return Err(format!(
                "canonical_workspaces[{index}].protected must be true"
            ));
        }
        if !ids.insert(id.clone()) || !refs.insert(reference.clone()) || !roles.insert(role.clone())
        {
            return Err("canonical workspace ids, refs, and roles must be unique".to_string());
        }
        let path_value = match entry.get("path") {
            Some(Value::Null) | None => None,
            Some(value) => Some(nonempty_policy_string(
                Some(value),
                &format!("canonical_workspaces[{index}].path"),
            )?),
        };
        canonical_workspaces.push(CanonicalWorkspace {
            id,
            role,
            reference,
            path: path_value,
            protected: true,
        });
    }
    let expected_roles: BTreeSet<&str> = match repository_role.as_str() {
        "production_product" => ["release", "integration"].into_iter().collect(),
        "supporting_project" => ["working"].into_iter().collect(),
        _ => BTreeSet::new(),
    };
    if repository_role != "role_unresolved"
        && roles.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected_roles
    {
        let missing = expected_roles
            .difference(&roles.iter().map(String::as_str).collect())
            .copied()
            .collect::<Vec<_>>();
        let extra = roles
            .iter()
            .map(String::as_str)
            .filter(|role| !expected_roles.contains(role))
            .collect::<Vec<_>>();
        return Err(format!(
            "canonical workspace roles do not match {repository_role}; missing={missing:?}, extra={extra:?}"
        ));
    }

    let retention_values = object
        .get("retention_exceptions")
        .and_then(Value::as_array)
        .ok_or_else(|| "retention_exceptions must be an array".to_string())?;
    let mut retention_ids = BTreeSet::new();
    for (index, item) in retention_values.iter().enumerate() {
        let entry = item
            .as_object()
            .ok_or_else(|| format!("retention_exceptions[{index}] must be an object"))?;
        let lane_id = nonempty_policy_string(
            entry.get("lane_id"),
            &format!("retention_exceptions[{index}].lane_id"),
        )?;
        nonempty_policy_string(
            entry.get("reason"),
            &format!("retention_exceptions[{index}].reason"),
        )?;
        nonempty_policy_string(
            entry.get("retained_by"),
            &format!("retention_exceptions[{index}].retained_by"),
        )?;
        let review_by = nonempty_policy_string(
            entry.get("review_by"),
            &format!("retention_exceptions[{index}].review_by"),
        )?;
        if DateTime::parse_from_rfc3339(&review_by).is_err() {
            return Err(format!(
                "retention_exceptions[{index}].review_by must be ISO-8601"
            ));
        }
        if !retention_ids.insert(lane_id) {
            return Err("retention exception lane_id values must be unique".to_string());
        }
    }

    let repository_id = match object.get("repository_id") {
        Some(Value::Null) | None => None,
        Some(value) => Some(nonempty_policy_string(Some(value), "repository_id")?),
    };
    Ok(Some(ParsedWorkspacePolicy {
        repository_id,
        repository_role,
        canonical_workspaces,
        retained_lane_count: retention_values.len() as u64,
        path,
    }))
}

fn empty_workspace_policy_projection(
    active_temporary_lanes: u64,
    policy_path: Option<&Path>,
    status: &str,
    disposition: &str,
    drift: Vec<String>,
) -> WorkspacePolicyProjection {
    WorkspacePolicyProjection {
        schema_version: WORKSPACE_POLICY_SCHEMA.to_string(),
        repository_role: "role_unresolved".to_string(),
        status: status.to_string(),
        disposition: disposition.to_string(),
        baseline_target: None,
        canonical_target: None,
        canonical_observed: 0,
        temporary_observed: 0,
        active_temporary_lanes,
        retained_lane_count: 0,
        managed_target_total: None,
        canonical_workspaces: Vec::new(),
        protected_refs: Vec::new(),
        lease_required_for: "temporary".to_string(),
        canonical_protection: "unresolved".to_string(),
        policy_path: policy_path.map(|value| value.to_string_lossy().to_string()),
        drift,
        repository_id: None,
    }
}

fn canonical_matches(
    record: &LiveWorktree,
    policy: Option<&ParsedWorkspacePolicy>,
    repository: &Path,
) -> bool {
    let Some(policy) = policy else {
        return record.path == canonical(repository);
    };
    policy.canonical_workspaces.iter().any(|workspace| {
        record.branch.as_deref() == Some(workspace.reference.as_str())
            || policy_workspace_path(repository, workspace.path.as_ref())
                .is_some_and(|path| path == record.path)
    })
}

fn project_workspace_policy(
    repository: &Path,
    live_records: &[LiveWorktree],
    active_temporary_lanes: u64,
) -> WorkspacePolicyProjection {
    let policy_path = workspace_policy_path(repository);
    let policy = match parse_workspace_policy(repository) {
        Ok(policy) => policy,
        Err(error) => {
            return empty_workspace_policy_projection(
                active_temporary_lanes,
                Some(&policy_path),
                "invalid",
                "policy_invalid",
                vec![format!("policy-invalid:{error}")],
            )
        }
    };
    let Some(policy) = policy else {
        return empty_workspace_policy_projection(
            active_temporary_lanes,
            None,
            "role_unresolved",
            "policy_missing",
            vec!["repository-role-unresolved".to_string()],
        );
    };
    let target = match policy.repository_role.as_str() {
        "production_product" => Some(2),
        "supporting_project" => Some(1),
        _ => None,
    };
    let expected_roles: BTreeSet<&str> = match policy.repository_role.as_str() {
        "production_product" => ["release", "integration"].into_iter().collect(),
        "supporting_project" => ["working"].into_iter().collect(),
        _ => BTreeSet::new(),
    };
    let observed_roles = live_records
        .iter()
        .flat_map(|record| {
            policy
                .canonical_workspaces
                .iter()
                .filter(move |workspace| {
                    record.branch.as_deref() == Some(workspace.reference.as_str())
                        || policy_workspace_path(repository, workspace.path.as_ref())
                            .is_some_and(|path| path == record.path)
                })
                .map(|workspace| workspace.role.as_str())
        })
        .collect::<BTreeSet<_>>();
    let mut drift = expected_roles
        .difference(&observed_roles)
        .map(|role| format!("missing-canonical:{role}"))
        .collect::<Vec<_>>();
    let canonical_observed = observed_roles.len() as u64;
    let temporary_observed = live_records
        .iter()
        .filter(|record| !canonical_matches(record, Some(&policy), repository))
        .count() as u64;
    let managed_target_total =
        target.map(|target| target + active_temporary_lanes + policy.retained_lane_count);
    let (status, disposition) = if policy.repository_role == "role_unresolved" {
        ("role_unresolved", "role_unresolved")
    } else if !drift.is_empty() {
        ("canonical_drift", "canonical_workspace_missing")
    } else {
        ("observed", "policy_observed")
    };
    drift.sort();
    WorkspacePolicyProjection {
        schema_version: WORKSPACE_POLICY_SCHEMA.to_string(),
        repository_id: policy.repository_id,
        repository_role: policy.repository_role,
        status: status.to_string(),
        disposition: disposition.to_string(),
        baseline_target: target,
        canonical_target: target,
        canonical_observed,
        temporary_observed,
        active_temporary_lanes,
        retained_lane_count: policy.retained_lane_count,
        managed_target_total,
        canonical_workspaces: policy.canonical_workspaces.clone(),
        protected_refs: policy
            .canonical_workspaces
            .iter()
            .filter(|workspace| workspace.protected)
            .map(|workspace| workspace.reference.clone())
            .collect(),
        lease_required_for: "temporary".to_string(),
        canonical_protection: "enforced".to_string(),
        policy_path: Some(policy.path.to_string_lossy().to_string()),
        drift,
    }
}

fn git(path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|error| format!("git unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_optional(path: &Path, args: &[&str]) -> Option<String> {
    git(path, args).ok().filter(|value| !value.is_empty())
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn common_git_dir(path: &Path) -> Result<PathBuf, String> {
    let value = git(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Ok(canonical(Path::new(&value)))
}

fn receipt_root(path: &Path) -> Result<PathBuf, String> {
    Ok(common_git_dir(path)?
        .join("isolated-change-workflow")
        .join("tasks"))
}

fn string_value(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn string_list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn receipt_integrity(value: &Value) -> &'static str {
    let schema = string_value(value, "schema_version").unwrap_or_default();
    let integrity = value.get("integrity");
    let algorithm = integrity
        .and_then(Value::as_object)
        .and_then(|value| value.get("algorithm"))
        .and_then(Value::as_str);
    let digest_shape_is_valid = integrity
        .and_then(Value::as_object)
        .and_then(|value| value.get("digest"))
        .and_then(Value::as_str)
        .map(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .unwrap_or(false);
    if schema == TASK_SCHEMA {
        if algorithm == Some("hmac-sha256") && digest_shape_is_valid {
            return "present_unverified";
        }
        return "invalid";
    }
    match schema.as_str() {
        LEGACY_TASK_SCHEMA => "legacy_unsigned",
        _ => "unsupported",
    }
}

fn disposition_action(dispositions: &[String], state: &str) -> String {
    let has = |value: &str| dispositions.iter().any(|item| item == value);
    if has("receipt_malformed") {
        return "Preserve the receipt and repair or replace it through the workflow owner.".into();
    }
    if has("receipt_integrity_invalid") {
        return "Preserve the known receipt and repair its missing or malformed integrity evidence before custody mutation.".into();
    }
    if has("receipt_schema_unsupported") {
        return "Preserve the lane and upgrade the receipt through the supported workflow.".into();
    }
    if has("legacy_unsigned_receipt") {
        return "Use the bounded legacy owner-return or adoption review; do not infer custody from age.".into();
    }
    if has("competing_custody") {
        return "Freeze competing mutation and resolve the exact custody claim before integration."
            .into();
    }
    if has("worktree_not_live") {
        return "Verify branch reachability and closure evidence before archiving or deleting anything.".into();
    }
    if has("worktree_binding_mismatch") || has("branch_binding_mismatch") {
        return "Preserve the lane and reconcile the receipt against live Git bindings.".into();
    }
    if has("head_binding_mismatch") {
        return "Re-read the exact branch head and require an owner-bound custody refresh.".into();
    }
    if has("live_git_evidence_unavailable") {
        return "Retry with complete live Git and process evidence; no adoption decision is authorized.".into();
    }
    match state {
        "adoptable" => "Recheck negative evidence and use an exact-head custody adoption claim.",
        "stale" => "Preserve the lane through the grace period and recheck live activity.",
        "active" => "Continue through the owning task and renew the lease.",
        "paused" => "Wait for the declared return condition or perform reviewed adoption.",
        "integrating" => "Use the exact-head integration lock and refreshed-target gates.",
        "closed" => "Verify reachability and retain the closure receipt as evidence.",
        _ => "Preserve the lane and resolve the named custody disposition before mutation.",
    }
    .into()
}

fn primary_disposition(dispositions: &[String]) -> String {
    dispositions
        .iter()
        .find(|item| item.as_str() != "receipt_integrity_unverified")
        .cloned()
        .or_else(|| dispositions.first().cloned())
        .unwrap_or_else(|| "custody_evidence_insufficient".to_string())
}

fn parse_timestamp(value: Option<&String>) -> Option<DateTime<Utc>> {
    value
        .and_then(|item| DateTime::parse_from_rfc3339(item).ok())
        .map(|item| item.with_timezone(&Utc))
}

fn lease_expiry(value: &Value) -> Option<DateTime<Utc>> {
    parse_timestamp(string_value(value, "lease_expires_at").as_ref()).or_else(|| {
        let activity = string_value(value, "last_activity_at")
            .or_else(|| string_value(value, "last_heartbeat_at"))
            .or_else(|| string_value(value, "created_at"));
        parse_timestamp(activity.as_ref())
            .map(|timestamp| timestamp + Duration::seconds(DEFAULT_LEASE_SECONDS))
    })
}

fn parse_worktrees(path: &Path) -> Result<Vec<(PathBuf, Option<String>, Option<String>)>, String> {
    let output = git(path, &["worktree", "list", "--porcelain"])?;
    let mut records = Vec::new();
    for block in output.split("\n\n") {
        let mut worktree = None;
        let mut branch = None;
        let mut head = None;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                worktree = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("branch ") {
                branch = Some(
                    value
                        .strip_prefix("refs/heads/")
                        .unwrap_or(value)
                        .to_string(),
                );
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                head = Some(value.to_string());
            }
        }
        if let Some(worktree) = worktree {
            records.push((worktree, branch, head));
        }
    }
    if records.is_empty() {
        records.push((
            path.to_path_buf(),
            git_optional(path, &["branch", "--show-current"]),
            git_optional(path, &["rev-parse", "HEAD"]),
        ));
    }
    Ok(records)
}

fn git_operations(path: &Path) -> Vec<String> {
    let Some(git_dir) = git_optional(path, &["rev-parse", "--path-format=absolute", "--git-dir"])
    else {
        return vec!["git-dir-unavailable".to_string()];
    };
    let root = canonical(Path::new(&git_dir));
    [
        ("MERGE_HEAD", root.join("MERGE_HEAD")),
        ("CHERRY_PICK_HEAD", root.join("CHERRY_PICK_HEAD")),
        ("REVERT_HEAD", root.join("REVERT_HEAD")),
        ("rebase-merge", root.join("rebase-merge")),
        ("rebase-apply", root.join("rebase-apply")),
    ]
    .into_iter()
    .filter_map(|(name, marker)| marker.exists().then_some(name.to_string()))
    .collect()
}

fn open_files(path: &Path) -> Option<bool> {
    #[cfg(target_os = "windows")]
    let _ = path;
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("lsof")
            .args(["-t", "+D"])
            .arg(path)
            .output()
            .ok()?;
        if output.status.success() {
            return Some(!output.stdout.is_empty());
        }
        if output.status.code() == Some(1) {
            return Some(false);
        }
        return None;
    }
    #[cfg(target_os = "windows")]
    None
}

fn live_worktrees(path: &Path) -> Result<Vec<LiveWorktree>, String> {
    let records = parse_worktrees(path)?;
    Ok(records
        .into_iter()
        .map(|(worktree, branch, head_sha)| {
            let worktree = canonical(&worktree);
            let clean = git(&worktree, &["status", "--porcelain=v2", "-z"])
                .map(|value| value.is_empty())
                .ok();
            LiveWorktree {
                path: worktree.clone(),
                branch,
                head_sha,
                clean,
                operations: git_operations(&worktree),
                open_files: open_files(&worktree),
            }
        })
        .collect())
}

fn changed_paths(path: &Path, base: Option<&String>, head: Option<&String>) -> Vec<String> {
    let (Some(base), Some(head)) = (base, head) else {
        return Vec::new();
    };
    let Ok(output) = git(
        path,
        &["diff", "--name-only", "-z", &format!("{base}..{head}")],
    ) else {
        return Vec::new();
    };
    output
        .split('\0')
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn classify(
    receipt: &Value,
    receipt_path: &Path,
    live: Option<&LiveWorktree>,
    duplicate: bool,
    now: DateTime<Utc>,
    repository: &Path,
) -> CustodyLane {
    let task_id = string_value(receipt, "task_id")
        .or_else(|| {
            receipt_path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string());
    let branch = string_value(receipt, "branch");
    let worktree = string_value(receipt, "worktree");
    let base_sha = string_value(receipt, "base_sha");
    let head_sha = string_value(receipt, "head_sha");
    let recorded_state = string_value(receipt, "state").or_else(|| string_value(receipt, "status"));
    let mut state = "unknown".to_string();
    let mut blockers = Vec::new();
    let mut evidence = vec![format!("receipt-integrity={}", receipt_integrity(receipt))];
    let integrity = receipt_integrity(receipt);
    let mut dispositions = vec![match integrity {
        "present_unverified" => "receipt_integrity_unverified",
        "legacy_unsigned" => "legacy_unsigned_receipt",
        "invalid" => "receipt_integrity_invalid",
        _ if string_value(receipt, "schema_version").as_deref() == Some("invalid") => {
            "receipt_malformed"
        }
        _ => "receipt_schema_unsupported",
    }
    .to_string()];
    let receipt_is_supported = integrity == "present_unverified";
    let recorded = recorded_state
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    if recorded == "closed" || string_value(receipt, "status").as_deref() == Some("finished") {
        state = "closed".to_string();
        evidence.push("receipt-state=closed".to_string());
    } else if recorded == "integrating" {
        state = "integrating".to_string();
    } else if recorded == "paused" || string_value(receipt, "status").as_deref() == Some("released")
    {
        state = "paused".to_string();
    } else if integrity == "legacy_unsigned" {
        blockers.push("legacy-receipt-requires-review".to_string());
    } else if !receipt_is_supported {
        blockers.push("receipt-schema-or-integrity-unavailable".to_string());
    } else if duplicate {
        state = "contested".to_string();
        blockers.push("multiple-custody-records-bind-the-same-lane".to_string());
        dispositions.push("competing_custody".to_string());
    } else if live.is_none() {
        blockers.push("registered-worktree-is-not-live".to_string());
        dispositions.push("worktree_not_live".to_string());
    } else if let Some(live) = live {
        let mut identity_blockers = 0;
        if let Some(expected) = worktree.as_ref() {
            if canonical(Path::new(expected)) != live.path {
                blockers.push("worktree-path-mismatch".to_string());
                dispositions.push("worktree_binding_mismatch".to_string());
                identity_blockers += 1;
            }
        } else {
            blockers.push("receipt-worktree-missing".to_string());
            dispositions.push("worktree_binding_mismatch".to_string());
            identity_blockers += 1;
        }
        if branch.as_deref() != live.branch.as_deref() {
            blockers.push("branch-mismatch".to_string());
            dispositions.push("branch_binding_mismatch".to_string());
            identity_blockers += 1;
        }
        if let (Some(recorded_head), Some(live_head)) = (head_sha.as_ref(), live.head_sha.as_ref())
        {
            if recorded_head != live_head {
                blockers.push("head-sha-mismatch".to_string());
                dispositions.push("head_binding_mismatch".to_string());
                identity_blockers += 1;
            }
        } else {
            blockers.push("head-sha-unavailable".to_string());
            dispositions.push("head_binding_mismatch".to_string());
            identity_blockers += 1;
        }
        if !live.operations.is_empty() {
            blockers.push("git-operation-in-progress".to_string());
            dispositions.push("git_operation_active".to_string());
            evidence.extend(
                live.operations
                    .iter()
                    .map(|operation| format!("operation={operation}")),
            );
        }
        match live.clean {
            Some(true) => evidence.push("worktree-clean".to_string()),
            Some(false) => {
                blockers.push("worktree-dirty".to_string());
                dispositions.push("dirty_worktree".to_string());
            }
            None => {
                blockers.push("worktree-status-unavailable".to_string());
                dispositions.push("live_git_evidence_unavailable".to_string());
            }
        }
        match live.open_files {
            Some(false) => evidence.push("no-open-files-observed".to_string()),
            Some(true) => {
                blockers.push("open-files-observed".to_string());
                dispositions.push("open_files_observed".to_string());
            }
            None => {
                blockers.push("open-file-evidence-unavailable".to_string());
                dispositions.push("live_git_evidence_unavailable".to_string());
            }
        }

        let expiry = lease_expiry(receipt);
        if let Some(expiry) = expiry {
            if identity_blockers > 0 {
                state = "unknown".to_string();
            } else if live.clean.is_none() || live.open_files.is_none() {
                state = "unknown".to_string();
            } else if now <= expiry {
                state = "active".to_string();
                dispositions.push("lease_current".to_string());
            } else if now <= expiry + Duration::seconds(DEFAULT_ADOPTION_SECONDS) {
                state = "stale".to_string();
                dispositions.push("lease_expired_grace".to_string());
            } else if live.clean == Some(true)
                && live.open_files == Some(false)
                && live.operations.is_empty()
            {
                state = "adoptable".to_string();
                dispositions.push("adoption_ready".to_string());
            } else {
                state = "stale".to_string();
                dispositions.push("adoption_blocked".to_string());
            }
        } else {
            blockers.push("lease-expiry-unavailable".to_string());
            dispositions.push("lease_expiry_unavailable".to_string());
        }
    }

    if state == "unknown" && blockers.is_empty() {
        blockers.push("insufficient-live-custody-evidence".to_string());
        dispositions.push("custody_evidence_insufficient".to_string());
    }
    dispositions.sort();
    dispositions.dedup();
    if !blockers.is_empty() && state == "unknown" {
        evidence.extend(blockers.iter().map(|blocker| format!("blocker={blocker}")));
    }
    let declared_scope = string_list(receipt, "declared_scope");
    let live_head = live.and_then(|item| item.head_sha.clone());
    let observed_head = live_head.clone().or(head_sha.clone());
    let mut paths = changed_paths(repository, base_sha.as_ref(), observed_head.as_ref());
    if paths.is_empty() {
        paths = string_list(receipt, "changed_paths");
    }

    let next_action = disposition_action(&dispositions, &state);
    CustodyLane {
        task_id,
        work_item_id: string_value(receipt, "work_item_id")
            .or_else(|| string_value(receipt, "task")),
        branch,
        worktree,
        base_ref: string_value(receipt, "base_ref"),
        base_sha,
        head_sha: observed_head,
        workspace_class: "temporary".to_string(),
        lease_required: true,
        recorded_state,
        state,
        disposition: primary_disposition(&dispositions),
        next_action,
        dispositions,
        custodian: string_value(receipt, "custodian")
            .or_else(|| string_value(receipt, "thread_id")),
        declared_scope,
        changed_paths: paths,
        created_at: string_value(receipt, "created_at"),
        last_activity_at: string_value(receipt, "last_activity_at")
            .or_else(|| string_value(receipt, "last_heartbeat_at")),
        lease_expires_at: lease_expiry(receipt)
            .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true)),
        provider_review: string_value(receipt, "provider_review"),
        blockers,
        evidence,
        receipt: Some(receipt_path.to_string_lossy().to_string()),
    }
}

fn overlap_paths(left: &CustodyLane, right: &CustodyLane) -> Vec<String> {
    let left_paths = left
        .changed_paths
        .iter()
        .chain(left.declared_scope.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let right_paths = right
        .changed_paths
        .iter()
        .chain(right.declared_scope.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    left_paths.intersection(&right_paths).cloned().collect()
}

fn counts(lanes: &[CustodyLane]) -> CustodyCounts {
    let mut states = BTreeMap::new();
    for lane in lanes {
        *states.entry(lane.state.clone()).or_insert(0) += 1;
    }
    CustodyCounts { states }
}

fn disposition_counts(lanes: &[CustodyLane]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for lane in lanes {
        *counts.entry(lane.disposition.clone()).or_insert(0) += 1;
    }
    counts
}

fn read_receipts(root: &Path) -> Vec<(PathBuf, Value)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let value = fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str::<Value>(&content).ok())
                .unwrap_or_else(|| serde_json::json!({"schema_version": "invalid", "task_id": path.file_stem().and_then(|value| value.to_str()).unwrap_or("unknown")}));
            (path, value)
        })
        .collect()
}

pub fn project(path: &Path) -> Result<CustodySnapshot, String> {
    let repository = canonical(path);
    let root = receipt_root(&repository)?;
    let live_records = live_worktrees(&repository)?;
    let live_by_path = live_records
        .iter()
        .map(|record| (record.path.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let receipts = read_receipts(&root);
    let now = Utc::now();
    let mut lanes = Vec::new();
    let mut integrity_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut binding_counts: BTreeMap<String, usize> = BTreeMap::new();

    for (_receipt_path, receipt) in &receipts {
        let receipt_status = receipt_integrity(receipt);
        *integrity_counts
            .entry(receipt_status.to_string())
            .or_insert(0) += 1;
        let binding = string_value(receipt, "worktree")
            .map(|value| canonical(Path::new(&value)).to_string_lossy().to_string())
            .unwrap_or_default();
        if !binding.is_empty() {
            *binding_counts.entry(binding).or_insert(0) += 1;
        }
    }

    for (receipt_path, receipt) in receipts {
        let binding = string_value(&receipt, "worktree").map(|value| canonical(Path::new(&value)));
        let live = binding
            .as_ref()
            .and_then(|path| live_by_path.get(path).copied());
        let duplicate = binding
            .as_ref()
            .and_then(|path| binding_counts.get(&path.to_string_lossy().to_string()))
            .is_some_and(|count| *count > 1);
        lanes.push(classify(
            &receipt,
            &receipt_path,
            live,
            duplicate,
            now,
            &repository,
        ));
    }

    let mut overlaps = Vec::new();
    for left_index in 0..lanes.len() {
        for right_index in (left_index + 1)..lanes.len() {
            let paths = overlap_paths(&lanes[left_index], &lanes[right_index]);
            if !paths.is_empty() {
                overlaps.push(CustodyOverlap {
                    left_task_id: lanes[left_index].task_id.clone(),
                    right_task_id: lanes[right_index].task_id.clone(),
                    paths,
                    status: if lanes[left_index].state == "closed"
                        || lanes[right_index].state == "closed"
                    {
                        "historical_overlap".to_string()
                    } else {
                        "integration_serialize".to_string()
                    },
                });
            }
        }
    }

    let bound_paths = lanes
        .iter()
        .filter_map(|lane| lane.worktree.as_ref())
        .map(|path| canonical(Path::new(path)))
        .collect::<BTreeSet<_>>();
    let repository_path = canonical(&repository);
    let active_temporary_lanes = lanes.iter().filter(|lane| lane.state != "closed").count() as u64;
    let workspace_policy =
        project_workspace_policy(&repository, &live_records, active_temporary_lanes);
    let policy_for_matching = parse_workspace_policy(&repository).ok().flatten();
    let unleased_worktrees = live_records
        .iter()
        .filter(|record| {
            !canonical_matches(record, policy_for_matching.as_ref(), &repository)
                && !bound_paths.contains(&record.path)
                && record.path != repository_path
        })
        .map(|record| record.path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let counts = counts(&lanes);
    let disposition_counts = disposition_counts(&lanes);
    let blocked = lanes
        .iter()
        .any(|lane| matches!(lane.state.as_str(), "unknown" | "contested"))
        || !unleased_worktrees.is_empty()
        || matches!(
            workspace_policy.status.as_str(),
            "canonical_drift" | "invalid"
        );
    let status = if blocked {
        "attention_required"
    } else {
        "observed"
    };
    let next_safe_step = if workspace_policy.status == "invalid" {
        "Repair the invalid workspace policy before relying on canonical protection or temporary-lane counts.".to_string()
    } else if workspace_policy.status == "canonical_drift" {
        "Restore the role-defined canonical workspaces before treating temporary-lane counts as settled.".to_string()
    } else if !unleased_worktrees.is_empty() {
        "Register each unleased task worktree through isolated-change-workflow before editing or integrating it.".to_string()
    } else if lanes.iter().any(|lane| lane.state == "adoptable") {
        "Recheck live negative evidence, then use isolated-change-workflow adopt with an exact generation and head.".to_string()
    } else if lanes.iter().any(|lane| lane.state == "contested") {
        "Resolve competing custody and integration order before mutation.".to_string()
    } else if lanes.iter().any(|lane| lane.state == "unknown") {
        "Preserve the lane and obtain the missing live receipt, worktree, or integrity evidence."
            .to_string()
    } else {
        "Use the isolated-change-workflow integration queue for reviewed changes; this projection grants no mutation authority.".to_string()
    };

    let mut integrity = integrity_counts
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect::<BTreeMap<_, _>>();
    integrity.insert(
        "hmac_identity".to_string(),
        "not_verified_by_pronto".to_string(),
    );
    integrity.insert("live_git".to_string(), "observed".to_string());
    integrity.insert("provider_review".to_string(), "not_queried".to_string());

    Ok(CustodySnapshot {
        schema_version: CUSTODY_PROJECTION_SCHEMA.to_string(),
        generated_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        repository: repository.to_string_lossy().to_string(),
        receipt_root: root.to_string_lossy().to_string(),
        source: "live_git_plus_local_isolated_change_receipts".to_string(),
        read_only: true,
        implementation_allowed: false,
        mutation_risk: "read-only".to_string(),
        status: status.to_string(),
        next_safe_step,
        lanes,
        unleased_worktrees,
        counts,
        disposition_counts,
        overlaps,
        workspace_policy,
        integrity,
    })
}

#[tauri::command]
pub fn get_custody(repository: String) -> Result<CustodySnapshot, String> {
    project(Path::new(&repository))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pronto-custody-{suffix}"));
        fs::create_dir_all(&path).expect("directory");
        git(&path, &["init", "-q"]).expect("git init");
        fs::write(path.join("README.md"), "custody\n").expect("file");
        git(&path, &["add", "README.md"]).expect("git add");
        Command::new("git")
            .arg("-C")
            .arg(&path)
            .args([
                "-c",
                "user.name=Pronto Test",
                "-c",
                "user.email=pronto@example.invalid",
                "commit",
                "-qm",
                "initial",
            ])
            .status()
            .expect("git commit")
            .success()
            .then_some(())
            .expect("commit status");
        path
    }

    fn write_supporting_workspace_policy(repo: &Path) {
        let branch = git(repo, &["branch", "--show-current"]).expect("branch");
        let policy = serde_json::json!({
            "schema_version": WORKSPACE_POLICY_SCHEMA,
            "repository_role": "supporting_project",
            "canonical_workspaces": [{
                "id": "working",
                "role": "working",
                "ref": branch,
                "protected": true
            }],
            "retention_exceptions": []
        });
        fs::create_dir_all(repo.join(".agents")).expect("policy directory");
        fs::write(
            repo.join(DEFAULT_WORKSPACE_POLICY_RELATIVE_PATH),
            serde_json::to_vec_pretty(&policy).expect("policy JSON"),
        )
        .expect("workspace policy");
    }

    #[test]
    fn workspace_policy_separates_canonical_and_temporary_worktrees() {
        let repo = temp_repo();
        write_supporting_workspace_policy(&repo);
        let worktree = repo.with_file_name(format!(
            "{}-lane",
            repo.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("repo")
        ));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "codex/custody-policy-test",
                worktree.to_str().expect("path"),
                "HEAD",
            ],
        )
        .expect("worktree add");

        let snapshot = project(&repo).expect("projection");
        assert_eq!(
            snapshot.workspace_policy.repository_role,
            "supporting_project"
        );
        assert_eq!(snapshot.workspace_policy.baseline_target, Some(1));
        assert_eq!(snapshot.workspace_policy.canonical_observed, 1);
        assert_eq!(snapshot.workspace_policy.temporary_observed, 1);
        assert_eq!(snapshot.workspace_policy.active_temporary_lanes, 0);
        assert_eq!(snapshot.workspace_policy.status, "observed");
        assert_eq!(snapshot.unleased_worktrees.len(), 1);
        assert_eq!(
            snapshot.unleased_worktrees[0],
            canonical(&worktree).to_string_lossy()
        );

        fs::remove_dir_all(&worktree).ok();
        git(&repo, &["worktree", "prune"]).ok();
        fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn invalid_workspace_policy_is_explicit_and_does_not_authorize_mutation() {
        let repo = temp_repo();
        fs::create_dir_all(repo.join(".agents")).expect("policy directory");
        fs::write(
            repo.join(DEFAULT_WORKSPACE_POLICY_RELATIVE_PATH),
            "{\"schema_version\":\"workspace-policy/v1\",\"repository_role\":\"production_product\"}",
        )
        .expect("invalid policy");

        let snapshot = project(&repo).expect("projection");
        assert_eq!(snapshot.workspace_policy.status, "invalid");
        assert_eq!(snapshot.workspace_policy.disposition, "policy_invalid");
        assert!(snapshot.workspace_policy.drift[0].starts_with("policy-invalid:"));
        assert!(snapshot.implementation_allowed == false);
        assert_eq!(snapshot.next_safe_step, "Repair the invalid workspace policy before relying on canonical protection or temporary-lane counts.");

        fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn projection_is_read_only_and_reports_unleased_worktree() {
        let repo = temp_repo();
        let worktree = repo.with_file_name(format!(
            "{}-lane",
            repo.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("repo")
        ));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "codex/custody-test",
                worktree.to_str().expect("path"),
                "HEAD",
            ],
        )
        .expect("worktree add");
        let before = git(&repo, &["status", "--porcelain=v2"]).expect("status");
        let snapshot = project(&repo).expect("projection");
        let after = git(&repo, &["status", "--porcelain=v2"]).expect("status");
        assert!(snapshot.read_only);
        assert!(!snapshot.implementation_allowed);
        assert_eq!(before, after);
        assert_eq!(
            snapshot.unleased_worktrees,
            vec![canonical(&worktree).to_string_lossy().to_string()]
        );
        fs::remove_dir_all(&worktree).ok();
        git(&repo, &["worktree", "prune"]).ok();
        fs::remove_dir_all(&repo).ok();
    }

    #[test]
    fn invalid_and_unsupported_receipts_keep_distinct_dispositions() {
        let repo = temp_repo();
        let root = receipt_root(&repo).expect("receipt root");
        fs::create_dir_all(&root).expect("receipt root");
        let worktree = repo.with_file_name(format!(
            "{}-lane",
            repo.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("repo")
        ));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "codex/custody-test",
                worktree.to_str().expect("path"),
                "HEAD",
            ],
        )
        .expect("worktree add");
        let receipt = serde_json::json!({
            "schema_version": "isolated-change-task/v2",
            "task_id": "custody-test",
            "worktree": worktree,
            "branch": "codex/custody-test",
            "state": "active",
            "head_sha": git(&repo, &["rev-parse", "HEAD"]).expect("head"),
            "last_activity_at": "2000-01-01T00:00:00Z"
        });
        fs::write(
            root.join("custody-test.json"),
            serde_json::to_vec(&receipt).expect("json"),
        )
        .expect("receipt");
        let snapshot = project(&repo).expect("projection");
        assert_eq!(snapshot.lanes.len(), 1);
        assert_eq!(snapshot.lanes[0].state, "unknown");
        assert_eq!(snapshot.lanes[0].disposition, "receipt_integrity_invalid");
        assert!(snapshot.lanes[0].next_action.contains("integrity"));
        assert!(snapshot.lanes[0]
            .blockers
            .iter()
            .any(|item| item.contains("integrity")));
        assert!(!snapshot.lanes.iter().any(|lane| lane.state == "adoptable"));
        fs::write(
            root.join("unsupported.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "isolated-change-task/v9",
                "task_id": "unsupported",
                "state": "active"
            }))
            .expect("unsupported receipt"),
        )
        .expect("unsupported receipt write");
        let snapshot = project(&repo).expect("projection");
        let unsupported = snapshot
            .lanes
            .iter()
            .find(|lane| lane.task_id == "unsupported")
            .expect("unsupported lane");
        assert_eq!(unsupported.state, "unknown");
        assert_eq!(unsupported.disposition, "receipt_schema_unsupported");
        fs::remove_dir_all(&worktree).ok();
        git(&repo, &["worktree", "prune"]).ok();
        fs::remove_dir_all(&repo).ok();
    }
}
