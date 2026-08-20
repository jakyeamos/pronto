fn parse_audit_run(directory: &Path) -> Option<AuditRun> {
    let summary_path = directory.join("summary.json");
    let summary = read_json(&summary_path)?;
    let as_of = json_string_at(&summary, &["as_of"]);
    if as_of
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_none()
    {
        return None;
    }
    let findings = fs::read_dir(directory.join("findings"))
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .filter_map(|entry| {
            let path = entry.path();
            let payload = read_json(&path)?;
            Some(AuditFinding {
                path,
                canonical_path: json_string_at(&payload, &["canonical_path"])
                    .or_else(|| json_string_at(&payload, &["path"])),
                remote_key: [
                    "remote_key",
                    "remote_url",
                    "remote_identity",
                    "identity_key",
                ]
                .iter()
                .find_map(|key| json_string_at(&payload, &[*key]))
                .and_then(|value| remote_identity(&value)),
                mean_maturity: json_number_at(&payload, &["mean_maturity"]),
                mean_maturity_display: json_display_at(&payload, &["mean_maturity"]),
                scored_dimension_count: json_u64_at(&payload, &["scored_dimension_count"]).or_else(
                    || {
                        payload
                            .get("dimension_results")
                            .and_then(Value::as_array)
                            .map(|values| values.len() as u64)
                    },
                ),
                dimension_scores: payload
                    .get("dimension_scores")
                    .and_then(Value::as_object)
                    .map(|scores| {
                        scores
                            .iter()
                            .filter_map(|(dimension, score)| {
                                score.as_f64().map(|value| (dimension.clone(), value))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    Some(AuditRun {
        audit_id: json_string_at(&summary, &["audit_id"]),
        as_of,
        summary_path,
        mean_maturity: json_number_at(&summary, &["mean_maturity"]),
        mean_maturity_display: json_display_at(&summary, &["mean_maturity"]),
        scored_dimension_count: json_u64_at(&summary, &["scored_dimension_count"]),
        findings,
    })
}

fn canonical_path_matches(candidate: Option<&str>, repository_path: &str) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    canonical_path(candidate) == canonical_path(repository_path)
}

fn canonical_path(path: &str) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn remote_identity(value: &str) -> Option<String> {
    let mut normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if let Some(stripped) = normalized.strip_prefix("git@") {
        normalized = stripped.replacen(':', "/", 1);
    } else {
        for prefix in ["https://", "http://"] {
            if let Some(stripped) = normalized.strip_prefix(prefix) {
                normalized = stripped.to_string();
                break;
            }
        }
    }
    if let Some(stripped) = normalized.strip_prefix("github.com/") {
        normalized = stripped.to_string();
    }
    Some(
        normalized
            .trim_end_matches(".git")
            .trim_end_matches('/')
            .to_string(),
    )
}

fn parse_qr_status(value: Option<&str>) -> QualityGateStatus {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "passed" | "pass" | "success" => QualityGateStatus::Passed,
        "failed" | "fail" | "failure" => QualityGateStatus::Failed,
        "not configured" | "not_configured" | "unconfigured" => QualityGateStatus::NotConfigured,
        _ => QualityGateStatus::Blocked,
    }
}

fn severity_counts(payload: &Value) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    merge_severity_counts(&mut counts, payload.get("severity_counts"));
    merge_severity_counts(
        &mut counts,
        payload
            .get("finding_counts")
            .and_then(|value| value.get("severity_counts")),
    );
    if counts.is_empty() {
        merge_severity_counts(
            &mut counts,
            payload
                .get("summary")
                .and_then(|summary| summary.get("severity_counts")),
        );
        merge_severity_counts(
            &mut counts,
            payload
                .get("summary")
                .and_then(|summary| summary.get("finding_counts"))
                .and_then(|value| value.get("severity_counts")),
        );
    }
    if counts.is_empty() {
        for finding in payload
            .get("findings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let severity = finding
                .get("severity")
                .or_else(|| finding.get("priority"))
                .and_then(Value::as_str)
                .map(normalize_severity);
            if let Some(severity) = severity {
                *counts.entry(severity).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn fleet_dimension_scores(findings: &[Value]) -> (BTreeMap<String, f64>, Option<f64>) {
    let mut scores = BTreeMap::new();
    for finding in findings {
        if finding
            .get("applicable")
            .and_then(Value::as_bool)
            .is_some_and(|applicable| !applicable)
        {
            continue;
        }
        let Some(dimension) = finding.get("dimension").and_then(Value::as_str) else {
            continue;
        };
        let Some(score) = finding.get("score").and_then(Value::as_f64) else {
            continue;
        };
        scores.insert(dimension.to_string(), score);
    }
    let mean = (!scores.is_empty()).then(|| {
        let total = scores.values().sum::<f64>();
        total / scores.len() as f64
    });
    (scores, mean)
}

fn fleet_score(scores: &BTreeMap<String, f64>) -> Option<f64> {
    (!scores.is_empty()).then(|| scores.values().sum::<f64>() / scores.len() as f64)
}

fn merge_agent_usability_dimensions(
    scores: &mut BTreeMap<String, f64>,
    gaps: &mut Vec<QualityMaturityGap>,
    assessment: &AgentUsabilityMaturity,
) {
    if assessment.applicability == "not_applicable" || assessment.status == "not_applicable" {
        return;
    }
    for lane in assessment.lanes.iter().filter(|lane| lane.applicable) {
        let Some(score) = lane
            .score
            .filter(|score| score.is_finite() && (0.0..=4.0).contains(score))
        else {
            continue;
        };
        merge_agent_usability_dimension(
            scores,
            gaps,
            format!("agent_usability.{}", lane.id),
            score,
            &lane.status,
            &lane.message,
        );
    }
    let growth = &assessment.growth_health;
    let growth_score = growth.score.or(match growth.status.as_str() {
        "blocked" => Some(0.0),
        "attention" => Some(2.0),
        "healthy" => Some(4.0),
        _ => None,
    });
    if let Some(score) = growth_score.filter(|score| score.is_finite()) {
        merge_agent_usability_dimension(
            scores,
            gaps,
            "agent_usability.growth_health".to_string(),
            score,
            &growth.status,
            &growth.message,
        );
    }
}

fn merge_agent_usability_dimension(
    scores: &mut BTreeMap<String, f64>,
    gaps: &mut Vec<QualityMaturityGap>,
    dimension: String,
    score: f64,
    status: &str,
    message: &str,
) {
    scores.insert(dimension.clone(), score);
    if score < 4.0 {
        gaps.push(QualityMaturityGap {
            dimension,
            status: status.to_string(),
            score: Some(score),
            message: message.chars().take(240).collect(),
        });
    }
}

fn fleet_maturity_gaps(findings: &[Value]) -> Vec<QualityMaturityGap> {
    findings
        .iter()
        .filter(|finding| {
            finding.get("status").and_then(Value::as_str) != Some("not_applicable")
                && finding
                    .get("score")
                    .and_then(Value::as_f64)
                    .map_or(true, |score| score < 4.0)
        })
        .filter_map(|finding| {
            Some(QualityMaturityGap {
                dimension: finding.get("dimension")?.as_str()?.to_string(),
                status: finding
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                score: finding.get("score").and_then(Value::as_f64),
                message: finding
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Evidence is incomplete.")
                    .chars()
                    .take(240)
                    .collect(),
            })
        })
        .collect()
}

fn fleet_severity_counts(findings: &[Value]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for finding in findings {
        let severity = finding
            .get("severity")
            .or_else(|| finding.get("priority"))
            .and_then(Value::as_str)
            .map(normalize_severity)
            .unwrap_or_else(|| "unknown".to_string());
        *counts.entry(severity).or_insert(0) += 1;
    }
    counts
}

fn merge_severity_counts(counts: &mut BTreeMap<String, u64>, value: Option<&Value>) {
    let Some(object) = value.and_then(Value::as_object) else {
        return;
    };
    for (severity, count) in object {
        if let Some(count) = count.as_u64() {
            counts.insert(normalize_severity(severity), count);
        }
    }
}

fn normalize_severity(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "blocker" | "critical" | "crit" => "critical".to_string(),
        "high" | "error" => "high".to_string(),
        "medium" | "warning" | "warn" => "medium".to_string(),
        "low" | "info" | "informational" => "low".to_string(),
        other => other.to_string(),
    }
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    for character in value.trim().to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_string()
}

fn read_json(path: &Path) -> Option<Value> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn artifact_path(run_dir: &Path, name: &str) -> Option<String> {
    let path = run_dir.join(name);
    path.is_file().then(|| path.to_string_lossy().to_string())
}

fn json_string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_str().map(str::to_string)
}

fn json_number_at(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_f64()
}

fn json_display_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    match current {
        Value::Number(number) => Some(number.to_string()),
        Value::String(string) => Some(string.clone()),
        _ => None,
    }
}

fn json_u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_u64()
}
