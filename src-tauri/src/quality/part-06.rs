pub fn evaluate_ci_readiness_for_ideal_with_configuration(
    gates: &[QualityGate],
    ideal_gate_ids: &[String],
    configured_gate_ids: &[String],
) -> QualityReadiness {
    let mut applicable_gate_ids = Vec::new();
    let mut seen_gate_ids = HashSet::new();
    for gate_id in ideal_gate_ids {
        let normalized_gate_id = normalize_gate_id(gate_id);
        if seen_gate_ids.insert(normalized_gate_id.clone()) {
            applicable_gate_ids.push(normalized_gate_id);
        }
    }
    let mut readiness = QualityReadiness {
        applicable_gate_ids: applicable_gate_ids.clone(),
        ..QualityReadiness::default()
    };
    let configured_gate_ids = configured_gate_ids
        .iter()
        .map(|gate_id| normalize_gate_id(gate_id))
        .collect::<HashSet<_>>();

    for gate_id in &applicable_gate_ids {
        if configured_gate_ids.contains(gate_id) {
            readiness.configured_gate_ids.push(gate_id.clone());
        } else {
            readiness.unconfigured_gate_ids.push(gate_id.clone());
        }
        let Some(gate) = gates.iter().find(|gate| gate.id == *gate_id) else {
            readiness.missing_gate_ids.push(gate_id.clone());
            continue;
        };
        if gate.status != QualityGateStatus::NotConfigured && !gate.evidence.is_empty() {
            readiness.covered_gate_ids.push(gate_id.clone());
        }
        if gate.status == QualityGateStatus::Passed && gate.freshness == QualityFreshness::Fresh {
            readiness.fresh_passing_gate_ids.push(gate_id.clone());
        }
        match gate.status {
            QualityGateStatus::Passed if gate.freshness == QualityFreshness::Fresh => {}
            QualityGateStatus::Passed => readiness.stale_gate_ids.push(gate_id.clone()),
            QualityGateStatus::Failed => readiness.failed_gate_ids.push(gate_id.clone()),
            QualityGateStatus::Blocked => readiness.blocked_gate_ids.push(gate_id.clone()),
            QualityGateStatus::NotConfigured => readiness.missing_gate_ids.push(gate_id.clone()),
        }
    }

    let applicable_gate_count = applicable_gate_ids.len();
    if applicable_gate_count > 0 {
        let configuration_score =
            (readiness.configured_gate_ids.len() as f64 / applicable_gate_count as f64) * 4.0;
        let configuration_score = (configuration_score * 100.0).round() / 100.0;
        readiness.configuration_score = Some(configuration_score);
        readiness.configuration_score_display = Some(format_quality_score(configuration_score));
        let evidence_coverage_score =
            (readiness.covered_gate_ids.len() as f64 / applicable_gate_count as f64) * 4.0;
        let evidence_coverage_score = (evidence_coverage_score * 100.0).round() / 100.0;
        readiness.evidence_coverage_score = Some(evidence_coverage_score);
        readiness.evidence_coverage_score_display =
            Some(format_quality_score(evidence_coverage_score));
        let fresh_passing_score =
            (readiness.fresh_passing_gate_ids.len() as f64 / applicable_gate_count as f64) * 4.0;
        let fresh_passing_score = (fresh_passing_score * 100.0).round() / 100.0;
        readiness.score = Some(fresh_passing_score);
        readiness.score_display = Some(format_quality_score(fresh_passing_score));
    }
    readiness
}

fn normalize_repository_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn recommendation_matrix_profiles() -> Vec<(String, Vec<String>)> {
    RECOMMENDATION_MATRIX_MARKDOWN
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with('|') || !line.ends_with('|') {
                return None;
            }
            let cells = line[1..line.len() - 1]
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.len() < 11
                || cells[0] == "Project"
                || cells[0].chars().all(|character| character == '-')
            {
                return None;
            }
            let project = normalize_repository_key(cells[0]);
            if project.is_empty() || matches!(cells[1], "PENDING" | "FIXTURE?") {
                return Some((project, Vec::new()));
            }
            let ideal_gate_ids = RECOMMENDATION_MATRIX_GATE_COLUMNS
                .iter()
                .filter_map(|(gate_id, column)| {
                    matches!(cells[*column], "K" | "N" | "A" | "C").then(|| (*gate_id).to_string())
                })
                .collect::<Vec<_>>();
            Some((project, ideal_gate_ids))
        })
        .collect()
}

fn invalid_ci_gate_profile(message: impl Into<String>) -> CiGateProfile {
    CiGateProfile {
        source: "invalid_repository_contract".to_string(),
        contract_path: Some(CI_GATE_PROFILE_RELATIVE_PATH.to_string()),
        error: Some(message.into()),
        ..CiGateProfile::default()
    }
}

pub fn normalize_declared_gate_id(value: &str) -> Result<String, String> {
    let declared = value.trim().to_ascii_lowercase();
    if CI_STANDARD_GATE_IDS.contains(&declared.as_str()) {
        return Ok(declared);
    }
    let Some(custom_value) = declared.strip_prefix("custom:") else {
        return Err(format!(
            "Unknown CI gate '{value}'. Use a standard gate id or an explicit custom:<snake_case_id>."
        ));
    };
    let custom_slug = slug(custom_value);
    if custom_slug.is_empty() {
        return Err("A custom CI gate id must include a value after 'custom:'.".to_string());
    }
    let normalized = format!("custom:{custom_slug}");
    if normalized != declared {
        return Err(format!(
            "Custom CI gate '{value}' must use its stable normalized id '{normalized}'."
        ));
    }
    if CI_STANDARD_GATE_IDS.contains(&custom_slug.as_str()) {
        return Err(format!(
            "Custom CI gate '{value}' duplicates the standard '{custom_slug}' gate."
        ));
    }
    Ok(normalized)
}

fn validate_repository_ci_gate_profile(
    contract: RepositoryCiGateProfileContract,
) -> Result<CiGateProfile, String> {
    if contract.schema_version != CI_GATE_PROFILE_SCHEMA {
        return Err(format!(
            "Unsupported CI gate profile schema '{}'; expected '{}'.",
            contract.schema_version, CI_GATE_PROFILE_SCHEMA
        ));
    }
    let profile_reason = contract.reason.trim();
    if profile_reason.is_empty() {
        return Err("The CI gate profile must include a non-empty reason.".to_string());
    }

    let mut profile = CiGateProfile {
        source: "repository_contract".to_string(),
        contract_path: Some(CI_GATE_PROFILE_RELATIVE_PATH.to_string()),
        reason: Some(profile_reason.to_string()),
        ..CiGateProfile::default()
    };
    let mut seen = HashSet::new();

    for gate in contract.gates {
        let gate_id = normalize_declared_gate_id(&gate.id)?;
        if !seen.insert(gate_id.clone()) {
            return Err(format!("CI gate '{gate_id}' is declared more than once."));
        }
        let reason = gate.reason.trim();
        if reason.is_empty() {
            return Err(format!(
                "CI gate '{gate_id}' must include a non-empty reason."
            ));
        }
        let is_custom = gate_id.starts_with("custom:");
        if is_custom && matches!(gate.classification, CiGateClassification::NotApplicable) {
            return Err(format!(
                "Custom CI gate '{gate_id}' cannot be not_applicable; omit it when it does not apply."
            ));
        }
        let label = if is_custom {
            gate.label
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("Custom CI gate '{gate_id}' must include a label."))?
                .to_string()
        } else {
            gate.label
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| gate_label(&gate_id))
        };
        profile.gate_labels.insert(gate_id.clone(), label);
        profile
            .gate_reasons
            .insert(gate_id.clone(), reason.to_string());
        match gate.classification {
            CiGateClassification::Required => profile.required_gate_ids.push(gate_id),
            CiGateClassification::Optional => profile.optional_gate_ids.push(gate_id),
            CiGateClassification::NotApplicable => profile.not_applicable_gate_ids.push(gate_id),
        }
    }

    let missing_standard = CI_STANDARD_GATE_IDS
        .iter()
        .filter(|gate_id| !seen.contains(**gate_id))
        .copied()
        .collect::<Vec<_>>();
    if !missing_standard.is_empty() {
        return Err(format!(
            "The CI gate profile must classify every standard gate; missing: {}.",
            missing_standard.join(", ")
        ));
    }
    profile.required_gate_ids.sort();
    profile.optional_gate_ids.sort();
    profile.not_applicable_gate_ids.sort();
    Ok(profile)
}

pub fn ci_gate_profile_for_repository(repository: &RepositorySnapshot) -> CiGateProfile {
    let contract_path = Path::new(&repository.path).join(CI_GATE_PROFILE_RELATIVE_PATH);
    if contract_path.is_file() {
        let contract = fs::read_to_string(&contract_path)
            .map_err(|error| format!("Could not read the CI gate profile: {error}"))
            .and_then(|contents| {
                serde_json::from_str::<RepositoryCiGateProfileContract>(&contents)
                    .map_err(|error| format!("Invalid CI gate profile JSON: {error}"))
            })
            .and_then(validate_repository_ci_gate_profile);
        return contract.unwrap_or_else(invalid_ci_gate_profile);
    }

    let key = normalize_repository_key(&repository.name);
    let matches = recommendation_matrix_profiles()
        .into_iter()
        .filter(|(project, _)| project == &key)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return CiGateProfile {
            error: Some(if matches.is_empty() {
                format!(
                    "No repository CI gate profile or unique compatibility-matrix row was found for '{}'.",
                    repository.name
                )
            } else {
                format!(
                    "The compatibility matrix contains more than one row for '{}'.",
                    repository.name
                )
            }),
            ..CiGateProfile::default()
        };
    }
    let required_gate_ids = matches
        .into_iter()
        .next()
        .map(|(_, gates)| gates)
        .unwrap_or_default();
    if required_gate_ids.is_empty() {
        return CiGateProfile {
            error: Some(format!(
                "The compatibility-matrix row for '{}' is pending or declares no applicable gates.",
                repository.name
            )),
            ..CiGateProfile::default()
        };
    }
    let required = required_gate_ids.iter().cloned().collect::<HashSet<_>>();
    CiGateProfile {
        source: "recommendation_matrix".to_string(),
        reason: Some(
            "Compatibility profile from docs/quality-gate-recommendation-matrix.md; add a repository contract to classify current standard and custom gates."
                .to_string(),
        ),
        required_gate_ids,
        not_applicable_gate_ids: CI_STANDARD_GATE_IDS
            .iter()
            .filter(|gate_id| !required.contains(**gate_id))
            .map(|gate_id| (*gate_id).to_string())
            .collect(),
        gate_labels: CI_STANDARD_GATE_IDS
            .iter()
            .map(|gate_id| ((*gate_id).to_string(), gate_label(gate_id)))
            .collect(),
        ..CiGateProfile::default()
    }
}

pub fn ideal_gate_ids_for_repository(repository: &RepositorySnapshot) -> Option<Vec<String>> {
    let profile = ci_gate_profile_for_repository(repository);
    (!profile.required_gate_ids.is_empty()).then_some(profile.required_gate_ids)
}

pub fn update_ci_readiness_summary(
    portfolio: &mut QualityPortfolioSnapshot,
    repositories: &[RepositorySnapshot],
) {
    let scores = repositories
        .iter()
        .filter_map(|repository| repository.quality.ci_readiness.score)
        .collect::<Vec<_>>();
    portfolio.ci_readiness_repository_count = scores.len();
    portfolio.ci_readiness_unscored_repository_count =
        repositories.len().saturating_sub(scores.len());
    portfolio.ci_readiness_full_repository_count =
        scores.iter().filter(|score| **score >= 4.0).count();
    portfolio.ci_readiness_score = if scores.is_empty() {
        None
    } else {
        let score = scores.iter().sum::<f64>() / scores.len() as f64;
        Some((score * 100.0).round() / 100.0)
    };
    portfolio.ci_readiness_score_display = portfolio.ci_readiness_score.map(format_quality_score);
    let evidence_coverage_scores = repositories
        .iter()
        .filter_map(|repository| repository.quality.ci_readiness.evidence_coverage_score)
        .collect::<Vec<_>>();
    portfolio.ci_evidence_coverage_score = average_quality_scores(&evidence_coverage_scores);
    portfolio.ci_evidence_coverage_score_display = portfolio
        .ci_evidence_coverage_score
        .map(format_quality_score);
    let configuration_scores = repositories
        .iter()
        .filter_map(|repository| repository.quality.ci_readiness.configuration_score)
        .collect::<Vec<_>>();
    portfolio.ci_configuration_score = average_quality_scores(&configuration_scores);
    portfolio.ci_configuration_score_display =
        portfolio.ci_configuration_score.map(format_quality_score);
    portfolio.ci_evidence_fresh_passing_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.fresh_passing_gate_ids.len())
        .sum();
    portfolio.ci_evidence_ideal_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.applicable_gate_ids.len())
        .sum();
    portfolio.ci_configuration_configured_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.configured_gate_ids.len())
        .sum();
    portfolio.ci_configuration_ideal_gate_count = repositories
        .iter()
        .map(|repository| repository.quality.ci_readiness.applicable_gate_ids.len())
        .sum();
    let configured_profiles = repositories
        .iter()
        .filter(|repository| {
            !repository
                .quality
                .ci_readiness
                .applicable_gate_ids
                .is_empty()
        })
        .collect::<Vec<_>>();
    portfolio.ci_configuration_repository_count = configured_profiles.len();
    portfolio.ci_configuration_unscored_repository_count =
        repositories.len().saturating_sub(configured_profiles.len());
    portfolio.ci_configuration_full_repository_count = configured_profiles
        .iter()
        .filter(|repository| {
            repository
                .quality
                .ci_readiness
                .unconfigured_gate_ids
                .is_empty()
        })
        .count();
    portfolio.ci_profile_repository_contract_count = repositories
        .iter()
        .filter(|repository| {
            repository.quality.ci_readiness.profile_source == "repository_contract"
        })
        .count();
    portfolio.ci_profile_compatibility_count = repositories
        .iter()
        .filter(|repository| {
            repository.quality.ci_readiness.profile_source == "recommendation_matrix"
        })
        .count();
    portfolio.ci_profile_invalid_count = repositories
        .iter()
        .filter(|repository| {
            repository.quality.ci_readiness.profile_source == "invalid_repository_contract"
        })
        .count();
    portfolio.ci_profile_unavailable_count = repositories
        .iter()
        .filter(|repository| repository.quality.ci_readiness.profile_source == "unavailable")
        .count();
    portfolio.ci_readiness_open_gate_counts = BTreeMap::new();
    for repository in repositories {
        let readiness = &repository.quality.ci_readiness;
        for gate_id in readiness
            .missing_gate_ids
            .iter()
            .chain(readiness.stale_gate_ids.iter())
            .chain(readiness.failed_gate_ids.iter())
            .chain(readiness.blocked_gate_ids.iter())
        {
            *portfolio
                .ci_readiness_open_gate_counts
                .entry(gate_id.clone())
                .or_default() += 1;
        }
    }
}

const LEGACY_COMPOSITE_MATURITY_DIMENSIONS: [&str; 10] = [
    "ci.configuration",
    "ci.evidence_coverage",
    "ci.fresh_passing",
    "project_compass.mvp_progress",
    "project_compass.complete_product_progress",
    "mac_control.implementation_contract",
    "mac_control.live_task_evidence",
    "mac_control.task_usability",
    "web_readiness.user_journey",
    "product_readiness",
];
