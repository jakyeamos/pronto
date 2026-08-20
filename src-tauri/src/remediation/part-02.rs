fn goal_definition(target_state: &str) -> Option<RemediationGoalProfile> {
    let (label, required, optional, evidence_max_age_days, closure_criteria) = match target_state {
        "public_release" => (
            "Public release",
            ALL_GATE_IDS.to_vec(),
            Vec::new(),
            7,
            vec![
                "The canonical branch and provider state are reconciled.",
                "All release-quality gates have fresh passing evidence.",
                "The repository has an explicit release rule and release recipe.",
                "Public, optional-adapter, and local-only surfaces are classified and the built distribution passes the public-release boundary checks.",
                "Packaging, documentation, versioning, and release evidence are verified.",
            ],
        ),
        "deployed_product" => (
            "Deployed product",
            vec![
                "build",
                "tests",
                "runtime_smoke",
                "lint",
                "typecheck",
                "secrets_scan",
                "dependency_audit",
            ],
            vec!["formatter", "dead_code"],
            7,
            vec![
                "The canonical branch and provider state are reconciled.",
                "Build, test, security, and runtime evidence are fresh and passing.",
                "The deployed operating surface is verified.",
            ],
        ),
        "active_maintained" => (
            "Active maintained repository",
            vec![
                "build",
                "tests",
                "lint",
                "formatter",
                "typecheck",
                "dead_code",
                "secrets_scan",
                "dependency_audit",
            ],
            vec!["runtime_smoke"],
            14,
            vec![
                "The canonical branch and provider state are reconciled.",
                "The maintained development gates have fresh passing evidence.",
                "No active quality or maturity blocker remains.",
            ],
        ),
        "clean_only" => (
            "Clean and preserved",
            Vec::new(),
            vec!["tests", "secrets_scan", "dependency_audit"],
            30,
            vec![
                "All work is intentionally committed, published, or explicitly preserved.",
                "The canonical branch and provider relationship are understood.",
                "No dirty, divergent, unpublished, or ambiguous workspace remains.",
            ],
        ),
        "prototype" => (
            "Preserved prototype",
            Vec::new(),
            vec!["build", "tests", "secrets_scan", "dependency_audit"],
            30,
            vec![
                "The prototype state and limitations are documented.",
                "Unique work is intentionally preserved.",
                "No unresolved safety blocker remains.",
            ],
        ),
        "archived" => (
            "Archived repository",
            Vec::new(),
            Vec::new(),
            30,
            vec![
                "Final work is published or explicitly preserved.",
                "The repository lifecycle is confirmed as archived.",
                "No active worktree, branch operation, or ambiguous unpublished work remains.",
            ],
        ),
        "github_only" => (
            "GitHub only",
            Vec::new(),
            Vec::new(),
            30,
            vec![
                "The repository is intentionally retained on GitHub only because local storage is constrained.",
                "A fresh provider snapshot confirms the GitHub repository and its remote identity.",
                "The terminal remediation task is recorded as GitHub only.",
            ],
        ),
        _ => return None,
    };
    let maturity_policy = maturity_policy_for_target(target_state);
    let maturity_gate_ids = maturity_policy
        .as_ref()
        .map(|_| vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()])
        .unwrap_or_default();
    Some(RemediationGoalProfile {
        schema_version: REMEDIATION_GOAL_SCHEMA.to_string(),
        target_state: target_state.to_string(),
        label: label.to_string(),
        required_gate_ids: required.into_iter().map(str::to_string).collect(),
        optional_gate_ids: optional.into_iter().map(str::to_string).collect(),
        maturity_gate_ids,
        evidence_max_age_days,
        closure_criteria: closure_criteria.into_iter().map(str::to_string).collect(),
        maturity_policy,
        contract_path: REMEDIATION_GOAL_PATH.to_string(),
        ..RemediationGoalProfile::default()
    })
}

fn maturity_improvement_rule() -> String {
    format!(
        "Reaching {MATURITY_CLOSURE_TARGET:.1}/4 clears blocking maturity remediation; a {MATURITY_IDEAL_SCORE:.1}/4 ideal claim additionally requires every configured maturity gate, including Mac Control where applicable, to be fresh and passing. Continue material, evidence-backed improvements toward the ideal without keeping the repository in the active queue solely for stretch work."
    )
}

fn maturity_integrity_rule() -> String {
    "Do not add or accept superficial documentation, configuration, tests, tab stops, Accessibility-only routes, or visual evidence solely to raise the score; each claimed improvement must close a real applicable gap and preserve structural evidence versus behavior-proof boundaries.".to_string()
}

fn maturity_policy_for_target(target_state: &str) -> Option<RemediationMaturityPolicy> {
    matches!(
        target_state,
        "public_release" | "deployed_product" | "active_maintained"
    )
    .then(|| RemediationMaturityPolicy {
        minimum_closure_score: MATURITY_CLOSURE_TARGET,
        ideal_score: MATURITY_IDEAL_SCORE,
        scoring_owner: "Quality Runner canonical maturity feed".to_string(),
        improvement_rule: maturity_improvement_rule(),
        integrity_rule: maturity_integrity_rule(),
        ideal_gate_ids: vec![mac_control_maturity::MAC_CONTROL_GATE_ID.to_string()],
    })
}

fn inferred_goal(repository: &RepositorySnapshot) -> (&'static str, String) {
    let lifecycle = repository.lifecycle.to_ascii_lowercase();
    let candidate = repository.lifecycle_candidate.to_ascii_lowercase();
    if is_github_only_label(&repository.lifecycle)
        || is_github_only_label(&repository.lifecycle_candidate)
    {
        return (
            "github_only",
            "The repository lifecycle records a storage-preserving GitHub-only disposition."
                .to_string(),
        );
    }
    if lifecycle.contains("archiv") || candidate.contains("archiv") {
        return (
            "archived",
            "The repository lifecycle indicates archived or archive-candidate status.".to_string(),
        );
    }
    if ["prototype", "experimental", "incubator"]
        .iter()
        .any(|signal| lifecycle.contains(signal) || candidate.contains(signal))
    {
        return (
            "prototype",
            "The repository lifecycle indicates prototype or experimental work.".to_string(),
        );
    }
    if repository.release_rule.is_some()
        || repository.release_recipe.is_some()
        || repository.confirmed_release_version.is_some()
        || !repository.releases.is_empty()
    {
        return (
            "public_release",
            "Release configuration or release history is present in the Pronto snapshot."
                .to_string(),
        );
    }
    if lifecycle.contains("active") || candidate.contains("active") {
        return (
            "active_maintained",
            "The repository lifecycle indicates active maintained work.".to_string(),
        );
    }
    (
        "clean_only",
        "No release, deployment, or active-product signal is confirmed; clean preservation is the conservative default.".to_string(),
    )
}

fn is_github_only_label(value: &str) -> bool {
    value.trim().to_ascii_lowercase().replace(['_', '-'], " ") == "github only"
}

fn normalized_gate_ids(values: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for value in values {
        normalized.push(quality::normalize_declared_gate_id(value)?);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalized_maturity_gate_ids(values: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for value in values {
        let gate_id = value.trim().to_ascii_lowercase();
        if !ALL_MATURITY_GATE_IDS.contains(&gate_id.as_str()) {
            return Err(format!("Unknown remediation maturity gate '{value}'."));
        }
        normalized.push(gate_id);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalized_phase_token(value: &str, label: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || !normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(format!(
            "Remediation phase {label} '{value}' must contain only letters, numbers, underscores, or hyphens."
        ));
    }
    Ok(normalized)
}

fn normalized_remediation_phases(
    values: &[RemediationPhaseDefinition],
) -> Result<Vec<RemediationPhaseDefinition>, String> {
    let mut known_phase_ids = DEFAULT_REMEDIATION_PHASE_IDS
        .iter()
        .map(|value| (*value).to_string())
        .collect::<HashSet<_>>();
    known_phase_ids.insert(UNCLASSIFIED_REMEDIATION_PHASE_ID.to_string());
    let mut claimed_domains = HashSet::new();
    let mut normalized = Vec::new();

    for value in values {
        let id = normalized_phase_token(&value.id, "id")?;
        if !known_phase_ids.insert(id.clone()) {
            return Err(format!(
                "Remediation phase id '{}' is duplicated or reserved by the planner.",
                value.id
            ));
        }
        if value.title.trim().is_empty()
            || value.summary.trim().is_empty()
            || value.completion_criterion.trim().is_empty()
        {
            return Err(format!(
                "Remediation phase '{id}' requires a non-empty title, summary, and completion_criterion."
            ));
        }
        let mut domains = Vec::new();
        for domain in &value.domains {
            let domain = normalized_phase_token(domain, "domain")?;
            if !claimed_domains.insert(domain.clone()) {
                return Err(format!(
                    "Remediation action domain '{domain}' is claimed by more than one repository phase."
                ));
            }
            domains.push(domain);
        }
        domains.sort();
        domains.dedup();
        if domains.is_empty() {
            return Err(format!(
                "Remediation phase '{id}' must claim at least one action domain."
            ));
        }
        let after_phase_id = value
            .after_phase_id
            .as_deref()
            .map(|after| normalized_phase_token(after, "after_phase_id"))
            .transpose()?;
        if let Some(after) = after_phase_id.as_ref() {
            if after == &id || !known_phase_ids.contains(after) {
                return Err(format!(
                    "Remediation phase '{id}' references unknown or later phase '{after}'."
                ));
            }
        }
        normalized.push(RemediationPhaseDefinition {
            id,
            title: value.title.trim().to_string(),
            summary: value.summary.trim().to_string(),
            domains,
            completion_criterion: value.completion_criterion.trim().to_string(),
            after_phase_id,
        });
    }

    Ok(normalized)
}

fn inferred_goal_profile(
    repository: &RepositorySnapshot,
    error: Option<String>,
) -> RemediationGoalProfile {
    let (target_state, reason) = inferred_goal(repository);
    let mut profile = goal_definition(target_state).expect("inferred goal must be supported");
    profile.source = "inferred".to_string();
    profile.confidence = "Medium".to_string();
    profile.reason = reason;
    profile.error = error;
    profile
}

fn resolve_goal_profile_base(repository: &RepositorySnapshot) -> RemediationGoalProfile {
    let contract_path = Path::new(&repository.path).join(REMEDIATION_GOAL_PATH);
    if !contract_path.is_file() {
        return inferred_goal_profile(repository, None);
    }
    let contract = fs::read_to_string(&contract_path)
        .map_err(|error| format!("Could not read the remediation goal contract: {error}"))
        .and_then(|contents| {
            serde_json::from_str::<RepositoryGoalContract>(&contents)
                .map_err(|error| format!("Invalid remediation goal JSON: {error}"))
        });
    let contract = match contract {
        Ok(contract) => contract,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    if contract.schema_version != REMEDIATION_GOAL_SCHEMA {
        return inferred_goal_profile(
            repository,
            Some(format!(
                "Unsupported remediation goal schema '{}'; expected '{}'.",
                contract.schema_version, REMEDIATION_GOAL_SCHEMA
            )),
        );
    }
    if contract.reason.trim().is_empty() {
        return inferred_goal_profile(
            repository,
            Some("The remediation goal contract must include a non-empty reason.".to_string()),
        );
    }
    let Some(mut profile) = goal_definition(contract.target_state.trim()) else {
        return inferred_goal_profile(
            repository,
            Some(format!(
                "Unknown remediation target state '{}'.",
                contract.target_state
            )),
        );
    };
    let additional_required = match normalized_gate_ids(&contract.additional_required_gate_ids) {
        Ok(gates) => gates,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    let optional = match normalized_gate_ids(&contract.optional_gate_ids) {
        Ok(gates) => gates,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    let additional_maturity =
        match normalized_maturity_gate_ids(&contract.additional_maturity_gate_ids) {
            Ok(gates) => gates,
            Err(error) => return inferred_goal_profile(repository, Some(error)),
        };
    let remediation_phases = match normalized_remediation_phases(&contract.remediation_phases) {
        Ok(phases) => phases,
        Err(error) => return inferred_goal_profile(repository, Some(error)),
    };
    profile.required_gate_ids.extend(additional_required);
    profile.required_gate_ids.sort();
    profile.required_gate_ids.dedup();
    profile.optional_gate_ids.extend(optional);
    profile.optional_gate_ids.sort();
    profile.optional_gate_ids.dedup();
    profile
        .optional_gate_ids
        .retain(|gate| !profile.required_gate_ids.contains(gate));
    profile.maturity_gate_ids.extend(additional_maturity);
    profile.maturity_gate_ids.sort();
    profile.maturity_gate_ids.dedup();
    if let Some(days) = contract.evidence_max_age_days {
        if !(1..=90).contains(&days) {
            return inferred_goal_profile(
                repository,
                Some("evidence_max_age_days must be between 1 and 90.".to_string()),
            );
        }
        profile.evidence_max_age_days = days;
    }
    profile.source = "repository_contract".to_string();
    profile.confidence = "High".to_string();
    profile.reason = contract.reason.trim().to_string();
    profile.remediation_phases = remediation_phases;
    profile
}

fn resolve_goal_profile(repository: &RepositorySnapshot) -> RemediationGoalProfile {
    let mut profile = resolve_goal_profile_base(repository);
    if matches!(
        profile.target_state.as_str(),
        "public_release" | "deployed_product" | "active_maintained"
    ) && repository.quality.ci_readiness.profile_source == "repository_contract"
    {
        profile.required_gate_ids.extend(
            repository
                .quality
                .ci_readiness
                .applicable_gate_ids
                .iter()
                .filter(|gate_id| gate_id.starts_with("custom:"))
                .cloned(),
        );
        profile.required_gate_ids.sort();
        profile.required_gate_ids.dedup();
        profile
            .optional_gate_ids
            .retain(|gate_id| !profile.required_gate_ids.contains(gate_id));
    }
    profile
}

fn goal_requires_provider(goal: &RemediationGoalProfile) -> bool {
    goal.target_state != "prototype"
}

fn goal_requires_quality_evidence(goal: &RemediationGoalProfile) -> bool {
    matches!(
        goal.target_state.as_str(),
        "public_release" | "deployed_product" | "active_maintained"
    )
}

fn goal_requires_maturity(goal: &RemediationGoalProfile) -> bool {
    matches!(
        goal.target_state.as_str(),
        "public_release" | "deployed_product" | "active_maintained"
    )
}

pub(crate) fn repository_requires_maturity(repository: &RepositorySnapshot) -> bool {
    goal_requires_maturity(&resolve_goal_profile(repository))
}

pub(crate) fn repository_requires_public_release_boundary(repository: &RepositorySnapshot) -> bool {
    resolve_goal_profile(repository).target_state == "public_release"
}

pub(crate) fn repository_requires_maturity_gate(
    repository: &RepositorySnapshot,
    gate_id: &str,
) -> bool {
    resolve_goal_profile(repository)
        .maturity_gate_ids
        .iter()
        .any(|configured| configured == gate_id)
}
