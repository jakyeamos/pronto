use crate::core::RepositorySnapshot;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

pub const RELEASE_BOUNDARY_SCHEMA: &str = "quality-runner-release-boundary/v2";
pub const RELEASE_BOUNDARY_RELATIVE_PATH: &str = ".quality-runner/release-boundary.json";
const RELEASE_BOUNDARY_MATRIX_PATH: &str = ".agents/change-surface-matrix.json";
const RELEASE_BOUNDARY_MAX_AGE_DAYS: i64 = 7;
const REQUIRED_CHECK_IDS: [&str; 6] = [
    "source_provenance",
    "surface_classification",
    "tracked_public_content",
    "public_adapter_fixtures",
    "distribution_archives",
    "clean_room_install",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseBoundaryArtifact {
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseBoundaryCheck {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReleaseBoundarySnapshot {
    pub schema: Option<String>,
    pub status: String,
    pub freshness: String,
    pub generated_at: Option<String>,
    pub scanned_commit: Option<String>,
    pub scanned_branch: Option<String>,
    pub producer_version: Option<String>,
    pub report_path: Option<String>,
    pub matrix_path: Option<String>,
    pub matrix_sha256: Option<String>,
    pub artifacts: Vec<ReleaseBoundaryArtifact>,
    pub checks: Vec<ReleaseBoundaryCheck>,
    pub blocking_check_ids: Vec<String>,
    pub detail: String,
}

impl Default for ReleaseBoundarySnapshot {
    fn default() -> Self {
        Self {
            schema: None,
            status: "Missing".to_string(),
            freshness: "Unknown".to_string(),
            generated_at: None,
            scanned_commit: None,
            scanned_branch: None,
            producer_version: None,
            report_path: None,
            matrix_path: None,
            matrix_sha256: None,
            artifacts: Vec::new(),
            checks: Vec::new(),
            blocking_check_ids: vec!["receipt_missing".to_string()],
            detail: format!(
                "Run Quality Runner release-boundary to create {RELEASE_BOUNDARY_RELATIVE_PATH}."
            ),
        }
    }
}

impl ReleaseBoundarySnapshot {
    pub fn is_release_ready(&self) -> bool {
        self.status == "Passed"
            && self.freshness == "Fresh"
            && self.blocking_check_ids.is_empty()
            && self.checks.iter().all(|check| check.status == "passed")
    }
}

pub fn import_release_boundary(repository: &RepositorySnapshot) -> ReleaseBoundarySnapshot {
    let report_path = Path::new(&repository.path).join(RELEASE_BOUNDARY_RELATIVE_PATH);
    if !report_path.is_file() {
        return ReleaseBoundarySnapshot::default();
    }
    let report_label = RELEASE_BOUNDARY_RELATIVE_PATH.to_string();
    let contents = match fs::read_to_string(&report_path) {
        Ok(contents) => contents,
        Err(error) => {
            return invalid_snapshot(
                Some(report_label),
                format!("Release-boundary receipt could not be read: {error}"),
            );
        }
    };
    let payload = match serde_json::from_str::<Value>(&contents) {
        Ok(payload) => payload,
        Err(error) => {
            return invalid_snapshot(
                Some(report_label),
                format!("Release-boundary receipt is not valid JSON: {error}"),
            );
        }
    };
    let matrix_path = Path::new(&repository.path).join(RELEASE_BOUNDARY_MATRIX_PATH);
    let current_matrix_sha256 = sha256_path(&matrix_path);
    evaluate_receipt(
        &payload,
        Some(report_label),
        Some(Path::new(&repository.path)),
        repository.branch.as_str(),
        repository.workspace.last_commit.as_deref(),
        current_matrix_sha256.as_deref(),
        Utc::now(),
    )
}

fn evaluate_receipt(
    payload: &Value,
    report_path: Option<String>,
    repository_root: Option<&Path>,
    current_branch: &str,
    current_commit: Option<&str>,
    current_matrix_sha256: Option<&str>,
    now: DateTime<Utc>,
) -> ReleaseBoundarySnapshot {
    let schema = string_at(payload, &["schema"]);
    let generated_at = string_at(payload, &["generated_at"]);
    let scanned_branch = string_at(payload, &["repository", "branch"]);
    let scanned_commit = string_at(payload, &["repository", "head_sha"]);
    let producer_version = string_at(payload, &["producer", "version"]);
    let matrix_path = string_at(payload, &["matrix", "path"]);
    let matrix_sha256 = string_at(payload, &["matrix", "sha256"]);
    if schema.as_deref() != Some(RELEASE_BOUNDARY_SCHEMA) {
        let observed = schema.clone().unwrap_or_else(|| "missing".to_string());
        return ReleaseBoundarySnapshot {
            schema,
            status: "Audit required".to_string(),
            freshness: "Unknown".to_string(),
            generated_at,
            scanned_commit,
            scanned_branch,
            producer_version,
            report_path,
            matrix_path,
            matrix_sha256,
            blocking_check_ids: vec!["schema_version".to_string()],
            detail: format!(
                "Receipt schema {observed} is legacy or unsupported; regenerate {RELEASE_BOUNDARY_SCHEMA} evidence."
            ),
            ..ReleaseBoundarySnapshot::default()
        };
    }

    let checks = payload
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|check| {
            Some(ReleaseBoundaryCheck {
                id: string_at(check, &["id"])?,
                status: string_at(check, &["status"])?,
                reason: string_at(check, &["reason"]),
            })
        })
        .collect::<Vec<_>>();
    let artifacts = payload
        .get("artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artifact| {
            Some(ReleaseBoundaryArtifact {
                kind: string_at(artifact, &["kind"])?,
                path: string_at(artifact, &["path"])?,
                sha256: string_at(artifact, &["sha256"])?,
                size_bytes: artifact.get("size_bytes")?.as_u64()?,
            })
        })
        .collect::<Vec<_>>();
    let mut blocking = payload
        .get("blocking_check_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut validation_errors = Vec::new();
    if contains_absolute_path(payload) {
        validation_errors.push("receipt_contains_absolute_path");
    }
    let observed_checks = checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    if checks.len() != REQUIRED_CHECK_IDS.len()
        || REQUIRED_CHECK_IDS
            .iter()
            .any(|required| !observed_checks.contains(required))
    {
        validation_errors.push("required_checks_missing");
    }
    if checks.iter().any(|check| check.status != "passed") {
        validation_errors.push("checks_not_passed");
    }
    let artifact_kinds = artifacts
        .iter()
        .map(|artifact| artifact.kind.as_str())
        .collect::<BTreeSet<_>>();
    if artifacts.len() != 2 || artifact_kinds != BTreeSet::from(["sdist", "wheel"]) {
        validation_errors.push("artifact_set_invalid");
    }
    if artifacts.iter().any(|artifact| {
        !valid_relative_path(&artifact.path)
            || !valid_sha256(&artifact.sha256)
            || artifact.size_bytes == 0
    }) {
        validation_errors.push("artifact_evidence_invalid");
    }
    if repository_root.is_some_and(|root| !artifact_files_match(root, &artifacts)) {
        validation_errors.push("artifact_digest_mismatch");
    }
    if matrix_path.as_deref() != Some(RELEASE_BOUNDARY_MATRIX_PATH)
        || !matrix_sha256.as_deref().is_some_and(valid_sha256)
    {
        validation_errors.push("matrix_evidence_invalid");
    } else if current_matrix_sha256 != matrix_sha256.as_deref() {
        validation_errors.push("matrix_digest_mismatch");
    }
    if string_at(payload, &["status"]).as_deref() != Some("passed") || !blocking.is_empty() {
        validation_errors.push("producer_status_blocked");
    }
    if producer_version.as_deref().is_none_or(str::is_empty) {
        validation_errors.push("producer_version_missing");
    }
    if string_at(payload, &["producer", "name"]).as_deref() != Some("quality-runner") {
        validation_errors.push("producer_identity_invalid");
    }
    if payload
        .get("repository")
        .and_then(|repository| repository.get("dirty_path_count"))
        .and_then(Value::as_u64)
        != Some(0)
    {
        validation_errors.push("source_provenance_dirty");
    }
    let distribution_classes = payload
        .get("distribution_classes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if distribution_classes != BTreeSet::from(["local_only", "public_adapter", "public_core"]) {
        validation_errors.push("distribution_classes_invalid");
    }
    if !validation_errors.is_empty() {
        blocking.extend(validation_errors.into_iter().map(str::to_string));
    }
    blocking.sort();
    blocking.dedup();

    let freshness = receipt_freshness(
        generated_at.as_deref(),
        scanned_branch.as_deref(),
        scanned_commit.as_deref(),
        current_branch,
        current_commit,
        now,
    );
    let status = if blocking.is_empty() {
        "Passed"
    } else {
        "Blocked"
    };
    let detail = if status == "Passed" && freshness == "Fresh" {
        "The public-release receipt is current, exact-target, policy-matched, and passing."
            .to_string()
    } else if status == "Passed" {
        format!("The receipt checks passed, but its target evidence is {freshness}.")
    } else {
        format!(
            "The public-release receipt is blocked by: {}.",
            blocking.join(", ")
        )
    };
    ReleaseBoundarySnapshot {
        schema,
        status: status.to_string(),
        freshness,
        generated_at,
        scanned_commit,
        scanned_branch,
        producer_version,
        report_path,
        matrix_path,
        matrix_sha256,
        artifacts,
        checks,
        blocking_check_ids: blocking,
        detail,
    }
}

fn invalid_snapshot(report_path: Option<String>, detail: String) -> ReleaseBoundarySnapshot {
    ReleaseBoundarySnapshot {
        status: "Blocked".to_string(),
        report_path,
        blocking_check_ids: vec!["receipt_invalid".to_string()],
        detail,
        ..ReleaseBoundarySnapshot::default()
    }
}

fn receipt_freshness(
    generated_at: Option<&str>,
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
    current_branch: &str,
    current_commit: Option<&str>,
    now: DateTime<Utc>,
) -> String {
    let recent = generated_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|value| {
            let age = now.signed_duration_since(value.with_timezone(&Utc));
            age >= Duration::minutes(-5) && age <= Duration::days(RELEASE_BOUNDARY_MAX_AGE_DAYS)
        });
    if recent
        && scanned_branch == Some(current_branch)
        && current_commit.is_some()
        && scanned_commit == current_commit
    {
        "Fresh"
    } else if scanned_branch.is_some() || scanned_commit.is_some() {
        "Stale"
    } else {
        "Unknown"
    }
    .to_string()
}

pub fn project_for_target(
    snapshot: &mut ReleaseBoundarySnapshot,
    target_branch: &str,
    target_commit: &str,
) {
    snapshot.freshness = if snapshot.scanned_branch.as_deref() == Some(target_branch)
        && snapshot.scanned_commit.as_deref() == Some(target_commit)
        && snapshot
            .generated_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|value| {
                let age = Utc::now().signed_duration_since(value.with_timezone(&Utc));
                age >= Duration::minutes(-5) && age <= Duration::days(RELEASE_BOUNDARY_MAX_AGE_DAYS)
            }) {
        "Fresh"
    } else {
        "Stale"
    }
    .to_string();
}

fn sha256_path(path: &Path) -> Option<String> {
    let contents = fs::read(path).ok()?;
    let mut digest = Sha256::new();
    digest.update(contents);
    Some(format!("{:x}", digest.finalize()))
}

fn artifact_files_match(root: &Path, artifacts: &[ReleaseBoundaryArtifact]) -> bool {
    artifacts.iter().all(|artifact| {
        let path = root.join(&artifact.path);
        path.is_file()
            && path.metadata().ok().map(|metadata| metadata.len()) == Some(artifact.size_bytes)
            && sha256_path(&path).as_deref() == Some(artifact.sha256.as_str())
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute()
        && !value.is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn contains_absolute_path(value: &Value) -> bool {
    match value {
        Value::String(value) => Path::new(value).is_absolute(),
        Value::Array(values) => values.iter().any(contains_absolute_path),
        Value::Object(values) => values.values().any(contains_absolute_path),
        _ => false,
    }
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn receipt(schema: &str) -> Value {
        json!({
            "schema": schema,
            "status": "passed",
            "generated_at": Utc::now().to_rfc3339(),
            "producer": {"name": "quality-runner", "version": "0.6.0"},
            "repository": {"id": "fixture", "branch": "main", "head_sha": "a".repeat(40), "dirty_path_count": 0},
            "matrix": {"path": RELEASE_BOUNDARY_MATRIX_PATH, "sha256": "b".repeat(64)},
            "distribution_classes": ["local_only", "public_adapter", "public_core"],
            "artifacts": [
                {"kind": "wheel", "path": "fixture.whl", "sha256": "c".repeat(64), "size_bytes": 10},
                {"kind": "sdist", "path": "fixture.tar.gz", "sha256": "d".repeat(64), "size_bytes": 20}
            ],
            "checks": REQUIRED_CHECK_IDS.iter().map(|id| json!({"id": id, "status": "passed", "violations": []})).collect::<Vec<_>>(),
            "blocking_check_ids": []
        })
    }

    #[test]
    fn current_exact_receipt_is_release_ready() {
        let snapshot = evaluate_receipt(
            &receipt(RELEASE_BOUNDARY_SCHEMA),
            Some(RELEASE_BOUNDARY_RELATIVE_PATH.to_string()),
            None,
            "main",
            Some(&"a".repeat(40)),
            Some(&"b".repeat(64)),
            Utc::now(),
        );

        assert!(snapshot.is_release_ready());
        assert_eq!(snapshot.artifacts.len(), 2);
    }

    #[test]
    fn legacy_receipt_requires_audit() {
        let snapshot = evaluate_receipt(
            &receipt("quality-runner-release-boundary/v1"),
            None,
            None,
            "main",
            Some(&"a".repeat(40)),
            Some(&"b".repeat(64)),
            Utc::now(),
        );

        assert_eq!(snapshot.status, "Audit required");
        assert!(!snapshot.is_release_ready());
    }

    #[test]
    fn stale_target_and_policy_mismatch_cannot_pass() {
        let snapshot = evaluate_receipt(
            &receipt(RELEASE_BOUNDARY_SCHEMA),
            None,
            None,
            "dev",
            Some(&"e".repeat(40)),
            Some(&"f".repeat(64)),
            Utc::now(),
        );

        assert_eq!(snapshot.status, "Blocked");
        assert_eq!(snapshot.freshness, "Stale");
        assert!(snapshot
            .blocking_check_ids
            .contains(&"matrix_digest_mismatch".to_string()));
        assert!(!snapshot.is_release_ready());
    }

    #[test]
    fn absolute_paths_and_incomplete_checks_are_rejected() {
        let mut payload = receipt(RELEASE_BOUNDARY_SCHEMA);
        payload["artifacts"][0]["path"] = json!("/Users/operator/private.whl");
        payload["checks"] = json!([]);
        let snapshot = evaluate_receipt(
            &payload,
            None,
            None,
            "main",
            Some(&"a".repeat(40)),
            Some(&"b".repeat(64)),
            Utc::now(),
        );

        assert!(snapshot
            .blocking_check_ids
            .contains(&"receipt_contains_absolute_path".to_string()));
        assert!(snapshot
            .blocking_check_ids
            .contains(&"required_checks_missing".to_string()));
    }

    #[test]
    fn changed_or_missing_artifacts_invalidate_the_receipt() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pronto-release-boundary-artifacts-{}-{nonce}",
            std::process::id()
        ));
        let dist = root.join("dist");
        fs::create_dir_all(&dist).expect("artifact fixture should be writable");
        let wheel = dist.join("fixture.whl");
        let sdist = dist.join("fixture.tar.gz");
        fs::write(&wheel, b"wheel").expect("wheel fixture should be writable");
        fs::write(&sdist, b"sdist").expect("sdist fixture should be writable");
        let mut payload = receipt(RELEASE_BOUNDARY_SCHEMA);
        payload["artifacts"] = json!([
            {
                "kind": "wheel",
                "path": "dist/fixture.whl",
                "sha256": sha256_path(&wheel).expect("wheel digest"),
                "size_bytes": 5
            },
            {
                "kind": "sdist",
                "path": "dist/fixture.tar.gz",
                "sha256": sha256_path(&sdist).expect("sdist digest"),
                "size_bytes": 5
            }
        ]);

        let passing = evaluate_receipt(
            &payload,
            None,
            Some(&root),
            "main",
            Some(&"a".repeat(40)),
            Some(&"b".repeat(64)),
            Utc::now(),
        );
        assert!(passing.is_release_ready());

        fs::write(&wheel, b"tampered").expect("wheel fixture should be changeable");
        let tampered = evaluate_receipt(
            &payload,
            None,
            Some(&root),
            "main",
            Some(&"a".repeat(40)),
            Some(&"b".repeat(64)),
            Utc::now(),
        );
        assert!(tampered
            .blocking_check_ids
            .contains(&"artifact_digest_mismatch".to_string()));
        fs::remove_dir_all(root).expect("artifact fixture should be removable");
    }
}
