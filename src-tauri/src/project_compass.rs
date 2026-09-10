use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

const CONTRACT_RELATIVE_PATH: &str = ".project-compass/contract.json";

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
    #[serde(default)]
    pub family: Option<Value>,
    #[serde(default = "projection_unknown")]
    pub projection_status: String,
    #[serde(default)]
    pub modernization_assessment: Option<Value>,
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
            family: None,
            projection_status: projection_unknown(),
            modernization_assessment: None,
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

fn projection_unknown() -> String {
    "unavailable".to_string()
}

/// The Python producer owns validation, scoring, authority, and coverage semantics.
pub fn inspect(repository: &Path) -> ProjectCompassSummary {
    if !repository.join(CONTRACT_RELATIVE_PATH).exists() {
        return ProjectCompassSummary::default();
    }
    match crate::project_compass_bridge::invoke("family", repository, &[]) {
        Ok(payload) => {
            let mut summary = from_projection(payload, repository);
            let proposal = repository.join(".project-compass/modernization.json");
            if proposal.is_file() {
                summary.modernization_assessment = Some(
                    crate::project_compass_bridge::invoke("assess", repository,
                        &["--proposal".into(), proposal.to_string_lossy().into_owned()])
                    .unwrap_or_else(|error| serde_json::json!({"schema":"compass-modernization-assessment/v1",
                        "disposition":"insufficient_evidence", "reasons":[error], "execution_authority":false, "implementation_work":[]}))
                );
            }
            summary
        }
        Err(error) => invalid(format!("Compass producer unavailable: {error}")),
    }
}

pub fn from_projection(payload: Value, repository: &Path) -> ProjectCompassSummary {
    if payload.get("schema").and_then(Value::as_str) != Some("compass-family/v1") {
        return invalid("Unsupported Compass family projection; update the canonical helper");
    }
    let expected = repository
        .canonicalize()
        .unwrap_or_else(|_| repository.to_path_buf());
    if payload.pointer("/source/workspace").and_then(Value::as_str) != expected.to_str() {
        return invalid("Compass projection belongs to a different workspace");
    }
    let mut summary: ProjectCompassSummary =
        match serde_json::from_value(payload["root_summary"].clone()) {
            Ok(summary) => summary,
            Err(error) => return invalid(format!("Invalid producer root summary: {error}")),
        };
    summary.projection_status = payload["status"].as_str().unwrap_or("unknown").to_string();
    summary.family = Some(payload);
    summary
}

pub fn family_attention(summary: &ProjectCompassSummary) -> usize {
    let Some(family) = &summary.family else {
        return 0;
    };
    let nodes = family["nodes"].as_array().map_or(0, |nodes| {
        nodes
            .iter()
            .filter(|node| node["valid"] != true || node["status"] == "draft")
            .count()
    });
    let links = family["relationships"].as_array().map_or(0, |links| {
        links
            .iter()
            .filter(|link| link["valid"] != true || link["status"] != "aligned")
            .count()
    });
    let bootstrap = family["bootstrap"]["subsystems"].as_array().map_or(1, |items| {
        items.iter().filter(|item| item["status"] != "linked").count()
    });
    nodes + links + bootstrap + family["errors"].as_array().map_or(0, Vec::len)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../fixtures/compass/family.json")).unwrap()
    }
    #[test]
    fn consumes_canonical_family_without_promoting_draft_or_hiding_invalid_child() {
        let mut payload = fixture();
        payload["source"]["workspace"] = serde_json::json!("/fixture");
        let summary = from_projection(payload, Path::new("/fixture"));
        assert_eq!(summary.status, "Ready");
        assert_eq!(summary.mvp.progress_percent, Some(75));
        assert!(family_attention(&summary) >= 2);
        assert_eq!(
            summary.family.as_ref().unwrap()["nodes"][1]["authority"],
            "draft"
        );
    }
    #[test]
    fn rejects_unsupported_and_mismatched_projection() {
        let mut payload = fixture();
        payload["schema"] = serde_json::json!("compass-family/v99");
        assert_eq!(
            from_projection(payload, Path::new("/fixture")).status,
            "Invalid"
        );
        assert_eq!(
            from_projection(fixture(), Path::new("/different")).status,
            "Invalid"
        );
    }
    #[test]
    fn legacy_cached_summary_has_explicit_unavailable_projection() {
        let summary: ProjectCompassSummary =
            serde_json::from_value(fixture()["root_summary"].clone()).unwrap();
        assert_eq!(summary.projection_status, "unavailable");
        assert!(summary.family.is_none());
    }
}
