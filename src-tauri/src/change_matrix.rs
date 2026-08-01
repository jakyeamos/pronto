use crate::skills::SkillRecord;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "pronto-change-matrix/v1";
const MATRIX_SCHEMA: &str = "change-surface-matrix/v1";
const POINTER_SCHEMA: &str = "change-surface-pointer/v1";
const MAX_MATRIX_BYTES: u64 = 256 * 1024;
const REPOSITORY_LOCATIONS: [&str; 4] = [
    ".agents/change-surface-matrix.json",
    ".context/change-surface-matrix.json",
    "docs/change-surface-matrix.json",
    "change-surface-matrix.json",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeMatrixSurface {
    pub id: String,
    pub scope: String,
    pub path: Option<String>,
    pub owner: Option<String>,
    pub condition: Option<String>,
    pub status: String,
    pub operations: Vec<String>,
    pub validation: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeTopologyFact {
    pub surface: String,
    pub status: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeMatrixPrivacy {
    pub raw_code: bool,
    pub raw_diffs: bool,
    pub raw_transcripts: bool,
    pub credentials: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeMatrixReport {
    pub schema_version: String,
    pub generated_at: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub operation: Option<String>,
    pub status: String,
    pub matrix_path: Option<String>,
    pub expected_contract_location: String,
    pub score: Option<u8>,
    pub maturity_impact: String,
    pub gaps: Vec<String>,
    pub surfaces: Vec<ChangeMatrixSurface>,
    pub known_topology: Vec<ChangeTopologyFact>,
    pub privacy: ChangeMatrixPrivacy,
    pub writes_performed: bool,
}

pub fn inspect_repository(
    path: &Path,
    subject_id: &str,
    remote_url: Option<&str>,
    operation: Option<&str>,
) -> ChangeMatrixReport {
    let expected = path.join(REPOSITORY_LOCATIONS[0]);
    let matrix_path = REPOSITORY_LOCATIONS
        .iter()
        .map(|relative| path.join(relative))
        .find(|candidate| candidate.is_file());
    let mut topology = vec![ChangeTopologyFact {
        surface: "repository".into(),
        status: "observed".into(),
        evidence: path.display().to_string(),
    }];
    if let Some(remote_url) = remote_url {
        topology.push(ChangeTopologyFact {
            surface: "remote".into(),
            status: "observed".into(),
            evidence: remote_url.to_string(),
        });
    }
    inspect_path(
        matrix_path.as_deref(),
        &expected,
        "repository",
        subject_id,
        operation,
        topology,
    )
}

pub fn inspect_skill(skill: &SkillRecord, operation: Option<&str>) -> ChangeMatrixReport {
    let hosted = skill
        .sources
        .iter()
        .find(|source| source.hosted_in_jakye_agent_setup);
    let preferred = hosted.or_else(|| skill.sources.first());
    let expected = preferred
        .and_then(|source| Path::new(&source.path).parent())
        .map(|parent| parent.join("change-surface-matrix.json"))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".agents/skills")
                .join(&skill.id)
                .join("change-surface-matrix.json")
        });
    let matrix_path = skill
        .sources
        .iter()
        .filter_map(|source| Path::new(&source.path).parent())
        .map(|parent| parent.join("change-surface-matrix.json"))
        .find(|candidate| candidate.is_file());
    let mut topology = skill
        .sources
        .iter()
        .map(|source| ChangeTopologyFact {
            surface: source.root.clone(),
            status: if source.hosted_in_jakye_agent_setup {
                "hosted".into()
            } else {
                "observed".into()
            },
            evidence: format!("{} · sha256 {}", source.path, source.sha256),
        })
        .collect::<Vec<_>>();
    topology.push(ChangeTopologyFact {
        surface: "jakye-agent-setup".into(),
        status: if skill.hosted_in_jakye_agent_setup {
            "applicable".into()
        } else {
            "not_applicable".into()
        },
        evidence: format!(
            "hosted_in_jakye_agent_setup={}",
            skill.hosted_in_jakye_agent_setup
        ),
    });
    for (provider, state) in &skill.providers {
        topology.push(ChangeTopologyFact {
            surface: format!("provider:{provider}"),
            status: state.state.clone(),
            evidence: state.reason.clone(),
        });
    }
    inspect_path(
        matrix_path.as_deref(),
        &expected,
        "skill",
        &skill.id,
        operation,
        topology,
    )
}

fn inspect_path(
    matrix_path: Option<&Path>,
    expected: &Path,
    subject_kind: &str,
    subject_id: &str,
    operation: Option<&str>,
    known_topology: Vec<ChangeTopologyFact>,
) -> ChangeMatrixReport {
    let Some(path) = matrix_path else {
        return ChangeMatrixReport {
            schema_version: SCHEMA.into(),
            generated_at: Utc::now().to_rfc3339(),
            subject_kind: subject_kind.into(),
            subject_id: subject_id.into(),
            operation: operation.map(str::to_string),
            status: "missing".into(),
            matrix_path: None,
            expected_contract_location: expected.display().to_string(),
            score: Some(0),
            maturity_impact: "change_surface_coverage is reduced to 0/4 when no matrix or routing contract exists.".into(),
            gaps: vec!["No existing matrix is available to explain propagation.".into()],
            surfaces: Vec::new(),
            known_topology,
            privacy: privacy(),
            writes_performed: false,
        };
    };
    let result = read_and_assess(path, operation);
    match result {
        Ok((score, status, gaps, surfaces)) => ChangeMatrixReport {
            schema_version: SCHEMA.into(),
            generated_at: Utc::now().to_rfc3339(),
            subject_kind: subject_kind.into(),
            subject_id: subject_id.into(),
            operation: operation.map(str::to_string),
            status,
            matrix_path: Some(path.display().to_string()),
            expected_contract_location: expected.display().to_string(),
            score: Some(score),
            maturity_impact: format!(
                "Existing matrix evidence currently supports {score}/4 change-surface maturity."
            ),
            gaps,
            surfaces,
            known_topology,
            privacy: privacy(),
            writes_performed: false,
        },
        Err(error) => ChangeMatrixReport {
            schema_version: SCHEMA.into(),
            generated_at: Utc::now().to_rfc3339(),
            subject_kind: subject_kind.into(),
            subject_id: subject_id.into(),
            operation: operation.map(str::to_string),
            status: "unknown".into(),
            matrix_path: Some(path.display().to_string()),
            expected_contract_location: expected.display().to_string(),
            score: Some(2),
            maturity_impact:
                "Structured evidence exists but cannot be treated as validated or current.".into(),
            gaps: vec![error],
            surfaces: Vec::new(),
            known_topology,
            privacy: privacy(),
            writes_performed: false,
        },
    }
}

fn read_and_assess(
    path: &Path,
    operation: Option<&str>,
) -> Result<(u8, String, Vec<String>, Vec<ChangeMatrixSurface>), String> {
    read_and_assess_inner(path, operation, false)
}

fn read_and_assess_inner(
    path: &Path,
    operation: Option<&str>,
    followed_pointer: bool,
) -> Result<(u8, String, Vec<String>, Vec<ChangeMatrixSurface>), String> {
    if path.is_symlink() {
        return Err("Matrix path is a symlink and was not followed.".into());
    }
    let metadata = fs::metadata(path).map_err(|error| format!("Cannot inspect matrix: {error}"))?;
    if metadata.len() > MAX_MATRIX_BYTES {
        return Err("Matrix exceeds the bounded inspection size.".into());
    }
    let payload: Value = serde_json::from_str(
        &fs::read_to_string(path).map_err(|error| format!("Cannot read matrix: {error}"))?,
    )
    .map_err(|error| format!("Matrix is invalid JSON: {error}"))?;
    if payload.get("schema_version").and_then(Value::as_str) == Some(POINTER_SCHEMA) {
        if followed_pointer {
            return Err("Nested change-matrix pointers are not supported.".into());
        }
        let target = payload
            .get("artifact_path")
            .or_else(|| payload.get("target"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Change-matrix pointer target is missing.".to_string())?;
        let target = Path::new(target);
        let resolved = if target.is_absolute() {
            target.to_path_buf()
        } else {
            path.parent().unwrap_or_else(|| Path::new(".")).join(target)
        };
        return read_and_assess_inner(&resolved, operation, true);
    }
    if payload.get("schema_version").and_then(Value::as_str) != Some(MATRIX_SCHEMA) {
        return Err("Matrix schema_version is missing or unsupported.".into());
    }
    let mut gaps = Vec::new();
    if !payload
        .get("owner")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        gaps.push("Matrix owner is missing.".into());
    }
    if matrix_is_stale(payload.get("last_reviewed").and_then(Value::as_str)) {
        gaps.push("Matrix freshness is stale or unknown.".into());
    }
    let raw_surfaces = payload
        .get("surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| "Matrix surfaces are missing.".to_string())?;
    let mut surfaces = Vec::new();
    for (index, value) in raw_surfaces.iter().enumerate() {
        let Some(surface) = value.as_object() else {
            gaps.push(format!("Surface {} is invalid.", index + 1));
            continue;
        };
        let status = surface
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("applicable");
        let operations = string_array(surface.get("operations"));
        let validation = string_array(surface.get("validation"));
        if status == "not_applicable" {
            if !surface
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
                || string_array(surface.get("evidence")).is_empty()
            {
                gaps.push(format!(
                    "Surface {} has unsupported not_applicable status.",
                    index + 1
                ));
            }
        } else {
            if surface.get("owner").and_then(Value::as_str).is_none() {
                gaps.push(format!("Surface {} owner is missing.", index + 1));
            }
            if surface.get("condition").and_then(Value::as_str).is_none() {
                gaps.push(format!("Surface {} condition is missing.", index + 1));
            }
            if validation.is_empty() {
                gaps.push(format!("Surface {} validation is missing.", index + 1));
            }
            if !["add", "change", "remove"]
                .iter()
                .all(|operation| operations.iter().any(|item| item == operation))
            {
                gaps.push(format!(
                    "Surface {} does not cover add/change/remove.",
                    index + 1
                ));
            }
            if matches!(status, "unknown" | "unresolved" | "stale" | "contradictory") {
                gaps.push(format!("Surface {} remains {status}.", index + 1));
            }
        }
        if operation.is_none_or(|selected| operations.iter().any(|item| item == selected)) {
            surfaces.push(ChangeMatrixSurface {
                id: surface
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                scope: surface
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                path: surface
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                owner: surface
                    .get("owner")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                condition: surface
                    .get("condition")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                status: status.to_string(),
                operations,
                validation,
                reason: surface
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
    }
    if payload
        .get("unresolved_surfaces")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        gaps.push("Unresolved surfaces remain.".into());
    }
    if !gaps.is_empty() {
        return Ok((2, "incomplete".into(), gaps, surfaces));
    }
    let operation_evidence = payload.get("operation_evidence").and_then(Value::as_object);
    let exercised = ["add", "change", "remove"].iter().all(|operation| {
        operation_evidence
            .and_then(|items| items.get(*operation))
            .is_some_and(|item| {
                item.get("status").and_then(Value::as_str) == Some("passed")
                    && !string_array(item.get("evidence")).is_empty()
            })
    });
    if exercised {
        Ok((4, "available".into(), Vec::new(), surfaces))
    } else {
        Ok((
            3,
            "available".into(),
            vec!["Add/change/remove behavioral evidence is incomplete.".into()],
            surfaces,
        ))
    }
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn matrix_is_stale(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let reviewed = DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map(|value| {
                value
                    .and_hms_opt(0, 0, 0)
                    .expect("midnight is valid")
                    .and_utc()
            })
        });
    reviewed
        .map(|reviewed| Utc::now() - reviewed > Duration::days(90))
        .unwrap_or(true)
}

fn privacy() -> ChangeMatrixPrivacy {
    ChangeMatrixPrivacy {
        raw_code: false,
        raw_diffs: false,
        raw_transcripts: false,
        credentials: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "pronto-change-matrix-{label}-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn complete_matrix() -> Value {
        json!({
            "schema_version": MATRIX_SCHEMA,
            "subject": {"kind": "repository", "id": "fixture"},
            "owner": "fixture-owner",
            "last_reviewed": Utc::now().date_naive().to_string(),
            "surfaces": [
                {
                    "id": "source",
                    "scope": "local",
                    "path": "src/",
                    "owner": "fixture-owner",
                    "condition": "source changes",
                    "operations": ["add", "change", "remove"],
                    "validation": ["cargo test"],
                    "status": "applicable"
                },
                {
                    "id": "consumer",
                    "scope": "external",
                    "path": "consumer",
                    "owner": "consumer-owner",
                    "condition": "contract changes",
                    "operations": ["add", "change", "remove"],
                    "validation": ["consumer test"],
                    "status": "applicable"
                }
            ],
            "unresolved_surfaces": [],
            "operation_evidence": {
                "add": {"status": "passed", "evidence": ["fixtures/add.json"]},
                "change": {"status": "passed", "evidence": ["fixtures/change.json"]},
                "remove": {"status": "passed", "evidence": ["fixtures/remove.json"]}
            }
        })
    }

    #[test]
    fn missing_matrix_reports_expected_location_without_writing() {
        let root = fixture_root("missing");
        fs::create_dir_all(&root).expect("fixture root");
        let before = fs::read_dir(&root).expect("fixture root").count();
        let report = inspect_repository(&root, "fixture", None, Some("change"));
        let after = fs::read_dir(&root).expect("fixture root").count();
        assert_eq!(report.status, "missing");
        assert_eq!(report.score, Some(0));
        assert!(!report.writes_performed);
        assert_eq!(before, after);
        fs::remove_dir(&root).expect("remove fixture");
    }

    #[test]
    fn available_matrix_filters_operation_and_reports_privacy_contract() {
        let root = fixture_root("available");
        let matrix = root.join(REPOSITORY_LOCATIONS[0]);
        fs::create_dir_all(matrix.parent().expect("matrix parent")).expect("fixture root");
        fs::write(&matrix, complete_matrix().to_string()).expect("fixture matrix");

        let report = inspect_repository(&root, "fixture", None, Some("remove"));

        assert_eq!(report.status, "available");
        assert_eq!(report.score, Some(4));
        assert_eq!(report.surfaces.len(), 2);
        assert!(report
            .surfaces
            .iter()
            .all(|surface| surface.operations.contains(&"remove".to_string())));
        assert!(!report.privacy.raw_code);
        assert!(!report.privacy.raw_diffs);
        assert!(!report.writes_performed);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn validated_pointer_is_followed_without_synthesizing_a_matrix() {
        let root = fixture_root("pointer");
        let pointer = root.join(REPOSITORY_LOCATIONS[0]);
        let artifact = root.join("evidence/matrix.json");
        fs::create_dir_all(pointer.parent().expect("pointer parent")).expect("fixture root");
        fs::create_dir_all(artifact.parent().expect("artifact parent")).expect("fixture evidence");
        fs::write(
            &pointer,
            json!({"schema_version": POINTER_SCHEMA, "artifact_path": "../evidence/matrix.json"})
                .to_string(),
        )
        .expect("fixture pointer");
        fs::write(&artifact, complete_matrix().to_string()).expect("fixture matrix");

        let report = inspect_repository(&root, "fixture", None, None);

        assert_eq!(report.status, "available");
        assert_eq!(report.score, Some(4));
        assert_eq!(
            report.matrix_path.as_deref(),
            Some(pointer.to_string_lossy().as_ref())
        );
        assert!(!report.writes_performed);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
