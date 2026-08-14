use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CONFIG_SCHEMA: &str = "pronto-installed-runtime-parity/v1";
pub const INSTALL_SCHEMA: &str = "installed-runtime-build/v1";
pub const PROCESS_SCHEMA: &str = "installed-runtime-process/v1";
pub const CONFIG_RELATIVE_PATH: &str = ".pronto/installed-runtime-parity.json";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const DEFAULT_MAX_AGE_HOURS: i64 = 0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledRuntimeIssue {
    pub stage: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledRuntimeTargetSnapshot {
    pub id: String,
    pub label: String,
    pub status: String,
    pub source_revision: Option<String>,
    pub build_revision: Option<String>,
    pub process_id: Option<i32>,
    pub observed_at: Option<String>,
    pub issues: Vec<InstalledRuntimeIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledRuntimeSnapshot {
    pub schema_version: String,
    pub applicability: String,
    pub status: String,
    pub summary: String,
    pub config_path: Option<String>,
    pub targets: Vec<InstalledRuntimeTargetSnapshot>,
}

impl Default for InstalledRuntimeSnapshot {
    fn default() -> Self {
        Self {
            schema_version: "pronto-installed-runtime-parity-snapshot/v1".to_string(),
            applicability: "not_applicable".to_string(),
            status: "not_applicable".to_string(),
            summary: "No installed-runtime parity contract is configured.".to_string(),
            config_path: None,
            targets: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RuntimeParityConfig {
    schema_version: String,
    targets: Vec<RuntimeTargetConfig>,
}

#[derive(Debug, Deserialize)]
struct RuntimeTargetConfig {
    id: String,
    label: String,
    install_manifest: String,
    runtime_manifest: String,
    runtime_executable: String,
    #[serde(default = "default_max_age_hours")]
    max_age_hours: i64,
}

#[derive(Debug, Deserialize)]
struct InstallManifest {
    schema_version: String,
    source_revision: String,
    built_artifact_sha256: String,
    installed_artifact_sha256: String,
    installed_at: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    schema_version: String,
    source_revision: String,
    running_artifact_sha256: String,
    process_id: i32,
    started_at: String,
}

fn default_max_age_hours() -> i64 {
    DEFAULT_MAX_AGE_HOURS
}

fn read_bounded_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("manifest is not a regular file".to_string());
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(format!("manifest exceeds {MAX_MANIFEST_BYTES} bytes"));
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn read_owner_manifest<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err("manifest is not owner-only".to_string());
    }
    read_bounded_json(path)
}

fn expand_home_path(value: &str) -> Result<PathBuf, String> {
    if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_string())?;
        let suffix = value.strip_prefix("~/").unwrap_or("");
        return Ok(PathBuf::from(home).join(suffix));
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("manifest and executable paths must be absolute or start with ~/".to_string())
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_revision(value: &str) -> bool {
    (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn process_matches(process_id: i32, expected_executable: &Path) -> bool {
    if process_id <= 0 {
        return false;
    }
    let output = Command::new("/bin/ps")
        .args(["-p", &process_id.to_string(), "-o", "comm="])
        .output();
    let Ok(output) = output else { return false };
    output.status.success()
        && String::from_utf8_lossy(&output.stdout).trim() == expected_executable.to_string_lossy()
}

fn issue(stage: &str, status: &str, message: impl Into<String>) -> InstalledRuntimeIssue {
    InstalledRuntimeIssue {
        stage: stage.to_string(),
        status: status.to_string(),
        message: message.into(),
    }
}

fn evaluate_target_with_probe(
    config: &RuntimeTargetConfig,
    source_revision: Option<&str>,
    now: DateTime<Utc>,
    process_probe: &dyn Fn(i32, &Path) -> bool,
) -> InstalledRuntimeTargetSnapshot {
    let mut target = InstalledRuntimeTargetSnapshot {
        id: config.id.clone(),
        label: config.label.clone(),
        status: "unverifiable".to_string(),
        source_revision: source_revision.map(str::to_string),
        build_revision: None,
        process_id: None,
        observed_at: None,
        issues: Vec::new(),
    };
    let install_path = match expand_home_path(&config.install_manifest) {
        Ok(path) => path,
        Err(error) => {
            target.issues.push(issue("contract", "invalid", error));
            return target;
        }
    };
    let runtime_path = match expand_home_path(&config.runtime_manifest) {
        Ok(path) => path,
        Err(error) => {
            target.issues.push(issue("contract", "invalid", error));
            return target;
        }
    };
    let executable_path = match expand_home_path(&config.runtime_executable) {
        Ok(path) => path,
        Err(error) => {
            target.issues.push(issue("contract", "invalid", error));
            return target;
        }
    };

    let install: InstallManifest = match read_owner_manifest(&install_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            target.status = "not_installed".to_string();
            target.issues.push(issue(
                "install",
                "missing_or_invalid",
                format!("Installed build identity is unavailable: {error}"),
            ));
            return target;
        }
    };
    target.build_revision = Some(install.source_revision.clone());
    target.observed_at = Some(install.installed_at.clone());
    if install.schema_version != INSTALL_SCHEMA
        || !valid_digest(&install.built_artifact_sha256)
        || !valid_digest(&install.installed_artifact_sha256)
    {
        target.issues.push(issue(
            "install",
            "invalid",
            "Installed build identity has an unsupported schema or digest.",
        ));
        return target;
    }
    if !valid_revision(&install.source_revision) || source_revision.is_none() {
        target.issues.push(issue(
            "build",
            "unverifiable",
            "The packaged build does not contain a verifiable source revision.",
        ));
    } else if source_revision != Some(install.source_revision.as_str()) {
        target.issues.push(issue(
            "build",
            "build_stale",
            "Repository source is newer or different from the packaged build.",
        ));
    }
    if install.built_artifact_sha256 != install.installed_artifact_sha256 {
        target.issues.push(issue(
            "install",
            "install_stale",
            "The installed artifact does not match the packaged build.",
        ));
    }

    let runtime: RuntimeManifest = match read_owner_manifest(&runtime_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            target.status = if target.issues.is_empty() {
                "not_running".to_string()
            } else {
                target.issues[0].status.clone()
            };
            target.issues.push(issue(
                "runtime",
                "not_running",
                format!("Running process identity is unavailable: {error}"),
            ));
            return target;
        }
    };
    target.process_id = Some(runtime.process_id);
    target.observed_at = Some(runtime.started_at.clone());
    if runtime.schema_version != PROCESS_SCHEMA || !valid_digest(&runtime.running_artifact_sha256) {
        target.issues.push(issue(
            "runtime",
            "invalid",
            "Running process identity has an unsupported schema or digest.",
        ));
        return target;
    }
    if runtime.source_revision != install.source_revision
        || runtime.running_artifact_sha256 != install.installed_artifact_sha256
    {
        target.issues.push(issue(
            "runtime",
            "restart_required",
            "The running process does not match the installed artifact.",
        ));
    }
    if !process_probe(runtime.process_id, &executable_path) {
        target.issues.push(issue(
            "runtime",
            "not_running",
            "The recorded PID is not running the declared executable.",
        ));
    }
    match DateTime::parse_from_rfc3339(&runtime.started_at) {
        Ok(started_at)
            if config.max_age_hours > 0
                && now.signed_duration_since(started_at.with_timezone(&Utc))
                    > Duration::hours(config.max_age_hours) =>
        {
            target.issues.push(issue(
                "runtime",
                "evidence_stale",
                "The running-process identity is older than the contract freshness window.",
            ));
        }
        Err(_) => target.issues.push(issue(
            "runtime",
            "invalid",
            "The running-process timestamp is not RFC 3339.",
        )),
        _ => {}
    }

    target.status = target
        .issues
        .first()
        .map(|item| item.status.clone())
        .unwrap_or_else(|| "current".to_string());
    target
}

pub fn evaluate(repository_path: &Path, source_revision: Option<&str>) -> InstalledRuntimeSnapshot {
    evaluate_with(
        repository_path,
        source_revision,
        Utc::now(),
        &process_matches,
    )
}

fn evaluate_with(
    repository_path: &Path,
    source_revision: Option<&str>,
    now: DateTime<Utc>,
    process_probe: &dyn Fn(i32, &Path) -> bool,
) -> InstalledRuntimeSnapshot {
    let config_path = repository_path.join(CONFIG_RELATIVE_PATH);
    if !config_path.is_file() {
        return InstalledRuntimeSnapshot::default();
    }
    let config: RuntimeParityConfig = match read_bounded_json(&config_path) {
        Ok(config) => config,
        Err(error) => {
            return InstalledRuntimeSnapshot {
                applicability: "applicable".to_string(),
                status: "unverifiable".to_string(),
                summary: format!("Installed-runtime parity contract is invalid: {error}"),
                config_path: Some(CONFIG_RELATIVE_PATH.to_string()),
                ..InstalledRuntimeSnapshot::default()
            }
        }
    };
    if config.schema_version != CONFIG_SCHEMA || config.targets.is_empty() {
        return InstalledRuntimeSnapshot {
            applicability: "applicable".to_string(),
            status: "unverifiable".to_string(),
            summary: "Installed-runtime parity contract has an unsupported schema or no targets."
                .to_string(),
            config_path: Some(CONFIG_RELATIVE_PATH.to_string()),
            ..InstalledRuntimeSnapshot::default()
        };
    }
    let targets = config
        .targets
        .iter()
        .map(|target| evaluate_target_with_probe(target, source_revision, now, process_probe))
        .collect::<Vec<_>>();
    let status = if targets.iter().all(|target| target.status == "current") {
        "current"
    } else {
        "attention_required"
    };
    let issue_count = targets
        .iter()
        .map(|target| target.issues.len())
        .sum::<usize>();
    InstalledRuntimeSnapshot {
        schema_version: "pronto-installed-runtime-parity-snapshot/v1".to_string(),
        applicability: "applicable".to_string(),
        status: status.to_string(),
        summary: if issue_count == 0 {
            format!("{} installed runtime target(s) match source, build, install, and process identity.", targets.len())
        } else {
            format!("{issue_count} installed-runtime parity issue(s) require attention.")
        },
        config_path: Some(CONFIG_RELATIVE_PATH.to_string()),
        targets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "pronto-installed-runtime-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&path).expect("temp root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn revision(byte: char) -> String {
        std::iter::repeat_n(byte, 40).collect()
    }

    fn fixture(build: &str, installed: &str, running: &str) -> (TempRoot, PathBuf) {
        let root = TempRoot::new();
        let install = root.path().join("install.json");
        let runtime = root.path().join("runtime.json");
        let executable = root.path().join("daemon");
        fs::write(&executable, "fixture").expect("executable fixture");
        fs::create_dir_all(root.path().join("repo/.pronto")).expect("contract directory");
        fs::write(
            &install,
            serde_json::to_vec(&serde_json::json!({
                "schema_version": INSTALL_SCHEMA,
                "source_revision": revision('a'),
                "built_artifact_sha256": build,
                "installed_artifact_sha256": installed,
                "installed_at": "2026-08-14T04:00:00Z"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::set_permissions(&install, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(
            &runtime,
            serde_json::to_vec(&serde_json::json!({
                "schema_version": PROCESS_SCHEMA,
                "source_revision": revision('a'),
                "running_artifact_sha256": running,
                "process_id": 42,
                "started_at": "2026-08-14T04:30:00Z"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(
            root.path()
                .join("repo/.pronto/installed-runtime-parity.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": CONFIG_SCHEMA,
                "targets": [{
                    "id": "daemon",
                    "label": "Daemon",
                    "install_manifest": install,
                    "runtime_manifest": runtime,
                    "runtime_executable": executable,
                    "max_age_hours": 24
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        let repo = root.path().join("repo");
        (root, repo)
    }

    #[test]
    fn reports_current_only_when_all_four_identities_match() {
        let value = digest('a');
        let (_root, repo) = fixture(&value, &value, &value);
        let snapshot = evaluate_with(
            &repo,
            Some(&revision('a')),
            DateTime::parse_from_rfc3339("2026-08-14T05:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            &|_, _| true,
        );
        assert_eq!(snapshot.status, "current");
        assert_eq!(snapshot.targets[0].status, "current");
        assert!(snapshot.targets[0].issues.is_empty());
    }

    #[test]
    fn distinguishes_build_install_and_restart_lag() {
        let (_root, repo) = fixture(&digest('a'), &digest('b'), &digest('c'));
        let snapshot = evaluate_with(
            &repo,
            Some(&revision('b')),
            DateTime::parse_from_rfc3339("2026-08-14T05:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            &|_, _| true,
        );
        let statuses = snapshot.targets[0]
            .issues
            .iter()
            .map(|item| item.status.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            statuses,
            ["build_stale", "install_stale", "restart_required"]
        );
    }

    #[test]
    fn stale_or_reused_pid_is_not_reported_as_running() {
        let value = digest('a');
        let (_root, repo) = fixture(&value, &value, &value);
        let snapshot = evaluate_with(
            &repo,
            Some(&revision('a')),
            DateTime::parse_from_rfc3339("2026-08-14T05:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            &|_, _| false,
        );
        assert!(snapshot.targets[0]
            .issues
            .iter()
            .any(|item| item.status == "not_running"));
    }

    #[test]
    fn missing_contract_is_explicitly_not_applicable() {
        let root = TempRoot::new();
        let snapshot = evaluate(root.path(), Some("source-a"));
        assert_eq!(snapshot.status, "not_applicable");
        assert_eq!(snapshot.applicability, "not_applicable");
    }
}
