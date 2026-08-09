use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

const CONTRACT_RELATIVE_PATH: &str = ".project-compass/contract.json";
const MAX_CONTRACT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCompassTargetSummary {
    pub progress_percent: Option<u8>,
    #[serde(default)]
    pub scored_outcome_count: usize,
    #[serde(default)]
    pub covered_pillar_count: usize,
    #[serde(default)]
    pub total_pillar_count: usize,
    pub confidence: String,
    pub confidence_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCompassBlockerSummary {
    pub outcome_id: String,
    pub outcome_name: String,
    pub kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCompassDriftSummary {
    pub kind: String,
    pub summary: String,
    pub observed_at: String,
}

impl Default for ProjectCompassTargetSummary {
    fn default() -> Self {
        Self {
            progress_percent: None,
            scored_outcome_count: 0,
            covered_pillar_count: 0,
            total_pillar_count: 0,
            confidence: "unknown".to_string(),
            confidence_percent: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCompassSummary {
    pub status: String,
    pub contract_path: String,
    pub revision: Option<u64>,
    pub updated_at: Option<String>,
    pub project_name: Option<String>,
    pub identity: Option<String>,
    pub audience: Option<String>,
    pub mvp: ProjectCompassTargetSummary,
    pub complete_product: ProjectCompassTargetSummary,
    pub open_blockers: usize,
    pub open_drift: usize,
    #[serde(default)]
    pub open_blocker_items: Vec<ProjectCompassBlockerSummary>,
    #[serde(default)]
    pub open_drift_items: Vec<ProjectCompassDriftSummary>,
    pub error: Option<String>,
}

impl Default for ProjectCompassSummary {
    fn default() -> Self {
        Self {
            status: "Missing".to_string(),
            contract_path: CONTRACT_RELATIVE_PATH.to_string(),
            revision: None,
            updated_at: None,
            project_name: None,
            identity: None,
            audience: None,
            mvp: ProjectCompassTargetSummary::default(),
            complete_product: ProjectCompassTargetSummary::default(),
            open_blockers: 0,
            open_drift: 0,
            open_blocker_items: Vec::new(),
            open_drift_items: Vec::new(),
            error: None,
        }
    }
}

fn invalid(message: impl Into<String>) -> ProjectCompassSummary {
    ProjectCompassSummary {
        status: "Invalid".to_string(),
        error: Some(message.into()),
        ..ProjectCompassSummary::default()
    }
}

fn required_object<'a>(
    value: &'a Value,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{key} must be an object"))
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| format!("{key} must be a non-empty string"))
}

fn rounded_percent(value: f64) -> u8 {
    (value + 0.5).floor().clamp(0.0, 100.0) as u8
}

fn open_blocker_items(contract: &Value) -> Result<Vec<ProjectCompassBlockerSummary>, String> {
    let mut summaries = Vec::new();
    for (pillar_index, pillar) in contract
        .get("pillars")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        for (outcome_index, outcome) in pillar
            .get("outcomes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let path = format!("pillars[{pillar_index}].outcomes[{outcome_index}]");
            let outcome_id =
                required_string(outcome, "id").map_err(|error| format!("{path}.{error}"))?;
            let outcome_name =
                required_string(outcome, "name").map_err(|error| format!("{path}.{error}"))?;
            let blockers = outcome
                .get("blockers")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("{path}.blockers must be an array"))?;
            for (blocker_index, blocker) in blockers.iter().enumerate() {
                let blocker_path = format!("{path}.blockers[{blocker_index}]");
                summaries.push(ProjectCompassBlockerSummary {
                    outcome_id: outcome_id.clone(),
                    outcome_name: outcome_name.clone(),
                    kind: required_string(blocker, "kind")
                        .map_err(|error| format!("{blocker_path}.{error}"))?,
                    summary: required_string(blocker, "summary")
                        .map_err(|error| format!("{blocker_path}.{error}"))?,
                });
            }
        }
    }
    Ok(summaries)
}

fn open_drift_items(drift: &[Value]) -> Result<Vec<ProjectCompassDriftSummary>, String> {
    drift
        .iter()
        .enumerate()
        .filter(|(_, item)| item.get("status").and_then(Value::as_str) == Some("open"))
        .map(|(index, item)| {
            let path = format!("drift[{index}]");
            let observed_at =
                required_string(item, "observed_at").map_err(|error| format!("{path}.{error}"))?;
            if chrono::DateTime::parse_from_rfc3339(&observed_at).is_err() {
                return Err(format!("{path}.observed_at must be an RFC 3339 timestamp"));
            }
            Ok(ProjectCompassDriftSummary {
                kind: required_string(item, "kind").map_err(|error| format!("{path}.{error}"))?,
                summary: required_string(item, "summary")
                    .map_err(|error| format!("{path}.{error}"))?,
                observed_at,
            })
        })
        .collect()
}

fn target_summary(contract: &Value, target: &str) -> Result<ProjectCompassTargetSummary, String> {
    let pillars = contract
        .get("pillars")
        .and_then(Value::as_array)
        .filter(|pillars| !pillars.is_empty())
        .ok_or_else(|| "pillars must be a non-empty array".to_string())?;
    let total_pillar_count = pillars.len();
    let mut pillar_scores = Vec::new();
    let mut confidence_values = Vec::new();
    let mut scored_outcome_count = 0;
    let mut covered_pillar_count = 0;

    for (pillar_index, pillar) in pillars.iter().enumerate() {
        let outcomes = pillar
            .get("outcomes")
            .and_then(Value::as_array)
            .filter(|outcomes| !outcomes.is_empty())
            .ok_or_else(|| format!("pillars[{pillar_index}].outcomes must be a non-empty array"))?;
        let mut maturity_values = Vec::new();
        for (outcome_index, outcome) in outcomes.iter().enumerate() {
            let targets = outcome
                .get("targets")
                .and_then(Value::as_array)
                .filter(|targets| !targets.is_empty())
                .ok_or_else(|| {
                    format!("pillars[{pillar_index}].outcomes[{outcome_index}].targets must be a non-empty array")
                })?;
            if targets
                .iter()
                .any(|value| !matches!(value.as_str(), Some("mvp") | Some("complete_product")))
            {
                return Err(format!(
                    "pillars[{pillar_index}].outcomes[{outcome_index}].targets contains an invalid target"
                ));
            }
            if outcome.get("blockers").and_then(Value::as_array).is_none() {
                return Err(format!(
                    "pillars[{pillar_index}].outcomes[{outcome_index}].blockers must be an array"
                ));
            }
            if !targets.iter().any(|value| value.as_str() == Some(target)) {
                continue;
            }
            let maturity = outcome
                .get("maturity")
                .and_then(Value::as_u64)
                .filter(|value| matches!(value, 0 | 25 | 50 | 75 | 100))
                .ok_or_else(|| {
                    format!("pillars[{pillar_index}].outcomes[{outcome_index}].maturity is invalid")
                })?;
            let confidence = match outcome.get("confidence").and_then(Value::as_str) {
                Some("low") => 0.25,
                Some("medium") => 0.6,
                Some("high") => 1.0,
                _ => {
                    return Err(format!(
                        "pillars[{pillar_index}].outcomes[{outcome_index}].confidence is invalid"
                    ))
                }
            };
            maturity_values.push(maturity as f64);
            confidence_values.push(confidence);
            scored_outcome_count += 1;
        }
        if !maturity_values.is_empty() {
            covered_pillar_count += 1;
            pillar_scores.push(maturity_values.iter().sum::<f64>() / maturity_values.len() as f64);
        }
    }

    if pillar_scores.is_empty() {
        return Ok(ProjectCompassTargetSummary {
            scored_outcome_count,
            covered_pillar_count,
            total_pillar_count,
            ..ProjectCompassTargetSummary::default()
        });
    }
    let progress = rounded_percent(pillar_scores.iter().sum::<f64>() / pillar_scores.len() as f64);
    let confidence_percent = rounded_percent(
        100.0 * confidence_values.iter().sum::<f64>() / confidence_values.len() as f64,
    );
    let confidence = if confidence_percent >= 80 {
        "high"
    } else if confidence_percent >= 50 {
        "medium"
    } else {
        "low"
    };

    Ok(ProjectCompassTargetSummary {
        progress_percent: Some(progress),
        scored_outcome_count,
        covered_pillar_count,
        total_pillar_count,
        confidence: confidence.to_string(),
        confidence_percent,
    })
}

pub fn inspect(repository: &Path) -> ProjectCompassSummary {
    let contract_path = repository.join(CONTRACT_RELATIVE_PATH);
    let metadata = match fs::symlink_metadata(&contract_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ProjectCompassSummary::default()
        }
        Err(error) => return invalid(format!("cannot inspect contract: {error}")),
    };
    if metadata.file_type().is_symlink() {
        return invalid("contract must be a regular file inside the repository");
    }
    if !metadata.is_file() {
        return invalid("contract path is not a regular file");
    }
    if metadata.len() > MAX_CONTRACT_BYTES {
        return invalid("contract exceeds the 1 MiB read limit");
    }

    let bytes = match fs::read(&contract_path) {
        Ok(bytes) => bytes,
        Err(error) => return invalid(format!("cannot read contract: {error}")),
    };
    let contract: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => return invalid(format!("contract is not valid JSON: {error}")),
    };
    if contract.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return invalid("schema_version must be 1");
    }
    let revision = match contract.get("revision").and_then(Value::as_u64) {
        Some(revision) if revision > 0 => revision,
        _ => return invalid("revision must be an integer >= 1"),
    };
    let project = match required_object(&contract, "project") {
        Ok(project) => Value::Object(project.clone()),
        Err(error) => return invalid(error),
    };
    let project_name = match required_string(&project, "name") {
        Ok(value) => value,
        Err(error) => return invalid(error),
    };
    let identity = match required_string(&project, "identity") {
        Ok(value) => value,
        Err(error) => return invalid(error),
    };
    let audience = match required_string(&project, "audience") {
        Ok(value) => value,
        Err(error) => return invalid(error),
    };
    for field in ["core_loop", "north_star"] {
        if let Err(error) = required_string(&project, field) {
            return invalid(error);
        }
    }
    if project.get("not_this").and_then(Value::as_array).is_none() {
        return invalid("project.not_this must be an array");
    }
    let targets = match required_object(&contract, "targets") {
        Ok(targets) => Value::Object(targets.clone()),
        Err(error) => return invalid(error),
    };
    for target in ["mvp", "complete_product"] {
        let target_value = match required_object(&targets, target) {
            Ok(value) => Value::Object(value.clone()),
            Err(error) => return invalid(error),
        };
        if let Err(error) = required_string(&target_value, "definition") {
            return invalid(format!("targets.{target}.{error}"));
        }
    }
    let source_layers = match required_object(&contract, "source_layers") {
        Ok(source_layers) => source_layers,
        Err(error) => return invalid(error),
    };
    for layer in ["intended", "planned", "implemented", "verified"] {
        if source_layers.get(layer).and_then(Value::as_array).is_none() {
            return invalid(format!("source_layers.{layer} must be an array"));
        }
    }
    let drift = match contract.get("drift").and_then(Value::as_array) {
        Some(drift) => drift,
        None => return invalid("drift must be an array"),
    };
    if drift.iter().any(|item| {
        !matches!(
            item.get("status").and_then(Value::as_str),
            Some("open") | Some("accepted") | Some("resolved") | Some("superseded")
        )
    }) {
        return invalid("drift contains an invalid status");
    }
    let updated_at = match required_string(&contract, "updated_at") {
        Ok(value) => value,
        Err(error) => return invalid(error),
    };
    if chrono::DateTime::parse_from_rfc3339(&updated_at).is_err() {
        return invalid("updated_at must be an RFC 3339 timestamp");
    }
    let mvp = match target_summary(&contract, "mvp") {
        Ok(summary) => summary,
        Err(error) => return invalid(error),
    };
    let complete_product = match target_summary(&contract, "complete_product") {
        Ok(summary) => summary,
        Err(error) => return invalid(error),
    };
    let open_blocker_items = match open_blocker_items(&contract) {
        Ok(items) => items,
        Err(error) => return invalid(error),
    };
    let open_drift_items = match open_drift_items(drift) {
        Ok(items) => items,
        Err(error) => return invalid(error),
    };
    let open_blockers = open_blocker_items.len();
    let open_drift = open_drift_items.len();

    ProjectCompassSummary {
        status: "Ready".to_string(),
        contract_path: CONTRACT_RELATIVE_PATH.to_string(),
        revision: Some(revision),
        updated_at: Some(updated_at),
        project_name: Some(project_name),
        identity: Some(identity),
        audience: Some(audience),
        mvp,
        complete_product,
        open_blockers,
        open_drift,
        open_blocker_items,
        open_drift_items,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn fixture_root() -> std::path::PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-project-compass-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(root.join(".project-compass")).expect("fixture should be creatable");
        root
    }

    #[test]
    fn missing_contract_is_visible_without_becoming_invalid() {
        let root = fixture_root();
        let summary = inspect(&root);
        assert_eq!(summary.status, "Missing");
        assert_eq!(summary.contract_path, CONTRACT_RELATIVE_PATH);
        fs::remove_dir_all(root).expect("fixture should be removable");
    }

    #[test]
    fn scores_each_target_by_pillar_and_reports_drift() {
        let root = fixture_root();
        let contract = serde_json::json!({
            "schema_version": 1,
            "revision": 3,
            "project": {
                "name": "Soundscape",
                "identity": "A social music home",
                "audience": "Close friends",
                "core_loop": "Share and react to music",
                "north_star": "Meaningful musical connection",
                "not_this": ["A generic streaming service"]
            },
            "targets": {
                "mvp": {"definition": "A coherent first social loop"},
                "complete_product": {"definition": "The ratified full experience"}
            },
            "pillars": [
                {
                    "outcomes": [
                        {"id": "social-loop", "name": "The social loop works", "targets": ["mvp", "complete_product"], "maturity": 75, "confidence": "high", "blockers": []},
                        {"id": "real-use-proof", "name": "The loop is proven in real use", "targets": ["mvp"], "maturity": 25, "confidence": "medium", "blockers": [{"kind": "verification", "summary": "Needs proof"}]}
                    ]
                },
                {
                    "outcomes": [
                        {"id": "shared-home", "name": "Friends share a home", "targets": ["mvp", "complete_product"], "maturity": 100, "confidence": "high", "blockers": []}
                    ]
                }
            ],
            "drift": [
                {"kind": "verification-gap", "summary": "The intended loop lacks real-use proof.", "status": "open", "observed_at": "2026-07-28T00:00:00Z", "evidence": []},
                {"kind": "intent-change", "summary": "A broader audience was accepted.", "status": "accepted", "observed_at": "2026-07-28T00:00:00Z", "evidence": []}
            ],
            "source_layers": {
                "intended": [],
                "planned": [],
                "implemented": [],
                "verified": []
            },
            "updated_at": "2026-07-28T00:00:00Z"
        });
        fs::write(
            root.join(CONTRACT_RELATIVE_PATH),
            serde_json::to_vec_pretty(&contract).expect("contract should serialize"),
        )
        .expect("contract should be writable");

        let summary = inspect(&root);
        assert_eq!(summary.status, "Ready");
        assert_eq!(summary.mvp.progress_percent, Some(75));
        assert_eq!(summary.mvp.scored_outcome_count, 3);
        assert_eq!(summary.mvp.covered_pillar_count, 2);
        assert_eq!(summary.mvp.total_pillar_count, 2);
        assert_eq!(summary.complete_product.progress_percent, Some(88));
        assert_eq!(summary.complete_product.scored_outcome_count, 2);
        assert_eq!(summary.complete_product.covered_pillar_count, 2);
        assert_eq!(summary.complete_product.total_pillar_count, 2);
        assert_eq!(summary.mvp.confidence_percent, 87);
        assert_eq!(summary.open_blockers, 1);
        assert_eq!(summary.open_drift, 1);
        assert_eq!(summary.open_blocker_items[0].summary, "Needs proof");
        assert_eq!(
            summary.open_drift_items[0].summary,
            "The intended loop lacks real-use proof."
        );
        fs::remove_dir_all(root).expect("fixture should be removable");
    }

    #[test]
    fn invalid_contract_fails_closed() {
        let root = fixture_root();
        fs::write(root.join(CONTRACT_RELATIVE_PATH), b"{not json")
            .expect("contract should be writable");
        let summary = inspect(&root);
        assert_eq!(summary.status, "Invalid");
        assert!(summary.error.is_some());
        fs::remove_dir_all(root).expect("fixture should be removable");
    }
}
