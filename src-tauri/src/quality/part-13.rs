fn valid_measurement_confidence(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let Some(confidence) = value.as_object() else {
        return false;
    };
    let level = confidence.get("level").and_then(Value::as_str);
    if !matches!(level, Some("low" | "medium" | "high"))
        || confidence.get("deterministic_replay") != Some(&Value::Bool(true))
        || !json_string_array(confidence.get("basis"))
        || !json_string_array(confidence.get("limitations"))
    {
        return false;
    }
    let Some(population) = confidence
        .get("population_coverage")
        .and_then(Value::as_object)
    else {
        return false;
    };
    let expected = population
        .get("expected_repository_count")
        .and_then(Value::as_u64);
    let observed = population
        .get("observed_repository_count")
        .and_then(Value::as_u64);
    let gaps = confidence
        .get("unresolved_measurement_gap_count")
        .and_then(Value::as_u64);
    if expected.is_none()
        || observed.is_none()
        || gaps.is_none()
        || population
            .get("excluded_repository_count")
            .and_then(Value::as_u64)
            .is_none()
    {
        return false;
    }
    level != Some("high")
        || (population.get("status").and_then(Value::as_str) == Some("complete")
            && expected == observed
            && gaps == Some(0)
            && confidence
                .get("limitations")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty))
}

fn feed_measurement_confidence(feed: &Value) -> Option<QualityMeasurementConfidence> {
    let confidence = feed.get("measurement_confidence")?.as_object()?;
    let population = confidence.get("population_coverage")?.as_object()?;
    Some(QualityMeasurementConfidence {
        level: confidence.get("level")?.as_str()?.to_string(),
        basis: json_string_values(confidence.get("basis")),
        limitations: json_string_values(confidence.get("limitations")),
        population_status: population.get("status")?.as_str()?.to_string(),
        expected_repository_count: population.get("expected_repository_count")?.as_u64()?,
        observed_repository_count: population.get("observed_repository_count")?.as_u64()?,
        excluded_repository_count: population.get("excluded_repository_count")?.as_u64()?,
        unresolved_measurement_gap_count: confidence
            .get("unresolved_measurement_gap_count")?
            .as_u64()?,
        deterministic_replay: confidence.get("deterministic_replay")?.as_bool()?,
    })
}

fn json_string_array(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().all(Value::is_string))
}

fn json_string_values(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn valid_repository_maturity_projection(repository: &Value) -> bool {
    let Some(model) = repository
        .get("repository_maturity")
        .and_then(Value::as_object)
    else {
        return false;
    };
    if model.get("schema").and_then(Value::as_str) != Some("quality-runner-repository-maturity/v2")
        || model.get("score").and_then(Value::as_f64)
            != repository.get("maturity_score").and_then(Value::as_f64)
    {
        return false;
    }
    let Some(pillars) = model.get("pillars").and_then(Value::as_array) else {
        return false;
    };
    let expected = [
        "correctness_reliability",
        "security_privacy_supply_chain",
        "maintainability_evolvability",
        "operability_release_safety",
        "user_facing_quality",
        "human_agent_usability",
        "governance_sustainability",
    ];
    pillars.len() == expected.len()
        && pillars.iter().zip(expected).all(|(pillar, expected_id)| {
            pillar.get("id").and_then(Value::as_str) == Some(expected_id)
                && pillar
                    .get("weight")
                    .and_then(Value::as_f64)
                    .is_some_and(|weight| weight.is_finite() && weight > 0.0)
                && pillar.get("score").is_some_and(|score| {
                    score.is_null()
                        || score
                            .as_f64()
                            .is_some_and(|value| value.is_finite() && (0.0..=4.0).contains(&value))
                })
        })
        && (pillars
            .iter()
            .filter_map(|pillar| pillar.get("weight").and_then(Value::as_f64))
            .sum::<f64>()
            - 1.0)
            .abs()
            < 0.000_001
}

fn maturity_feed_hash(feed: &Value) -> Option<String> {
    let mut content = feed.clone();
    content.as_object_mut()?.remove("provenance_hash");
    let payload = serde_json::to_string(&content).ok()?;
    let digest = Sha256::digest(payload.as_bytes());
    Some(format!("{digest:x}"))
}

fn has_non_empty_string(value: &serde_json::Map<String, Value>, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|item| !item.trim().is_empty())
}

fn feed_tree_is_safe(value: &Value, key: Option<&str>, parent_key: Option<&str>) -> bool {
    if let Some(key) = key {
        let normalized = key.to_ascii_lowercase();
        if MATURITY_FEED_FORBIDDEN_KEYS.contains(&normalized.as_str())
            || normalized == "raw_credentials"
            || normalized == "raw_output"
            || normalized == "command_output"
            || normalized == "stdout"
            || normalized == "stderr"
        {
            let privacy_flag = parent_key == Some("privacy")
                && [
                    "raw_paths",
                    "raw_prompts",
                    "raw_code",
                    "raw_diffs",
                    "raw_transcripts",
                    "credentials",
                ]
                .contains(&normalized.as_str());
            if !privacy_flag {
                return false;
            }
        }
    }
    match value {
        Value::Object(object) => object
            .iter()
            .all(|(child_key, child_value)| feed_tree_is_safe(child_value, Some(child_key), key)),
        Value::Array(array) => array
            .iter()
            .all(|child| feed_tree_is_safe(child, None, key)),
        _ => true,
    }
}

fn feed_scored_dimension_count(feed: &Value) -> u64 {
    feed.get("repositories")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|repository| repository.get("dimension_scores"))
        .filter_map(Value::as_object)
        .map(|scores| scores.values().filter(|value| value.is_number()).count() as u64)
        .sum()
}

fn repository_feed_id(repository: &RepositorySnapshot) -> String {
    let identity = repository_identity_key(repository);
    let payload = serde_json::to_string(&[identity]).unwrap_or_else(|_| "[]".to_string());
    let digest = Sha256::digest(payload.as_bytes());
    let hex = format!("{digest:x}");
    format!("repo-{}", &hex[..16])
}

fn repository_identity_key(repository: &RepositorySnapshot) -> String {
    if let Some(origin) = repository.remote_url.as_deref().and_then(normalized_origin) {
        return format!("origin:{origin}");
    }
    if let Some(common) = common_git_dir(Path::new(&repository.path)) {
        return format!("common:{common}");
    }
    format!("path:{}", identity_path(&repository.path))
}

fn normalized_origin(value: &str) -> Option<String> {
    let mut value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    if let Some(stripped) = value.strip_prefix("git@") {
        let (host, path) = stripped.split_once(':')?;
        return Some(format!("{host}/{}", strip_git_suffix(path)));
    }
    for scheme in ["https://", "http://", "ssh://", "git://"] {
        if let Some(stripped) = value.strip_prefix(scheme) {
            value = stripped.to_string();
            break;
        }
    }
    if let Some(at) = value.find('@') {
        value = value[at + 1..].to_string();
    }
    let value = value.trim_start_matches('/');
    let (host, path) = value.split_once('/')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{host}/{}", strip_git_suffix(path)))
}

fn strip_git_suffix(value: &str) -> String {
    value
        .trim_matches('/')
        .strip_suffix(".git")
        .unwrap_or(value.trim_matches('/'))
        .trim_matches('/')
        .to_string()
}

fn common_git_dir(path: &Path) -> Option<String> {
    let mut child = Command::new("git")
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + QUALITY_GIT_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(StdDuration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    Some(
        fs::canonicalize(&path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string(),
    )
}

fn identity_path(path: &str) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .to_string()
}

pub fn safe_report_path(candidate: &Path, allowed_roots: &[PathBuf]) -> Result<PathBuf, String> {
    let canonical_candidate = fs::canonicalize(candidate)
        .map_err(|error| format!("Quality report is not available: {error}"))?;
    let allowed = allowed_roots
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .collect::<Vec<_>>();
    if allowed
        .iter()
        .any(|root| canonical_candidate.starts_with(root))
    {
        Ok(canonical_candidate)
    } else {
        Err("Quality reports can only be opened from configured QR or audit roots".to_string())
    }
}

fn add_evidence(gates: &mut Vec<QualityGate>, evidence: QualityEvidence) {
    let id = evidence.id.clone();
    let label = gate_label(&id);
    if let Some(gate) = gates.iter_mut().find(|gate| gate.id == id) {
        if gate.label == gate_label(&id) && label != gate.label {
            gate.label = label;
        }
        gate.evidence.push(evidence);
    } else {
        gates.push(QualityGate {
            id,
            label,
            status: QualityGateStatus::NotConfigured,
            freshness: QualityFreshness::Unknown,
            evidence: vec![evidence],
        });
    }
    gates.sort_by_key(|gate| gate_sort_key(&gate.id));
}

fn gate_sort_key(id: &str) -> (usize, String) {
    let canonical_index = CANONICAL_GATE_DEFINITIONS
        .iter()
        .position(|(candidate, _)| *candidate == id);
    if let Some(index) = canonical_index {
        return (index, id.to_string());
    }
    let conditional_index = CONDITIONAL_GATE_DEFINITIONS
        .iter()
        .position(|(candidate, _)| *candidate == id);
    (
        CANONICAL_GATE_DEFINITIONS.len()
            + conditional_index.unwrap_or(CONDITIONAL_GATE_DEFINITIONS.len()),
        id.to_string(),
    )
}

fn evidence_freshness(evidence: &[&QualityEvidence]) -> QualityFreshness {
    if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Conflicted)
    {
        QualityFreshness::Conflicted
    } else if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Fresh)
    {
        QualityFreshness::Fresh
    } else if evidence
        .iter()
        .any(|item| item.freshness == QualityFreshness::Stale)
    {
        QualityFreshness::Stale
    } else {
        QualityFreshness::Unknown
    }
}

fn ci_evidence(
    repository: &RepositorySnapshot,
    remote: Option<&RemoteRepositorySnapshot>,
) -> Vec<QualityEvidence> {
    let mut checks = Vec::<(&CheckSnapshot, Option<&str>, Option<&str>)>::new();
    if let Some(pull_request) = repository
        .pull_requests
        .iter()
        .filter(|pull_request| pull_request.head_branch == repository.branch)
        .find(|pull_request| !pull_request.checks.is_empty())
    {
        checks.extend(pull_request.checks.iter().map(|check| {
            (
                check,
                Some(pull_request.head_branch.as_str()),
                pull_request.head_commit.as_deref(),
            )
        }));
    } else if let Some(remote) = remote {
        checks.extend(remote.ci_checks.iter().map(|check| {
            (
                check,
                remote.ci_branch.as_deref(),
                check.head_sha.as_deref(),
            )
        }));
    }
    checks
        .into_iter()
        .map(|(check, branch, commit)| {
            let id = normalize_gate_id(&check.context);
            let status = github_check_status(check);
            let freshness = evaluate_freshness_at(
                Some(&check.last_refreshed_at),
                commit,
                branch,
                repository.workspace.last_commit.as_deref(),
                Some(repository.branch.as_str()),
                Utc::now(),
            );
            QualityEvidence {
                id,
                source: QualitySource::Ci,
                status,
                freshness,
                observed_at: Some(check.last_refreshed_at.clone()),
                scanned_commit: commit.map(str::to_string),
                scanned_branch: branch.map(str::to_string),
                command: None,
                source_label: format!("GitHub check · {}", check.context),
                report_path: None,
                report_url: check.html_url.clone(),
                report_kind: Some("GitHub check run".to_string()),
                detail: check
                    .conclusion
                    .clone()
                    .unwrap_or_else(|| check.state.clone()),
                verification_level: QualityVerificationLevel::SourceInferred,
                target_kind: Some("source".to_string()),
                target_url: None,
                target_provider: Some("github".to_string()),
                deployment_id: None,
            }
        })
        .collect()
}

fn github_check_status(check: &CheckSnapshot) -> QualityGateStatus {
    let state = check.state.to_ascii_lowercase();
    let conclusion = check
        .conclusion
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        conclusion.as_str(),
        "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure"
    ) {
        QualityGateStatus::Failed
    } else if matches!(conclusion.as_str(), "success" | "neutral") {
        QualityGateStatus::Passed
    } else if state == "completed" && conclusion == "skipped" {
        QualityGateStatus::Blocked
    } else {
        QualityGateStatus::Blocked
    }
}
