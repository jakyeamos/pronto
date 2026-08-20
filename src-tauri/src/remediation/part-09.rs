fn build_ui_coverage(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    actions: &[RemediationAction],
) -> Vec<RemediationCoverage> {
    let compass = &repository.project_compass;
    let active_conditions = repository
        .conditions
        .iter()
        .filter(|condition| condition.status == "Active")
        .count();
    let unhealthy_submodules = repository
        .submodules
        .iter()
        .filter(|submodule| submodule.status != "Checked out")
        .count();
    let eligible_branches = repository
        .branches
        .iter()
        .filter(|branch| branch.integration_state == "Integration eligible")
        .count();
    let workspace_count = repository.workspaces.len().max(1);
    let coverage = vec![
        coverage_for_prefixes(
            "scope",
            "Repository scope",
            &format!(
                "Lifecycle: {} · candidate: {} · target branch: {} · Git default: {}.",
                repository.lifecycle,
                repository.lifecycle_candidate,
                repository
                    .target_branch
                    .as_deref()
                    .or(repository.default_branch.as_deref())
                    .unwrap_or("Unknown"),
                repository.default_branch.as_deref().unwrap_or("Unknown")
            ),
            &["scope:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "project_compass",
            "Project Compass",
            &format!(
                "Status: {} · blockers: {} · drift: {} · MVP progress: {}.",
                compass.status,
                compass.open_blockers,
                compass.open_drift,
                compass
                    .mvp
                    .progress_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "Unknown".to_string())
            ),
            &["product_truth:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "provider",
            "Provider and remote evidence",
            &format!(
                "Provider: {} · remote: {} · last fetch: {}.",
                repository.provider_state,
                repository.remote_url.as_deref().unwrap_or("Missing"),
                repository.last_fetch_at.as_deref().unwrap_or("Unknown")
            ),
            &["provider:remote-freshness"],
            !goal_requires_provider(goal),
            actions,
        ),
        coverage_for_prefixes(
            "pull_requests",
            "Pull request evidence",
            &format!(
                "{} pull request snapshot(s); {} open.",
                repository.pull_requests.len(),
                repository
                    .pull_requests
                    .iter()
                    .filter(|pull_request| pull_request.state.eq_ignore_ascii_case("open"))
                    .count()
            ),
            &["provider:pull-request:"],
            !goal_requires_provider(goal),
            actions,
        ),
        coverage_for_prefixes(
            "releases",
            "Published release evidence",
            &format!(
                "{} release snapshot(s); latest fetch {}.",
                repository.releases.len(),
                repository.last_fetch_at.as_deref().unwrap_or("Unknown")
            ),
            &["release_evidence:"],
            goal.target_state != "public_release",
            actions,
        ),
        coverage_for_prefixes(
            "quality_evidence",
            "Quality evidence",
            &format!(
                "Findings freshness: {} · observed: {}.",
                repository.quality.findings.freshness.as_str(),
                repository
                    .quality
                    .findings
                    .observed_at
                    .as_deref()
                    .unwrap_or("Unknown")
            ),
            &["evidence_refresh:"],
            !goal_requires_quality_evidence(goal),
            actions,
        ),
        coverage_for_prefixes(
            "ci_gates",
            "CI gates",
            &format!(
                "{} required gate(s) for the '{}' goal.",
                goal.required_gate_ids.len(),
                goal.label
            ),
            &["ci_ideal:"],
            goal.required_gate_ids.is_empty(),
            actions,
        ),
        coverage_for_prefixes(
            "quality_findings",
            "Quality findings",
            &format!(
                "{} detected finding(s); {} actionable; {} reviewed; {} high-severity.",
                repository.quality.findings.total,
                repository.quality.findings.actionable_total,
                repository.quality.findings.reviewed_total,
                repository.quality.findings.high_severity_total
            ),
            &["qr_findings:"],
            !goal_requires_quality_evidence(goal),
            actions,
        ),
        coverage_for_prefixes(
            "maturity",
            "Repository maturity",
            &format!(
                "Score: {} · freshness: {}.",
                repository
                    .quality
                    .maturity
                    .score
                    .map(|score| format!("{score:.3}/4"))
                    .unwrap_or_else(|| "Unknown".to_string()),
                repository.quality.maturity.freshness.as_str()
            ),
            &["maturity:"],
            !goal_requires_maturity(goal),
            actions,
        ),
        coverage_for_prefixes(
            "workspaces",
            "Workspaces",
            &format!("{workspace_count} workspace(s) inspected for ownership, operations, dirt, sync, and remote freshness."),
            &[
                "branch_hygiene:activity:",
                "branch_hygiene:operation:",
                "branch_hygiene:dirty:",
                "branch_hygiene:sync:",
                "branch_hygiene:remote-freshness:",
            ],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "branches",
            "Branches and integration",
            &format!(
                "{} branch record(s); {} integration-eligible.",
                repository.branches.len(),
                eligible_branches
            ),
            &["branch_hygiene:integrate:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "submodules",
            "Submodules",
            &format!(
                "{} submodule(s); {} require attention.",
                repository.submodules.len(),
                unhealthy_submodules
            ),
            &["repository_health:submodule:"],
            repository.submodules.is_empty(),
            actions,
        ),
        coverage_for_prefixes(
            "conditions",
            "Repository conditions",
            &format!(
                "{} condition(s); {} active.",
                repository.conditions.len(),
                active_conditions
            ),
            &["branch_hygiene:", "repository_health:condition:"],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "release_preparation",
            "Release preparation",
            &format!(
                "Rule: {} · recipe: {} · confirmed version: {}.",
                if repository.release_rule.is_some() {
                    "configured"
                } else {
                    "missing"
                },
                if repository.release_recipe.is_some() {
                    "configured"
                } else {
                    "missing"
                },
                repository
                    .confirmed_release_version
                    .as_deref()
                    .unwrap_or("Unknown")
            ),
            &[
                "scope:release-contract",
                "release_evidence:",
                PUBLIC_RELEASE_BOUNDARY_ACTION_KEY,
            ],
            goal.target_state != "public_release",
            actions,
        ),
        coverage_for_prefixes(
            "agent_permission",
            "Agent permission",
            &format!(
                "Current repository permission: {}. This remains a safety boundary, not inferred remediation work.",
                repository.ai_permission
            ),
            &[],
            false,
            actions,
        ),
        coverage_for_prefixes(
            "analytics",
            "Repository analytics",
            "Historical activity and trend analytics are represented as informational evidence; they do not create remediation work by themselves.",
            &[],
            false,
            actions,
        ),
    ];
    debug_assert_eq!(
        coverage
            .iter()
            .map(|entry| entry.surface.as_str())
            .collect::<Vec<_>>(),
        UI_TRACKED_SURFACE_IDS
    );
    coverage
}

fn coverage_for_prefixes(
    surface: &str,
    label: &str,
    detail: &str,
    prefixes: &[&str],
    not_applicable: bool,
    actions: &[RemediationAction],
) -> RemediationCoverage {
    let matching = actions
        .iter()
        .filter(|action| {
            prefixes
                .iter()
                .any(|prefix| action.stable_key.starts_with(prefix))
        })
        .collect::<Vec<_>>();
    let status = if not_applicable {
        "not_applicable"
    } else if matching.is_empty() {
        "clear"
    } else if matching.iter().any(|action| action.status == "blocked") {
        "blocked"
    } else if matching
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress"))
    {
        "attention"
    } else if matching.iter().any(|action| action.status == "deferred") {
        "deferred"
    } else {
        "verified"
    };
    RemediationCoverage {
        surface: surface.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        detail: detail.to_string(),
        action_ids: matching.iter().map(|action| action.id.clone()).collect(),
    }
}

fn default_remediation_phase_definitions() -> Vec<RemediationPhaseDefinition> {
    vec![
        RemediationPhaseDefinition {
            id: "preserve_and_reconcile".to_string(),
            title: "Preserve and reconcile repository work".to_string(),
            summary: "Protect active or ambiguous work, then make workspaces, branches, operations, and the canonical target intentional.".to_string(),
            domains: ["scope", "repository_health", "branch_hygiene"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            completion_criterion: "Every scoped workspace, branch, operation, and repository-health action is verified or explicitly deferred with evidence.".to_string(),
            after_phase_id: None,
        },
        RemediationPhaseDefinition {
            id: "product_and_provider_truth".to_string(),
            title: "Reconcile product and provider truth".to_string(),
            summary: "Align the intended product outcome with fresh provider-native repository, pull-request, and release evidence.".to_string(),
            domains: ["product_truth", "provider"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            completion_criterion: "Product intent and provider-native branch, pull-request, and release evidence satisfy the repository goal.".to_string(),
            after_phase_id: Some("preserve_and_reconcile".to_string()),
        },
        RemediationPhaseDefinition {
            id: "quality_and_maturity".to_string(),
            title: "Reach quality and maturity threshold".to_string(),
            summary: "Refresh required evidence, clear actionable findings and gate failures, and reach the applicable maturity floor without manufacturing evidence.".to_string(),
            domains: ["evidence_refresh", "ci_ideal", "qr_findings", "maturity"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            completion_criterion: "Required gates and quality evidence are fresh, actionable findings are cleared, and applicable maturity reaches its minimum threshold.".to_string(),
            after_phase_id: Some("product_and_provider_truth".to_string()),
        },
        RemediationPhaseDefinition {
            id: "public_distribution_boundary".to_string(),
            title: "Prove the public distribution boundary".to_string(),
            summary: "Separate public product surfaces from optional adapters and local-only operations, then verify the packaged artifact in an isolated environment.".to_string(),
            domains: vec!["release_boundary".to_string()],
            completion_criterion: "Every release-relevant surface is classified, the artifact allowlist passes, isolated installation succeeds, and optional integrations rely only on sanitized contracts.".to_string(),
            after_phase_id: Some("quality_and_maturity".to_string()),
        },
        RemediationPhaseDefinition {
            id: "verify_and_close".to_string(),
            title: "Refresh and re-evaluate".to_string(),
            summary: "Re-run the scoped evidence sources after the material work and determine whether the current queue still has actionable work.".to_string(),
            domains: vec!["verification".to_string()],
            completion_criterion: "A fresh scoped remediation projection reports the current queue state and records any resolved actions in history without treating the repository as permanently complete.".to_string(),
            after_phase_id: Some("public_distribution_boundary".to_string()),
        },
    ]
}

fn ordered_remediation_phase_definitions(
    goal: &RemediationGoalProfile,
) -> Vec<RemediationPhaseDefinition> {
    let repository_domains = goal
        .remediation_phases
        .iter()
        .flat_map(|phase| phase.domains.iter().cloned())
        .collect::<HashSet<_>>();
    let mut definitions = default_remediation_phase_definitions();
    for definition in &mut definitions {
        definition
            .domains
            .retain(|domain| !repository_domains.contains(domain));
    }

    let mut insertion_tails = HashMap::<String, String>::new();
    for repository_phase in &goal.remediation_phases {
        let insertion_index =
            if let Some(requested_anchor) = repository_phase.after_phase_id.as_ref() {
                let effective_anchor = insertion_tails
                    .get(requested_anchor)
                    .unwrap_or(requested_anchor);
                definitions
                    .iter()
                    .position(|phase| &phase.id == effective_anchor)
                    .map(|index| index + 1)
                    .unwrap_or(definitions.len())
            } else {
                definitions
                    .iter()
                    .position(|phase| phase.id == "verify_and_close")
                    .unwrap_or(definitions.len())
            };
        definitions.insert(insertion_index, repository_phase.clone());
        if let Some(requested_anchor) = repository_phase.after_phase_id.as_ref() {
            insertion_tails.insert(requested_anchor.clone(), repository_phase.id.clone());
        }
    }
    definitions
}

fn explanation_phase(
    definition: &RemediationPhaseDefinition,
    matching: &[&RemediationAction],
) -> RemediationExplanationPhase {
    let status = if matching.iter().any(|action| action.status == "blocked") {
        "blocked"
    } else if matching.iter().any(|action| action.status == "in_progress") {
        "in_progress"
    } else {
        "open"
    };
    RemediationExplanationPhase {
        id: definition.id.clone(),
        title: definition.title.clone(),
        summary: definition.summary.clone(),
        status: status.to_string(),
        steps: matching
            .iter()
            .map(|action| RemediationExplanationStep {
                action_id: action.id.clone(),
                title: action.title.clone(),
                summary: action.summary.clone(),
                status: action.status.clone(),
                priority: action.priority.clone(),
                completion_criteria: action.acceptance_criteria.clone(),
            })
            .collect(),
        completion_criterion: definition.completion_criterion.clone(),
    }
}
