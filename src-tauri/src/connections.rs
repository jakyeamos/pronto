use crate::core::{RemoteRepositorySnapshot, RepositorySnapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_DISCOVERY_FILES: usize = 450;
const MAX_SOURCE_BYTES: u64 = 64 * 1024;

static NEXT_MANUAL_CONNECTION_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionEvidence {
    pub adapter: String,
    pub source_path: Option<String>,
    pub detail: String,
    pub observed_at: String,
    pub freshness: String,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub identity: String,
    pub repository_id: Option<String>,
    pub origin: String,
    pub confidence: String,
    pub status: String,
    pub label_override: Option<String>,
    pub kind_override: Option<String>,
    pub last_seen_at: Option<String>,
    pub evidence: Vec<ConnectionEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Connection {
    pub id: String,
    pub fingerprint: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relationship_type: String,
    pub label: String,
    pub origin: String,
    pub review_state: String,
    pub confidence: String,
    pub status: String,
    pub label_override: Option<String>,
    pub last_seen_at: Option<String>,
    pub evidence: Vec<ConnectionEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowStep {
    pub id: String,
    pub order: u32,
    pub node_id: String,
    pub action_label: String,
    pub command: Option<String>,
    pub connection_id: Option<String>,
    pub evidence: Vec<ConnectionEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub origin: String,
    pub status: String,
    pub review_state: String,
    pub participating_repositories: Vec<String>,
    pub name_override: Option<String>,
    pub last_seen_at: Option<String>,
    pub evidence: Vec<ConnectionEvidence>,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionAdapterStatus {
    pub id: String,
    pub enabled: bool,
    pub freshness: String,
    pub permission_state: String,
    pub failure_message: Option<String>,
    pub last_run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ConnectionsSnapshot {
    pub nodes: Vec<ConnectionNode>,
    pub connections: Vec<Connection>,
    pub workflows: Vec<Workflow>,
    pub adapters: Vec<ConnectionAdapterStatus>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionNodeInput {
    pub node_id: Option<String>,
    pub kind: String,
    pub label: String,
    pub identity: Option<String>,
    pub repository_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInput {
    pub connection_id: Option<String>,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relationship_type: String,
    pub label: Option<String>,
    pub confidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepInput {
    pub node_id: String,
    pub action_label: String,
    pub command: Option<String>,
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub workflow_id: Option<String>,
    pub name: String,
    pub scope: String,
    pub repository_ids: Vec<String>,
    pub steps: Vec<WorkflowStepInput>,
}

pub fn default_adapters() -> Vec<ConnectionAdapterStatus> {
    vec![
        ConnectionAdapterStatus {
            id: "static-discovery".to_string(),
            enabled: true,
            freshness: "Not run".to_string(),
            permission_state: "Local read-only".to_string(),
            failure_message: None,
            last_run_at: None,
        },
        ConnectionAdapterStatus {
            id: "deep-code".to_string(),
            enabled: false,
            freshness: "Not analyzed".to_string(),
            permission_state: "Opt-in local read-only".to_string(),
            failure_message: Some(
                "Only validated JavaScript/TypeScript, Rust, and Python fixtures are analyzed; unsupported languages remain not analyzed.".to_string(),
            ),
            last_run_at: None,
        },
        ConnectionAdapterStatus {
            id: "runtime-provider".to_string(),
            enabled: false,
            freshness: "Not enabled".to_string(),
            permission_state: "Opt-in provider/runtime read-only".to_string(),
            failure_message: Some(
                "No network or runtime query is performed unless this adapter is explicitly enabled.".to_string(),
            ),
            last_run_at: None,
        },
    ]
}

pub fn normalize_snapshot(snapshot: &mut ConnectionsSnapshot) {
    if snapshot.adapters.is_empty() {
        snapshot.adapters = default_adapters();
    } else {
        for default in default_adapters() {
            if !snapshot
                .adapters
                .iter()
                .any(|adapter| adapter.id == default.id)
            {
                snapshot.adapters.push(default);
            }
        }
    }
    snapshot.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    snapshot
        .connections
        .sort_by(|left, right| left.fingerprint.cmp(&right.fingerprint));
    snapshot
        .workflows
        .sort_by(|left, right| left.id.cmp(&right.id));
    for workflow in &mut snapshot.workflows {
        workflow.steps.sort_by_key(|step| step.order);
    }
}

pub fn redacted_command(command: &str) -> String {
    let mut redacted = Vec::new();
    let mut redact_next = false;
    for token in command.split_whitespace() {
        if redact_next {
            redacted.push("[REDACTED]".to_string());
            redact_next = false;
            continue;
        }
        let key = token.split('=').next().unwrap_or(token);
        let lower = token.to_ascii_lowercase();
        if is_secret_key(key) && !token.contains('=') {
            redacted.push(token.to_string());
            redact_next = true;
            continue;
        }
        if token.contains('=') && is_secret_key(key) {
            redacted.push(format!("{key}=[REDACTED]"));
            continue;
        }
        if looks_like_literal_token(&lower) {
            redacted.push("[REDACTED]".to_string());
            continue;
        }
        redacted.push(token.to_string());
    }
    redacted.join(" ")
}

fn is_secret_key(key: &str) -> bool {
    let normalized = key
        .trim_start_matches('-')
        .replace('-', "_")
        .replace('.', "_")
        .to_ascii_lowercase();
    normalized == "token"
        || normalized == "password"
        || normalized == "secret"
        || normalized == "api_key"
        || normalized == "apikey"
        || normalized == "authorization"
        || normalized == "access_key"
        || normalized == "private_key"
        || normalized == "key"
        || normalized.contains("token")
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("authorization")
        || normalized.contains("access_key")
        || normalized.contains("private_key")
}

fn looks_like_literal_token(value: &str) -> bool {
    let value = value.trim_matches(['"', '\'']);
    value.len() >= 12
        && [
            "ghp_",
            "github_pat_",
            "glpat-",
            "xoxb-",
            "xoxp-",
            "sk-",
            "akia",
        ]
        .iter()
        .any(|prefix| value.to_ascii_lowercase().starts_with(prefix))
}

pub fn refresh_snapshot(
    previous: &ConnectionsSnapshot,
    repositories: &[RepositorySnapshot],
    remote_repositories: &[RemoteRepositorySnapshot],
    target_repository_ids: Option<&HashSet<String>>,
    observed_at: &str,
) -> ConnectionsSnapshot {
    let enabled = |id: &str, default: bool| {
        previous
            .adapters
            .iter()
            .find(|adapter| adapter.id == id)
            .map(|adapter| adapter.enabled)
            .unwrap_or(default)
    };
    let include_static = enabled("static-discovery", true);
    let include_deep_code = enabled("deep-code", false);
    let include_runtime = enabled("runtime-provider", false);
    let mut discovered = DiscoveryBuilder::default();
    let selected_repositories = repositories
        .iter()
        .filter(|repository| {
            target_repository_ids
                .map(|targets| targets.contains(&repository.id))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    for repository in &selected_repositories {
        discovered.repository(repository, observed_at);
        if include_static {
            discovered.static_repository(repository, observed_at);
        }
        if include_deep_code {
            discovered.deep_code(repository, observed_at);
        }
        if include_runtime {
            discovered.runtime(repository, remote_repositories, observed_at);
        }
    }
    discovered.cross_repository_workflows(&selected_repositories, observed_at);
    let deep_supported_files = discovered.deep_supported_files;
    let deep_unsupported_files = discovered.deep_unsupported_files;
    let mut result = merge_snapshot(
        previous,
        discovered.finish(),
        target_repository_ids,
        observed_at,
    );
    if include_deep_code && deep_unsupported_files > 0 {
        if let Some(adapter) = result
            .adapters
            .iter_mut()
            .find(|adapter| adapter.id == "deep-code")
        {
            adapter.failure_message = Some(format!(
                "{deep_unsupported_files} unsupported source file(s) were not analyzed; validated JavaScript/TypeScript, Rust, and Python files are the only deep-code fixture languages."
            ));
            if deep_supported_files == 0 {
                adapter.freshness = "Not analyzed".to_string();
            }
        }
    }
    result
}

#[derive(Default)]
struct DiscoveryBuilder {
    nodes: HashMap<String, ConnectionNode>,
    connections: HashMap<String, Connection>,
    workflows: HashMap<String, Workflow>,
    deep_supported_files: usize,
    deep_unsupported_files: usize,
}

impl DiscoveryBuilder {
    fn repository(&mut self, repository: &RepositorySnapshot, observed_at: &str) {
        let identity = format!("repository:{}", repository.id);
        self.node(
            "repository",
            repository.name.clone(),
            identity,
            Some(repository.id.clone()),
            evidence(
                "static-discovery",
                Some(repository.path.clone()),
                "Registered repository snapshot",
                observed_at,
                None,
            ),
        );
        for workspace in &repository.workspaces {
            let workspace_identity = format!("workspace:{}", workspace.id);
            let workspace_id = self.node(
                "workspace",
                if workspace.is_primary {
                    format!("{} workspace", repository.name)
                } else {
                    workspace.branch.clone()
                },
                workspace_identity,
                Some(repository.id.clone()),
                evidence(
                    "static-discovery",
                    Some(workspace.path.clone()),
                    "Workspace found in the local repository snapshot",
                    observed_at,
                    None,
                ),
            );
            self.edge(
                &format!("connection-node:repository:{}", repository.id),
                &workspace_id,
                "runtime",
                "has workspace",
                "High",
                evidence(
                    "static-discovery",
                    Some(workspace.path.clone()),
                    "Local workspace relationship",
                    observed_at,
                    None,
                ),
            );
        }
    }

    fn static_repository(&mut self, repository: &RepositorySnapshot, observed_at: &str) {
        let repository_node = format!("connection-node:repository:{}", repository.id);
        if let Some(remote_url) = repository.remote_url.as_deref() {
            let remote_identity = format!("remote:{}", safe_remote_identity(remote_url));
            let remote_id = self.node(
                "service",
                remote_label(remote_url),
                remote_identity,
                None,
                evidence(
                    "static-discovery",
                    Some(repository.path.clone()),
                    "Git origin remote",
                    observed_at,
                    None,
                ),
            );
            self.edge(
                &repository_node,
                &remote_id,
                "handoff",
                "publishes to remote",
                "High",
                evidence(
                    "static-discovery",
                    Some(repository.path.clone()),
                    "git remote get-url origin",
                    observed_at,
                    None,
                ),
            );
        }
        for submodule in &repository.submodules {
            let submodule_identity = format!("submodule:{}:{}", repository.id, submodule.path);
            let target_id = self.node(
                "repository",
                submodule.path.clone(),
                submodule_identity,
                None,
                evidence(
                    "static-discovery",
                    Some(
                        Path::new(&repository.path)
                            .join(&submodule.path)
                            .to_string_lossy()
                            .to_string(),
                    ),
                    "Git submodule",
                    observed_at,
                    None,
                ),
            );
            self.edge(
                &repository_node,
                &target_id,
                "dependency",
                "contains submodule",
                "High",
                evidence(
                    "static-discovery",
                    Some(
                        Path::new(&repository.path)
                            .join(".gitmodules")
                            .to_string_lossy()
                            .to_string(),
                    ),
                    "Git submodule relationship",
                    observed_at,
                    None,
                ),
            );
        }

        let root = Path::new(&repository.path);
        self.package_manifest(repository, root, "package.json", observed_at);
        self.text_dependencies(
            repository,
            root,
            "Cargo.toml",
            &[
                "[dependencies]",
                "[dev-dependencies]",
                "[build-dependencies]",
            ],
            observed_at,
        );
        self.text_dependencies(repository, root, "requirements.txt", &[""], observed_at);
        self.text_dependencies(
            repository,
            root,
            "pyproject.toml",
            &["dependencies", "optional-dependencies"],
            observed_at,
        );
        self.workspace_configuration(repository, root, observed_at);
        for lockfile in [
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
            "bun.lockb",
            "bun.lock",
            "Cargo.lock",
            "poetry.lock",
            "Pipfile.lock",
            "uv.lock",
        ] {
            self.lockfile(repository, root, lockfile, observed_at);
        }
        self.ci_workflows(repository, root, observed_at);
        for deployment in [
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
            "vercel.json",
            "netlify.toml",
            "fly.toml",
            "render.yaml",
            "railway.json",
        ] {
            let path = root.join(deployment);
            if path.is_file() {
                let kind = if deployment.contains("compose") || deployment == "Dockerfile" {
                    "tool"
                } else {
                    "environment"
                };
                let node_id = self.node(
                    kind,
                    deployment.to_string(),
                    format!("deployment:{}:{deployment}", repository.id),
                    Some(repository.id.clone()),
                    evidence(
                        "static-discovery",
                        Some(path.to_string_lossy().to_string()),
                        "Deployment configuration detected",
                        observed_at,
                        None,
                    ),
                );
                self.edge(
                    &repository_node,
                    &node_id,
                    "deployment",
                    "deploys with",
                    "Medium",
                    evidence(
                        "static-discovery",
                        Some(path.to_string_lossy().to_string()),
                        "Deployment configuration detected",
                        observed_at,
                        None,
                    ),
                );
            }
        }
    }

    fn workspace_configuration(
        &mut self,
        repository: &RepositorySnapshot,
        root: &Path,
        observed_at: &str,
    ) {
        let package_path = root.join("package.json");
        if let Some(contents) = read_bounded(&package_path) {
            if let Ok(payload) = serde_json::from_str::<Value>(&contents) {
                let package_workspaces = payload
                    .get("workspaces")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                let object_workspaces = payload
                    .get("workspaces")
                    .and_then(Value::as_object)
                    .and_then(|value| value.get("packages"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                for pattern in package_workspaces.into_iter().chain(object_workspaces) {
                    self.workspace_member(repository, &package_path, &pattern, observed_at);
                }
            }
        }

        let pnpm_path = root.join("pnpm-workspace.yaml");
        if let Some(contents) = read_bounded(&pnpm_path) {
            let mut in_packages = false;
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed == "packages:" {
                    in_packages = true;
                    continue;
                }
                if in_packages && trimmed.ends_with(':') && !trimmed.starts_with('-') {
                    in_packages = false;
                }
                if !in_packages {
                    continue;
                }
                let Some(pattern) = trimmed.strip_prefix('-') else {
                    continue;
                };
                let pattern = pattern.trim().trim_matches(['"', '\'']);
                if !pattern.is_empty() {
                    self.workspace_member(repository, &pnpm_path, pattern, observed_at);
                }
            }
        }

        let cargo_path = root.join("Cargo.toml");
        if let Some(contents) = read_bounded(&cargo_path) {
            let mut in_workspace = false;
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    in_workspace = trimmed == "[workspace]";
                    continue;
                }
                if !in_workspace {
                    continue;
                }
                let Some((key, value)) = trimmed.split_once('=') else {
                    continue;
                };
                if key.trim() != "members" {
                    continue;
                }
                for pattern in value
                    .trim()
                    .trim_matches(['[', ']'])
                    .split(',')
                    .map(|value| value.trim().trim_matches(['"', '\'']))
                    .filter(|value| !value.is_empty())
                {
                    self.workspace_member(repository, &cargo_path, pattern, observed_at);
                }
            }
        }
    }

    fn workspace_member(
        &mut self,
        repository: &RepositorySnapshot,
        source_path: &Path,
        pattern: &str,
        observed_at: &str,
    ) {
        let node_id = self.node(
            "workspace",
            pattern.to_string(),
            format!("workspace:{}:{pattern}", repository.id),
            Some(repository.id.clone()),
            evidence(
                "static-discovery",
                Some(source_path.to_string_lossy().to_string()),
                "Workspace member configuration",
                observed_at,
                None,
            ),
        );
        self.edge(
            &format!("connection-node:repository:{}", repository.id),
            &node_id,
            "dependency",
            "workspace member",
            "High",
            evidence(
                "static-discovery",
                Some(source_path.to_string_lossy().to_string()),
                &format!("Workspace member pattern: {pattern}"),
                observed_at,
                None,
            ),
        );
    }

    fn lockfile(
        &mut self,
        repository: &RepositorySnapshot,
        root: &Path,
        filename: &str,
        observed_at: &str,
    ) {
        let path = root.join(filename);
        if !path.is_file() {
            return;
        }
        let node_id = self.node(
            "tool",
            filename.to_string(),
            format!("lockfile:{}:{filename}", repository.id),
            Some(repository.id.clone()),
            evidence(
                "static-discovery",
                Some(path.to_string_lossy().to_string()),
                "Dependency lockfile detected",
                observed_at,
                None,
            ),
        );
        self.edge(
            &format!("connection-node:repository:{}", repository.id),
            &node_id,
            "dependency",
            "uses dependency lockfile",
            "High",
            evidence(
                "static-discovery",
                Some(path.to_string_lossy().to_string()),
                "Dependency lockfile detected",
                observed_at,
                None,
            ),
        );
    }

    fn package_manifest(
        &mut self,
        repository: &RepositorySnapshot,
        root: &Path,
        filename: &str,
        observed_at: &str,
    ) {
        let path = root.join(filename);
        let Some(contents) = read_bounded(&path) else {
            return;
        };
        let Ok(payload) = serde_json::from_str::<Value>(&contents) else {
            return;
        };
        for section in [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ] {
            if let Some(dependencies) = payload.get(section).and_then(Value::as_object) {
                for name in dependencies.keys() {
                    let detail = format!("{section} entry");
                    let node_id = self.node(
                        "package",
                        name.clone(),
                        format!("npm:{name}"),
                        None,
                        evidence(
                            "static-discovery",
                            Some(path.to_string_lossy().to_string()),
                            &detail,
                            observed_at,
                            None,
                        ),
                    );
                    let detail = format!("{section} entry for {name}");
                    self.edge(
                        &format!("connection-node:repository:{}", repository.id),
                        &node_id,
                        "dependency",
                        &format!("{section} dependency"),
                        "Medium",
                        evidence(
                            "static-discovery",
                            Some(path.to_string_lossy().to_string()),
                            &detail,
                            observed_at,
                            None,
                        ),
                    );
                }
            }
        }
        if let Some(scripts) = payload.get("scripts").and_then(Value::as_object) {
            for (name, value) in scripts {
                let Some(command) = value.as_str() else {
                    continue;
                };
                let detail = format!("package.json scripts.{name}");
                self.workflow(
                    format!("{} · {}", repository.name, name),
                    format!("workflow:{}:script:{name}", repository.id),
                    "repository-local",
                    vec![repository.id.clone()],
                    vec![WorkflowStep {
                        id: format!("workflow-step:{}:script:{name}:0", repository.id),
                        order: 0,
                        node_id: format!("connection-node:repository:{}", repository.id),
                        action_label: format!("Run {name}"),
                        command: Some(redacted_command(command)),
                        connection_id: None,
                        evidence: vec![evidence(
                            "static-discovery",
                            Some(path.to_string_lossy().to_string()),
                            &detail,
                            observed_at,
                            Some(command),
                        )],
                    }],
                    vec![evidence(
                        "static-discovery",
                        Some(path.to_string_lossy().to_string()),
                        &detail,
                        observed_at,
                        Some(command),
                    )],
                    observed_at,
                );
            }
        }
    }

    fn text_dependencies(
        &mut self,
        repository: &RepositorySnapshot,
        root: &Path,
        filename: &str,
        sections: &[&str],
        observed_at: &str,
    ) {
        let path = root.join(filename);
        let Some(contents) = read_bounded(&path) else {
            return;
        };
        let mut current_section = String::new();
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed.trim_matches(['[', ']']).to_ascii_lowercase();
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            let in_section = sections.iter().any(|section| {
                section.is_empty() || current_section.contains(&section.to_ascii_lowercase())
            });
            if !in_section {
                continue;
            }
            let candidate = trimmed
                .split_once('=')
                .map(|(name, _)| name.trim())
                .unwrap_or_else(|| trimmed.split_whitespace().next().unwrap_or(trimmed))
                .trim_matches(['"', '\'', ',', ' ']);
            if candidate.is_empty() || candidate.starts_with('[') || candidate.len() > 100 {
                continue;
            }
            let node_id = self.node(
                "package",
                candidate.to_string(),
                format!("dependency:{filename}:{candidate}"),
                None,
                evidence(
                    "static-discovery",
                    Some(path.to_string_lossy().to_string()),
                    "Dependency manifest entry",
                    observed_at,
                    None,
                ),
            );
            self.edge(
                &format!("connection-node:repository:{}", repository.id),
                &node_id,
                "dependency",
                "manifest dependency",
                "Medium",
                evidence(
                    "static-discovery",
                    Some(path.to_string_lossy().to_string()),
                    "Dependency manifest entry",
                    observed_at,
                    None,
                ),
            );
        }
    }

    fn ci_workflows(&mut self, repository: &RepositorySnapshot, root: &Path, observed_at: &str) {
        let workflow_root = root.join(".github").join("workflows");
        let Ok(entries) = fs::read_dir(&workflow_root) else {
            return;
        };
        let ci_node = self.node(
            "tool",
            "GitHub Actions".to_string(),
            "tool:github-actions".to_string(),
            None,
            evidence(
                "static-discovery",
                Some(workflow_root.to_string_lossy().to_string()),
                "GitHub Actions workflow directory",
                observed_at,
                None,
            ),
        );
        self.edge(
            &format!("connection-node:repository:{}", repository.id),
            &ci_node,
            "tool",
            "runs with",
            "High",
            evidence(
                "static-discovery",
                Some(workflow_root.to_string_lossy().to_string()),
                "GitHub Actions workflow directory",
                observed_at,
                None,
            ),
        );
        let Ok(entries) = entries.collect::<Result<Vec<_>, _>>() else {
            return;
        };
        for entry in entries {
            let path = entry.path();
            let extension = path.extension().and_then(|value| value.to_str());
            if !matches!(extension, Some("yml") | Some("yaml")) {
                continue;
            }
            let Some(contents) = read_bounded(&path) else {
                continue;
            };
            let commands = contents
                .lines()
                .filter_map(|line| line.trim().strip_prefix("run:"))
                .map(str::trim)
                .filter(|command| !command.is_empty() && *command != "|")
                .map(|command| command.trim_matches(['"', '\'']).to_string())
                .collect::<Vec<_>>();
            if commands.is_empty() {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("workflow");
            let steps = commands
                .iter()
                .enumerate()
                .map(|(order, command)| WorkflowStep {
                    id: format!("workflow-step:{}:ci:{name}:{order}", repository.id),
                    order: order as u32,
                    node_id: ci_node.clone(),
                    action_label: "Run CI command".to_string(),
                    command: Some(redacted_command(command)),
                    connection_id: Some(connection_fingerprint(
                        &format!("connection-node:repository:{}", repository.id),
                        &ci_node,
                        "tool",
                    )),
                    evidence: vec![evidence(
                        "static-discovery",
                        Some(path.to_string_lossy().to_string()),
                        "GitHub Actions run step",
                        observed_at,
                        Some(command),
                    )],
                })
                .collect();
            self.workflow(
                format!("{} · CI · {name}", repository.name),
                format!("workflow:{}:ci:{name}", repository.id),
                "repository-local",
                vec![repository.id.clone()],
                steps,
                vec![evidence(
                    "static-discovery",
                    Some(path.to_string_lossy().to_string()),
                    "GitHub Actions workflow",
                    observed_at,
                    None,
                )],
                observed_at,
            );
        }
    }

    fn deep_code(&mut self, repository: &RepositorySnapshot, observed_at: &str) {
        let root = PathBuf::from(&repository.path);
        let mut files = Vec::new();
        collect_files(&root, &mut files);
        for path in files.into_iter().take(MAX_DISCOVERY_FILES) {
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if !matches!(extension, "js" | "jsx" | "ts" | "tsx" | "rs" | "py") {
                if matches!(
                    extension,
                    "c" | "cc"
                        | "cpp"
                        | "cs"
                        | "go"
                        | "java"
                        | "kt"
                        | "kts"
                        | "m"
                        | "mm"
                        | "php"
                        | "rb"
                        | "swift"
                        | "scala"
                ) {
                    self.deep_unsupported_files += 1;
                }
                continue;
            }
            self.deep_supported_files += 1;
            let Some(contents) = read_bounded(&path) else {
                continue;
            };
            for module_name in module_references(&contents, extension) {
                let node_id = self.node(
                    "module",
                    module_name.clone(),
                    format!("module:{}:{module_name}", repository.id),
                    Some(repository.id.clone()),
                    evidence(
                        "deep-code",
                        Some(path.to_string_lossy().to_string()),
                        "Validated source import/reference",
                        observed_at,
                        None,
                    ),
                );
                self.edge(
                    &format!("connection-node:repository:{}", repository.id),
                    &node_id,
                    "dependency",
                    "imports module",
                    "Low",
                    evidence(
                        "deep-code",
                        Some(path.to_string_lossy().to_string()),
                        &format!("Source reference: {module_name}"),
                        observed_at,
                        None,
                    ),
                );
            }
        }
    }

    fn runtime(
        &mut self,
        repository: &RepositorySnapshot,
        remote_repositories: &[RemoteRepositorySnapshot],
        observed_at: &str,
    ) {
        let Some(remote_url) = repository.remote_url.as_deref() else {
            return;
        };
        let Some(remote) = remote_repositories
            .iter()
            .find(|remote| remote_matches(remote_url, remote))
        else {
            return;
        };
        let node_id = self.node(
            "service",
            format!("{} checks", remote.full_name),
            format!("provider-runtime:{}", remote.id),
            Some(repository.id.clone()),
            evidence(
                "runtime-provider",
                None,
                "Existing local provider snapshot; no network query performed",
                observed_at,
                None,
            ),
        );
        self.edge(
            &format!("connection-node:repository:{}", repository.id),
            &node_id,
            "runtime",
            "has provider runtime evidence",
            "Medium",
            evidence(
                "runtime-provider",
                None,
                "Existing local provider snapshot; no network query performed",
                observed_at,
                None,
            ),
        );
    }

    fn cross_repository_workflows(
        &mut self,
        repositories: &[&RepositorySnapshot],
        observed_at: &str,
    ) {
        let paths = repositories
            .iter()
            .map(|repository| (canonical_path(&repository.path), *repository))
            .collect::<HashMap<_, _>>();
        for repository in repositories {
            for submodule in &repository.submodules {
                let candidate = Path::new(&repository.path).join(&submodule.path);
                let Some(target) = paths.get(&canonical_path(&candidate)) else {
                    continue;
                };
                let source_node = format!("connection-node:repository:{}", repository.id);
                let target_node = format!("connection-node:repository:{}", target.id);
                let connection_id =
                    connection_fingerprint(&source_node, &target_node, "dependency");
                let evidence = evidence(
                    "static-discovery",
                    Some(
                        Path::new(&repository.path)
                            .join(".gitmodules")
                            .to_string_lossy()
                            .to_string(),
                    ),
                    "Registered repository matches a Git submodule path",
                    observed_at,
                    None,
                );
                self.edge(
                    &source_node,
                    &target_node,
                    "dependency",
                    "uses repository submodule",
                    "High",
                    evidence.clone(),
                );
                self.workflow(
                    format!("{} → {} · submodule handoff", repository.name, target.name),
                    format!("workflow:{}:submodule:{}", repository.id, target.id),
                    "cross-repository",
                    vec![repository.id.clone(), target.id.clone()],
                    vec![
                        WorkflowStep {
                            id: format!("workflow-step:{}:{}:0", repository.id, target.id),
                            order: 0,
                            node_id: source_node,
                            action_label: "Resolve submodule".to_string(),
                            command: None,
                            connection_id: Some(connection_id.clone()),
                            evidence: vec![evidence.clone()],
                        },
                        WorkflowStep {
                            id: format!("workflow-step:{}:{}:1", repository.id, target.id),
                            order: 1,
                            node_id: target_node,
                            action_label: "Use linked repository".to_string(),
                            command: None,
                            connection_id: Some(connection_id),
                            evidence: vec![evidence.clone()],
                        },
                    ],
                    vec![evidence],
                    observed_at,
                );
            }
        }
    }

    fn node(
        &mut self,
        kind: &str,
        label: String,
        identity: String,
        repository_id: Option<String>,
        item_evidence: ConnectionEvidence,
    ) -> String {
        let id = format!("connection-node:{identity}");
        self.nodes
            .entry(identity.clone())
            .and_modify(|node| {
                node.last_seen_at = Some(item_evidence.observed_at.clone());
                if !node.evidence.contains(&item_evidence) {
                    node.evidence.push(item_evidence.clone());
                }
            })
            .or_insert_with(|| ConnectionNode {
                id: id.clone(),
                kind: kind.to_string(),
                label,
                identity,
                repository_id,
                origin: "Discovered".to_string(),
                confidence: "Medium".to_string(),
                status: "Active".to_string(),
                label_override: None,
                kind_override: None,
                last_seen_at: Some(item_evidence.observed_at.clone()),
                evidence: vec![item_evidence],
            });
        id
    }

    fn edge(
        &mut self,
        source_node_id: &str,
        target_node_id: &str,
        relationship_type: &str,
        label: &str,
        confidence: &str,
        item_evidence: ConnectionEvidence,
    ) -> String {
        let fingerprint = connection_fingerprint(source_node_id, target_node_id, relationship_type);
        let id = format!("connection:{fingerprint}");
        self.connections
            .entry(fingerprint.clone())
            .and_modify(|connection| {
                connection.last_seen_at = Some(item_evidence.observed_at.clone());
                if !connection.evidence.contains(&item_evidence) {
                    connection.evidence.push(item_evidence.clone());
                }
            })
            .or_insert_with(|| Connection {
                id: id.clone(),
                fingerprint,
                source_node_id: source_node_id.to_string(),
                target_node_id: target_node_id.to_string(),
                relationship_type: relationship_type.to_string(),
                label: label.to_string(),
                origin: "Discovered".to_string(),
                review_state: "Suggested".to_string(),
                confidence: confidence.to_string(),
                status: "Active".to_string(),
                label_override: None,
                last_seen_at: Some(item_evidence.observed_at.clone()),
                evidence: vec![item_evidence],
            });
        id
    }

    fn workflow(
        &mut self,
        name: String,
        id: String,
        scope: &str,
        repositories: Vec<String>,
        steps: Vec<WorkflowStep>,
        item_evidence: Vec<ConnectionEvidence>,
        observed_at: &str,
    ) {
        self.workflows.insert(
            id.clone(),
            Workflow {
                id,
                name,
                scope: scope.to_string(),
                origin: "Discovered".to_string(),
                status: "Active".to_string(),
                review_state: "Suggested".to_string(),
                participating_repositories: repositories,
                name_override: None,
                last_seen_at: Some(observed_at.to_string()),
                evidence: item_evidence,
                steps,
            },
        );
    }

    fn finish(self) -> ConnectionsSnapshot {
        let mut snapshot = ConnectionsSnapshot {
            nodes: self.nodes.into_values().collect(),
            connections: self.connections.into_values().collect(),
            workflows: self.workflows.into_values().collect(),
            adapters: Vec::new(),
            generated_at: String::new(),
        };
        normalize_snapshot(&mut snapshot);
        snapshot
    }
}

fn merge_snapshot(
    previous: &ConnectionsSnapshot,
    discovered: ConnectionsSnapshot,
    target_repository_ids: Option<&HashSet<String>>,
    observed_at: &str,
) -> ConnectionsSnapshot {
    let mut result = previous.clone();
    let targeted = |repository_id: Option<&String>| {
        target_repository_ids
            .map(|targets| repository_id.is_some_and(|id| targets.contains(id)))
            .unwrap_or(true)
    };
    let mut previous_nodes = result
        .nodes
        .drain(..)
        .map(|node| (node.identity.clone(), node))
        .collect::<HashMap<_, _>>();
    for node in discovered.nodes {
        let key = node.identity.clone();
        if let Some(existing) = previous_nodes.get_mut(&key) {
            if existing.origin == "Manual" {
                existing.last_seen_at = node.last_seen_at.clone();
                for item in node.evidence {
                    if !existing.evidence.contains(&item) {
                        existing.evidence.push(item);
                    }
                }
                continue;
            }
            let hidden = existing.status == "Hidden";
            let label_override = existing.label_override.clone();
            let kind_override = existing.kind_override.clone();
            *existing = node;
            existing.status = if hidden { "Hidden" } else { "Active" }.to_string();
            existing.label_override = label_override;
            existing.kind_override = kind_override;
        } else {
            previous_nodes.insert(key, node);
        }
    }
    for node in previous_nodes.values_mut() {
        if node.origin != "Manual"
            && targeted(node.repository_id.as_ref())
            && node.last_seen_at.as_deref() != Some(observed_at)
        {
            node.status = if node.status == "Hidden" {
                "Hidden".to_string()
            } else {
                "Stale".to_string()
            };
        }
    }
    result.nodes = previous_nodes.into_values().collect();

    let mut previous_connections = result
        .connections
        .drain(..)
        .map(|connection| (connection.fingerprint.clone(), connection))
        .collect::<HashMap<_, _>>();
    for connection in discovered.connections {
        let key = connection.fingerprint.clone();
        if let Some(existing) = previous_connections.get_mut(&key) {
            let hidden = existing.status == "Hidden" || existing.review_state == "Hidden";
            let review_state = existing.review_state.clone();
            let label_override = existing.label_override.clone();
            *existing = connection;
            existing.status = if hidden { "Hidden" } else { "Active" }.to_string();
            existing.review_state = review_state;
            existing.label_override = label_override;
        } else {
            previous_connections.insert(key, connection);
        }
    }
    for connection in previous_connections.values_mut() {
        let repository_id = result
            .nodes
            .iter()
            .find(|node| node.id == connection.source_node_id)
            .and_then(|node| node.repository_id.as_ref());
        if connection.origin != "Manual"
            && targeted(repository_id)
            && connection.last_seen_at.as_deref() != Some(observed_at)
        {
            connection.status = if connection.review_state == "Hidden" {
                "Hidden".to_string()
            } else {
                "Stale".to_string()
            };
        }
    }
    result.connections = previous_connections.into_values().collect();

    let mut previous_workflows = result
        .workflows
        .drain(..)
        .map(|workflow| (workflow.id.clone(), workflow))
        .collect::<HashMap<_, _>>();
    for workflow in discovered.workflows {
        let key = workflow.id.clone();
        if let Some(existing) = previous_workflows.get_mut(&key) {
            let hidden = existing.status == "Hidden" || existing.review_state == "Hidden";
            let review_state = existing.review_state.clone();
            let name_override = existing.name_override.clone();
            *existing = workflow;
            existing.status = if hidden { "Hidden" } else { "Active" }.to_string();
            existing.review_state = review_state;
            existing.name_override = name_override;
        } else {
            previous_workflows.insert(key, workflow);
        }
    }
    for workflow in previous_workflows.values_mut() {
        let is_targeted = target_repository_ids
            .map(|targets| {
                workflow
                    .participating_repositories
                    .iter()
                    .any(|id| targets.contains(id))
            })
            .unwrap_or(true);
        if workflow.origin != "Manual"
            && is_targeted
            && workflow.last_seen_at.as_deref() != Some(observed_at)
        {
            workflow.status = if workflow.review_state == "Hidden" {
                "Hidden".to_string()
            } else {
                "Stale".to_string()
            };
        }
    }
    result.workflows = previous_workflows.into_values().collect();
    result.generated_at = observed_at.to_string();
    result.adapters = previous.adapters.clone();
    for adapter in &mut result.adapters {
        if adapter.id == "static-discovery" && adapter.enabled {
            adapter.freshness = "Fresh".to_string();
            adapter.last_run_at = Some(observed_at.to_string());
            adapter.failure_message = None;
        } else if adapter.id == "static-discovery" && !adapter.enabled {
            adapter.freshness = "Not enabled".to_string();
        } else if adapter.id == "deep-code" && adapter.enabled {
            adapter.freshness = "Fresh".to_string();
            adapter.last_run_at = Some(observed_at.to_string());
        } else if adapter.id == "runtime-provider" && adapter.enabled {
            adapter.freshness = "Existing local evidence only".to_string();
            adapter.last_run_at = Some(observed_at.to_string());
        }
    }
    normalize_snapshot(&mut result);
    result
}

pub fn manual_node(input: ConnectionNodeInput, observed_at: &str) -> ConnectionNode {
    let node_id = input
        .node_id
        .unwrap_or_else(|| next_manual_id("connection-node"));
    let identity = input
        .identity
        .unwrap_or_else(|| format!("manual:{node_id}"));
    ConnectionNode {
        id: node_id,
        kind: input.kind,
        label: input.label,
        identity,
        repository_id: input.repository_id,
        origin: "Manual".to_string(),
        confidence: "Confirmed".to_string(),
        status: "Active".to_string(),
        label_override: None,
        kind_override: None,
        last_seen_at: Some(observed_at.to_string()),
        evidence: vec![evidence(
            "manual",
            None,
            "Added locally by the user",
            observed_at,
            None,
        )],
    }
}

pub fn manual_connection(input: ConnectionInput, observed_at: &str) -> Connection {
    let id = input
        .connection_id
        .unwrap_or_else(|| next_manual_id("connection"));
    let fingerprint = format!("manual:{id}");
    Connection {
        id,
        fingerprint,
        source_node_id: input.source_node_id,
        target_node_id: input.target_node_id,
        relationship_type: input.relationship_type,
        label: input
            .label
            .unwrap_or_else(|| "Manual relationship".to_string()),
        origin: "Manual".to_string(),
        review_state: "Confirmed".to_string(),
        confidence: input.confidence.unwrap_or_else(|| "Confirmed".to_string()),
        status: "Active".to_string(),
        label_override: None,
        last_seen_at: Some(observed_at.to_string()),
        evidence: vec![evidence(
            "manual",
            None,
            "Added locally by the user",
            observed_at,
            None,
        )],
    }
}

pub fn manual_workflow(input: WorkflowInput, observed_at: &str) -> Workflow {
    let id = input
        .workflow_id
        .unwrap_or_else(|| next_manual_id("workflow"));
    Workflow {
        id: id.clone(),
        name: input.name,
        scope: input.scope,
        origin: "Manual".to_string(),
        status: "Active".to_string(),
        review_state: "Confirmed".to_string(),
        participating_repositories: input.repository_ids,
        name_override: None,
        last_seen_at: Some(observed_at.to_string()),
        evidence: vec![evidence(
            "manual",
            None,
            "Added locally by the user",
            observed_at,
            None,
        )],
        steps: input
            .steps
            .into_iter()
            .enumerate()
            .map(|(order, step)| WorkflowStep {
                id: format!("{id}:step:{order}"),
                order: order as u32,
                node_id: step.node_id,
                action_label: step.action_label,
                command: step.command.as_deref().map(redacted_command),
                connection_id: step.connection_id,
                evidence: vec![evidence(
                    "manual",
                    None,
                    "Added locally by the user",
                    observed_at,
                    step.command.as_deref(),
                )],
            })
            .collect(),
    }
}

pub fn find_node_mut<'a>(
    snapshot: &'a mut ConnectionsSnapshot,
    id: &str,
) -> Option<&'a mut ConnectionNode> {
    snapshot.nodes.iter_mut().find(|node| node.id == id)
}

pub fn find_connection_mut<'a>(
    snapshot: &'a mut ConnectionsSnapshot,
    id: &str,
) -> Option<&'a mut Connection> {
    snapshot
        .connections
        .iter_mut()
        .find(|connection| connection.id == id)
}

pub fn find_workflow_mut<'a>(
    snapshot: &'a mut ConnectionsSnapshot,
    id: &str,
) -> Option<&'a mut Workflow> {
    snapshot
        .workflows
        .iter_mut()
        .find(|workflow| workflow.id == id)
}

fn next_manual_id(prefix: &str) -> String {
    format!(
        "{prefix}:manual:{}:{}",
        std::process::id(),
        NEXT_MANUAL_CONNECTION_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn connection_fingerprint(source: &str, target: &str, relationship_type: &str) -> String {
    format!("{relationship_type}:{source}->{target}")
}

fn evidence(
    adapter: &str,
    source_path: Option<String>,
    detail: &str,
    observed_at: &str,
    command: Option<&str>,
) -> ConnectionEvidence {
    ConnectionEvidence {
        adapter: adapter.to_string(),
        source_path,
        detail: detail.to_string(),
        observed_at: observed_at.to_string(),
        freshness: "Fresh".to_string(),
        command: command.map(redacted_command),
    }
}

fn read_bounded(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_SOURCE_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn remote_label(remote: &str) -> String {
    safe_remote_identity(remote)
        .trim_end_matches('/')
        .rsplit_once('/')
        .map(|(_, name)| name.trim_end_matches(".git").to_string())
        .unwrap_or_else(|| safe_remote_identity(remote))
}

fn safe_remote_identity(remote: &str) -> String {
    let trimmed = remote.trim();
    let Some((scheme, remainder)) = trimmed.split_once("://") else {
        return trimmed.to_string();
    };
    let without_query = remainder
        .split_once(['?', '#'])
        .map(|(value, _)| value)
        .unwrap_or(remainder);
    let without_credentials = without_query
        .rsplit_once('@')
        .map(|(_, value)| value)
        .unwrap_or(without_query);
    format!("{scheme}://{without_credentials}")
}

fn remote_matches(local_url: &str, remote: &RemoteRepositorySnapshot) -> bool {
    let local_identity = remote_identity(local_url);
    [remote.full_name.as_str(), remote.html_url.as_str()]
        .iter()
        .map(|value| remote_identity(value))
        .any(|identity| !identity.is_empty() && identity == local_identity)
}

fn remote_identity(value: &str) -> String {
    let normalized = value.trim().trim_end_matches('/').to_ascii_lowercase();
    if let Some((_, path)) = normalized.split_once("github.com/") {
        return path.trim_end_matches(".git").to_string();
    }
    if let Some(path) = normalized.strip_prefix("git@github.com:") {
        return path.trim_end_matches(".git").to_string();
    }
    normalized.trim_end_matches(".git").to_string()
}

fn canonical_path(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    if files.len() >= MAX_DISCOVERY_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let Ok(entries) = entries.collect::<Result<Vec<_>, _>>() else {
        return;
    };
    for entry in entries {
        if files.len() >= MAX_DISCOVERY_FILES {
            return;
        }
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if name == ".git"
            || name == "node_modules"
            || name == "target"
            || name == "dist"
            || name == "build"
            || name == ".venv"
            || name == "venv"
        {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn module_references(contents: &str, extension: &str) -> Vec<String> {
    let mut modules = HashSet::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        let candidate = if matches!(extension, "js" | "jsx" | "ts" | "tsx") {
            trimmed
                .split_once(" from ")
                .map(|(_, value)| value)
                .or_else(|| trimmed.strip_prefix("import("))
                .or_else(|| trimmed.strip_prefix("require("))
        } else if extension == "rs" {
            trimmed.strip_prefix("use ")
        } else if extension == "py" {
            trimmed
                .strip_prefix("import ")
                .or_else(|| trimmed.strip_prefix("from "))
        } else {
            None
        };
        let Some(candidate) = candidate else {
            continue;
        };
        let module = candidate
            .trim_matches(['"', '\'', ';', '(', ')'])
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(';')
            .to_string();
        if !module.is_empty() && module.len() < 160 {
            modules.insert(module);
        }
    }
    let mut modules = modules.into_iter().collect::<Vec<_>>();
    modules.sort();
    modules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{RepositorySnapshot, WorkspaceActivity, WorkspaceSummary};
    use crate::quality::QualitySnapshot;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    fn fixture_directory() -> PathBuf {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pronto-connections-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("connection fixture should be creatable");
        path
    }

    fn fixture_repository(path: &Path) -> RepositorySnapshot {
        let workspace = WorkspaceSummary {
            id: "workspace-1".to_string(),
            path: path.to_string_lossy().to_string(),
            is_primary: true,
            branch: "main".to_string(),
            dirty: false,
            added: 0,
            removed: 0,
            line_totals_partial: false,
            sync_state: "Synced".to_string(),
            remote_freshness: "Local only".to_string(),
            ahead: 0,
            behind: 0,
            upstream: None,
            operation: None,
            last_commit: None,
            last_commit_at: None,
            last_activity_at: None,
            integration_state: "No integration target".to_string(),
            target_branch: None,
            target_confidence: "Unknown".to_string(),
            role: "Primary".to_string(),
            role_confidence: "High".to_string(),
            activity: WorkspaceActivity::default(),
        };
        RepositorySnapshot {
            id: "repository-1".to_string(),
            name: "fixture-repository".to_string(),
            path: path.to_string_lossy().to_string(),
            locality: "Local only".to_string(),
            lifecycle: "Active".to_string(),
            lifecycle_candidate: "Active".to_string(),
            remote_url: Some("https://github.com/example/fixture-repository.git".to_string()),
            provider_state: "Not connected".to_string(),
            branch: "main".to_string(),
            default_branch: Some("main".to_string()),
            workspace: workspace.clone(),
            workspaces: vec![workspace],
            branches: Vec::new(),
            submodules: Vec::new(),
            pull_requests: Vec::new(),
            releases: Vec::new(),
            quality: QualitySnapshot::default(),
            release_rule: None,
            release_recipe: None,
            confirmed_release_version: None,
            ai_permission: "Disabled".to_string(),
            conditions: Vec::new(),
            last_scan_at: "2026-07-26T12:00:00Z".to_string(),
            last_fetch_at: None,
            last_activity_at: None,
        }
    }

    #[test]
    fn redacts_secret_flags_and_assignments_without_resolving_values() {
        let command = "pnpm deploy --token super-secret TOKEN=literal SECRET=other --api-key=third AWS_SECRET_ACCESS_KEY=hidden --authorization bearer ghp_1234567890";
        let redacted = redacted_command(command);
        assert_eq!(
            redacted,
            "pnpm deploy --token [REDACTED] TOKEN=[REDACTED] SECRET=[REDACTED] --api-key=[REDACTED] AWS_SECRET_ACCESS_KEY=[REDACTED] --authorization [REDACTED] [REDACTED]"
        );
        assert!(!redacted.contains("super-secret"));
        assert!(!redacted.contains("literal"));
        assert!(!redacted.contains("hidden"));
        assert!(!redacted.contains("ghp_"));
    }

    #[test]
    fn static_discovery_deduplicates_nodes_and_preserves_command_evidence() {
        let root = fixture_directory();
        fs::create_dir_all(root.join(".github/workflows")).expect("workflow directory");
        fs::write(
            root.join("package.json"),
            r#"{"workspaces":["packages/*"],"dependencies":{"react":"^18.0.0"},"scripts":{"deploy":"pnpm deploy --token super-secret"}}"#,
        )
        .expect("package manifest");
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").expect("lockfile");
        fs::write(root.join("Dockerfile"), "FROM node:22\n").expect("dockerfile");
        fs::write(
            root.join(".github/workflows/ci.yml"),
            "steps:\n  - run: pnpm test --api-key=workflow-secret\n",
        )
        .expect("workflow manifest");
        let repository = fixture_repository(&root);
        let snapshot = refresh_snapshot(
            &ConnectionsSnapshot::default(),
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:01:00Z",
        );

        assert_eq!(
            snapshot
                .nodes
                .iter()
                .filter(|node| node.identity == "npm:react")
                .count(),
            1
        );
        assert!(snapshot
            .nodes
            .iter()
            .any(|node| node.identity == "workspace:repository-1:packages/*"));
        assert!(snapshot
            .nodes
            .iter()
            .any(|node| node.identity == "lockfile:repository-1:pnpm-lock.yaml"));
        assert!(snapshot
            .connections
            .iter()
            .any(|connection| connection.relationship_type == "dependency"));
        assert!(snapshot
            .connections
            .iter()
            .any(|connection| connection.relationship_type == "deployment"));
        let commands = snapshot
            .workflows
            .iter()
            .flat_map(|workflow| workflow.steps.iter())
            .filter_map(|step| step.command.as_deref())
            .collect::<Vec<_>>();
        assert!(commands
            .iter()
            .any(|command| command.contains("[REDACTED]")));
        assert!(commands.iter().all(|command| !command.contains("secret")));

        let refreshed = refresh_snapshot(
            &snapshot,
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:02:00Z",
        );
        let first_node_ids = snapshot
            .nodes
            .iter()
            .map(|node| node.identity.clone())
            .collect::<HashSet<_>>();
        let refreshed_node_ids = refreshed
            .nodes
            .iter()
            .map(|node| node.identity.clone())
            .collect::<HashSet<_>>();
        assert_eq!(first_node_ids, refreshed_node_ids);
        let first_fingerprints = snapshot
            .connections
            .iter()
            .map(|connection| connection.fingerprint.clone())
            .collect::<HashSet<_>>();
        let refreshed_fingerprints = refreshed
            .connections
            .iter()
            .map(|connection| connection.fingerprint.clone())
            .collect::<HashSet<_>>();
        assert_eq!(first_fingerprints, refreshed_fingerprints);
        fs::remove_dir_all(root).expect("connection fixture should be removable");
    }

    #[test]
    fn does_not_persist_credentials_from_remote_urls() {
        let root = fixture_directory();
        let mut repository = fixture_repository(&root);
        repository.remote_url =
            Some("https://deploy:super-secret@example.com/acme/fixture.git".to_string());
        let snapshot = refresh_snapshot(
            &ConnectionsSnapshot::default(),
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:02:30Z",
        );
        let remote = snapshot
            .nodes
            .iter()
            .find(|node| node.kind == "service")
            .expect("remote service node should be discovered");
        assert_eq!(
            remote.identity,
            "remote:https://example.com/acme/fixture.git"
        );
        assert_eq!(remote.label, "fixture");
        assert!(!serde_json::to_string(remote)
            .expect("remote node should serialize")
            .contains("super-secret"));
        fs::remove_dir_all(root).expect("connection fixture should be removable");
    }

    #[test]
    fn supports_multiple_relationship_types_and_stale_manual_records() {
        let mut builder = DiscoveryBuilder::default();
        let a = builder.node(
            "repository",
            "A".to_string(),
            "repository:a".to_string(),
            Some("a".to_string()),
            evidence("test", None, "fixture", "2026-07-26T12:00:00Z", None),
        );
        let b = builder.node(
            "service",
            "B".to_string(),
            "service:b".to_string(),
            None,
            evidence("test", None, "fixture", "2026-07-26T12:00:00Z", None),
        );
        let item_evidence = evidence("test", None, "fixture", "2026-07-26T12:00:00Z", None);
        builder.edge(
            &a,
            &b,
            "dependency",
            "depends",
            "High",
            item_evidence.clone(),
        );
        builder.edge(&a, &b, "runtime", "runs", "Medium", item_evidence);
        let snapshot = builder.finish();
        assert_eq!(snapshot.connections.len(), 2);
        assert_ne!(
            snapshot.connections[0].fingerprint,
            snapshot.connections[1].fingerprint
        );

        let root = fixture_directory();
        let repository = fixture_repository(&root);
        fs::write(
            root.join("package.json"),
            r#"{"dependencies":{"old":"1.0.0"}}"#,
        )
        .expect("package manifest");
        let first = refresh_snapshot(
            &ConnectionsSnapshot::default(),
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:03:00Z",
        );
        let manual_node = manual_node(
            ConnectionNodeInput {
                node_id: Some("manual:person".to_string()),
                kind: "person".to_string(),
                label: "Reviewer".to_string(),
                identity: Some("manual:person".to_string()),
                repository_id: None,
            },
            "2026-07-26T12:03:00Z",
        );
        let mut with_manual = first.clone();
        with_manual.nodes.push(manual_node.clone());
        with_manual.connections.push(manual_connection(
            ConnectionInput {
                connection_id: Some("manual:review".to_string()),
                source_node_id: manual_node.id.clone(),
                target_node_id: first
                    .nodes
                    .iter()
                    .find(|node| node.identity == "repository:repository-1")
                    .expect("repository node")
                    .id
                    .clone(),
                relationship_type: "handoff".to_string(),
                label: Some("Review handoff".to_string()),
                confidence: None,
            },
            "2026-07-26T12:03:00Z",
        ));
        fs::remove_file(root.join("package.json")).expect("package manifest should be removable");
        let second = refresh_snapshot(
            &with_manual,
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:04:00Z",
        );
        assert!(second
            .nodes
            .iter()
            .any(|node| node.identity == "npm:old" && node.status == "Stale"));
        assert!(second
            .nodes
            .iter()
            .any(|node| node.id == "manual:person" && node.status == "Active"));
        assert!(second
            .connections
            .iter()
            .any(|connection| connection.id == "manual:review" && connection.status == "Active"));
        fs::remove_dir_all(root).expect("connection fixture should be removable");
    }

    #[test]
    fn preserves_manual_nodes_when_identity_matches_discovery() {
        let root = fixture_directory();
        let repository = fixture_repository(&root);
        fs::write(
            root.join("package.json"),
            r#"{"dependencies":{"shared-package":"1.0.0"}}"#,
        )
        .expect("package manifest");
        let discovered = refresh_snapshot(
            &ConnectionsSnapshot::default(),
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:07:00Z",
        );
        let mut previous = discovered.clone();
        previous.nodes.push(manual_node(
            ConnectionNodeInput {
                node_id: Some("manual:shared-package".to_string()),
                kind: "service".to_string(),
                label: "Reviewed package".to_string(),
                identity: Some("npm:shared-package".to_string()),
                repository_id: None,
            },
            "2026-07-26T12:07:00Z",
        ));
        let refreshed = refresh_snapshot(
            &previous,
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:08:00Z",
        );
        let node = refreshed
            .nodes
            .iter()
            .find(|node| node.identity == "npm:shared-package")
            .expect("shared package node should remain");
        assert_eq!(node.id, "manual:shared-package");
        assert_eq!(node.origin, "Manual");
        assert_eq!(node.label, "Reviewed package");
        fs::remove_dir_all(root).expect("connection fixture should be removable");
    }

    #[test]
    fn deep_code_is_opt_in_and_unsupported_languages_are_not_analyzed() {
        let root = fixture_directory();
        fs::create_dir_all(root.join("src")).expect("source directory");
        fs::write(root.join("src/main.ts"), "import x from './local';\n")
            .expect("typescript fixture");
        fs::write(root.join("src/main.go"), "package main\nimport \"fmt\"\n").expect("go fixture");
        let repository = fixture_repository(&root);
        let default_snapshot = refresh_snapshot(
            &ConnectionsSnapshot::default(),
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:05:00Z",
        );
        assert!(!default_snapshot
            .nodes
            .iter()
            .any(|node| node.kind == "module"));
        let mut opted_in = default_snapshot.clone();
        opted_in
            .adapters
            .iter_mut()
            .find(|adapter| adapter.id == "deep-code")
            .expect("deep-code adapter")
            .enabled = true;
        let analyzed = refresh_snapshot(
            &opted_in,
            std::slice::from_ref(&repository),
            &[],
            None,
            "2026-07-26T12:06:00Z",
        );
        assert!(analyzed
            .nodes
            .iter()
            .any(|node| node.kind == "module" && node.label == "./local"));
        let deep_adapter = analyzed
            .adapters
            .iter()
            .find(|adapter| adapter.id == "deep-code")
            .expect("deep-code adapter");
        assert!(deep_adapter
            .failure_message
            .as_deref()
            .is_some_and(|message| message.contains("unsupported")));
        fs::remove_dir_all(root).expect("connection fixture should be removable");
    }
}
