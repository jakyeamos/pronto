pub fn normalize_gate_id(value: &str) -> String {
    if let Some(custom_value) = value.trim().to_ascii_lowercase().strip_prefix("custom:") {
        return format!("custom:{}", slug(custom_value));
    }
    let slug = slug(value);
    match slug.as_str() {
        "build" | "compile" | "bundle" => "build".to_string(),
        "verify_and_build" => "build".to_string(),
        "smoke" | "smoke_test" | "runtime_smoke" | "runtime_smoke_test" | "runtime_test" => {
            "runtime_smoke".to_string()
        }
        "test" | "tests" | "test_suite" | "unit_test" | "unit_tests" | "integration_test"
        | "integration_tests" | "e2e_test" | "e2e_tests" | "full_suite" => "tests".to_string(),
        "lint" | "linting" => "lint".to_string(),
        "format" | "formatter" | "formatting" | "fmt" => "formatter".to_string(),
        "typecheck" | "type_check" | "typechecking" | "check_types" => "typecheck".to_string(),
        "dead_code" | "deadcode" | "unused_code" => "dead_code".to_string(),
        "secrets_scan"
        | "secret_scan"
        | "secret_scanning"
        | "security_secrets_scan"
        | "secret_scanning_gitleaks"
        | "gitleaks" => "secrets_scan".to_string(),
        "debloat" | "repository_debloat" | "repository_debloat_maturity" => "debloat".to_string(),
        "dependency_audit"
        | "dependency_scan"
        | "dependency_check"
        | "security_dependency_audit"
        | "software_composition_analysis" => "dependency_audit".to_string(),
        "web_readiness" | "web_production_readiness" | "production_web_readiness" => {
            "web_readiness".to_string()
        }
        value if value.starts_with("unit_tests_") => "tests".to_string(),
        value if value.starts_with("integration_tests_") => "tests".to_string(),
        value if value.starts_with("e2e_tests_") => "tests".to_string(),
        value if value.starts_with("full_suite_") => "tests".to_string(),
        _ => format!("custom:{slug}"),
    }
}

pub fn gate_label(id: &str) -> String {
    CANONICAL_GATE_DEFINITIONS
        .iter()
        .chain(CONDITIONAL_GATE_DEFINITIONS.iter())
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| {
            let value = id.strip_prefix("custom:").unwrap_or(id);
            value
                .split('_')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    chars
                        .next()
                        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
}

pub fn normalize_requirement_source(value: &str) -> Option<QualitySource> {
    QualitySource::parse(value)
}

pub fn evaluate_freshness_at(
    observed_at: Option<&str>,
    scanned_commit: Option<&str>,
    scanned_branch: Option<&str>,
    current_commit: Option<&str>,
    current_branch: Option<&str>,
    now: DateTime<Utc>,
) -> QualityFreshness {
    let Some(observed_at) = observed_at else {
        return QualityFreshness::Unknown;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(observed_at) else {
        return QualityFreshness::Unknown;
    };
    let age = now.signed_duration_since(parsed.with_timezone(&Utc));
    if age > Duration::days(MAX_EVIDENCE_AGE_DAYS) {
        return QualityFreshness::Stale;
    }
    match (scanned_commit, current_commit) {
        (Some(scanned), Some(current)) => {
            if scanned == current {
                QualityFreshness::Fresh
            } else {
                QualityFreshness::Stale
            }
        }
        (Some(_), None) | (None, Some(_)) => QualityFreshness::Unknown,
        (None, None) => match (scanned_branch, current_branch) {
            (Some(scanned), Some(current)) if scanned != current => QualityFreshness::Stale,
            _ => QualityFreshness::Unknown,
        },
    }
}

pub fn evaluate_audit_freshness_at(
    observed_at: Option<&str>,
    now: DateTime<Utc>,
) -> QualityFreshness {
    let Some(observed_at) = observed_at else {
        return QualityFreshness::Unknown;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(observed_at) else {
        return QualityFreshness::Unknown;
    };
    if now.signed_duration_since(parsed.with_timezone(&Utc)) > Duration::days(MAX_EVIDENCE_AGE_DAYS)
    {
        QualityFreshness::Stale
    } else {
        QualityFreshness::Fresh
    }
}

pub fn aggregate_gate_status(
    evidence: &[QualityEvidence],
) -> (QualityGateStatus, QualityFreshness) {
    if evidence.is_empty() {
        return (QualityGateStatus::NotConfigured, QualityFreshness::Unknown);
    }
    let has_passed = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Passed);
    let has_failed = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Failed);
    let has_blocked = evidence
        .iter()
        .any(|item| item.status == QualityGateStatus::Blocked);
    let conflict = has_passed && (has_failed || has_blocked);
    let status = if conflict {
        QualityGateStatus::Blocked
    } else if has_blocked {
        QualityGateStatus::Blocked
    } else if has_failed {
        QualityGateStatus::Failed
    } else if has_passed {
        QualityGateStatus::Passed
    } else {
        QualityGateStatus::NotConfigured
    };
    let freshness = if conflict {
        QualityFreshness::Conflicted
    } else if status == QualityGateStatus::Passed
        && evidence.iter().any(|item| {
            item.status == QualityGateStatus::Passed && item.freshness == QualityFreshness::Fresh
        })
    {
        QualityFreshness::Fresh
    } else if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Stale)
    {
        QualityFreshness::Stale
    } else if evidence
        .iter()
        .all(|item| item.freshness == QualityFreshness::Unknown)
    {
        QualityFreshness::Unknown
    } else {
        QualityFreshness::Unknown
    };
    (status, freshness)
}

fn parse_verification_level(value: Option<&str>) -> QualityVerificationLevel {
    match value.unwrap_or_default() {
        "source_inferred" => QualityVerificationLevel::SourceInferred,
        "artifact_inspected" => QualityVerificationLevel::ArtifactInspected,
        "browser_rendered" => QualityVerificationLevel::BrowserRendered,
        "deployment_verified" => QualityVerificationLevel::DeploymentVerified,
        _ => QualityVerificationLevel::Unknown,
    }
}

fn web_readiness_gate_status(status: &str) -> QualityGateStatus {
    match status {
        "ready" | "warnings" => QualityGateStatus::Passed,
        "blocked" => QualityGateStatus::Failed,
        "not_applicable" => QualityGateStatus::NotConfigured,
        _ => QualityGateStatus::Blocked,
    }
}

fn web_readiness_display_status(status: &str) -> String {
    match status {
        "ready" => "Ready",
        "warnings" => "Warnings",
        "blocked" => "Blocked",
        "not_applicable" => "Not applicable",
        _ => "Unknown",
    }
    .to_string()
}

fn web_readiness_target_level(kind: &str) -> QualityVerificationLevel {
    match kind {
        "deployment" => QualityVerificationLevel::DeploymentVerified,
        "browser" => QualityVerificationLevel::BrowserRendered,
        "artifact" => QualityVerificationLevel::ArtifactInspected,
        "source" => QualityVerificationLevel::SourceInferred,
        _ => QualityVerificationLevel::Unknown,
    }
}

fn invalid_web_readiness_evidence(report_path: &Path, detail: String) -> QualityEvidence {
    QualityEvidence {
        id: "web_readiness".to_string(),
        source: QualitySource::Qr,
        status: QualityGateStatus::Blocked,
        freshness: QualityFreshness::Unknown,
        observed_at: None,
        scanned_commit: None,
        scanned_branch: None,
        command: Some("qr web-readiness . --json".to_string()),
        source_label: "Quality Runner web readiness".to_string(),
        report_path: Some(report_path.to_string_lossy().to_string()),
        report_url: None,
        report_kind: Some("Quality Runner web readiness".to_string()),
        detail,
        verification_level: QualityVerificationLevel::Unknown,
        target_kind: None,
        target_url: None,
        target_provider: None,
        deployment_id: None,
    }
}

fn import_web_readiness(
    repository: &RepositorySnapshot,
) -> (WebReadinessSnapshot, Option<QualityEvidence>) {
    let report_path = Path::new(&repository.path).join(WEB_READINESS_RELATIVE_PATH);
    if !report_path.is_file() {
        return (WebReadinessSnapshot::default(), None);
    }
    let invalid = |detail: String| {
        (
            WebReadinessSnapshot {
                report_path: Some(report_path.to_string_lossy().to_string()),
                applicability_reason: Some(detail.clone()),
                ..WebReadinessSnapshot::default()
            },
            Some(invalid_web_readiness_evidence(&report_path, detail)),
        )
    };
    let contents = match fs::read_to_string(&report_path) {
        Ok(contents) => contents,
        Err(error) => return invalid(format!("Web-readiness report could not be read: {error}")),
    };
    let payload = match serde_json::from_str::<Value>(&contents) {
        Ok(payload) => payload,
        Err(error) => return invalid(format!("Web-readiness report is not valid JSON: {error}")),
    };
    if json_string_at(&payload, &["schema"]).as_deref() != Some(WEB_READINESS_SCHEMA) {
        return invalid(format!(
            "Web-readiness report must use schema {WEB_READINESS_SCHEMA}"
        ));
    }

    let status = json_string_at(&payload, &["status"]).unwrap_or_else(|| "unknown".to_string());
    let observed_at = json_string_at(&payload, &["generated_at"]);
    let scanned_commit = json_string_at(&payload, &["repository", "head_sha"]);
    let scanned_branch = json_string_at(&payload, &["repository", "branch"]);
    let applicability = json_string_at(&payload, &["applicability", "status"])
        .unwrap_or_else(|| "unknown".to_string());
    let target_kind = json_string_at(&payload, &["target", "kind"]).unwrap_or_default();
    let verification_level = web_readiness_target_level(&target_kind);
    let freshness = evaluate_freshness_at(
        observed_at.as_deref(),
        scanned_commit.as_deref(),
        scanned_branch.as_deref(),
        repository.workspace.last_commit.as_deref(),
        Some(repository.branch.as_str()),
        Utc::now(),
    );
    let checks = payload
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|check| {
            let id = json_string_at(check, &["id"])?;
            let routes = check
                .get("evidence")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| json_string_at(item, &["route"]))
                .collect::<Vec<_>>();
            Some(WebReadinessCheck {
                id,
                label: json_string_at(check, &["label"]).unwrap_or_else(|| "Web check".to_string()),
                category: json_string_at(check, &["category"])
                    .unwrap_or_else(|| "baseline".to_string()),
                policy: json_string_at(check, &["policy"]).unwrap_or_else(|| "block".to_string()),
                status: json_string_at(check, &["status"]).unwrap_or_else(|| "unknown".to_string()),
                verification_level: parse_verification_level(
                    json_string_at(check, &["verification_level"]).as_deref(),
                ),
                detail: json_string_at(check, &["detail"]).unwrap_or_default(),
                routes,
            })
        })
        .collect::<Vec<_>>();
    let warning_count = checks
        .iter()
        .filter(|check| check.policy == "warn" && check.status != "passed")
        .count() as u64;
    let target = WebReadinessTarget {
        kind: target_kind.clone(),
        commit: json_string_at(&payload, &["target", "commit"]),
        url: json_string_at(&payload, &["target", "url"]),
        provider: json_string_at(&payload, &["target", "provider"]),
        deployment_id: json_string_at(&payload, &["target", "deployment_id"]),
        artifact_digest: json_string_at(&payload, &["target", "artifact_digest"]),
    };
    let snapshot = WebReadinessSnapshot {
        status: web_readiness_display_status(&status),
        applicability,
        applicability_reason: json_string_at(&payload, &["applicability", "reason"]),
        freshness: freshness.clone(),
        observed_at: observed_at.clone(),
        scanned_commit: scanned_commit.clone(),
        scanned_branch: scanned_branch.clone(),
        report_path: Some(report_path.to_string_lossy().to_string()),
        target: target.clone(),
        passed_count: json_u64_at(&payload, &["summary", "passed"]).unwrap_or(0),
        failed_count: json_u64_at(&payload, &["summary", "failed"]).unwrap_or(0),
        blocked_count: json_u64_at(&payload, &["summary", "blocked"]).unwrap_or(0),
        unknown_count: json_u64_at(&payload, &["summary", "unknown"]).unwrap_or(0),
        warning_count,
        checks,
    };
    let detail = format!(
        "{} web readiness: {} passed, {} failed, {} blocked, {} unknown, {} warning",
        snapshot.status,
        snapshot.passed_count,
        snapshot.failed_count,
        snapshot.blocked_count,
        snapshot.unknown_count,
        snapshot.warning_count
    );
    let evidence = QualityEvidence {
        id: "web_readiness".to_string(),
        source: QualitySource::Qr,
        status: web_readiness_gate_status(&status),
        freshness,
        observed_at,
        scanned_commit,
        scanned_branch,
        command: Some("qr web-readiness . --json".to_string()),
        source_label: "Quality Runner web readiness".to_string(),
        report_path: snapshot.report_path.clone(),
        report_url: target.url.clone(),
        report_kind: Some("Quality Runner web readiness".to_string()),
        detail,
        verification_level,
        target_kind: (!target.kind.is_empty()).then_some(target.kind),
        target_url: target.url,
        target_provider: target.provider,
        deployment_id: target.deployment_id,
    };
    (snapshot, Some(evidence))
}

fn ensure_profile_gates(gates: &mut Vec<QualityGate>, profile: &CiGateProfile) {
    for gate_id in profile
        .required_gate_ids
        .iter()
        .chain(profile.optional_gate_ids.iter())
    {
        let label = profile
            .gate_labels
            .get(gate_id)
            .cloned()
            .unwrap_or_else(|| gate_label(gate_id));
        if let Some(gate) = gates.iter_mut().find(|gate| gate.id == *gate_id) {
            gate.label = label;
        } else {
            gates.push(QualityGate {
                id: gate_id.clone(),
                label,
                status: QualityGateStatus::NotConfigured,
                freshness: QualityFreshness::Unknown,
                evidence: Vec::new(),
            });
        }
    }
    gates.sort_by_key(|gate| gate_sort_key(&gate.id));
}

fn apply_ci_gate_profile(readiness: &mut QualityReadiness, profile: &CiGateProfile) {
    readiness.profile_source = profile.source.clone();
    readiness.profile_contract_path = profile.contract_path.clone();
    readiness.profile_reason = profile.reason.clone();
    readiness.profile_error = profile.error.clone();
    readiness.optional_gate_ids = profile.optional_gate_ids.clone();
    readiness.not_applicable_gate_ids = profile.not_applicable_gate_ids.clone();
    readiness.gate_labels = profile.gate_labels.clone();
    readiness.gate_reasons = profile.gate_reasons.clone();
}
