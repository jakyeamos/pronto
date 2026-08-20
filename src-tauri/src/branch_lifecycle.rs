//! Read-only branch lifecycle and admission projection.
//!
//! This module turns the branch, workspace, and custody evidence already
//! collected by Pronto into a bounded lifecycle signal. It never treats age
//! as deletion authority: expired and integrated branches remain disposition
//! work until live target, remote, worktree, ownership, and provider checks
//! prove that ordinary pruning is safe.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::{BranchSummary, WorkspaceSummary};
use crate::custody::{CustodyLane, CustodySnapshot};

pub const BRANCH_LIFECYCLE_SCHEMA: &str = "pronto-branch-lifecycle/v1";
pub const FEATURE_BRANCH_SOFT_LIMIT: usize = 5;
pub const FEATURE_BRANCH_HARD_LIMIT: usize = 8;
pub const BRANCH_WARNING_AFTER_HOURS: i64 = 48;
pub const BRANCH_EXPIRY_AFTER_HOURS: i64 = 14 * 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchLifecyclePolicy {
    pub soft_limit: usize,
    pub hard_limit: usize,
    pub warning_after_hours: i64,
    pub expiry_after_hours: i64,
}

impl Default for BranchLifecyclePolicy {
    fn default() -> Self {
        Self {
            soft_limit: FEATURE_BRANCH_SOFT_LIMIT,
            hard_limit: FEATURE_BRANCH_HARD_LIMIT,
            warning_after_hours: BRANCH_WARNING_AFTER_HOURS,
            expiry_after_hours: BRANCH_EXPIRY_AFTER_HOURS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BranchLifecycleEntry {
    pub name: String,
    pub role: String,
    pub counted: bool,
    pub status: String,
    pub integration_state: String,
    pub age_hours: Option<i64>,
    pub last_activity_at: Option<String>,
    pub workspace_id: Option<String>,
    pub worktree_path: Option<String>,
    pub custody_task_id: Option<String>,
    pub custodian: Option<String>,
    pub created_at: Option<String>,
    pub lease_expires_at: Option<String>,
    pub lease_status: String,
    pub reason: String,
    pub next_safe_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchLifecycleSnapshot {
    pub schema_version: String,
    pub target_branch: Option<String>,
    pub policy: BranchLifecyclePolicy,
    pub status: String,
    pub admission: String,
    pub feature_branch_count: usize,
    pub active_feature_branch_count: usize,
    pub retirement_review_count: usize,
    pub expired_count: usize,
    pub unknown_count: usize,
    pub branches: Vec<BranchLifecycleEntry>,
    pub next_safe_step: String,
    pub read_only: bool,
}

impl Default for BranchLifecycleSnapshot {
    fn default() -> Self {
        Self {
            schema_version: BRANCH_LIFECYCLE_SCHEMA.to_string(),
            target_branch: None,
            policy: BranchLifecyclePolicy::default(),
            status: "evidence_unknown".to_string(),
            admission: "review_required".to_string(),
            feature_branch_count: 0,
            active_feature_branch_count: 0,
            retirement_review_count: 0,
            expired_count: 0,
            unknown_count: 0,
            branches: Vec::new(),
            next_safe_step: "Refresh branch, workspace, and custody evidence before creating another feature branch.".to_string(),
            read_only: true,
        }
    }
}

fn parsed_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn age_hours(value: Option<&str>, now: DateTime<Utc>) -> Option<i64> {
    parsed_timestamp(value).map(|timestamp| now.signed_duration_since(timestamp).num_hours().max(0))
}

fn is_protected_branch(
    branch: &BranchSummary,
    target_branch: Option<&str>,
    default_branch: Option<&str>,
) -> bool {
    if target_branch.is_some_and(|target| target == branch.name)
        || default_branch.is_some_and(|default| default == branch.name)
    {
        return true;
    }
    matches!(
        branch.role.as_str(),
        "Production" | "Integration" | "Release" | "Hotfix"
    ) || matches!(
        branch.name.to_ascii_lowercase().as_str(),
        "main" | "master" | "dev" | "develop" | "development" | "staging"
    )
}

fn custody_lane<'a>(custody: &'a CustodySnapshot, branch: &str) -> Option<&'a CustodyLane> {
    custody
        .lanes
        .iter()
        .find(|lane| lane.branch.as_deref() == Some(branch))
}

fn workspace_for_branch<'a>(
    workspaces: &'a [WorkspaceSummary],
    branch: &BranchSummary,
) -> Option<&'a WorkspaceSummary> {
    branch.workspace_id.as_deref().and_then(|workspace_id| {
        workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
    })
}

fn lease_status(lane: Option<&CustodyLane>, now: DateTime<Utc>) -> String {
    let Some(lane) = lane else {
        return "unleased".to_string();
    };
    if matches!(lane.state.as_str(), "closed" | "released") {
        return "closed".to_string();
    }
    let Some(expires_at) = parsed_timestamp(lane.lease_expires_at.as_deref()) else {
        return "unknown".to_string();
    };
    if expires_at < now {
        "expired".to_string()
    } else {
        "current".to_string()
    }
}

fn last_activity<'a>(
    branch: &'a BranchSummary,
    workspace: Option<&'a WorkspaceSummary>,
    lane: Option<&'a CustodyLane>,
) -> Option<&'a str> {
    lane.and_then(|lane| lane.last_activity_at.as_deref())
        .or_else(|| workspace.and_then(|workspace| workspace.last_activity_at.as_deref()))
        .or(branch.last_commit_at.as_deref())
}

fn workspace_is_active(workspace: Option<&WorkspaceSummary>) -> bool {
    workspace.is_some_and(|workspace| {
        workspace.dirty || workspace.operation.is_some() || workspace.activity.state == "Active"
    })
}

fn branch_needs_lease(branch: &BranchSummary, workspace: Option<&WorkspaceSummary>) -> bool {
    branch.role == "Agent task" || workspace.is_some_and(|workspace| !workspace.is_primary)
}

fn branch_entry(
    branch: &BranchSummary,
    workspaces: &[WorkspaceSummary],
    custody: &CustodySnapshot,
    target_branch: Option<&str>,
    default_branch: Option<&str>,
    policy: &BranchLifecyclePolicy,
    now: DateTime<Utc>,
) -> BranchLifecycleEntry {
    let workspace = workspace_for_branch(workspaces, branch);
    let lane = custody_lane(custody, &branch.name);
    let last_activity_at = last_activity(branch, workspace, lane).map(str::to_string);
    let age = age_hours(last_activity_at.as_deref(), now);
    let protected = is_protected_branch(branch, target_branch, default_branch);
    let counted = !protected && matches!(branch.role.as_str(), "Feature" | "Agent task");
    let lease_status = lease_status(lane, now);
    let lease_missing = counted && branch_needs_lease(branch, workspace) && lane.is_none();
    let evidence_unavailable = workspace.is_some_and(|workspace| !workspace.status_available)
        || branch.integration_state == "Unknown"
        || branch.integration_state == "Target unknown"
        || (counted && age.is_none());
    let active = workspace_is_active(workspace);
    let retirement_review = matches!(
        branch.integration_state.as_str(),
        "Already integrated" | "No unique commits"
    ) && !active;
    let expired = counted
        && !active
        && (lease_status == "expired"
            || age.is_some_and(|hours| hours >= policy.expiry_after_hours));

    let (status, reason, next_safe_step) = if protected {
        (
            "protected",
            "Protected integration, release, or production branch; excluded from the feature budget.",
            "Leave this branch in place and evaluate feature branches against the configured target.",
        )
    } else if evidence_unavailable {
        (
            "unknown",
            "Branch, workspace, or activity evidence is incomplete.",
            "Refresh live Git and custody evidence before creating another feature branch or pruning this ref.",
        )
    } else if expired {
        (
            "expired",
            "The branch has exceeded its short-lived lease window; expiry requires a disposition, not deletion.",
            "Create a disposition to fold, preserve, or explicitly extend the lease; do not delete from age alone.",
        )
    } else if retirement_review {
        (
            "retirement_review",
            "Pronto observed no unique commits relative to the configured target.",
            "Verify live target ancestry or patch equivalence, remote/PR/protection state, and worktree ownership before ordinary branch -d.",
        )
    } else if age.is_some_and(|hours| hours >= policy.warning_after_hours) {
        (
            "stale_warning",
            "The branch has had no observed activity inside the short-lived warning window.",
            "Finish focused tests and fold or preserve the branch before the expiry window; refresh if activity evidence is stale.",
        )
    } else if lease_missing {
        (
            "lease_review",
            "This agent or linked worktree branch has no recorded custody lease.",
            "Register or reconcile the branch through isolated-change-workflow before extending work or integrating it.",
        )
    } else {
        (
            "active",
            "The branch remains within the short-lived lifecycle budget.",
            "Run targeted verification, fold the completed change into the configured target, then retire the branch and worktree.",
        )
    };

    BranchLifecycleEntry {
        name: branch.name.clone(),
        role: branch.role.clone(),
        counted,
        status: status.to_string(),
        integration_state: branch.integration_state.clone(),
        age_hours: age,
        last_activity_at,
        workspace_id: branch.workspace_id.clone(),
        worktree_path: workspace
            .map(|workspace| workspace.path.clone())
            .or_else(|| lane.and_then(|lane| lane.worktree.clone())),
        custody_task_id: lane.map(|lane| lane.task_id.clone()),
        custodian: lane.and_then(|lane| lane.custodian.clone()),
        created_at: lane.and_then(|lane| lane.created_at.clone()),
        lease_expires_at: lane.and_then(|lane| lane.lease_expires_at.clone()),
        lease_status,
        reason: reason.to_string(),
        next_safe_step: next_safe_step.to_string(),
    }
}

pub fn project(
    branches: &[BranchSummary],
    workspaces: &[WorkspaceSummary],
    custody: &CustodySnapshot,
    target_branch: Option<&str>,
    default_branch: Option<&str>,
) -> BranchLifecycleSnapshot {
    let policy = BranchLifecyclePolicy::default();
    let now = Utc::now();
    let mut entries = branches
        .iter()
        .map(|branch| {
            branch_entry(
                branch,
                workspaces,
                custody,
                target_branch,
                default_branch,
                &policy,
                now,
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));

    let feature_branch_count = entries.iter().filter(|entry| entry.counted).count();
    let active_feature_branch_count = entries
        .iter()
        .filter(|entry| {
            entry.counted && matches!(entry.status.as_str(), "active" | "stale_warning")
        })
        .count();
    let retirement_review_count = entries
        .iter()
        .filter(|entry| entry.status == "retirement_review")
        .count();
    let expired_count = entries
        .iter()
        .filter(|entry| entry.status == "expired")
        .count();
    let unknown_count = entries
        .iter()
        .filter(|entry| entry.status == "unknown" || entry.status == "lease_review")
        .count();

    let (status, admission, next_safe_step) = if feature_branch_count >= policy.hard_limit {
        (
            "admission_blocked",
            "blocked",
            "Stop creating feature branches at the hard limit. Fold, preserve, or retire existing branches with live proof before admitting another.",
        )
    } else if unknown_count > 0 {
        (
            "evidence_unknown",
            "review_required",
            "Refresh incomplete branch, workspace, or custody evidence before admitting another feature branch.",
        )
    } else if expired_count > 0 || retirement_review_count > 0 {
        (
            "disposition_required",
            "review_required",
            "Resolve expired or target-relative retirement candidates before admitting another feature branch.",
        )
    } else if feature_branch_count >= policy.soft_limit {
        (
            "warning",
            "warning",
            "The soft feature-branch limit is reached. Start no additional branch without a named owner, target, lease, and near-term fold plan.",
        )
    } else {
        (
            "within_budget",
            "allowed",
            "A new short-lived feature branch is within the configured budget; record its owner, target, lease, and expiry before starting work.",
        )
    };

    BranchLifecycleSnapshot {
        schema_version: BRANCH_LIFECYCLE_SCHEMA.to_string(),
        target_branch: target_branch.map(str::to_string),
        policy,
        status: status.to_string(),
        admission: admission.to_string(),
        feature_branch_count,
        active_feature_branch_count,
        retirement_review_count,
        expired_count,
        unknown_count,
        branches: entries,
        next_safe_step: next_safe_step.to_string(),
        read_only: true,
    }
}

#[cfg(test)]
#[path = "branch_lifecycle_tests.rs"]
mod tests;
