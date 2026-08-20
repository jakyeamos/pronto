fn release_rule_commit_type_present(
    commits: &[ReleaseCommitSummary],
    requested_types: &[String],
) -> bool {
    requested_types.iter().any(|requested| {
        commits.iter().any(|commit| match requested.as_str() {
            "breaking" => commit.category == "Breaking",
            "feat" => commit.category == "Features",
            "fix" => commit.category == "Fixes",
            "perf" => commit.category == "Performance",
            requested => commit
                .subject
                .split_once(':')
                .map(|(kind, _)| kind.trim().trim_end_matches('!') == requested)
                .unwrap_or(false),
        })
    })
}

fn evaluate_release_rule(
    rule: &ReleaseRuleConfig,
    baseline: Option<&ReleaseSnapshot>,
    commits: &[ReleaseCommitSummary],
) -> (ReleaseRuleResult, Vec<ReleaseRuleTrace>) {
    let mut results = Vec::new();
    let mut trace = Vec::new();
    let (baseline_result, baseline_value) = if !release_rule_needs_baseline(rule) {
        (
            ReleaseRuleResult::Passed,
            "No commit threshold clauses · quality evidence drives this rule".to_string(),
        )
    } else {
        match baseline {
            Some(release) => (
                ReleaseRuleResult::Passed,
                format!("Published baseline {}", release.tag),
            ),
            None if rule.allow_first_release => (
                ReleaseRuleResult::Passed,
                "No published baseline · first-release path enabled".to_string(),
            ),
            None => (
                ReleaseRuleResult::Failed,
                "No published baseline · first-release path not enabled".to_string(),
            ),
        }
    };
    results.push(baseline_result);
    trace.push(ReleaseRuleTrace {
        label: "Published baseline".to_string(),
        status: release_rule_status(baseline_result).to_string(),
        value: baseline_value,
        source: "Published GitHub Release snapshot and local rule configuration".to_string(),
    });

    if let Some(min_commits) = rule.min_commits {
        let result = if commits.len() as u64 >= min_commits {
            ReleaseRuleResult::Passed
        } else {
            ReleaseRuleResult::Failed
        };
        results.push(result);
        trace.push(ReleaseRuleTrace {
            label: format!("At least {min_commits} commits"),
            status: release_rule_status(result).to_string(),
            value: format!("{} commits since baseline", commits.len()),
            source: "git log".to_string(),
        });
    }

    if let Some(min_elapsed_days) = rule.min_elapsed_days {
        let (result, value) = match baseline.and_then(|release| release.published_at.as_deref()) {
            Some(published_at) => match DateTime::parse_from_rfc3339(published_at) {
                Ok(date) => {
                    let elapsed_days = Utc::now()
                        .signed_duration_since(date.with_timezone(&Utc))
                        .num_days()
                        .max(0);
                    (
                        if elapsed_days >= min_elapsed_days as i64 {
                            ReleaseRuleResult::Passed
                        } else {
                            ReleaseRuleResult::Failed
                        },
                        format!("{elapsed_days} days since publication"),
                    )
                }
                Err(_) => (
                    ReleaseRuleResult::Unknown,
                    "Published timestamp could not be parsed".to_string(),
                ),
            },
            None => (
                ReleaseRuleResult::Unknown,
                "Published timestamp unavailable".to_string(),
            ),
        };
        results.push(result);
        trace.push(ReleaseRuleTrace {
            label: format!("At least {min_elapsed_days} elapsed days"),
            status: release_rule_status(result).to_string(),
            value,
            source: "Published GitHub Release timestamp".to_string(),
        });
    }

    if !rule.required_commit_types.is_empty() {
        let result = if release_rule_commit_type_present(commits, &rule.required_commit_types) {
            ReleaseRuleResult::Passed
        } else {
            ReleaseRuleResult::Failed
        };
        results.push(result);
        trace.push(ReleaseRuleTrace {
            label: "Configured commit type present".to_string(),
            status: release_rule_status(result).to_string(),
            value: format!(
                "{} in {}",
                rule.required_commit_types.join(", "),
                commits.len()
            ),
            source: "Deterministic conventional-commit mapping".to_string(),
        });
    }

    (
        combine_release_rule_results(&rule.operator, &results),
        trace,
    )
}

fn evaluate_release_rule_with_quality(
    repository: &RepositorySnapshot,
    rule: &ReleaseRuleConfig,
    baseline: Option<&ReleaseSnapshot>,
    commits: &[ReleaseCommitSummary],
) -> (ReleaseRuleResult, Vec<ReleaseRuleTrace>) {
    let (base_result, mut trace) = evaluate_release_rule(rule, baseline, commits);
    let mut quality_result = ReleaseRuleResult::Passed;
    for requirement in &rule.required_quality_gates {
        let (status, freshness, detail) = quality::evaluate_requirement(repository, requirement);
        let result = match status {
            QualityGateStatus::Failed => ReleaseRuleResult::Failed,
            QualityGateStatus::Passed if freshness == QualityFreshness::Fresh => {
                ReleaseRuleResult::Passed
            }
            QualityGateStatus::Passed => ReleaseRuleResult::Blocked,
            QualityGateStatus::Blocked | QualityGateStatus::NotConfigured => {
                ReleaseRuleResult::Blocked
            }
        };
        if requirement.policy == quality::QualityRequirementPolicy::Block {
            if result == ReleaseRuleResult::Failed {
                quality_result = ReleaseRuleResult::Failed;
            } else if result == ReleaseRuleResult::Blocked
                && quality_result == ReleaseRuleResult::Passed
            {
                quality_result = ReleaseRuleResult::Blocked;
            }
        }
        let minimum_level = requirement
            .minimum_verification_level
            .as_ref()
            .map(quality::QualityVerificationLevel::as_str)
            .unwrap_or("any");
        trace.push(ReleaseRuleTrace {
            label: format!(
                "Quality gate · {} · {} · {}",
                quality::gate_label(&requirement.gate_id),
                requirement.source.as_str(),
                match requirement.policy {
                    quality::QualityRequirementPolicy::Block => "Block",
                    quality::QualityRequirementPolicy::Warn => "Warn",
                }
            ),
            status: format!("{} · {}", status.as_str(), freshness.as_str()),
            value: format!("{detail} · minimum verification: {minimum_level}"),
            source: format!("Imported {} evidence", requirement.source.as_str()),
        });
    }
    let result = if quality_result == ReleaseRuleResult::Passed {
        base_result
    } else {
        quality_result
    };
    (result, trace)
}

fn release_threshold_condition(
    repository: &RepositorySnapshot,
    provider_ready: bool,
    expected: &[ExpectedCondition],
    observed_at: &str,
) -> Option<Condition> {
    let rule = repository.release_rule.as_ref()?;
    if !provider_context_available(repository, provider_ready)
        && rule.required_quality_gates.is_empty()
    {
        return None;
    }
    let baseline = latest_published_release(repository);
    let base = baseline
        .as_ref()
        .and_then(|release| release.target_commit.as_deref())
        .or(repository.workspace.target_branch.as_deref());
    let commits = match base {
        Some(base) => release_commits(
            Path::new(&repository.workspace.path),
            base,
            &repository.workspace.branch,
        )
        .ok()?,
        None => Vec::new(),
    };
    let (result, trace) =
        evaluate_release_rule_with_quality(repository, rule, baseline.as_ref(), &commits);
    if result != ReleaseRuleResult::Passed {
        return None;
    }
    let trace_evidence = trace
        .iter()
        .map(|item| {
            evidence(
                item.label.as_str(),
                format!("{} · {}", item.status, item.value),
                "Deterministic release rule trace",
                observed_at,
            )
        })
        .collect::<Vec<_>>();
    Some(condition(
        &repository.id,
        "release-threshold",
        "Configured release threshold met",
        format!("{} passed for {}.", rule.name, repository.name),
        4,
        condition_fingerprint(
            "release-threshold",
            &[
                rule.name.clone(),
                rule.operator.clone(),
                base.unwrap_or_default().to_string(),
                repository.workspace.branch.clone(),
                commits.len().to_string(),
                rule.required_quality_gates
                    .iter()
                    .map(|requirement| {
                        format!("{}:{}", requirement.gate_id, requirement.source.as_str())
                    })
                    .collect::<Vec<_>>()
                    .join(","),
            ],
        ),
        "A user-configured deterministic release rule evaluated true using the published baseline, committed range, and configured clauses.",
        trace_evidence,
        Vec::new(),
        Some("High"),
        repository.last_fetch_at.clone(),
        expected,
    ))
}

fn apply_release_threshold_conditions(state: &mut StoreState) {
    let observed_at = iso_now();
    let provider_ready = state.provider_status.state == "Ready";
    for repository in &mut state.repositories {
        repository
            .conditions
            .retain(|condition| condition.kind != "release-threshold");
        if let Some(threshold) = release_threshold_condition(
            repository,
            provider_ready,
            &state.expected_conditions,
            &observed_at,
        ) {
            repository.conditions.push(threshold);
            repository.conditions.sort_by_key(|item| item.priority);
        }
    }
}

fn provider_context_available(repository: &RepositorySnapshot, provider_ready: bool) -> bool {
    provider_ready && repository.provider_state.starts_with("GitHub connected")
}

fn prepare_pull_request(
    repository: &RepositorySnapshot,
    workspace: &WorkspaceSummary,
    provider_available: bool,
) -> PullRequestPreparation {
    let observed_at = iso_now();
    let base_branch = workspace.target_branch.clone();
    let commit_count = unique_commits(
        Path::new(&workspace.path),
        &workspace.branch,
        base_branch.as_deref(),
    );
    let existing_pull_request = repository
        .pull_requests
        .iter()
        .filter(|pull_request| {
            pull_request.state.eq_ignore_ascii_case("open")
                && pull_request.head_branch == workspace.branch
                && base_branch
                    .as_deref()
                    .is_some_and(|base| pull_request.base_branch == base)
        })
        .max_by_key(|pull_request| pull_request.number)
        .cloned();
    let checks_state = existing_pull_request
        .as_ref()
        .map(|pull_request| pull_request.checks_state.clone())
        .unwrap_or_else(|| "Unknown — provider snapshot unavailable".to_string());
    let reviews_state = existing_pull_request
        .as_ref()
        .map(|pull_request| pull_request.reviews_state.clone())
        .unwrap_or_else(|| "Unknown — provider snapshot unavailable".to_string());
    let mergeability = existing_pull_request
        .as_ref()
        .map(|pull_request| pull_request.mergeability.clone())
        .unwrap_or_else(|| "Unknown — provider snapshot unavailable".to_string());
    let mut reasons = Vec::new();
    if base_branch.is_none() {
        reasons.push("Target branch is unknown".to_string());
    }
    if !workspace.status_available {
        reasons.push(workspace_status_unavailable_reason(workspace));
    }
    if commit_count == 0 {
        reasons.push("Branch has no unique commits relative to the target".to_string());
    }
    if workspace.dirty {
        reasons.push("Workspace has uncommitted changes".to_string());
    }
    if !provider_available {
        reasons.push(
            "GitHub provider context is unavailable; pull request creation remains blocked"
                .to_string(),
        );
    }
    let mut evidence_items = vec![
        evidence(
            "Head branch",
            workspace.branch.clone(),
            "Local workspace scan",
            &observed_at,
        ),
        evidence(
            "Base branch",
            base_branch.clone().unwrap_or_else(|| "Unknown".to_string()),
            "Workspace target inference",
            &observed_at,
        ),
        evidence(
            "Commit count",
            commit_count.to_string(),
            "git rev-list",
            &observed_at,
        ),
        evidence(
            "Workspace",
            if !workspace.status_available {
                workspace_status_unavailable_reason(workspace)
            } else if workspace.dirty {
                "Dirty · commit preparation blocked".to_string()
            } else {
                "Clean".to_string()
            },
            "git status --porcelain=v2",
            &observed_at,
        ),
        evidence(
            "Push state",
            workspace.sync_state.clone(),
            "git status --porcelain=v2",
            &observed_at,
        ),
        evidence(
            "Provider",
            repository.provider_state.clone(),
            "Local provider snapshot",
            &observed_at,
        ),
    ];
    if let Some(pull_request) = existing_pull_request.as_ref() {
        evidence_items.push(evidence(
            "Existing pull request",
            format!("#{} · {}", pull_request.number, pull_request.title),
            "Stored provider snapshot",
            &observed_at,
        ));
    }
    PullRequestPreparation {
        repository_id: repository.id.clone(),
        workspace_id: workspace.id.clone(),
        head_branch: workspace.branch.clone(),
        base_branch,
        commit_count,
        status_available: workspace.status_available,
        status_error: workspace.status_error.clone(),
        dirty: workspace.dirty,
        ahead: workspace.ahead,
        behind: workspace.behind,
        upstream: workspace.upstream.clone(),
        provider_state: repository.provider_state.clone(),
        checks_state,
        reviews_state,
        mergeability,
        status: if reasons.is_empty() {
            "Evidence ready".to_string()
        } else {
            "Blocked".to_string()
        },
        reasons,
        evidence: evidence_items,
        existing_pull_request,
    }
}
