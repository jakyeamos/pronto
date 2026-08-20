fn prepare_release(
    repository: &RepositorySnapshot,
    workspace: &WorkspaceSummary,
    provider_available: bool,
) -> ReleasePreparation {
    let observed_at = iso_now();
    let public_release_boundary_required =
        remediation::repository_requires_public_release_boundary(repository);
    let release_boundary = &repository.quality.release_boundary;
    let release_boundary_ready =
        !public_release_boundary_required || release_boundary.is_release_ready();
    let behavior_assurance = &repository.quality.behavior_assurance;
    let behavior_assurance_ready = behavior_assurance.release_ready;
    let target_branch = repository
        .target_branch
        .clone()
        .or_else(|| repository.default_branch.clone())
        .or_else(|| workspace.target_branch.clone());
    let connected = provider_context_available(repository, provider_available);
    let baseline = connected
        .then(|| latest_published_release(repository))
        .flatten();
    let baseline_status = if !connected {
        "Provider release data unavailable".to_string()
    } else if baseline.is_some() {
        "Published release baseline".to_string()
    } else {
        "No published release baseline".to_string()
    };
    let range_base = baseline
        .as_ref()
        .and_then(|release| release.target_commit.as_deref())
        .or(target_branch.as_deref());
    let (commits_since_baseline, commit_range_error) = match range_base {
        Some(base) => match release_commits(Path::new(&workspace.path), base, &workspace.branch) {
            Ok(commits) => (commits, None),
            Err(error) => (Vec::new(), Some(error)),
        },
        None => (Vec::new(), None),
    };
    let candidate_bump = highest_release_bump(&commits_since_baseline);
    let candidate_version = baseline
        .as_ref()
        .and_then(|release| candidate_version(release, candidate_bump.as_deref()));
    let mut grouped = BTreeMap::<String, Vec<ReleaseCommitSummary>>::new();
    for commit in &commits_since_baseline {
        grouped
            .entry(commit.category.clone())
            .or_default()
            .push(commit.clone());
    }
    let notes = grouped
        .into_iter()
        .map(|(category, commits)| ReleaseNoteSection { category, commits })
        .collect::<Vec<_>>();
    let configured_rule = repository.release_rule.as_ref();
    let (rule_result, rule_trace) = if commit_range_error.is_some() {
        (Some(ReleaseRuleResult::Unknown), Vec::new())
    } else if connected
        || configured_rule.is_some_and(|rule| !rule.required_quality_gates.is_empty())
    {
        configured_rule
            .map(|rule| {
                evaluate_release_rule_with_quality(
                    repository,
                    rule,
                    baseline.as_ref(),
                    &commits_since_baseline,
                )
            })
            .map_or((None, Vec::new()), |(result, trace)| (Some(result), trace))
    } else {
        (None, Vec::new())
    };
    let rule_status = if !connected
        && !configured_rule.is_some_and(|rule| !rule.required_quality_gates.is_empty())
    {
        "Unknown — provider release data unavailable".to_string()
    } else if let Some(result) = rule_result {
        match result {
            ReleaseRuleResult::Passed => "Configured release threshold met".to_string(),
            ReleaseRuleResult::Failed => "Configured release threshold not met".to_string(),
            ReleaseRuleResult::Blocked => "Release rule blocked by quality evidence".to_string(),
            ReleaseRuleResult::Unknown => "Release threshold evidence incomplete".to_string(),
        }
    } else {
        "Not configured — commits are shown without threshold evaluation".to_string()
    };
    let mut reasons = Vec::new();
    if let Some(error) = commit_range_error.as_ref() {
        reasons.push(error.clone());
    }
    if target_branch.is_none() {
        reasons.push("Target branch is unknown".to_string());
    }
    if !workspace.status_available {
        reasons.push(workspace_status_unavailable_reason(workspace));
    }
    if !connected {
        reasons.push(
            "Published GitHub release data is unavailable; no release threshold is evaluated"
                .to_string(),
        );
    } else if baseline.is_none() && configured_rule.is_some_and(release_rule_needs_baseline) {
        if configured_rule.is_none() {
            reasons.push("First-release rule is not confirmed".to_string());
        } else if configured_rule.is_some_and(|rule| !rule.allow_first_release) {
            reasons.push("First-release rule is not enabled".to_string());
        }
    }
    if workspace.dirty {
        reasons.push(
            "Workspace has uncommitted changes; release preparation cannot start".to_string(),
        );
    }
    if rule_result == Some(ReleaseRuleResult::Failed) {
        reasons.push("Configured release threshold did not pass".to_string());
    }
    if rule_result == Some(ReleaseRuleResult::Blocked) {
        reasons.push(
            "Required quality evidence is blocked, stale, missing, or conflicting".to_string(),
        );
    }
    if rule_result == Some(ReleaseRuleResult::Unknown) {
        reasons.push("Release threshold evidence is incomplete".to_string());
    }
    if public_release_boundary_required && !release_boundary_ready {
        reasons.push(format!(
            "Public-release boundary evidence is {} and {}; regenerate the v2 receipt for this exact target",
            release_boundary.status.to_ascii_lowercase(),
            release_boundary.freshness.to_ascii_lowercase()
        ));
    }
    if !behavior_assurance_ready {
        reasons.push(format!(
            "Behavior assurance is {} · {} · {}; resolve the Tier-0 contract and receipt gaps before release",
            behavior_assurance.contract_status,
            behavior_assurance.result_status,
            behavior_assurance.freshness
        ));
    }
    let mut evidence_items = vec![
        evidence(
            "Target branch",
            target_branch
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            "Configured repository target or observed Git default",
            &observed_at,
        ),
        evidence(
            "Baseline",
            baseline_status.clone(),
            "Published GitHub Release snapshot",
            &observed_at,
        ),
        evidence(
            "Commits since baseline",
            commit_range_error
                .as_ref()
                .map(|error| format!("Unknown · {error}"))
                .unwrap_or_else(|| commits_since_baseline.len().to_string()),
            &format!(
                "bounded git log · max {RELEASE_COMMIT_LIMIT} commits · {RELEASE_GIT_TIMEOUT_SECONDS}s deadline"
            ),
            &observed_at,
        ),
        evidence(
            "Rule",
            rule_status.clone(),
            "Local release configuration",
            &observed_at,
        ),
    ];
    if let Some(bump) = candidate_bump.as_ref() {
        evidence_items.push(evidence(
            "Candidate bump",
            bump.clone(),
            "Deterministic conventional-commit mapping",
            &observed_at,
        ));
    }
    if !workspace.status_available {
        evidence_items.push(evidence(
            "Workspace",
            workspace_status_unavailable_reason(workspace),
            "git status --porcelain=v2",
            &observed_at,
        ));
    } else if workspace.dirty {
        evidence_items.push(evidence(
            "Starting state",
            "Dirty · release preparation blocked".to_string(),
            "git status --porcelain=v2",
            &observed_at,
        ));
    }
    let version_status = match (
        candidate_version.as_ref(),
        repository.confirmed_release_version.as_ref(),
    ) {
        (Some(candidate), Some(confirmed)) if candidate == confirmed => {
            "Candidate version confirmed".to_string()
        }
        (Some(_), Some(_)) => "Confirmed version does not match current candidate".to_string(),
        (Some(_), None) => "Candidate requires user confirmation".to_string(),
        (None, Some(_)) => "Confirmed version has no current candidate".to_string(),
        (None, None) => {
            "Candidate unavailable until a published baseline and deterministic bump exist"
                .to_string()
        }
    };
    if candidate_version.is_some() {
        if repository.confirmed_release_version.is_none() {
            reasons.push("Candidate version requires explicit user confirmation".to_string());
        } else if repository.confirmed_release_version != candidate_version {
            reasons.push(
                "Stored release version confirmation does not match the candidate".to_string(),
            );
        }
    } else if repository.confirmed_release_version.is_some() {
        reasons.push("Stored release version confirmation has no current candidate".to_string());
    }
    evidence_items.push(evidence(
        "Version confirmation",
        version_status.clone(),
        "Deterministic candidate and local user confirmation",
        &observed_at,
    ));
    if public_release_boundary_required {
        evidence_items.push(evidence(
            "Public-release boundary",
            format!(
                "{} · {}{}",
                release_boundary.status,
                release_boundary.freshness,
                if release_boundary.blocking_check_ids.is_empty() {
                    String::new()
                } else {
                    format!(
                        " · blocking: {}",
                        release_boundary.blocking_check_ids.join(", ")
                    )
                }
            ),
            release_boundary
                .report_path
                .as_deref()
                .unwrap_or(".quality-runner/release-boundary.json"),
            &observed_at,
        ));
    }
    evidence_items.push(evidence(
        "Behavior assurance",
        format!(
            "{} · {} · {} · {}/{} required scenarios",
            behavior_assurance.contract_status,
            behavior_assurance.result_status,
            behavior_assurance.freshness,
            behavior_assurance.passed_scenario_count,
            behavior_assurance.required_scenario_count
        ),
        behavior_assurance
            .detail
            .as_deref()
            .unwrap_or("Quality Runner behavior-assurance projection"),
        &observed_at,
    ));
    let missing_baseline =
        baseline.is_none() && configured_rule.is_some_and(release_rule_needs_baseline);
    let blocked = target_branch.is_none()
        || !connected
        || !workspace.status_available
        || workspace.dirty
        || !release_boundary_ready
        || !behavior_assurance_ready
        || commit_range_error.is_some()
        || (missing_baseline && configured_rule.is_none())
        || (missing_baseline && configured_rule.is_some_and(|rule| !rule.allow_first_release))
        || rule_result == Some(ReleaseRuleResult::Failed)
        || rule_result == Some(ReleaseRuleResult::Blocked)
        || rule_result == Some(ReleaseRuleResult::Unknown);
    let recommendation = release_recommendation(
        baseline.as_ref(),
        &commits_since_baseline,
        candidate_bump.as_deref(),
        candidate_version.as_deref(),
        rule_result,
        blocked,
    );
    evidence_items.push(evidence(
        "Release recommendation",
        recommendation.label.clone(),
        &format!(
            "Advisory only · {} · readiness gates override commit classification",
            recommendation.basis
        ),
        &observed_at,
    ));
    ReleasePreparation {
        repository_id: repository.id.clone(),
        target_branch,
        baseline_status,
        baseline,
        commits_since_baseline,
        rule_status,
        threshold_label: configured_rule.map(|rule| rule.name.clone()),
        rule_trace,
        candidate_bump,
        candidate_version,
        version_status,
        recommendation,
        release_boundary_status: public_release_boundary_required.then(|| {
            format!(
                "{} · {}",
                release_boundary.status, release_boundary.freshness
            )
        }),
        notes,
        status: if blocked {
            if connected && missing_baseline && configured_rule.is_none() {
                "First-release rule not confirmed".to_string()
            } else {
                "Blocked".to_string()
            }
        } else if rule_result == Some(ReleaseRuleResult::Failed) {
            "Threshold not met".to_string()
        } else {
            "Evidence ready".to_string()
        },
        reasons,
        evidence: evidence_items,
    }
}
