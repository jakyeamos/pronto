fn scan_workspace(
    path: &Path,
    is_primary: bool,
    default_branch: Option<&str>,
    repository_target_branch: Option<&str>,
    repository_target_confidence: &str,
    existing: Option<&RepositorySnapshot>,
) -> WorkspaceSummary {
    let (status_result, totals, operation, last_commit, last_commit_at, last_activity_at) =
        workspace_status(path);
    let (status_available, status, status_error) = match status_result {
        Ok(status) => (true, status, None),
        Err(error) => (
            false,
            ParsedStatus {
                branch: "Unknown".to_string(),
                ..ParsedStatus::default()
            },
            Some(error),
        ),
    };
    let remote_freshness = existing
        .and_then(|repository| repository.last_fetch_at.clone())
        .unwrap_or_else(|| "Not fetched by Pronto".to_string());
    let activity = collect_workspace_activity(
        path,
        status_available && status.dirty,
        if status_available { status.ahead } else { 0 },
    );
    let sync_state = if !status_available {
        "Git status unavailable".to_string()
    } else if status.upstream.is_none() {
        "No upstream".to_string()
    } else if status.ahead > 0 && status.behind > 0 {
        format!(
            "Diverged · {} ahead / {} behind",
            status.ahead, status.behind
        )
    } else if status.ahead > 0 {
        format!("Ahead by {}", status.ahead)
    } else if status.behind > 0 {
        format!("Behind by {}", status.behind)
    } else {
        "Synced".to_string()
    };
    let (role, role_confidence, target_branch, target_confidence) = if status_available {
        let (mut role, mut role_confidence) = branch_role(&status.branch, default_branch);
        let (mut target_branch, mut target_confidence) =
            target_for_branch(&status.branch, repository_target_branch);
        if target_branch.is_some() {
            target_confidence = repository_target_confidence.to_string();
        }
        if let Some(manifest) = activity.manifest.as_ref() {
            if repository_target_branch.is_none() {
                if let Some(manifest_target) = manifest.target_branch.as_ref() {
                    target_branch = Some(manifest_target.clone());
                    target_confidence = "High".to_string();
                }
            }
            if manifest.agent_type.is_some() && role != "Production" {
                role = "Agent task".to_string();
                role_confidence = "High".to_string();
            }
        }
        (role, role_confidence, target_branch, target_confidence)
    } else {
        (
            "Unknown".to_string(),
            "Unknown".to_string(),
            None,
            "Unknown".to_string(),
        )
    };
    let integration_state = if status_available {
        branch_integration_state(path, &status.branch, repository_target_branch, None)
    } else {
        "Unknown".to_string()
    };
    let provenance = workspace_provenance(path, is_primary, last_commit.clone(), &activity);
    let mut workspace = WorkspaceSummary {
        id: path_id("workspace", path),
        path: path.to_string_lossy().to_string(),
        is_primary,
        branch: status.branch,
        status_available,
        status_error,
        dirty: status_available && status.dirty,
        added: totals.added,
        removed: totals.removed,
        line_totals_partial: totals.partial,
        sync_state,
        remote_freshness,
        ahead: status.ahead,
        behind: status.behind,
        upstream: status.upstream,
        operation,
        last_commit,
        last_commit_at,
        last_activity_at,
        integration_state,
        target_branch,
        target_confidence,
        role,
        role_confidence,
        activity,
        provenance,
        sync_detail: None,
    };
    if workspace.status_available {
        workspace.integration_state = branch_integration_state(
            path,
            &workspace.branch,
            repository_target_branch,
            Some(&workspace),
        );
    }
    workspace
}

fn evidence(label: &str, value: String, source: &str, observed_at: &str) -> EvidenceItem {
    EvidenceItem {
        label: label.to_string(),
        value,
        source: source.to_string(),
        observed_at: observed_at.to_string(),
    }
}

fn condition_fingerprint(kind: &str, values: &[String]) -> String {
    format!("{kind}|{}", values.join("|"))
}

fn condition(
    repository_id: &str,
    kind: &str,
    title: &str,
    summary: String,
    priority: u8,
    fingerprint: String,
    rule: &str,
    evidence: Vec<EvidenceItem>,
    missing: Vec<String>,
    confidence: Option<&str>,
    freshness: Option<String>,
    expected: &[ExpectedCondition],
) -> Condition {
    let id = format!("{repository_id}:{kind}");
    let status = expected.iter().any(|item| {
        item.repository_id == repository_id
            && item.condition_id == id
            && item.fingerprint == fingerprint
    });
    Condition {
        id,
        kind: kind.to_string(),
        title: title.to_string(),
        summary,
        priority,
        status: if status {
            "Expected".to_string()
        } else {
            "Active".to_string()
        },
        fingerprint,
        rule: rule.to_string(),
        evidence,
        missing,
        confidence: confidence.map(str::to_string),
        freshness,
    }
}

fn build_conditions(
    repository_id: &str,
    workspace: &WorkspaceSummary,
    default_branch: Option<&str>,
    expected: &[ExpectedCondition],
    observed_at: &str,
) -> Vec<Condition> {
    let mut conditions = Vec::new();
    if !workspace.status_available {
        let status_error = workspace_status_unavailable_reason(workspace);
        conditions.push(condition(
            repository_id,
            "git-status-unavailable",
            "Git status unavailable",
            status_error.clone(),
            1,
            condition_fingerprint(
                "git-status-unavailable",
                &[workspace.path.clone(), status_error.clone()],
            ),
            "Pronto could not read the workspace's local Git status; branch, cleanliness, upstream, and sync state are unknown.",
            vec![evidence(
                "Git status",
                status_error,
                "git status --porcelain=v2",
                observed_at,
            )],
            vec!["Restore local Git status access, then run a scoped refresh.".to_string()],
            Some("High"),
            None,
            expected,
        ));
    }
    if workspace.activity.state == "Active" {
        let signal_evidence = workspace
            .activity
            .signals
            .iter()
            .map(|signal| {
                evidence(
                    "Activity signal",
                    format!("{} · {}", signal.source, signal.summary),
                    "Local process and manifest metadata",
                    observed_at,
                )
            })
            .collect::<Vec<_>>();
        conditions.push(condition(
            repository_id,
            "active-agent-workspace",
            "Agent workspace active",
            "Process or manifest evidence indicates this workspace is still active.".to_string(),
            2,
            condition_fingerprint(
                "active-agent",
                &[
                    workspace.branch.clone(),
                    workspace.activity.confidence.clone(),
                ],
            ),
            "An associated process or active agent manifest was detected without capturing terminal contents, prompts, or source text.",
            signal_evidence,
            vec!["Wait for the activity signal to end before integration or cleanup.".to_string()],
            Some(&workspace.activity.confidence),
            None,
            expected,
        ));
    }
    if let Some(operation) = &workspace.operation {
        conditions.push(condition(
            repository_id,
            "interrupted-operation",
            "Interrupted Git operation",
            operation.clone(),
            1,
            condition_fingerprint("operation", std::slice::from_ref(operation)),
            "A Git operation marker exists in the workspace metadata.",
            vec![evidence(
                "Operation",
                operation.clone(),
                "Git operation marker",
                observed_at,
            )],
            vec!["The operation must be completed or resolved outside Pronto.".to_string()],
            Some("High"),
            None,
            expected,
        ));
    }
    if workspace.dirty {
        let summary = if workspace.line_totals_partial {
            "Dirty · line totals partial".to_string()
        } else {
            format!("Dirty · +{} / −{}", workspace.added, workspace.removed)
        };
        conditions.push(condition(
      repository_id,
      "dirty-workspace",
      "Dirty workspace",
      summary,
      2,
      condition_fingerprint(
        "dirty",
        &[
          workspace.branch.clone(),
          workspace.added.to_string(),
          workspace.removed.to_string(),
          workspace.line_totals_partial.to_string(),
        ],
      ),
      "The local Git status contains uncommitted work; line totals are aggregated without exposing filenames or diff content.",
      vec![
        evidence("Branch", workspace.branch.clone(), "git status --porcelain=v2", observed_at),
        evidence("Added lines", workspace.added.to_string(), "git diff --numstat", observed_at),
        evidence("Removed lines", workspace.removed.to_string(), "git diff --numstat", observed_at),
      ],
      if workspace.line_totals_partial {
        vec!["Binary, unreadable, or oversized changes were not fully countable.".to_string()]
      } else {
        Vec::new()
      },
      Some("High"),
      None,
      expected,
    ));
    }
    if workspace.upstream.is_some() && workspace.ahead > 0 && workspace.behind > 0 {
        conditions.push(condition(
      repository_id,
      "diverged-branch",
      "Diverged branch",
      workspace.sync_state.clone(),
      3,
      condition_fingerprint(
        "diverged",
        &[workspace.branch.clone(), workspace.ahead.to_string(), workspace.behind.to_string()],
      ),
      "The local branch and its tracked upstream each contain commits the other side cannot reach.",
      vec![
        evidence("Ahead", workspace.ahead.to_string(), "git status --porcelain=v2", observed_at),
        evidence("Behind", workspace.behind.to_string(), "git status --porcelain=v2", observed_at),
      ],
      Vec::new(),
      Some("High"),
      Some(workspace.remote_freshness.clone()),
      expected,
    ));
    } else if workspace.upstream.is_some() && workspace.ahead > 0 {
        conditions.push(condition(
            repository_id,
            "unpushed-commits",
            "Unpushed commits",
            workspace.sync_state.clone(),
            5,
            condition_fingerprint(
                "ahead",
                &[workspace.branch.clone(), workspace.ahead.to_string()],
            ),
            "The local branch is ahead of its tracked upstream.",
            vec![evidence(
                "Ahead",
                workspace.ahead.to_string(),
                "git status --porcelain=v2",
                observed_at,
            )],
            Vec::new(),
            Some("High"),
            Some(workspace.remote_freshness.clone()),
            expected,
        ));
    } else if workspace.upstream.is_some() && workspace.behind > 0 {
        conditions.push(condition(
            repository_id,
            "behind-remote",
            "Behind tracked branch",
            workspace.sync_state.clone(),
            6,
            condition_fingerprint(
                "behind",
                &[workspace.branch.clone(), workspace.behind.to_string()],
            ),
            "The local branch is behind its tracked upstream.",
            vec![evidence(
                "Behind",
                workspace.behind.to_string(),
                "git status --porcelain=v2",
                observed_at,
            )],
            Vec::new(),
            Some("High"),
            Some(workspace.remote_freshness.clone()),
            expected,
        ));
    }
    if workspace.upstream.is_some() && workspace.remote_freshness == "Not fetched by Pronto" {
        conditions.push(condition(
            repository_id,
            "remote-stale",
            "Remote state stale",
            "Pronto has not recorded a successful fetch for this tracked branch.".to_string(),
            8,
            condition_fingerprint("remote-stale", std::slice::from_ref(&workspace.branch)),
            "Remote comparisons remain explicitly stale until Pronto records a successful fetch.",
            vec![evidence(
                "Freshness",
                workspace.remote_freshness.clone(),
                "Local Pronto state",
                observed_at,
            )],
            vec!["Run an explicit refresh when network access is available.".to_string()],
            Some("High"),
            Some(workspace.remote_freshness.clone()),
            expected,
        ));
    }
    if workspace.status_available
        && workspace.upstream.is_none()
        && default_branch.is_some_and(|default| default != workspace.branch)
    {
        conditions.push(condition(
            repository_id,
            "no-upstream",
            "No upstream",
            format!("{} has no tracked remote branch.", workspace.branch),
            5,
            condition_fingerprint("no-upstream", std::slice::from_ref(&workspace.branch)),
            "The current non-default branch has no configured upstream.",
            vec![evidence(
                "Branch",
                workspace.branch.clone(),
                "git status --porcelain=v2",
                observed_at,
            )],
            Vec::new(),
            Some("High"),
            None,
            expected,
        ));
    }
    if workspace.integration_state == "Integration eligible" {
        conditions.push(condition(
      repository_id,
      "integration-eligible",
      "Integration eligible",
      format!("{} has unique commits relative to {} and its workspace is clean.", workspace.branch, workspace.target_branch.clone().unwrap_or_else(|| "the target".to_string())),
      7,
      condition_fingerprint(
        "integration",
        &[
          workspace.branch.clone(),
          workspace.target_branch.clone().unwrap_or_default(),
          workspace.integration_state.clone(),
        ],
      ),
      "The branch has unique commits, a known target, a clean workspace, no detected operation, and no provider requirement known locally.",
      vec![
        evidence("Branch role", workspace.role.clone(), "Local branch name and default branch", observed_at),
        evidence("Target", workspace.target_branch.clone().unwrap_or_default(), "Local default branch", observed_at),
        evidence("Workspace", "Clean".to_string(), "git status --porcelain=v2", observed_at),
      ],
      vec!["GitHub checks and pull-request permissions are not connected in this slice.".to_string()],
      Some(&workspace.target_confidence),
      None,
      expected,
    ));
    }
    conditions.sort_by_key(|item| item.priority);
    conditions
}

fn observed_fetch_at(path: &Path) -> Option<String> {
    let fetch_head = git_static(path, &["rev-parse", "--git-path", "FETCH_HEAD"])?;
    let fetch_head = PathBuf::from(fetch_head);
    let fetch_head = if fetch_head.is_absolute() {
        fetch_head
    } else {
        path.join(fetch_head)
    };
    let modified = fs::metadata(fetch_head).ok()?.modified().ok()?;
    Some(DateTime::<Utc>::from(modified).to_rfc3339_opts(SecondsFormat::Secs, true))
}
