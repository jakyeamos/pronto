use super::*;
use crate::core::{ActivitySignal, WorkspaceActivity};

fn branch(name: &str, role: &str, state: &str, last_commit_at: &str) -> BranchSummary {
    BranchSummary {
        name: name.to_string(),
        role: role.to_string(),
        role_confidence: "High".to_string(),
        target_branch: Some("dev".to_string()),
        target_confidence: "High".to_string(),
        ahead: 1,
        behind: 0,
        integration_state: state.to_string(),
        workspace_id: None,
        last_commit: Some("abc".to_string()),
        last_commit_at: Some(last_commit_at.to_string()),
    }
}

fn workspace(id: &str, branch: &str, dirty: bool) -> WorkspaceSummary {
    WorkspaceSummary {
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        is_primary: false,
        branch: branch.to_string(),
        status_available: true,
        status_error: None,
        dirty,
        added: 0,
        removed: 0,
        line_totals_partial: false,
        sync_state: "Synced".to_string(),
        remote_freshness: "Fresh".to_string(),
        ahead: 1,
        behind: 0,
        upstream: Some("origin/feature".to_string()),
        operation: None,
        last_commit: Some("abc".to_string()),
        last_commit_at: Some("2026-08-20T12:00:00Z".to_string()),
        last_activity_at: Some("2026-08-20T12:00:00Z".to_string()),
        integration_state: "Integration eligible".to_string(),
        target_branch: Some("dev".to_string()),
        target_confidence: "High".to_string(),
        role: "Agent task".to_string(),
        role_confidence: "High".to_string(),
        activity: WorkspaceActivity {
            state: "Idle".to_string(),
            confidence: "High".to_string(),
            signals: Vec::<ActivitySignal>::new(),
            manifest: None,
        },
        provenance: Default::default(),
        sync_detail: None,
    }
}

#[test]
fn hard_limit_blocks_new_feature_branch_admission() {
    let branches = (0..FEATURE_BRANCH_HARD_LIMIT)
        .map(|index| {
            branch(
                &format!("feature/{index}"),
                "Feature",
                "Integration eligible",
                "2026-08-20T12:00:00Z",
            )
        })
        .collect::<Vec<_>>();

    let projection = project(
        &branches,
        &[],
        &CustodySnapshot::default(),
        Some("dev"),
        Some("main"),
    );

    assert_eq!(projection.feature_branch_count, FEATURE_BRANCH_HARD_LIMIT);
    assert_eq!(projection.status, "admission_blocked");
    assert_eq!(projection.admission, "blocked");
    assert!(projection.read_only);
}

#[test]
fn soft_limit_warns_before_hard_limit() {
    let branches = (0..FEATURE_BRANCH_SOFT_LIMIT)
        .map(|index| {
            branch(
                &format!("feature/{index}"),
                "Feature",
                "Integration eligible",
                "2026-08-20T12:00:00Z",
            )
        })
        .collect::<Vec<_>>();

    let projection = project(
        &branches,
        &[],
        &CustodySnapshot::default(),
        Some("dev"),
        Some("main"),
    );

    assert_eq!(projection.feature_branch_count, FEATURE_BRANCH_SOFT_LIMIT);
    assert_eq!(
        projection.active_feature_branch_count,
        FEATURE_BRANCH_SOFT_LIMIT
    );
    assert_eq!(projection.status, "warning");
    assert_eq!(projection.admission, "warning");
}

#[test]
fn protected_and_retirement_branches_do_not_count_as_active_feature_work() {
    let branches = vec![
        branch(
            "dev",
            "Integration",
            "No unique commits",
            "2026-08-20T12:00:00Z",
        ),
        branch(
            "main",
            "Production",
            "No unique commits",
            "2026-08-20T12:00:00Z",
        ),
        branch(
            "feature/merged",
            "Feature",
            "Already integrated",
            "2026-08-20T12:00:00Z",
        ),
        branch(
            "feature/live",
            "Feature",
            "Integration eligible",
            "2026-08-20T12:00:00Z",
        ),
    ];

    let projection = project(
        &branches,
        &[],
        &CustodySnapshot::default(),
        Some("dev"),
        Some("main"),
    );

    assert_eq!(projection.feature_branch_count, 2);
    assert_eq!(projection.retirement_review_count, 1);
    assert_eq!(projection.active_feature_branch_count, 1);
    assert_eq!(projection.status, "disposition_required");
    assert_eq!(
        projection
            .branches
            .iter()
            .find(|entry| entry.name == "feature/merged")
            .map(|entry| entry.status.as_str()),
        Some("retirement_review")
    );
}

#[test]
fn expired_agent_lease_requires_disposition_without_prune_authority() {
    let mut lane = CustodyLane::default();
    lane.task_id = "task-expired".to_string();
    lane.branch = Some("agent/expired".to_string());
    lane.worktree = Some("/tmp/expired".to_string());
    lane.state = "active".to_string();
    lane.lease_expires_at = Some("2026-08-19T12:00:00Z".to_string());
    let mut custody = CustodySnapshot::default();
    custody.lanes = vec![lane];
    let mut branch = branch(
        "agent/expired",
        "Agent task",
        "Integration eligible",
        "2026-08-20T12:00:00Z",
    );
    branch.workspace_id = Some("workspace-expired".to_string());

    let projection = project(
        &[branch],
        &[workspace("workspace-expired", "agent/expired", false)],
        &custody,
        Some("dev"),
        Some("main"),
    );

    let entry = &projection.branches[0];
    assert_eq!(entry.status, "expired");
    assert_eq!(entry.lease_status, "expired");
    assert_eq!(projection.admission, "review_required");
    assert!(entry.next_safe_step.contains("do not delete"));
}
