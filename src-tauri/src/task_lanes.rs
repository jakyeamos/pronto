use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TASK_LANE_SCHEMA: &str = "pronto-task-lanes/v1";
const DEFAULT_GRACE_SECONDS: u64 = 86_400;
const STATUS_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLaneReport {
    pub schema_version: String,
    pub generated_at: String,
    pub repository: String,
    pub source: TaskLaneSource,
    pub counts: TaskLaneCounts,
    pub lanes: Vec<TaskLane>,
    pub authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLaneSource {
    pub status: String,
    pub workflow_schema_version: Option<String>,
    pub integrity_authority: String,
    pub adoption_assessment: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLaneCounts {
    pub total: usize,
    pub active: usize,
    pub paused: usize,
    pub stale: usize,
    pub adoptable: usize,
    pub contested: usize,
    pub unknown: usize,
    pub integrating: usize,
    pub closed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLane {
    pub task_id: String,
    pub task: Option<String>,
    pub branch: Option<String>,
    pub worktree: Option<String>,
    pub thread_id: Option<String>,
    pub custody_state: String,
    pub custody_reason: String,
    pub workflow_status: Option<String>,
    pub workflow_classification: Option<String>,
    pub integrity_verified: bool,
    pub worktree_present: Option<bool>,
    pub worktree_clean: Option<bool>,
    pub branch_live: Option<String>,
    pub head_sha: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub adoption: TaskLaneAdoption,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLaneAdoption {
    pub eligible: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStatus {
    schema_version: Option<String>,
    #[serde(default)]
    tasks: Vec<WorkflowTask>,
}

#[derive(Debug, Deserialize)]
struct WorkflowTask {
    task_id: String,
    task: Option<String>,
    branch: Option<String>,
    worktree: Option<String>,
    thread_id: Option<String>,
    status: Option<String>,
    classification: Option<String>,
    #[serde(default)]
    integrity_verified: bool,
    worktree_present: Option<bool>,
    worktree_clean: Option<bool>,
    branch_live: Option<String>,
    head_sha: Option<String>,
    last_heartbeat_at: Option<String>,
}

pub fn inspect(repository: &Path) -> TaskLaneReport {
    let repository_display = repository.display().to_string();
    match invoke_workflow_status(repository) {
        Ok(payload) => report_from_payload(&repository_display, &payload),
        Err(error) => unavailable_report(&repository_display, error),
    }
}

fn invoke_workflow_status(repository: &Path) -> Result<String, String> {
    let script = workflow_script_path().ok_or_else(|| {
        "isolated-change workflow script was not found; custody evidence is unavailable".to_string()
    })?;
    let python = std::env::var("PRONTO_PYTHON").unwrap_or_else(|_| {
        if Path::new("/usr/local/bin/python3").is_file() {
            "/usr/local/bin/python3".to_string()
        } else {
            "python3".to_string()
        }
    });
    let mut child = Command::new(&python)
        .arg(&script)
        .arg("status")
        .arg("--repo")
        .arg(repository)
        .arg("--grace-seconds")
        .arg(DEFAULT_GRACE_SECONDS.to_string())
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "could not start isolated-change status with {}: {error}",
                script.display()
            )
        })?;

    let deadline = Instant::now() + Duration::from_secs(STATUS_TIMEOUT_SECONDS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "isolated-change status exceeded its {STATUS_TIMEOUT_SECONDS} second deadline"
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "could not wait for isolated-change status: {error}"
                ));
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not read isolated-change status: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("isolated-change status exited with {}", output.status)
        } else {
            format!("isolated-change status failed: {stderr}")
        });
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("isolated-change status was not UTF-8: {error}"))
}

fn workflow_script_path() -> Option<PathBuf> {
    if let Ok(configured) = std::env::var("PRONTO_ISOLATED_CHANGE_SCRIPT") {
        let path = PathBuf::from(configured);
        return path.is_file().then_some(path);
    }
    dirs::home_dir()
        .map(|home| home.join(".agents/skills/isolated-change-workflow/scripts/isolated_change.py"))
        .filter(|path| path.is_file())
}

fn report_from_payload(repository: &str, payload: &str) -> TaskLaneReport {
    let status = match serde_json::from_str::<WorkflowStatus>(payload) {
        Ok(status) => status,
        Err(error) => {
            return unavailable_report(
                repository,
                format!("isolated-change status returned invalid JSON: {error}"),
            )
        }
    };
    let workflow_schema_version = status.schema_version.clone();
    let lanes = status
        .tasks
        .into_iter()
        .map(classify_task)
        .collect::<Vec<_>>();
    let counts = count_lanes(&lanes);
    TaskLaneReport {
        schema_version: TASK_LANE_SCHEMA.to_string(),
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        repository: repository.to_string(),
        source: TaskLaneSource {
            status: "available".to_string(),
            workflow_schema_version,
            integrity_authority: "isolated-change-workflow status".to_string(),
            adoption_assessment: "lease-and-live-git".to_string(),
            detail: "A verified expired lease may become adoptable after the grace period when its worktree and recorded branch still match live Git. Agent process liveness is advisory; an agent that stops renewing cannot retain custody forever.".to_string(),
        },
        counts,
        lanes,
        authorization: "Read-only coordination evidence. This report does not authorize checkout mutation, adoption, integration, deletion, or release.".to_string(),
    }
}

fn unavailable_report(repository: &str, detail: String) -> TaskLaneReport {
    TaskLaneReport {
        schema_version: TASK_LANE_SCHEMA.to_string(),
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        repository: repository.to_string(),
        source: TaskLaneSource {
            status: "unavailable".to_string(),
            workflow_schema_version: None,
            integrity_authority: "unavailable".to_string(),
            adoption_assessment: "blocked".to_string(),
            detail,
        },
        counts: TaskLaneCounts::default(),
        lanes: Vec::new(),
        authorization: "Custody is unknown. Do not mutate, adopt, integrate, delete, or release a task lane from this report.".to_string(),
    }
}

fn classify_task(task: WorkflowTask) -> TaskLane {
    let classification = task.classification.as_deref().unwrap_or("unknown");
    let status = task.status.as_deref().unwrap_or("unknown");
    let (custody_state, custody_reason) = if !task.integrity_verified {
        (
            "unknown",
            "Receipt integrity is not verified, so custody cannot be inferred.",
        )
    } else if matches!(status, "finishing" | "integrating" | "reconciling") {
        (
            "integrating",
            "The owning workflow reports that integration or final reconciliation is in progress.",
        )
    } else if classification == "active_confirmed" {
        (
            "active",
            "A verified lease and live task lane confirm active custody.",
        )
    } else if status == "paused" {
        (
            "paused",
            "The verified owner explicitly paused the task lane without releasing custody.",
        )
    } else if matches!(classification, "owner_terminal_dirty" | "cleanup_blocked") {
        (
            "contested",
            "The prior owner is terminal or cleanup is blocked while recoverable work remains.",
        )
    } else if classification == "lease_expired_unverified"
        && task.worktree_present == Some(true)
        && task.branch_live.as_deref() == task.branch.as_deref()
    {
        (
            "adoptable",
            "The signed lease is past its grace period and live Git still binds the recorded branch to the recoverable worktree.",
        )
    } else if classification == "lease_expired_unverified" {
        (
            "stale",
            "The lease expired, but expiry alone does not prove abandonment or authorize takeover.",
        )
    } else if classification == "released_recoverable" {
        (
            "stale",
            "The owner released a recoverable lane, but required negative live evidence is incomplete.",
        )
    } else if classification == "removed" || matches!(status, "finished" | "completed" | "closed") {
        ("closed", "The verified workflow reports a terminal lane.")
    } else {
        (
            "unknown",
            "The workflow state does not satisfy a recognized custody proof.",
        )
    };

    let blockers = if custody_state == "adoptable" {
        Vec::new()
    } else if custody_state == "stale" {
        vec!["the recorded worktree and branch do not both match live Git".to_string()]
    } else if custody_state == "closed" {
        vec!["closed lanes are not adoption candidates".to_string()]
    } else {
        vec![format!(
            "custody state {custody_state} is not eligible for adoption"
        )]
    };

    TaskLane {
        task_id: task.task_id,
        task: task.task,
        branch: task.branch,
        worktree: task.worktree,
        thread_id: task.thread_id,
        custody_state: custody_state.to_string(),
        custody_reason: custody_reason.to_string(),
        workflow_status: task.status,
        workflow_classification: task.classification,
        integrity_verified: task.integrity_verified,
        worktree_present: task.worktree_present,
        worktree_clean: task.worktree_clean,
        branch_live: task.branch_live,
        head_sha: task.head_sha,
        last_heartbeat_at: task.last_heartbeat_at,
        adoption: TaskLaneAdoption {
            eligible: custody_state == "adoptable",
            blockers,
        },
    }
}

fn count_lanes(lanes: &[TaskLane]) -> TaskLaneCounts {
    let mut counts = TaskLaneCounts {
        total: lanes.len(),
        ..TaskLaneCounts::default()
    };
    for lane in lanes {
        match lane.custody_state.as_str() {
            "active" => counts.active += 1,
            "paused" => counts.paused += 1,
            "stale" => counts.stale += 1,
            "adoptable" => counts.adoptable += 1,
            "contested" => counts.contested += 1,
            "integrating" => counts.integrating += 1,
            "closed" => counts.closed += 1,
            _ => counts.unknown += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(tasks: &str) -> TaskLaneReport {
        report_from_payload(
            "/tmp/example",
            &format!(r#"{{"schema_version":"isolated-change-status/v2","tasks":{tasks}}}"#),
        )
    }

    #[test]
    fn active_signed_lane_projects_active_custody_without_adoption() {
        let report = report(
            r#"[{"task_id":"task-1","branch":"codex/task-1","status":"active","classification":"active_confirmed","integrity_verified":true,"worktree_present":true,"worktree_clean":false,"branch_live":"codex/task-1"}]"#,
        );
        assert_eq!(report.counts.active, 1);
        assert_eq!(report.lanes[0].custody_state, "active");
        assert!(!report.lanes[0].adoption.eligible);
    }

    #[test]
    fn expired_lane_becomes_adoptable_when_signed_custody_lapses_and_git_matches() {
        let report = report(
            r#"[{"task_id":"task-2","branch":"codex/task-2","status":"active","classification":"lease_expired_unverified","integrity_verified":true,"worktree_present":true,"worktree_clean":false,"branch_live":"codex/task-2"}]"#,
        );
        assert_eq!(report.counts.stale, 0);
        assert_eq!(report.counts.adoptable, 1);
        assert!(report.lanes[0].adoption.eligible);
        assert!(report.lanes[0].adoption.blockers.is_empty());
    }

    #[test]
    fn expired_lane_without_matching_live_git_stays_stale() {
        let report = report(
            r#"[{"task_id":"task-2b","branch":"codex/task-2b","status":"active","classification":"lease_expired_unverified","integrity_verified":true,"worktree_present":false,"worktree_clean":null,"branch_live":null}]"#,
        );
        assert_eq!(report.counts.stale, 1);
        assert_eq!(report.counts.adoptable, 0);
    }

    #[test]
    fn unsigned_legacy_lane_is_unknown_even_when_labeled_active() {
        let report = report(
            r#"[{"task_id":"task-3","status":"active","classification":"legacy_unsigned","integrity_verified":false,"worktree_present":false,"branch_live":null}]"#,
        );
        assert_eq!(report.counts.unknown, 1);
        assert_eq!(report.lanes[0].custody_state, "unknown");
    }

    #[test]
    fn terminal_dirty_lane_is_contested() {
        let report = report(
            r#"[{"task_id":"task-4","branch":"codex/task-4","status":"released","classification":"owner_terminal_dirty","integrity_verified":true,"worktree_present":true,"worktree_clean":false,"branch_live":"codex/task-4"}]"#,
        );
        assert_eq!(report.counts.contested, 1);
    }

    #[test]
    fn removed_lane_is_closed() {
        let report = report(
            r#"[{"task_id":"task-5","status":"finished","classification":"removed","integrity_verified":true,"worktree_present":false,"worktree_clean":true,"branch_live":null}]"#,
        );
        assert_eq!(report.counts.closed, 1);
    }

    #[test]
    fn malformed_status_is_structured_unavailable_evidence() {
        let report = report_from_payload("/tmp/example", "not-json");
        assert_eq!(report.source.status, "unavailable");
        assert_eq!(report.source.adoption_assessment, "blocked");
        assert!(report.lanes.is_empty());
    }
}
