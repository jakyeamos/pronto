use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::core::RepositorySnapshot;

const CONTRACT_RELATIVE_PATH: &str = ".pronto/showcase-goal.json";
const CONTRACT_SCHEMA: &str = "pronto-showcase-goal/v2";
const SNAPSHOT_SCHEMA: &str = "pronto-showcase/v2";
const MAX_CONTRACT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowcaseScoring {
    pub product_weight: f64,
    pub materials_weight: f64,
    pub priority_career_weight: f64,
    pub priority_product_weight: f64,
    pub priority_materials_gap_weight: f64,
    pub publishable_product_minimum: f64,
    pub publishable_materials_minimum: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowcaseDimension {
    pub status: String,
    pub score: Option<f64>,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowcaseContractEntry {
    pub repository_name: String,
    pub display_name: String,
    pub public_eligibility: String,
    pub disposition_source: String,
    #[serde(default)]
    pub work_disposition: Option<String>,
    #[serde(default)]
    pub work_disposition_summary: Option<String>,
    #[serde(default)]
    pub next_step_category: Option<String>,
    pub product_readiness: ShowcaseDimension,
    pub demo_materials: ShowcaseDimension,
    pub career_signal: ShowcaseDimension,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub missing_materials: Vec<String>,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ShowcaseContract {
    schema_version: String,
    target_publishable_demo_count: usize,
    reviewed_at: String,
    quality_bar_source: String,
    scoring: ShowcaseScoring,
    projects: Vec<ShowcaseContractEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowcaseProjectSnapshot {
    pub repository_name: String,
    pub display_name: String,
    pub repository_id: Option<String>,
    pub repository_path: Option<String>,
    pub registration_status: String,
    pub public_eligibility: String,
    pub disposition_source: String,
    pub work_disposition: String,
    pub work_disposition_summary: String,
    pub next_step_category: String,
    pub product_readiness: ShowcaseDimension,
    pub demo_materials: ShowcaseDimension,
    pub career_signal: ShowcaseDimension,
    pub showcase_score: Option<f64>,
    pub priority_score: Option<f64>,
    pub lane: String,
    pub publishable: bool,
    pub blockers: Vec<String>,
    pub missing_materials: Vec<String>,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowcaseGoalSnapshot {
    pub target_publishable_demo_count: usize,
    pub publishable_demo_count: usize,
    pub remaining_demo_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowcasePortfolioSnapshot {
    pub schema_version: String,
    pub status: String,
    pub contract_path: String,
    pub reviewed_at: Option<String>,
    pub quality_bar_source: Option<String>,
    pub goal: ShowcaseGoalSnapshot,
    pub scoring: Option<ShowcaseScoring>,
    pub public_queue: Vec<String>,
    pub private_client_count: usize,
    pub projects: Vec<ShowcaseProjectSnapshot>,
    pub error: Option<String>,
}

impl Default for ShowcasePortfolioSnapshot {
    fn default() -> Self {
        Self {
            schema_version: SNAPSHOT_SCHEMA.to_string(),
            status: "Missing".to_string(),
            contract_path: CONTRACT_RELATIVE_PATH.to_string(),
            reviewed_at: None,
            quality_bar_source: None,
            goal: ShowcaseGoalSnapshot {
                target_publishable_demo_count: 0,
                publishable_demo_count: 0,
                remaining_demo_count: 0,
                status: "Not configured".to_string(),
            },
            scoring: None,
            public_queue: Vec::new(),
            private_client_count: 0,
            projects: Vec::new(),
            error: None,
        }
    }
}

fn invalid(message: impl Into<String>) -> ShowcasePortfolioSnapshot {
    ShowcasePortfolioSnapshot {
        status: "Invalid".to_string(),
        error: Some(message.into()),
        ..ShowcasePortfolioSnapshot::default()
    }
}

fn rounded(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn validate_weight_group(values: &[f64], label: &str) -> Result<(), String> {
    if values.iter().any(|value| !(0.0..=1.0).contains(value)) {
        return Err(format!("{label} weights must each be between 0 and 1"));
    }
    let sum = values.iter().sum::<f64>();
    if (sum - 1.0).abs() > 0.000_001 {
        return Err(format!("{label} weights must sum to 1"));
    }
    Ok(())
}

fn validate_dimension(dimension: &ShowcaseDimension, path: &str) -> Result<(), String> {
    match dimension.status.as_str() {
        "assessed" => {
            let score = dimension
                .score
                .ok_or_else(|| format!("{path}.score is required when status is assessed"))?;
            if !(0.0..=5.0).contains(&score) {
                return Err(format!("{path}.score must be between 0 and 5"));
            }
        }
        "unknown" | "blocked" | "not_applicable" => {
            if dimension.score.is_some() {
                return Err(format!(
                    "{path}.score must be omitted when status is {}",
                    dimension.status
                ));
            }
        }
        _ => {
            return Err(format!(
                "{path}.status must be assessed, unknown, blocked, or not_applicable"
            ));
        }
    }
    if dimension.evidence.trim().is_empty() {
        return Err(format!("{path}.evidence must be non-empty"));
    }
    Ok(())
}

fn validate_contract(contract: &ShowcaseContract) -> Result<(), String> {
    if contract.schema_version != CONTRACT_SCHEMA {
        return Err(format!(
            "schema_version must be {CONTRACT_SCHEMA}, found {}",
            contract.schema_version
        ));
    }
    if contract.target_publishable_demo_count == 0 {
        return Err("target_publishable_demo_count must be greater than zero".to_string());
    }
    if chrono::DateTime::parse_from_rfc3339(&contract.reviewed_at).is_err() {
        return Err("reviewed_at must be an RFC 3339 timestamp".to_string());
    }
    if contract.quality_bar_source.trim().is_empty() {
        return Err("quality_bar_source must be non-empty".to_string());
    }
    validate_weight_group(
        &[
            contract.scoring.product_weight,
            contract.scoring.materials_weight,
        ],
        "showcase",
    )?;
    validate_weight_group(
        &[
            contract.scoring.priority_career_weight,
            contract.scoring.priority_product_weight,
            contract.scoring.priority_materials_gap_weight,
        ],
        "priority",
    )?;
    for (field, value) in [
        (
            "publishable_product_minimum",
            contract.scoring.publishable_product_minimum,
        ),
        (
            "publishable_materials_minimum",
            contract.scoring.publishable_materials_minimum,
        ),
    ] {
        if !(0.0..=5.0).contains(&value) {
            return Err(format!("scoring.{field} must be between 0 and 5"));
        }
    }
    let mut names = HashSet::new();
    for (index, project) in contract.projects.iter().enumerate() {
        let path = format!("projects[{index}]");
        if project.repository_name.trim().is_empty() || project.display_name.trim().is_empty() {
            return Err(format!(
                "{path}.repository_name and display_name must be non-empty"
            ));
        }
        if !names.insert(project.repository_name.to_lowercase()) {
            return Err(format!(
                "duplicate repository_name {}",
                project.repository_name
            ));
        }
        if !matches!(
            project.public_eligibility.as_str(),
            "public_showcase" | "private_client" | "not_applicable" | "blocked" | "unknown"
        ) {
            return Err(format!(
                "{path}.public_eligibility must be public_showcase, private_client, not_applicable, blocked, or unknown"
            ));
        }
        if project.disposition_source.trim().is_empty() || project.next_step.trim().is_empty() {
            return Err(format!(
                "{path}.disposition_source and next_step must be non-empty"
            ));
        }
        if project.public_eligibility == "public_showcase" {
            let disposition = project.work_disposition.as_deref().ok_or_else(|| {
                format!("{path}.work_disposition is required for public_showcase projects")
            })?;
            if !matches!(
                disposition,
                "largely_product_ready"
                    | "targeted_gap_closure"
                    | "material_build_or_restoration"
                    | "conditional_gate"
            ) {
                return Err(format!(
                    "{path}.work_disposition must be largely_product_ready, targeted_gap_closure, material_build_or_restoration, or conditional_gate"
                ));
            }
            if project
                .work_disposition_summary
                .as_deref()
                .is_none_or(|summary| summary.trim().is_empty())
            {
                return Err(format!(
                    "{path}.work_disposition_summary is required for public_showcase projects"
                ));
            }
            let category = project.next_step_category.as_deref().ok_or_else(|| {
                format!("{path}.next_step_category is required for public_showcase projects")
            })?;
            if !matches!(
                category,
                "product" | "demo_integration" | "evidence" | "content" | "packaging"
            ) {
                return Err(format!(
                    "{path}.next_step_category must be product, demo_integration, evidence, content, or packaging"
                ));
            }
        }
        validate_dimension(
            &project.product_readiness,
            &format!("{path}.product_readiness"),
        )?;
        validate_dimension(&project.demo_materials, &format!("{path}.demo_materials"))?;
        validate_dimension(&project.career_signal, &format!("{path}.career_signal"))?;
    }
    Ok(())
}

fn score(project: &ShowcaseContractEntry, scoring: &ShowcaseScoring) -> Option<f64> {
    Some(rounded(
        project.product_readiness.score? * scoring.product_weight
            + project.demo_materials.score? * scoring.materials_weight,
    ))
}

fn unassessed_repository(repository: &RepositorySnapshot) -> ShowcaseProjectSnapshot {
    let dimension = ShowcaseDimension {
        status: "unknown".to_string(),
        score: None,
        evidence: "Not assessed in the fleet showcase contract.".to_string(),
    };
    ShowcaseProjectSnapshot {
        repository_name: repository.name.clone(),
        display_name: repository.name.clone(),
        repository_id: Some(repository.id.clone()),
        repository_path: Some(repository.path.clone()),
        registration_status: "registered".to_string(),
        public_eligibility: "unknown".to_string(),
        disposition_source: "Registered in Pronto; showcase disposition not yet assessed."
            .to_string(),
        work_disposition: "unknown".to_string(),
        work_disposition_summary: "No reviewed Showcase work disposition exists.".to_string(),
        next_step_category: "evidence".to_string(),
        product_readiness: dimension.clone(),
        demo_materials: dimension.clone(),
        career_signal: dimension,
        showcase_score: None,
        priority_score: None,
        lane: "unknown".to_string(),
        publishable: false,
        blockers: Vec::new(),
        missing_materials: vec![
            "public-eligibility disposition".to_string(),
            "product-readiness audit".to_string(),
            "demo-material inventory".to_string(),
        ],
        next_step: "Audit eligibility, product readiness, career signal, and demo materials."
            .to_string(),
    }
}

fn priority(project: &ShowcaseContractEntry, scoring: &ShowcaseScoring) -> Option<f64> {
    if project.public_eligibility != "public_showcase" {
        return None;
    }
    Some(rounded(
        project.career_signal.score? * scoring.priority_career_weight
            + project.product_readiness.score? * scoring.priority_product_weight
            + (5.0 - project.demo_materials.score?) * scoring.priority_materials_gap_weight,
    ))
}

fn projected_work_disposition(project: &ShowcaseContractEntry) -> (String, String, String) {
    let fallback = match project.public_eligibility.as_str() {
        "private_client" => (
            "private_client",
            "Private audit context; no public gap-closure work is authorized.",
        ),
        "not_applicable" => (
            "not_applicable",
            "Supporting or provenance work; not a standalone Showcase project.",
        ),
        "blocked" => (
            "blocked",
            "A reviewed eligibility or ownership blocker prevents public work.",
        ),
        _ => ("unknown", "No reviewed Showcase work disposition exists."),
    };
    (
        project
            .work_disposition
            .clone()
            .unwrap_or_else(|| fallback.0.to_string()),
        project
            .work_disposition_summary
            .clone()
            .unwrap_or_else(|| fallback.1.to_string()),
        project
            .next_step_category
            .clone()
            .unwrap_or_else(|| "evidence".to_string()),
    )
}

fn project_lane(
    project: &ShowcaseContractEntry,
    scoring: &ShowcaseScoring,
) -> (&'static str, bool) {
    match project.public_eligibility.as_str() {
        "private_client" => return ("private_client", false),
        "not_applicable" => return ("not_applicable", false),
        "blocked" => return ("blocked", false),
        _ => {}
    }
    if !project.blockers.is_empty()
        || project.product_readiness.status == "blocked"
        || project.demo_materials.status == "blocked"
        || project.career_signal.status == "blocked"
    {
        return ("blocked", false);
    }
    let (Some(product), Some(materials)) = (
        project.product_readiness.score,
        project.demo_materials.score,
    ) else {
        return ("unknown", false);
    };
    if product >= scoring.publishable_product_minimum
        && materials >= scoring.publishable_materials_minimum
        && project.missing_materials.is_empty()
    {
        ("publish_ready", true)
    } else if product >= scoring.publishable_product_minimum {
        ("create_materials", false)
    } else {
        ("product_first", false)
    }
}

fn find_contract(repositories: &[RepositorySnapshot]) -> Result<Option<(String, String)>, String> {
    let mut matches = Vec::new();
    for repository in repositories {
        let path = Path::new(&repository.path).join(CONTRACT_RELATIVE_PATH);
        if path.is_file() {
            matches.push((repository.path.clone(), path));
        }
    }
    match matches.as_slice() {
        [] => Ok(None),
        [(repository_path, path)] => {
            let metadata = fs::metadata(path)
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            if metadata.len() > MAX_CONTRACT_BYTES {
                return Err(format!(
                    "{} exceeds the {} byte limit",
                    path.display(),
                    MAX_CONTRACT_BYTES
                ));
            }
            let contents = fs::read_to_string(path)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?;
            Ok(Some((repository_path.clone(), contents)))
        }
        _ => Err(format!(
            "multiple {} contracts were found; keep exactly one fleet-level contract",
            CONTRACT_RELATIVE_PATH
        )),
    }
}

pub fn inspect(repositories: &[RepositorySnapshot]) -> ShowcasePortfolioSnapshot {
    let Some((_contract_repository_path, contents)) = (match find_contract(repositories) {
        Ok(value) => value,
        Err(error) => return invalid(error),
    }) else {
        return ShowcasePortfolioSnapshot::default();
    };
    let contract = match serde_json::from_str::<ShowcaseContract>(&contents) {
        Ok(contract) => contract,
        Err(error) => return invalid(format!("could not decode showcase contract: {error}")),
    };
    if let Err(error) = validate_contract(&contract) {
        return invalid(error);
    }

    let mut projects = contract
        .projects
        .iter()
        .map(|project| {
            let repository = repositories.iter().find(|candidate| {
                candidate
                    .name
                    .eq_ignore_ascii_case(&project.repository_name)
            });
            let (lane, publishable) = project_lane(project, &contract.scoring);
            let (work_disposition, work_disposition_summary, next_step_category) =
                projected_work_disposition(project);
            ShowcaseProjectSnapshot {
                repository_name: project.repository_name.clone(),
                display_name: project.display_name.clone(),
                repository_id: repository.map(|value| value.id.clone()),
                repository_path: repository.map(|value| value.path.clone()),
                registration_status: if repository.is_some() {
                    "registered".to_string()
                } else {
                    "unregistered".to_string()
                },
                public_eligibility: project.public_eligibility.clone(),
                disposition_source: project.disposition_source.clone(),
                work_disposition,
                work_disposition_summary,
                next_step_category,
                product_readiness: project.product_readiness.clone(),
                demo_materials: project.demo_materials.clone(),
                career_signal: project.career_signal.clone(),
                showcase_score: score(project, &contract.scoring),
                priority_score: priority(project, &contract.scoring),
                lane: lane.to_string(),
                publishable,
                blockers: project.blockers.clone(),
                missing_materials: project.missing_materials.clone(),
                next_step: project.next_step.clone(),
            }
        })
        .collect::<Vec<_>>();
    let assessed_repository_ids = projects
        .iter()
        .filter_map(|project| project.repository_id.clone())
        .collect::<HashSet<_>>();
    projects.extend(
        repositories
            .iter()
            .filter(|repository| !assessed_repository_ids.contains(&repository.id))
            .map(unassessed_repository),
    );
    projects.sort_by(|left, right| {
        right
            .showcase_score
            .partial_cmp(&left.showcase_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .priority_score
                    .partial_cmp(&left.priority_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let publishable_demo_count = projects
        .iter()
        .filter(|project| project.publishable)
        .count();
    let remaining_demo_count = contract
        .target_publishable_demo_count
        .saturating_sub(publishable_demo_count);
    let private_client_count = projects
        .iter()
        .filter(|project| project.public_eligibility == "private_client")
        .count();
    let mut public_queue_projects = projects
        .iter()
        .filter(|project| project.lane == "create_materials")
        .collect::<Vec<_>>();
    public_queue_projects.sort_by(|left, right| {
        right
            .priority_score
            .partial_cmp(&left.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let public_queue = public_queue_projects
        .into_iter()
        .map(|project| project.display_name.clone())
        .collect::<Vec<_>>();

    ShowcasePortfolioSnapshot {
        schema_version: SNAPSHOT_SCHEMA.to_string(),
        status: "Ready".to_string(),
        contract_path: CONTRACT_RELATIVE_PATH.to_string(),
        reviewed_at: Some(contract.reviewed_at),
        quality_bar_source: Some(contract.quality_bar_source),
        goal: ShowcaseGoalSnapshot {
            target_publishable_demo_count: contract.target_publishable_demo_count,
            publishable_demo_count,
            remaining_demo_count,
            status: if remaining_demo_count == 0 {
                "Complete".to_string()
            } else {
                "In progress".to_string()
            },
        },
        scoring: Some(contract.scoring),
        public_queue,
        private_client_count,
        projects,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{RepositorySnapshot, WorkspaceActivity, WorkspaceSummary};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_REPOSITORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temporary_repository(name: &str) -> (std::path::PathBuf, RepositorySnapshot) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = TEMP_REPOSITORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-showcase-{name}-{}-{suffix}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(root.join(".pronto")).expect("contract directory");
        let path = root.to_string_lossy().to_string();
        let workspace = WorkspaceSummary {
            id: format!("workspace:{path}"),
            path: path.clone(),
            is_primary: true,
            branch: "main".to_string(),
            status_available: true,
            status_error: None,
            dirty: false,
            added: 0,
            removed: 0,
            line_totals_partial: false,
            sync_state: "Synced".to_string(),
            remote_freshness: "Fresh".to_string(),
            ahead: 0,
            behind: 0,
            upstream: Some("origin/main".to_string()),
            operation: None,
            last_commit: None,
            last_commit_at: None,
            last_activity_at: None,
            integration_state: "No unique commits".to_string(),
            target_branch: Some("main".to_string()),
            target_confidence: "High".to_string(),
            role: "Primary".to_string(),
            role_confidence: "High".to_string(),
            activity: WorkspaceActivity::default(),
            sync_detail: None,
        };
        let repository = RepositorySnapshot {
            id: format!("repository:{path}"),
            name: name.to_string(),
            path,
            locality: "Local".to_string(),
            lifecycle: "Active".to_string(),
            lifecycle_candidate: "Active".to_string(),
            remote_url: None,
            provider_state: "Local only".to_string(),
            branch: "main".to_string(),
            default_branch: Some("main".to_string()),
            target_branch: Some("main".to_string()),
            target_branch_configured: false,
            workspace: workspace.clone(),
            workspaces: vec![workspace],
            branches: Vec::new(),
            submodules: Vec::new(),
            pull_requests: Vec::new(),
            releases: Vec::new(),
            quality: Default::default(),
            project_compass: Default::default(),
            custody: Default::default(),
            release_rule: None,
            release_recipe: None,
            confirmed_release_version: None,
            ai_permission: "Blocked".to_string(),
            conditions: Vec::new(),
            last_scan_at: "2026-08-12T00:00:00Z".to_string(),
            last_fetch_at: None,
            last_activity_at: None,
        };
        (root, repository)
    }

    fn contract(projects: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "schema_version": CONTRACT_SCHEMA,
            "target_publishable_demo_count": 2,
            "reviewed_at": "2026-08-12T00:00:00Z",
            "quality_bar_source": "Authenticated Handshake AI Showcase audit",
            "scoring": {
                "product_weight": 0.6,
                "materials_weight": 0.4,
                "priority_career_weight": 0.5,
                "priority_product_weight": 0.3,
                "priority_materials_gap_weight": 0.2,
                "publishable_product_minimum": 3.5,
                "publishable_materials_minimum": 4.0
            },
            "projects": projects
        })
    }

    fn project(name: &str, eligibility: &str, product: f64, materials: f64) -> serde_json::Value {
        serde_json::json!({
            "repository_name": name,
            "display_name": name,
            "public_eligibility": eligibility,
            "disposition_source": "test fixture",
            "work_disposition": "targeted_gap_closure",
            "work_disposition_summary": "Close the bounded demo and evidence path.",
            "next_step_category": "demo_integration",
            "product_readiness": { "status": "assessed", "score": product, "evidence": "reviewed" },
            "demo_materials": { "status": "assessed", "score": materials, "evidence": "reviewed" },
            "career_signal": { "status": "assessed", "score": 5.0, "evidence": "reviewed" },
            "blockers": [],
            "missing_materials": if materials >= 4.0 { serde_json::json!([]) } else { serde_json::json!(["public no-auth project page"]) },
            "next_step": "Create a bounded demo artifact."
        })
    }

    #[test]
    fn excludes_private_client_work_before_public_scoring() {
        let (root, repository) = temporary_repository("pronto");
        let payload = contract(serde_json::json!([
            project("CrimClock", "private_client", 4.8, 4.8),
            project("Mac Control", "public_showcase", 4.5, 1.0),
            project("pronto", "public_showcase", 4.0, 1.0),
            project("CourtIQ", "public_showcase", 4.5, 4.5)
        ]));
        fs::write(
            root.join(CONTRACT_RELATIVE_PATH),
            serde_json::to_vec_pretty(&payload).expect("payload"),
        )
        .expect("write contract");

        let snapshot = inspect(&[repository]);

        assert_eq!(snapshot.status, "Ready");
        assert_eq!(snapshot.goal.publishable_demo_count, 1);
        assert_eq!(snapshot.public_queue, vec!["Mac Control", "pronto"]);
        let client = snapshot
            .projects
            .iter()
            .find(|project| project.display_name == "CrimClock")
            .expect("client project");
        assert_eq!(client.lane, "private_client");
        assert!(!client.publishable);
        assert_eq!(client.priority_score, None);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn preserves_unknown_and_not_applicable_dimensions() {
        let (root, repository) = temporary_repository("pronto");
        let payload = contract(serde_json::json!([
            {
                "repository_name": "Unknown",
                "display_name": "Unknown",
                "public_eligibility": "public_showcase",
                "disposition_source": "test fixture",
                "work_disposition": "conditional_gate",
                "work_disposition_summary": "Audit evidence before selecting work.",
                "next_step_category": "evidence",
                "product_readiness": { "status": "unknown", "evidence": "not reviewed" },
                "demo_materials": { "status": "unknown", "evidence": "not reviewed" },
                "career_signal": { "status": "unknown", "evidence": "not reviewed" },
                "blockers": [],
                "missing_materials": ["demo brief"],
                "next_step": "Audit first."
            },
            {
                "repository_name": "Config",
                "display_name": "Config",
                "public_eligibility": "not_applicable",
                "disposition_source": "test fixture",
                "product_readiness": { "status": "not_applicable", "evidence": "support repository" },
                "demo_materials": { "status": "not_applicable", "evidence": "support repository" },
                "career_signal": { "status": "not_applicable", "evidence": "support repository" },
                "blockers": [],
                "missing_materials": [],
                "next_step": "No showcase work."
            }
        ]));
        fs::write(
            root.join(CONTRACT_RELATIVE_PATH),
            serde_json::to_vec_pretty(&payload).expect("payload"),
        )
        .expect("write contract");

        let snapshot = inspect(&[repository]);

        let config = snapshot
            .projects
            .iter()
            .find(|project| project.display_name == "Config")
            .expect("not-applicable project");
        assert_eq!(config.lane, "not_applicable");
        let unknown = snapshot
            .projects
            .iter()
            .find(|project| project.display_name == "Unknown")
            .expect("unknown project");
        assert_eq!(unknown.lane, "unknown");
        let registered_unassessed = snapshot
            .projects
            .iter()
            .find(|project| project.display_name == "pronto")
            .expect("registered repository synthesized into the fleet audit");
        assert_eq!(registered_unassessed.public_eligibility, "unknown");
        assert_eq!(registered_unassessed.registration_status, "registered");
        assert_eq!(registered_unassessed.showcase_score, None);
        assert!(snapshot.projects.iter().all(|project| !project.publishable));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn unreviewed_repository_is_projected_without_mutating_showcase_contract() {
        let (root, existing_repository) = temporary_repository("pronto");
        let (new_root, new_repository) = temporary_repository("new-project");
        let mut payload = contract(serde_json::json!([project(
            "pronto",
            "not_applicable",
            4.0,
            4.0
        )]));
        payload["public_release_target_policy"] = serde_json::json!({
            "matrix_path": "showcase-materials/public-release-targets.json"
        });
        let contract_path = root.join(CONTRACT_RELATIVE_PATH);
        fs::write(
            &contract_path,
            serde_json::to_vec_pretty(&payload).expect("payload"),
        )
        .expect("write contract");

        let repositories = vec![existing_repository, new_repository.clone()];
        let first_contents = fs::read_to_string(&contract_path).expect("read contract");

        let snapshot = inspect(&repositories);
        let pending = snapshot
            .projects
            .iter()
            .find(|project| project.repository_name == "new-project")
            .expect("unreviewed repository should remain visible in projection");
        assert_eq!(pending.public_eligibility, "unknown");
        assert_eq!(pending.lane, "unknown");
        assert_eq!(pending.showcase_score, None);
        assert!(!pending.publishable);
        assert_eq!(
            fs::read_to_string(&contract_path).expect("read contract after projection"),
            first_contents
        );

        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_dir_all(new_root).expect("cleanup");
    }

    #[test]
    fn requires_granular_work_disposition_for_public_projects() {
        let (root, repository) = temporary_repository("pronto");
        let mut public_project = project("pronto", "public_showcase", 4.0, 1.0);
        public_project
            .as_object_mut()
            .expect("project object")
            .remove("next_step_category");
        let payload = contract(serde_json::json!([public_project]));
        fs::write(
            root.join(CONTRACT_RELATIVE_PATH),
            serde_json::to_vec_pretty(&payload).expect("payload"),
        )
        .expect("write contract");

        let snapshot = inspect(&[repository]);

        assert_eq!(snapshot.status, "Invalid");
        assert!(snapshot
            .error
            .as_deref()
            .is_some_and(|error| error.contains("next_step_category is required")));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
