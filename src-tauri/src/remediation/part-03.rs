fn goal_queue_rank(target_state: &str) -> u8 {
    match target_state {
        "public_release" => 0,
        "deployed_product" => 1,
        "active_maintained" => 2,
        "github_only" => 3,
        "clean_only" => 4,
        "prototype" => 5,
        "archived" => 6,
        _ => 7,
    }
}

pub fn empty_run() -> RemediationRun {
    RemediationRun {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        status: "not_run".to_string(),
        ..RemediationRun::default()
    }
}

pub fn sync_github_only_candidates(
    run: &mut RemediationRun,
    remote_repositories: &[RemoteRepositorySnapshot],
) {
    let mut candidates = remote_repositories
        .iter()
        .filter(|repository| {
            repository.provider.eq_ignore_ascii_case("github")
                && repository
                    .locality
                    .eq_ignore_ascii_case(GITHUB_ONLY_LOCALITY)
        })
        .map(|repository| GitHubOnlyCandidate {
            repository_id: repository.id.clone(),
            provider: repository.provider.clone(),
            full_name: repository.full_name.clone(),
            html_url: repository.html_url.clone(),
            archived: repository.archived,
            label: GITHUB_ONLY_LOCALITY.to_string(),
            status: "candidate".to_string(),
            last_remediation_task: GITHUB_ONLY_REMEDIATION_TASK.to_string(),
            observed_at: repository.last_refreshed_at.clone(),
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.full_name.cmp(&right.full_name));
    run.github_only_candidates = candidates;
}

pub fn rebuild_run(
    repositories: &[RepositorySnapshot],
    previous: &RemediationRun,
    source_refresh_id: Option<&str>,
) -> RemediationRun {
    rebuild_run_with_fleet_root(repositories, previous, source_refresh_id, None)
}

pub fn rebuild_run_with_fleet_root(
    repositories: &[RepositorySnapshot],
    previous: &RemediationRun,
    source_refresh_id: Option<&str>,
    fleet_audit_root: Option<&Path>,
) -> RemediationRun {
    let generated_at = Utc::now().to_rfc3339();
    let previous_by_repository = previous
        .plans
        .iter()
        .map(|plan| (plan.repository_id.as_str(), plan))
        .collect::<HashMap<_, _>>();
    let mut closures = previous.closures.clone();
    let mut exclusions = Vec::new();
    let mut plans = Vec::new();
    let mut eligible_repository_ids = Vec::new();
    let mut eligible_repository_paths = Vec::new();
    for repository in repositories {
        if let Some(reason) = exclusion_reason(repository) {
            exclusions.push(RemediationExclusion {
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                reason,
            });
            continue;
        }
        eligible_repository_ids.push(repository.id.clone());
        eligible_repository_paths.push(repository.path.clone());
        let previous_plan = previous_by_repository.get(repository.id.as_str()).copied();
        let plan = build_plan(
            repository,
            previous_plan,
            source_refresh_id,
            &generated_at,
            fleet_audit_root,
        );
        let retained_closure_is_current = previous_plan.is_none()
            && closures
                .iter()
                .filter(|closure| closure.repository_id == repository.id)
                .max_by(|left, right| left.closed_at.cmp(&right.closed_at))
                .is_some_and(|closure| closure_covers_plan(closure, &plan));
        if retained_closure_is_current {
            continue;
        }
        if plan_is_terminal(&plan) {
            if let Some(previous_plan) = previous_plan {
                closures.push(closure_from_transition(
                    repository,
                    previous_plan,
                    &plan,
                    &generated_at,
                    source_refresh_id,
                ));
            }
        } else {
            plans.push(plan);
        }
    }
    rank_active_plans(&mut plans);
    deduplicate_closures(&mut closures);
    exclusions.sort_by(|left, right| left.repository_name.cmp(&right.repository_name));
    eligible_repository_ids.sort();
    eligible_repository_paths.sort();
    let run_id = stable_id(
        &format!("run:{}:{}", generated_at, plans.len()),
        "remediation-run",
    );
    RemediationRun {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        id: run_id,
        generated_at,
        source_refresh_id: source_refresh_id.map(str::to_string),
        status: if previous.status.is_empty() {
            "not_run".to_string()
        } else {
            previous.status.clone()
        },
        message: previous.message.clone(),
        eligible_repository_ids,
        eligible_repository_paths,
        refresh_steps: previous.refresh_steps.clone(),
        excluded_repositories: exclusions,
        closures,
        github_only_candidates: previous.github_only_candidates.clone(),
        plans,
    }
}

pub fn sync_scope_metadata(run: &mut RemediationRun, repositories: &[RepositorySnapshot]) {
    let mut exclusions = Vec::new();
    let mut eligible_repository_ids = Vec::new();
    let mut eligible_repository_paths = Vec::new();
    for repository in repositories {
        if let Some(reason) = exclusion_reason(repository) {
            exclusions.push(RemediationExclusion {
                repository_id: repository.id.clone(),
                repository_name: repository.name.clone(),
                repository_path: repository.path.clone(),
                reason,
            });
        } else {
            eligible_repository_ids.push(repository.id.clone());
            eligible_repository_paths.push(repository.path.clone());
        }
    }
    exclusions.sort_by(|left, right| left.repository_name.cmp(&right.repository_name));
    eligible_repository_ids.sort();
    eligible_repository_paths.sort();
    run.excluded_repositories = exclusions;
    run.eligible_repository_ids = eligible_repository_ids;
    run.eligible_repository_paths = eligible_repository_paths;
}

pub fn set_refresh_metadata(
    run: &mut RemediationRun,
    refresh_id: &str,
    status: &str,
    message: Option<String>,
    eligible_repository_ids: Vec<String>,
    eligible_repository_paths: Vec<String>,
    refresh_steps: Vec<RemediationRefreshStep>,
) {
    run.source_refresh_id = Some(refresh_id.to_string());
    run.status = status.to_string();
    run.message = message;
    run.eligible_repository_ids = eligible_repository_ids;
    run.eligible_repository_paths = eligible_repository_paths;
    run.refresh_steps = refresh_steps;
}

pub fn recompute_plan_derived(plan: &mut RemediationPlan) {
    plan.progress = calculate_progress(&plan.actions);
    plan.tracks = build_tracks(&plan.actions);
    plan.status = plan_status(&plan.actions);
    plan.current_stage = current_stage(&plan.actions);
    plan.integration_only_remaining = integration_only_remaining(&plan.actions);
    plan.explanation = build_remediation_explanation(&plan.goal, &plan.actions, &plan.coverage);
}

pub fn sync_telescope_readiness(
    run: &mut RemediationRun,
    repository: &RepositorySnapshot,
    projection: &crate::telescope::TelescopeProjection,
) -> bool {
    let previous_plan = run
        .plans
        .iter()
        .find(|plan| plan.repository_id == repository.id)
        .cloned();
    let previous_actions = previous_plan
        .as_ref()
        .map(|plan| {
            plan.actions
                .iter()
                .map(|action| (action.stable_key.as_str(), action))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let current_keys = projection
        .knowledge_tasks
        .iter()
        .map(|task| task.stable_gap_key.as_str())
        .collect::<HashSet<_>>();
    let mut plan = build_plan(
        repository,
        previous_plan.as_ref(),
        Some(&projection.readiness_receipt.workspace_fingerprint),
        &projection.binding.generated_at,
        None,
    );

    // build_plan retains disappeared actions as verified history. Replace only
    // the still-current Telescope entries so status preservation and selective
    // reopening use the canonical remediation lifecycle.
    plan.actions.retain(|action| {
        action.domain != "telescope_readiness" || !current_keys.contains(action.stable_key.as_str())
    });
    for task in &projection.knowledge_tasks {
        let evidence = if task.evidence.is_empty() {
            vec![RemediationEvidence {
                source: "Pronto Telescope".to_string(),
                label: task
                    .stable_gap_key
                    .strip_prefix("telescope-readiness:")
                    .unwrap_or(&task.stable_gap_key)
                    .to_string(),
                status: "Open question".to_string(),
                freshness: task.freshness.clone(),
                observed_at: Some(projection.binding.generated_at.clone()),
                scanned_commit: projection.binding.commit.clone(),
                scanned_branch: Some(projection.binding.branch.clone()),
                report_path: None,
                detail: task.summary.clone(),
            }]
        } else {
            task.evidence
                .iter()
                .map(|anchor| RemediationEvidence {
                    source: "Pronto Telescope".to_string(),
                    label: task
                        .stable_gap_key
                        .strip_prefix("telescope-readiness:")
                        .unwrap_or(&task.stable_gap_key)
                        .to_string(),
                    status: "Source candidate".to_string(),
                    freshness: task.freshness.clone(),
                    observed_at: Some(projection.binding.generated_at.clone()),
                    scanned_commit: projection.binding.commit.clone(),
                    scanned_branch: Some(projection.binding.branch.clone()),
                    report_path: Some(anchor.path.clone()),
                    detail: task.summary.clone(),
                })
                .collect()
        };
        plan.actions.push(materialize_action(
            repository,
            ActionSeed {
                stable_key: task.stable_gap_key.clone(),
                domain: "telescope_readiness".to_string(),
                title: task.title.clone(),
                summary: format!("{} Unlocks: {}", task.question, task.unlocks.join(", ")),
                severity: if task.priority == "P1" {
                    "blocking"
                } else {
                    "enhancement"
                }
                .to_string(),
                priority: task.priority.clone(),
                weight: if task.priority == "P1" { 3 } else { 1 },
                acceptance_criteria: task.completion_criteria.clone(),
                evidence,
                related_finding_ids: Vec::new(),
                source_run_id: Some(projection.readiness_receipt.workspace_fingerprint.clone()),
            },
            &previous_actions,
            &projection.binding.generated_at,
        ));
    }
    recompute_plan_derived(&mut plan);

    let before = run
        .plans
        .iter()
        .find(|candidate| candidate.repository_id == repository.id)
        .and_then(|candidate| serde_json::to_value(candidate).ok());
    run.plans
        .retain(|candidate| candidate.repository_id != repository.id);
    if !plan_is_terminal(&plan) {
        run.plans.push(plan.clone());
    }
    rank_active_plans(&mut run.plans);
    let after = (!plan_is_terminal(&plan))
        .then(|| serde_json::to_value(&plan).ok())
        .flatten();
    before != after
}

pub fn normalize_queue(run: &mut RemediationRun, closed_at: &str) {
    let mut active_plans = Vec::new();
    for plan in std::mem::take(&mut run.plans) {
        if plan_is_terminal(&plan) {
            run.closures.push(closure_from_plan(
                &plan,
                closed_at,
                plan.source_refresh_id.as_deref(),
            ));
        } else {
            active_plans.push(plan);
        }
    }
    rank_active_plans(&mut active_plans);
    deduplicate_closures(&mut run.closures);
    run.plans = active_plans;
}

pub fn action_has_fresh_evidence(action: &RemediationAction) -> bool {
    action.evidence.iter().any(|item| {
        item.freshness.eq_ignore_ascii_case("fresh") && {
            let status = item.status.to_ascii_lowercase();
            ![
                "failed",
                "blocked",
                "missing",
                "stale",
                "unknown",
                "open",
                "dirty",
                "unconfirmed",
                "not configured",
            ]
            .iter()
            .any(|blocked| status.contains(blocked))
                && !status.contains("finding")
                && !status.contains("below target")
                && !status.contains("ahead")
                && !status.contains("behind")
        }
    })
}

fn plan_is_terminal(plan: &RemediationPlan) -> bool {
    matches!(plan.status.as_str(), "complete" | "deferred")
}

fn queue_status_rank(status: &str) -> u8 {
    match status {
        "blocked" => 0,
        "in_progress" => 1,
        _ => 2,
    }
}

fn queue_domain_rank(domain: &str) -> u8 {
    match domain {
        "scope" => 0,
        "product_truth" => 1,
        "branch_hygiene" => 2,
        "repository_health" => 3,
        "provider" => 4,
        "evidence_refresh" => 5,
        "ci_ideal" => 6,
        "qr_findings" => 7,
        "maturity" => 8,
        "verification" => 9,
        _ => 10,
    }
}

fn queue_priority_rank(priority: &str) -> u8 {
    match priority {
        "P0" => 0,
        "P1" => 1,
        "P2" => 2,
        "P3" => 3,
        _ => 4,
    }
}

fn queue_leverage(repository_name: &str) -> (u8, &'static str) {
    match repository_name.to_ascii_lowercase().as_str() {
        "pronto" => (0, "Fleet control plane"),
        "aios" => (1, "Agent coordination control plane"),
        "quality-runner" => (2, "Fleet evidence provider"),
        _ => (3, "Repository"),
    }
}

fn plan_queue_key(plan: &RemediationPlan) -> (u8, u8, u8, u8, u8, std::cmp::Reverse<u64>, String) {
    let active_actions = plan
        .actions
        .iter()
        .filter(|action| !matches!(action.status.as_str(), "verified" | "deferred"))
        .collect::<Vec<_>>();
    let domain_rank = active_actions
        .iter()
        .map(|action| queue_domain_rank(&action.domain))
        .min()
        .unwrap_or(u8::MAX);
    let priority_rank = active_actions
        .iter()
        .map(|action| queue_priority_rank(&action.priority))
        .min()
        .unwrap_or(u8::MAX);
    let active_weight = active_actions.iter().map(|action| action.weight).sum();
    (
        queue_status_rank(&plan.status),
        domain_rank,
        priority_rank,
        queue_leverage(&plan.repository_name).0,
        goal_queue_rank(&plan.goal.target_state),
        std::cmp::Reverse(active_weight),
        plan.repository_name.to_ascii_lowercase(),
    )
}

fn rank_active_plans(plans: &mut [RemediationPlan]) {
    plans.sort_by_key(plan_queue_key);
}

fn latest_plan_evidence_at(plan: &RemediationPlan) -> Option<String> {
    plan.actions
        .iter()
        .flat_map(|action| action.evidence.iter())
        .filter_map(|item| item.observed_at.clone())
        .max()
}

fn closure_covers_plan(closure: &RemediationClosure, plan: &RemediationPlan) -> bool {
    if closure.source_refresh_id != plan.source_refresh_id
        || closure.target_state != plan.goal.target_state
        || closure.goal_source != plan.goal.source
    {
        return false;
    }
    let Some(closed_at) = DateTime::parse_from_rfc3339(&closure.closed_at).ok() else {
        return false;
    };
    plan.actions
        .iter()
        .flat_map(|action| action.evidence.iter())
        .filter_map(|item| item.observed_at.as_deref())
        .all(|observed_at| {
            DateTime::parse_from_rfc3339(observed_at)
                .is_ok_and(|observed_at| observed_at <= closed_at)
        })
}

fn closure_from_plan(
    plan: &RemediationPlan,
    closed_at: &str,
    source_refresh_id: Option<&str>,
) -> RemediationClosure {
    let deferred_action_count = plan
        .actions
        .iter()
        .filter(|action| action.status == "deferred")
        .count();
    let verified_action_count = plan
        .actions
        .iter()
        .filter(|action| action.status == "verified")
        .count();
    let disposition = if plan.status == "deferred" {
        "deferred"
    } else {
        "verified"
    };
    let summary = if plan.actions.is_empty() {
        "A fresh remediation projection found no active actions.".to_string()
    } else {
        format!(
            "{} action(s) left the active queue with disposition '{}'.",
            plan.actions.len(),
            disposition
        )
    };
    RemediationClosure {
        id: stable_id(
            &format!(
                "closure:{}:{}:{}",
                plan.repository_id, closed_at, disposition
            ),
            "remediation-closure",
        ),
        repository_id: plan.repository_id.clone(),
        repository_name: plan.repository_name.clone(),
        repository_path: plan.repository_path.clone(),
        plan_id: plan.id.clone(),
        target_state: plan.goal.target_state.clone(),
        goal_source: plan.goal.source.clone(),
        maturity_policy: plan.goal.maturity_policy.clone(),
        closed_at: closed_at.to_string(),
        source_refresh_id: source_refresh_id.map(str::to_string),
        disposition: disposition.to_string(),
        summary,
        resolved_action_count: plan.actions.len(),
        verified_action_count,
        deferred_action_count,
        last_evidence_at: latest_plan_evidence_at(plan),
    }
}
