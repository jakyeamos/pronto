fn add_project_compass_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let compass = &repository.project_compass;
    match compass.status.as_str() {
        "Missing" => seeds.push(ActionSeed {
            stable_key: "product_truth:project-compass".to_string(),
            domain: "product_truth".to_string(),
            title: "Establish the Project Compass contract".to_string(),
            summary: "The repository has no Project Compass contract, so Pronto cannot relate remediation work to the intended product outcome.".to_string(),
            severity: "product_truth".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Use the Project Compass workflow to make an explicit product-truth decision; do not infer or silently invent the contract.".to_string(),
                format!("A valid contract is present at {}.", compass.contract_path),
                "Refresh Pronto and confirm Project Compass is Ready.".to_string(),
            ],
            evidence: vec![evidence(
                "Project Compass",
                "Product truth contract",
                "Missing",
                "Fresh",
                Some(&repository.last_scan_at),
                Some(&compass.contract_path),
                "The UI tracks Project Compass for this repository, but no contract exists.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        }),
        "Invalid" => seeds.push(ActionSeed {
            stable_key: "product_truth:project-compass".to_string(),
            domain: "product_truth".to_string(),
            title: "Repair the Project Compass contract".to_string(),
            summary: "The repository's Project Compass contract is present but invalid, so its product progress cannot be trusted.".to_string(),
            severity: "product_truth".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Repair the existing contract through the Project Compass workflow without inventing product truth.".to_string(),
                "The contract validates and Pronto reports Project Compass as Ready.".to_string(),
            ],
            evidence: vec![evidence(
                "Project Compass",
                "Product truth contract",
                "Invalid",
                "Fresh",
                compass.updated_at.as_deref().or(Some(&repository.last_scan_at)),
                Some(&compass.contract_path),
                compass.error.as_deref().unwrap_or("The contract is invalid."),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        }),
        "Ready" => {
            if compass.open_blockers > 0 || compass.open_drift > 0 {
                let mut compass_evidence = Vec::new();
                if compass.open_blockers > 0 {
                    compass_evidence.push(evidence(
                        "Project Compass",
                        "Open blockers",
                        &compass.open_blockers.to_string(),
                        "Fresh",
                        compass.updated_at.as_deref(),
                        Some(&compass.contract_path),
                        "Open product blockers require reconciliation in the canonical contract.",
                    ));
                }
                if compass.open_drift > 0 {
                    compass_evidence.push(evidence(
                        "Project Compass",
                        "Open drift",
                        &compass.open_drift.to_string(),
                        "Fresh",
                        compass.updated_at.as_deref(),
                        Some(&compass.contract_path),
                        "Open product-to-implementation drift requires reconciliation in the canonical contract.",
                    ));
                }
                seeds.push(ActionSeed {
                    stable_key: PROJECT_COMPASS_OPEN_ITEMS_KEY.to_string(),
                    domain: "product_truth".to_string(),
                    title: "Reconcile open Project Compass items".to_string(),
                    summary: format!(
                        "Project Compass records {} open blocker(s) and {} open drift item(s). They are one product-truth reconciliation action, not independent evidence of additional remediation work.",
                        compass.open_blockers, compass.open_drift
                    ),
                    severity: "product_truth".to_string(),
                    priority: "P1".to_string(),
                    weight: severity_weight("warning"),
                    acceptance_criteria: vec![
                        "Each blocker is resolved or explicitly dispositioned in the canonical Project Compass contract.".to_string(),
                        "Each drift item is reconciled in implementation or explicitly dispositioned in Project Compass.".to_string(),
                        "Pronto refreshes the contract and reports no unexplained open blocker or drift.".to_string(),
                    ],
                    evidence: compass_evidence,
                    related_finding_ids: Vec::new(),
                    source_run_id: None,
                });
            }
        }
        _ => seeds.push(ActionSeed {
            stable_key: "product_truth:project-compass".to_string(),
            domain: "product_truth".to_string(),
            title: "Resolve the unknown Project Compass state".to_string(),
            summary: format!(
                "Pronto received an unrecognized Project Compass state: '{}'.",
                compass.status
            ),
            severity: "product_truth".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Inspect the canonical contract and Project Compass tooling.".to_string(),
                "Refresh Pronto and confirm the state is Ready, Missing, or Invalid.".to_string(),
            ],
            evidence: vec![evidence(
                "Project Compass",
                "Product truth contract",
                &compass.status,
                "Unknown",
                compass.updated_at.as_deref(),
                Some(&compass.contract_path),
                "The Project Compass state is not recognized by the remediation planner.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        }),
    }
}

fn add_release_evidence_seeds(
    repository: &RepositorySnapshot,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    if goal.target_state != "public_release"
        || !repository.releases.is_empty()
        || repository
            .release_rule
            .as_ref()
            .map_or(true, |rule| rule.allow_first_release)
    {
        return;
    }
    seeds.push(ActionSeed {
        stable_key: "release_evidence:published-baseline".to_string(),
        domain: "provider".to_string(),
        title: "Resolve the missing published-release baseline".to_string(),
        summary: "The public-release goal requires release evidence, but no published baseline is available and the release rule does not allow a first release.".to_string(),
        severity: "release".to_string(),
        priority: "P1".to_string(),
        weight: 2,
        acceptance_criteria: vec![
            "Confirm whether this is an intentional first release or a missing provider snapshot."
                .to_string(),
            "If it is a first release, explicitly authorize that case in the release rule; otherwise refresh the published release evidence.".to_string(),
            "Release preparation reports an evidence-ready baseline disposition.".to_string(),
        ],
        evidence: vec![evidence(
            "Pronto release preparation",
            "Published release baseline",
            "Missing",
            if repository.last_fetch_at.is_some() {
                "Fresh"
            } else {
                "Unknown"
            },
            repository.last_fetch_at.as_deref(),
            None,
            "No published release is present and allow_first_release is false.",
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn add_provider_seed(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    let provider_state = repository.provider_state.to_ascii_lowercase();
    let connected = provider_state.contains("connected") && repository.last_fetch_at.is_some();
    let (title, summary, detail) = if repository.remote_url.is_none() {
        (
            "Confirm the repository remote/provider identity",
            "No remote URL is recorded, so provider freshness and CI context cannot be matched to this repository.",
            "Remote URL is missing from the local snapshot.",
        )
    } else if !connected {
        (
            "Refresh the provider snapshot",
            "A GitHub remote is known locally, but Pronto does not have a confirmed fresh provider snapshot for it.",
            "Remote detected; provider snapshot is not confirmed fresh.",
        )
    } else {
        return;
    };
    seeds.push(ActionSeed {
        stable_key: "provider:remote-freshness".to_string(),
        domain: "provider".to_string(),
        title: title.to_string(),
        summary: summary.to_string(),
        severity: "provider".to_string(),
        priority: "P1".to_string(),
        weight: 2,
        acceptance_criteria: vec![
            "The local remote maps to the intended provider repository.".to_string(),
            "A provider refresh records a successful fetch timestamp.".to_string(),
            "Remote evidence remains read-only and is not treated as a source edit.".to_string(),
        ],
        evidence: vec![evidence(
            "Pronto",
            "Provider freshness",
            &repository.provider_state,
            if connected { "Fresh" } else { "Unknown" },
            repository.last_fetch_at.as_deref(),
            None,
            detail,
        )],
        related_finding_ids: Vec::new(),
        source_run_id: None,
    });
}

fn add_pull_request_seeds(repository: &RepositorySnapshot, seeds: &mut Vec<ActionSeed>) {
    for pull_request in repository
        .pull_requests
        .iter()
        .filter(|pull_request| pull_request.state.eq_ignore_ascii_case("open"))
    {
        let checks_ready = pull_request.checks_state.eq_ignore_ascii_case("passed");
        let reviews_ready = pull_request
            .reviews_state
            .to_ascii_lowercase()
            .contains("approved");
        let mergeability = pull_request.mergeability.to_ascii_lowercase();
        let merge_ready = matches!(mergeability.as_str(), "clean" | "mergeable");
        if !pull_request.draft && checks_ready && reviews_ready && merge_ready {
            continue;
        }
        seeds.push(ActionSeed {
            stable_key: format!("provider:pull-request:{}", pull_request.number),
            domain: "provider".to_string(),
            title: format!("Resolve pull request evidence · #{}", pull_request.number),
            summary: format!(
                "Open pull request #{} is not fully ready: draft {} · checks {} · reviews {} · mergeability {}.",
                pull_request.number,
                pull_request.draft,
                pull_request.checks_state,
                pull_request.reviews_state,
                pull_request.mergeability
            ),
            severity: "provider".to_string(),
            priority: if pull_request.checks_state.eq_ignore_ascii_case("failed") {
                "P1"
            } else {
                "P2"
            }
            .to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Refresh provider-native pull request evidence for the exact head and base."
                    .to_string(),
                "Required checks, reviews, draft state, and mergeability are explicitly resolved or dispositioned.".to_string(),
                "Pronto records the resulting pull request snapshot without inferring provider success."
                    .to_string(),
            ],
            evidence: vec![evidence(
                "GitHub",
                &format!("Pull request #{}", pull_request.number),
                if pull_request.draft {
                    "Draft"
                } else {
                    &pull_request.checks_state
                },
                "Fresh",
                Some(&pull_request.last_refreshed_at),
                Some(&pull_request.html_url),
                &format!(
                    "{} → {} · reviews {} · mergeability {}.",
                    pull_request.head_branch,
                    pull_request.base_branch,
                    pull_request.reviews_state,
                    pull_request.mergeability
                ),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
    }
}

fn add_evidence_seed(
    repository: &RepositorySnapshot,
    qr_run: Option<&QrRunEvidence>,
    goal: &RemediationGoalProfile,
    seeds: &mut Vec<ActionSeed>,
) {
    let Some(run) = qr_run else {
        seeds.push(ActionSeed {
            stable_key: "evidence_refresh:qr-run".to_string(),
            domain: "evidence_refresh".to_string(),
            title: "Run a fresh Quality Runner audit".to_string(),
            summary: "No repository-local QR run is available, so findings and verification evidence are not current.".to_string(),
            severity: "evidence".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "QR doctor passes before execution.".to_string(),
                "A full QR audit is run for this repository.".to_string(),
                "The new run is replay-valid and its artifacts are imported by Pronto.".to_string(),
            ],
            evidence: vec![evidence(
                "Quality Runner",
                "Latest repository run",
                "Missing",
                "Unknown",
                None,
                None,
                "No .quality-runner/runs artifact was found.",
            )],
            related_finding_ids: Vec::new(),
            source_run_id: None,
        });
        return;
    };
    let freshness = freshness_for(run.observed_at.as_deref(), goal.evidence_max_age_days);
    if freshness != "Fresh" {
        seeds.push(ActionSeed {
            stable_key: "evidence_refresh:qr-run".to_string(),
            domain: "evidence_refresh".to_string(),
            title: "Refresh the Quality Runner evidence".to_string(),
            summary:
                "The latest QR artifact is present but no longer fresh for this remediation run."
                    .to_string(),
            severity: "evidence".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "QR doctor passes before execution.".to_string(),
                "A new full QR run is written for this repository.".to_string(),
                "The run timestamp and commit match the current local snapshot.".to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Quality Runner",
                "Latest repository run",
                "Present",
                &freshness,
                run.observed_at.as_deref(),
                run.run_dir.to_str(),
                "The repository has QR artifacts, but they are outside the fresh-evidence window.",
                run.scanned_branch.as_deref(),
                run.scanned_commit.as_deref(),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
    if repository.quality.findings.freshness.as_str() != "Fresh"
        && !seeds
            .iter()
            .any(|seed| seed.stable_key == "evidence_refresh:qr-run")
    {
        seeds.push(ActionSeed {
            stable_key: "evidence_refresh:pronto-import".to_string(),
            domain: "evidence_refresh".to_string(),
            title: "Re-import the QR result into Pronto".to_string(),
            summary: "A QR run exists, but the Pronto quality projection is not fresh against the current repository evidence.".to_string(),
            severity: "evidence".to_string(),
            priority: "P1".to_string(),
            weight: 2,
            acceptance_criteria: vec![
                "Pronto ingests the latest QR run without changing source files.".to_string(),
                "The finding report path and observed timestamp are present.".to_string(),
            ],
            evidence: vec![evidence_with_provenance(
                "Pronto",
                "Imported QR findings",
                "Stale",
                "Stale",
                repository.quality.findings.observed_at.as_deref(),
                repository.quality.findings.report_path.as_deref(),
                "The local quality projection is not fresh.",
                repository.quality.findings.scanned_branch.as_deref(),
                repository.quality.findings.scanned_commit.as_deref(),
            )],
            related_finding_ids: Vec::new(),
            source_run_id: Some(run.id.clone()),
        });
    }
}
