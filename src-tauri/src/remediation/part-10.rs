fn build_remediation_explanation(
    goal: &RemediationGoalProfile,
    actions: &[RemediationAction],
    coverage: &[RemediationCoverage],
) -> RemediationExplanation {
    let active_actions = actions
        .iter()
        .filter(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"))
        .collect::<Vec<_>>();
    let definitions = ordered_remediation_phase_definitions(goal);
    let verification_phase_id = definitions
        .iter()
        .find(|definition| {
            definition
                .domains
                .iter()
                .any(|domain| domain == "verification")
        })
        .map(|definition| definition.id.clone());
    let mut assigned_action_indices = HashSet::new();
    let mut phases = Vec::new();
    for definition in &definitions {
        let matching_indices = active_actions
            .iter()
            .enumerate()
            .filter_map(|(index, action)| {
                (!assigned_action_indices.contains(&index)
                    && definition.domains.contains(&action.domain))
                .then_some(index)
            })
            .collect::<Vec<_>>();
        if matching_indices.is_empty() {
            continue;
        }
        assigned_action_indices.extend(matching_indices.iter().copied());
        let matching = matching_indices
            .iter()
            .map(|index| active_actions[*index])
            .collect::<Vec<_>>();
        phases.push(explanation_phase(definition, &matching));
    }

    let unmatched = active_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            (!assigned_action_indices.contains(&index)).then_some(*action)
        })
        .collect::<Vec<_>>();
    if !unmatched.is_empty() {
        let fallback = explanation_phase(
            &RemediationPhaseDefinition {
                id: UNCLASSIFIED_REMEDIATION_PHASE_ID.to_string(),
                title: "Classify additional remediation work".to_string(),
                summary: "These active actions use domains that are not yet assigned to a default or repository-defined remediation phase. They remain visible until the work is resolved or the repository goal contract classifies them.".to_string(),
                domains: Vec::new(),
                completion_criterion: "Every listed action is resolved or assigned to an explicit repository remediation phase without being hidden from the active plan.".to_string(),
                after_phase_id: None,
            },
            &unmatched,
        );
        let insertion_index = verification_phase_id
            .as_ref()
            .and_then(|phase_id| phases.iter().position(|phase| &phase.id == phase_id))
            .unwrap_or(phases.len());
        phases.insert(insertion_index, fallback);
    }
    let active_action_count = active_actions.len();
    debug_assert_eq!(
        phases.iter().map(|phase| phase.steps.len()).sum::<usize>(),
        active_action_count,
        "every active remediation action must appear in exactly one explanation phase"
    );
    let summary = if phases.is_empty() {
        "No active remediation phase remains for this refresh. Refresh scoped evidence before treating the queue as current."
            .to_string()
    } else {
        let phase_noun = if phases.len() == 1 { "phase" } else { "phases" };
        let phase_verb = if phases.len() == 1 {
            "remains"
        } else {
            "remain"
        };
        let action_noun = if active_action_count == 1 {
            "action"
        } else {
            "actions"
        };
        format!(
            "{} ordered remediation {phase_noun} {phase_verb} across {active_action_count} active {action_noun}. Work from the first unresolved phase and verify each result before refreshing the queue.",
            phases.len(),
        )
    };
    let healthy_surfaces = coverage
        .iter()
        .filter(|entry| matches!(entry.status.as_str(), "clear" | "verified"))
        .map(|entry| RemediationHealthySurface {
            surface: entry.surface.clone(),
            label: entry.label.clone(),
            status: entry.status.clone(),
            detail: entry.detail.clone(),
        })
        .collect::<Vec<_>>();
    let mut closure_requirements = goal.closure_criteria.clone();
    if let Some(policy) = &goal.maturity_policy {
        let ideal_gate_summary = if policy.ideal_gate_ids.is_empty() {
            "no additional maturity gates".to_string()
        } else {
            format!(
                "configured maturity gates ({})",
                policy.ideal_gate_ids.join(", ")
            )
        };
        closure_requirements.push(format!(
            "Fresh applicable maturity evidence reaches at least {:.1}/4; {:.1}/4 remains the evidence-backed ideal, and the ideal additionally requires {} to be fresh and passing, not a requirement for leaving remediation.",
            policy.minimum_closure_score, policy.ideal_score, ideal_gate_summary
        ));
    }
    closure_requirements.push(
        "A final scoped refresh reports the current open or blocked remediation actions; resolved actions are recorded as history and new evidence may reopen work."
            .to_string(),
    );

    RemediationExplanation {
        authority: "Advisory only: this explanation orders evidence-backed work but does not authorize Git, provider, publication, release, or pruning mutations."
            .to_string(),
        summary,
        phases,
        healthy_surfaces,
        closure_requirements,
    }
}

fn evidence(
    source: &str,
    label: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    detail: &str,
) -> RemediationEvidence {
    evidence_with_provenance(
        source,
        label,
        status,
        freshness,
        observed_at,
        report_path,
        detail,
        None,
        None,
    )
}

fn evidence_with_provenance(
    source: &str,
    label: &str,
    status: &str,
    freshness: &str,
    observed_at: Option<&str>,
    report_path: Option<&str>,
    detail: &str,
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
) -> RemediationEvidence {
    RemediationEvidence {
        source: source.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        freshness: freshness.to_string(),
        observed_at: observed_at.map(str::to_string),
        scanned_branch: scanned_branch.map(str::to_string),
        scanned_commit: scanned_commit.map(str::to_string),
        report_path: report_path.map(str::to_string),
        detail: detail.to_string(),
    }
}

fn build_tracks(actions: &[RemediationAction]) -> Vec<RemediationTrack> {
    STAGE_ORDER
        .iter()
        .filter(|domain| **domain != "complete")
        .filter_map(|domain| {
            let matching = actions
                .iter()
                .filter(|action| action.domain == *domain)
                .collect::<Vec<_>>();
            if matching.is_empty() {
                return None;
            }
            Some(RemediationTrack {
                domain: (*domain).to_string(),
                label: domain_label(domain),
                status: track_status(&matching),
                action_ids: matching.iter().map(|action| action.id.clone()).collect(),
                verified_weight: matching
                    .iter()
                    .filter(|action| action.status == "verified")
                    .map(|action| action.weight)
                    .sum(),
                total_weight: matching
                    .iter()
                    .filter(|action| action.status != "deferred")
                    .map(|action| action.weight)
                    .sum(),
            })
        })
        .collect()
}

fn calculate_progress(actions: &[RemediationAction]) -> RemediationProgress {
    let deferred_weight = actions
        .iter()
        .filter(|action| action.status == "deferred")
        .map(|action| action.weight)
        .sum();
    let total_weight = actions
        .iter()
        .filter(|action| action.status != "deferred")
        .map(|action| action.weight)
        .sum();
    let verified_weight = actions
        .iter()
        .filter(|action| action.status == "verified")
        .map(|action| action.weight)
        .sum();
    let has_active_actions = actions
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress" | "blocked"));
    let percentage = if total_weight == 0 {
        100.0
    } else {
        let rounded = (verified_weight as f64 / total_weight as f64 * 100.0).round();
        if has_active_actions && rounded >= 100.0 {
            99.0
        } else {
            rounded
        }
    };
    RemediationProgress {
        verified_weight,
        total_weight,
        deferred_weight,
        percentage,
    }
}

fn plan_status(actions: &[RemediationAction]) -> String {
    if actions.iter().any(|action| action.status == "blocked") {
        return "blocked".to_string();
    }
    if actions
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress"))
    {
        return if actions.iter().any(|action| action.status == "in_progress") {
            "in_progress".to_string()
        } else {
            "open".to_string()
        };
    }
    if !actions.is_empty()
        && actions
            .iter()
            .all(|action| matches!(action.status.as_str(), "verified" | "deferred"))
    {
        return if actions.iter().any(|action| action.status == "deferred") {
            "deferred".to_string()
        } else {
            "complete".to_string()
        };
    }
    if actions.is_empty() {
        "complete".to_string()
    } else {
        "open".to_string()
    }
}

fn integration_only_remaining(actions: &[RemediationAction]) -> bool {
    if actions.iter().any(|action| action.status == "blocked") {
        return false;
    }
    let active_material_actions = actions
        .iter()
        .filter(|action| matches!(action.status.as_str(), "open" | "in_progress"))
        .filter(|action| action.stable_key != VERIFICATION_ACTION_KEY)
        .collect::<Vec<_>>();
    !active_material_actions.is_empty()
        && active_material_actions
            .iter()
            .all(|action| action.stable_key.starts_with("branch_hygiene:integrate:"))
}

fn current_stage(actions: &[RemediationAction]) -> String {
    STAGE_ORDER
        .iter()
        .find(|domain| {
            actions.iter().any(|action| {
                action.domain == **domain
                    && action.status != "verified"
                    && action.status != "deferred"
            })
        })
        .unwrap_or(&"complete")
        .to_string()
}

fn track_status(actions: &[&RemediationAction]) -> String {
    if actions.iter().any(|action| action.status == "blocked") {
        "blocked".to_string()
    } else if actions
        .iter()
        .any(|action| matches!(action.status.as_str(), "open" | "in_progress"))
    {
        "in_progress".to_string()
    } else if actions.iter().all(|action| action.status == "deferred") {
        "deferred".to_string()
    } else {
        "complete".to_string()
    }
}

fn domain_label(domain: &str) -> String {
    match domain {
        "evidence_refresh" => "Evidence refresh".to_string(),
        "ci_ideal" => "CI ideal state".to_string(),
        "qr_findings" => "QR findings".to_string(),
        "branch_hygiene" => "Branch hygiene".to_string(),
        value => value
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn stable_id(value: &str, prefix: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let hex = format!("{digest:x}");
    format!("{prefix}-{}", &hex[..16])
}

fn priority_for_weight(weight: u64) -> String {
    if weight >= 4 {
        "P0".to_string()
    } else if weight >= 3 {
        "P1".to_string()
    } else if weight >= 2 {
        "P2".to_string()
    } else {
        "P3".to_string()
    }
}

fn severity_weight(severity: &str) -> u64 {
    match severity.trim().to_ascii_lowercase().as_str() {
        "blocker" | "critical" => 4,
        "error" | "high" => 3,
        "warning" | "p1" => 2,
        "observation" | "info" | "p2" => 1,
        _ => 2,
    }
}

fn freshness_for(observed_at: Option<&str>, max_age_days: u64) -> String {
    let Some(observed_at) = observed_at else {
        return "Unknown".to_string();
    };
    let Ok(timestamp) = DateTime::parse_from_rfc3339(observed_at) else {
        return "Unknown".to_string();
    };
    let age = Utc::now() - timestamp.with_timezone(&Utc);
    if age >= Duration::zero() && age <= Duration::days(max_age_days as i64) {
        "Fresh".to_string()
    } else if age < Duration::zero() {
        "Unknown".to_string()
    } else {
        "Stale".to_string()
    }
}

fn latest_qr_run(repository_path: &Path, fleet_audit_root: Option<&Path>) -> Option<QrRunEvidence> {
    let local = latest_local_qr_run(repository_path);
    let fleet = fleet_qr_run(repository_path, fleet_audit_root);
    match (local, fleet) {
        (Some(local), Some(fleet)) => {
            if fleet.observed_at > local.observed_at
                || (fleet.observed_at == local.observed_at
                    && fleet.findings.len() >= local.findings.len())
            {
                Some(fleet)
            } else {
                Some(local)
            }
        }
        (Some(local), None) => Some(local),
        (None, Some(fleet)) => Some(fleet),
        (None, None) => None,
    }
}

fn latest_local_qr_run(repository_path: &Path) -> Option<QrRunEvidence> {
    let runs = repository_path.join(".quality-runner").join("runs");
    let entries = fs::read_dir(runs).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let run_dir = entry.path();
            let manifest = read_json(&run_dir.join("run-manifest.json"))?;
            let observed_at = first_string(
                &manifest,
                &[
                    &["created_at"],
                    &["started_at"],
                    &["completed_at"],
                    &["finished_at"],
                    &["generated_at"],
                    &["as_of"],
                ],
            );
            let scanned_branch = first_string(
                &manifest,
                &[
                    &["git", "branch"],
                    &["git_provenance", "branch"],
                    &["provenance", "branch"],
                    &["branch"],
                ],
            );
            let scanned_commit = first_string(
                &manifest,
                &[
                    &["git", "head_sha"],
                    &["git_provenance", "head_sha"],
                    &["provenance", "head_sha"],
                    &["head_sha"],
                ],
            );
            let id = first_string(&manifest, &[&["run_id"], &["id"]])
                .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
            Some(QrRunEvidence {
                id,
                run_dir: run_dir.clone(),
                observed_at,
                scanned_branch,
                scanned_commit,
                findings: parse_findings(&run_dir),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.observed_at
            .cmp(&right.observed_at)
            .then_with(|| left.run_dir.cmp(&right.run_dir))
    });
    candidates.pop()
}
