fn build_plan(
    repository: &RepositorySnapshot,
    previous: Option<&RemediationPlan>,
    source_refresh_id: Option<&str>,
    generated_at: &str,
    fleet_audit_root: Option<&Path>,
) -> RemediationPlan {
    let goal = resolve_goal_profile(repository);
    let qr_run = latest_qr_run(Path::new(&repository.path), fleet_audit_root);
    let mut seeds = Vec::new();
    add_scope_seed(repository, &goal, &mut seeds);
    add_goal_seeds(repository, &goal, &mut seeds);
    add_release_evidence_seeds(repository, &goal, &mut seeds);
    add_project_compass_seeds(repository, &mut seeds);
    add_installed_runtime_seed(repository, &mut seeds);
    if goal_requires_provider(&goal) {
        add_provider_seed(repository, &mut seeds);
        add_pull_request_seeds(repository, &mut seeds);
    }
    if goal_requires_quality_evidence(&goal) {
        add_evidence_seed(repository, qr_run.as_ref(), &goal, &mut seeds);
    }
    add_ci_ideal_seeds(repository, &goal, &mut seeds);
    add_qr_finding_seeds(repository, qr_run.as_ref(), &goal, &mut seeds);
    add_debloat_gate_seed(repository, qr_run.as_ref(), &mut seeds);
    add_branch_hygiene_seeds(repository, &mut seeds);
    add_submodule_seeds(repository, &mut seeds);
    add_evidence_contract_seeds(repository, &mut seeds);
    if goal_requires_maturity(&goal) {
        add_maturity_seeds(repository, &mut seeds);
        add_maturity_gate_seeds(repository, &goal, &mut seeds);
    }

    if !seeds.is_empty() || goal.target_state == "github_only" {
        let github_only_terminal_task = goal.target_state == "github_only";
        seeds.push(ActionSeed {
            stable_key: if github_only_terminal_task {
                GITHUB_ONLY_VERIFICATION_ACTION_KEY.to_string()
            } else {
                VERIFICATION_ACTION_KEY.to_string()
            },
            domain: "verification".to_string(),
            title: if github_only_terminal_task {
                GITHUB_ONLY_REMEDIATION_TASK.to_string()
            } else {
                "Verify the repository after remediation".to_string()
            },
            summary: if github_only_terminal_task {
                "Record the storage-preserving terminal disposition as GitHub only; no local checkout is required for this repository.".to_string()
            } else {
                "Re-run the eligible evidence sources and confirm the plan is clear before closing it.".to_string()
            },
            severity: "verification".to_string(),
            priority: "P2".to_string(),
            weight: 1,
            acceptance_criteria: if github_only_terminal_task {
                vec![
                    "A fresh provider snapshot confirms the GitHub repository and remote identity.".to_string(),
                    "The local checkout is intentionally absent or no longer required because local storage is constrained.".to_string(),
                    "The terminal remediation task is recorded as GitHub only.".to_string(),
                ]
            } else {
                vec![
                    "A fresh local snapshot is recorded.".to_string(),
                    "Project Compass, workspace, branch, submodule, condition, provider, quality, CI, and maturity evidence are rechecked where applicable.".to_string(),
                    "No unresolved blocking action remains.".to_string(),
                ]
            },
            evidence: vec![evidence(
                if github_only_terminal_task { "GitHub" } else { "Pronto" },
                if github_only_terminal_task {
                    "GitHub-only disposition"
                } else {
                    "Derived verification gate"
                },
                "Open",
                if github_only_terminal_task
                    && repository.provider_state.contains("GitHub connected")
                {
                    "Fresh"
                } else {
                    "Unknown"
                },
                repository
                    .last_fetch_at
                    .as_deref()
                    .or(Some(repository.last_scan_at.as_str())),
                None,
                if github_only_terminal_task {
                    "The provider snapshot must support the intentional GitHub-only storage disposition."
                } else {
                    "Verification is required after the source gaps are addressed."
                },
            )],
            related_finding_ids: Vec::new(),
            source_run_id: qr_run.as_ref().map(|run| run.id.clone()),
        });
    }

    let previous_actions = previous
        .map(|plan| {
            plan.actions
                .iter()
                .map(|action| (action.stable_key.as_str(), action))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let mut actions = seeds
        .into_iter()
        .map(|seed| materialize_action(repository, seed, &previous_actions, generated_at))
        .collect::<Vec<_>>();
    retain_resolved_action_history(&mut actions, previous, generated_at);
    let progress = calculate_progress(&actions);
    let coverage = build_ui_coverage(repository, &goal, &actions);
    let explanation = build_remediation_explanation(&goal, &actions, &coverage);
    let tracks = build_tracks(&actions);
    let status = plan_status(&actions);
    let current_stage = current_stage(&actions);
    let integration_only_remaining = integration_only_remaining(&actions);
    let plan_id = stable_id(&format!("plan:{}", repository.id), "remediation-plan");
    RemediationPlan {
        schema_version: REMEDIATION_SCHEMA.to_string(),
        id: plan_id,
        repository_id: repository.id.clone(),
        repository_name: repository.name.clone(),
        repository_path: repository.path.clone(),
        generated_at: generated_at.to_string(),
        source_refresh_id: source_refresh_id
            .map(str::to_string)
            .or_else(|| qr_run.as_ref().map(|run| run.id.clone())),
        goal,
        current_stage,
        status,
        integration_only_remaining,
        progress,
        coverage,
        explanation,
        tracks,
        actions,
    }
}

fn add_installed_runtime_seed(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let runtime = &repository.quality.installed_runtime;
    if runtime.applicability != "applicable" || runtime.status == "current" {
        return;
    }
    let statuses = runtime
        .targets
        .iter()
        .flat_map(|target| target.issues.iter().map(|item| item.status.clone()))
        .collect::<Vec<_>>();
    let freshness = if statuses.iter().any(|status| status == "evidence_stale") {
        "Stale"
    } else if statuses
        .iter()
        .any(|status| matches!(status.as_str(), "invalid" | "unverifiable"))
    {
        "Unknown"
    } else {
        "Fresh"
    };
    seeds.push(ActionSeed {
        stable_key: "repository_health:installed-runtime-parity".to_string(),
        domain: "repository_health".to_string(),
        title: "Reconcile the installed runtime".to_string(),
        summary: runtime.summary.clone(),
        severity: "runtime".to_string(),
        priority: "P1".to_string(),
        weight: 3,
        acceptance_criteria: vec![
            "The current source revision matches the packaged build revision.".to_string(),
            "The installed artifact digest matches the packaged build digest.".to_string(),
            "The running process PID and artifact digest match the installed executable."
                .to_string(),
            "Pronto reports installed runtime parity as current.".to_string(),
        ],
        evidence: vec![evidence(
            "Installed runtime parity",
            "Source, build, install, and process identity",
            "Attention required",
            freshness,
            Some(&repository.last_scan_at),
            runtime.config_path.as_deref(),
            &format!("Observed states: {}", statuses.join(", ")),
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn add_goal_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    if goal.source != "repository_contract" {
        let detail = goal.error.as_deref().unwrap_or(
            "No repository-owned remediation goal contract is present; this profile is inferred.",
        );
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-remediation-goal".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the repository remediation goal".to_string(),
            summary: format!(
                "Pronto inferred '{}' for this repository. Confirm the intended outcome before using it as the closure contract.",
                goal.label
            ),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                format!(
                    "Create {} with schema {}.",
                    REMEDIATION_GOAL_PATH, REMEDIATION_GOAL_SCHEMA
                ),
                "Record the intended target_state and a non-empty reason.".to_string(),
                "Refresh Pronto and confirm the profile source is repository_contract."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Remediation goal",
                "Inferred",
                "Unknown",
                Some(&repository.last_scan_at),
                Some(REMEDIATION_GOAL_PATH),
                detail,
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if goal.target_state == "public_release"
        && (repository.release_rule.is_none() || repository.release_recipe.is_none())
    {
        seeds.push(ActionSeed {
            stable_key: "scope:release-contract".to_string(),
            domain: "scope".to_string(),
            title: "Establish the release contract".to_string(),
            summary: "A public-release repository needs both an explicit release rule and an executable release recipe before it can leave remediation.".to_string(),
            severity: "release".to_string(),
            priority: "P1".to_string(),
            weight: 3,
            acceptance_criteria: vec![
                "A normalized release rule defines when a release is eligible.".to_string(),
                "A release recipe defines validation, generation, commit, and publication steps."
                    .to_string(),
                "Release preview can evaluate the repository without inventing provider evidence."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Release contract",
                "Incomplete",
                "Fresh",
                Some(&repository.last_scan_at),
                None,
                &format!(
                    "Release rule: {} · release recipe: {}.",
                    if repository.release_rule.is_some() {
                        "configured"
                    } else {
                        "missing"
                    },
                    if repository.release_recipe.is_some() {
                        "configured"
                    } else {
                        "missing"
                    }
                ),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if goal.target_state == "public_release"
        && !repository.quality.release_boundary.is_release_ready()
    {
        let boundary = &repository.quality.release_boundary;
        let blocker_detail = if boundary.blocking_check_ids.is_empty() {
            boundary.detail.clone()
        } else {
            format!(
                "Blocking checks: {}. {}",
                boundary.blocking_check_ids.join(", "),
                boundary.detail
            )
        };
        seeds.push(ActionSeed {
            stable_key: PUBLIC_RELEASE_BOUNDARY_ACTION_KEY.to_string(),
            domain: "release_boundary".to_string(),
            title: if boundary.status == "Missing" {
                "Create and pass the public-release boundary receipt".to_string()
            } else {
                "Refresh and pass the public-release boundary receipt".to_string()
            },
            summary: blocker_detail.clone(),
            severity: "release".to_string(),
            priority: "P1".to_string(),
            weight: 3,
            acceptance_criteria: vec![
                "Every release-relevant source, configuration, and documentation surface is classified as public_core, public_adapter, or local_only; unclassified surfaces block release preparation.".to_string(),
                "Tracked source and documentation contain no personal absolute paths, private repository inventories, credentials, or private operational defaults.".to_string(),
                "Every built distribution is inspected against an explicit artifact allowlist, including wheel and sdist contents for Python packages where applicable.".to_string(),
                "The packaged artifact installs and runs with an isolated temporary home while Pronto, Leverage, Mac Control, and other private workspace peers are absent.".to_string(),
                "Optional integrations are verified through sanitized contract fixtures or consumer-owned tests without copying private data or setup-specific policy into the public release.".to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Quality Runner release boundary",
                "Public-release receipt",
                &boundary.status,
                &boundary.freshness,
                boundary.generated_at.as_deref(),
                boundary.report_path.as_deref(),
                &blocker_detail,
                boundary.scanned_branch.as_deref(),
                boundary.scanned_commit.as_deref(),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if goal.target_state == "archived"
        && !repository.lifecycle.to_ascii_lowercase().contains("archiv")
    {
        seeds.push(ActionSeed {
            stable_key: "scope:align-archived-lifecycle".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the archived lifecycle".to_string(),
            summary: "The remediation goal is archived, but the repository lifecycle does not yet record an archived state.".to_string(),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The repository lifecycle is explicitly confirmed as archived.".to_string(),
                "No active ownership or release obligation remains.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Lifecycle alignment",
                &repository.lifecycle,
                "Fresh",
                Some(&repository.last_scan_at),
                Some(REMEDIATION_GOAL_PATH),
                "The archived goal and repository lifecycle must agree.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_scope_seed(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let lifecycle = repository.lifecycle.to_ascii_lowercase();
    let candidate = repository.lifecycle_candidate.to_ascii_lowercase();
    let github_only = goal.target_state == "github_only"
        || is_github_only_label(&repository.lifecycle)
        || is_github_only_label(&repository.lifecycle_candidate);
    if !github_only
        && (lifecycle.contains("unconfirmed") || candidate != lifecycle && !candidate.is_empty())
    {
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-lifecycle".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the repository scope".to_string(),
            summary: format!(
                "Pronto has lifecycle evidence of '{}' with candidate '{}'; confirm the repository's intended scope before prioritizing remediation.",
                repository.lifecycle, repository.lifecycle_candidate
            ),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The repository lifecycle is explicitly confirmed.".to_string(),
                "The decision is recorded in Pronto before work is planned.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Lifecycle and scope state",
                &repository.lifecycle,
                "Fresh",
                Some(&repository.last_scan_at),
                None,
                &format!("Candidate lifecycle: {}", repository.lifecycle_candidate),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
    if !github_only
        && repository
            .target_branch
            .as_ref()
            .or(repository.default_branch.as_ref())
            .is_none()
    {
        seeds.push(ActionSeed {
            stable_key: "scope:confirm-target-branch".to_string(),
            domain: "scope".to_string(),
            title: "Confirm the canonical integration branch".to_string(),
            summary: "Pronto has no configured target or observed default branch, so branch comparisons and integration targets are ambiguous.".to_string(),
            severity: "scope".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "The canonical integration branch is confirmed from repository or provider evidence.".to_string(),
                "Pronto records the target branch and can evaluate branch integration against it.".to_string(),
            ],
            evidence: vec![evidence(
                "Pronto",
                "Target branch",
                "Missing",
                "Unknown",
                Some(&repository.last_scan_at),
                None,
                "No target or default branch was resolved for this repository.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}
