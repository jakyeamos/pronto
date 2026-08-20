pub fn maturity_feed_import(
    feed_path: Option<&Path>,
    repositories: &[RepositorySnapshot],
) -> AuditImport {
    let Some(feed_path) = feed_path else {
        return AuditImport::default();
    };
    let mut portfolio = QualityPortfolioSnapshot {
        audit_root: Some(feed_path.to_string_lossy().to_string()),
        ..QualityPortfolioSnapshot::default()
    };
    if !feed_path.is_file() || feed_path.is_symlink() {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    }
    let Some(feed) = read_json(feed_path) else {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    };
    if !validate_maturity_feed(&feed) {
        portfolio.audit_status = "Unavailable".to_string();
        return AuditImport {
            portfolio,
            maturities: HashMap::new(),
            behavior_assurance: HashMap::new(),
        };
    }

    let source = feed.get("source").and_then(Value::as_object);
    let audit_id = source
        .and_then(|value| value.get("audit_id"))
        .and_then(Value::as_str);
    let as_of = source
        .and_then(|value| value.get("as_of"))
        .and_then(Value::as_str);
    let freshness = evaluate_audit_freshness_at(as_of, Utc::now());
    let feed_status = feed
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    portfolio.feed_schema = feed
        .get("schema")
        .and_then(Value::as_str)
        .map(str::to_string);
    portfolio.provenance_hash = feed
        .get("provenance_hash")
        .and_then(Value::as_str)
        .map(str::to_string);
    portfolio.quality_outcome_counts = feed
        .get("quality_outcome_counts")
        .filter(|value| value.is_object())
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    portfolio.quality_outcome_taxonomy = feed
        .get("quality_outcome_taxonomy")
        .filter(|value| value.is_object())
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    portfolio.behavior_assurance = feed
        .get("behavior_assurance")
        .filter(|value| value.is_object())
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    portfolio.latest_audit_id = audit_id.map(str::to_string);
    portfolio.latest_audit_at = as_of.map(str::to_string);
    portfolio.latest_audit_path = Some(feed_path.to_string_lossy().to_string());
    portfolio.maturity_score = feed.get("mean_maturity").and_then(Value::as_f64);
    portfolio.maturity_score_display = portfolio.maturity_score.map(|score| format!("{score:.3}"));
    portfolio.scored_dimension_count = Some(feed_scored_dimension_count(&feed));
    portfolio.measurement_confidence = feed_measurement_confidence(&feed);
    portfolio.audit_status = match freshness {
        QualityFreshness::Fresh if feed_status == "complete_with_blockers" => {
            "Ready with blockers".to_string()
        }
        QualityFreshness::Fresh => "Ready".to_string(),
        QualityFreshness::Stale => "Stale".to_string(),
        QualityFreshness::Unknown | QualityFreshness::Conflicted => "Unknown".to_string(),
    };

    let projections = feed
        .get("repositories")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_object())
        .collect::<Vec<_>>();
    let mut matches = HashMap::new();
    let mut behavior_assurance_matches = HashMap::new();
    for repository in repositories {
        let stable_id = repository_feed_id(repository);
        let projection = projections
            .iter()
            .find(|projection| {
                projection.get("repo_id").and_then(Value::as_str) == Some(stable_id.as_str())
            })
            .or_else(|| {
                projections.iter().find(|projection| {
                    projection
                        .get("local_identity")
                        .and_then(|value| value.get("primary_path"))
                        .and_then(Value::as_str)
                        .is_some_and(|path| canonical_path_matches(Some(path), &repository.path))
                })
            });
        let Some(projection) = projection else {
            continue;
        };
        if let Some(assurance) = projection
            .get("behavior_assurance")
            .filter(|value| value.is_object())
            .and_then(|value| {
                let mut assurance: BehaviorAssuranceRepositoryState =
                    serde_json::from_value(value.clone()).ok()?;
                assurance.normalize_state();
                Some(assurance)
            })
        {
            behavior_assurance_matches.insert(repository.id.clone(), assurance);
        }
        let score = projection.get("maturity_score").and_then(Value::as_f64);
        let projection_freshness = if score.is_some() {
            freshness.clone()
        } else {
            QualityFreshness::Unknown
        };
        let maturity = QualityMaturity {
            score,
            score_display: score.map(|value| format!("{value:.3}")),
            scored_dimension_count: projection
                .get("dimension_scores")
                .and_then(Value::as_object)
                .map(|scores| scores.values().filter(|value| value.is_number()).count() as u64),
            dimension_scores: projection
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
            gaps: projection
                .get("dimension_gaps")
                .and_then(Value::as_array)
                .map(|gaps| {
                    gaps.iter()
                        .filter_map(|gap| {
                            let gap = gap.as_object()?;
                            Some(QualityMaturityGap {
                                dimension: gap.get("dimension")?.as_str()?.to_string(),
                                status: gap
                                    .get("status")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unknown")
                                    .to_string(),
                                score: gap.get("score").and_then(Value::as_f64),
                                message: gap
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Evidence is incomplete.")
                                    .chars()
                                    .take(240)
                                    .collect(),
                            })
                        })
                        .take(MAX_MATURITY_GAPS)
                        .collect()
                })
                .unwrap_or_default(),
            quality_outcome: projection
                .get("quality_outcome")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            agent_usability: projection
                .get("agent_usability")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            repository_maturity: projection
                .get("repository_maturity")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            cache_design: projection
                .get("cache_design")
                .filter(|value| value.is_object())
                .and_then(|value| serde_json::from_value(value.clone()).ok()),
            ci_gate_audit: parse_ci_gate_audit(projection),
            audit_id: audit_id.map(str::to_string),
            observed_at: as_of.map(str::to_string),
            scanned_commit: projection
                .get("target_head")
                .and_then(Value::as_str)
                .map(str::to_string),
            scanned_branch: projection
                .get("target_branch")
                .and_then(Value::as_str)
                .map(str::to_string),
            freshness: projection_freshness,
            report_path: Some(feed_path.to_string_lossy().to_string()),
        };
        matches.insert(repository.id.clone(), maturity);
    }
    portfolio.matched_repository_count = matches.len();
    portfolio.behavior_assurance.state_counts.clear();
    for assurance in behavior_assurance_matches.values() {
        *portfolio
            .behavior_assurance
            .state_counts
            .entry(assurance.state.clone())
            .or_insert(0) += 1;
    }
    AuditImport {
        portfolio,
        maturities: matches,
        behavior_assurance: behavior_assurance_matches,
    }
}

fn parse_ci_gate_audit(
    projection: &serde_json::Map<String, Value>,
) -> Option<CiGateCandidateAudit> {
    let raw = projection.get("ci_gate_audit")?.as_object()?;
    if raw.is_empty() {
        return None;
    }
    let mut audit = serde_json::from_value::<CiGateCandidateAudit>(Value::Object(raw.clone()))
        .unwrap_or_default();
    let target_branch = projection.get("target_branch").and_then(Value::as_str);
    let target_head = projection.get("target_head").and_then(Value::as_str);
    if let Err(error) = validate_ci_gate_audit(raw, &audit, target_branch, target_head) {
        audit.status = "invalid".to_string();
        audit.error = Some(error);
        audit.candidate_count = 0;
        audit.candidates.clear();
    }
    Some(audit)
}

fn validate_ci_gate_audit(
    raw: &serde_json::Map<String, Value>,
    audit: &CiGateCandidateAudit,
    target_branch: Option<&str>,
    target_head: Option<&str>,
) -> Result<(), String> {
    if audit.schema != CI_GATE_AUDIT_SCHEMA {
        return Err("Unsupported Quality Runner custom-gate audit schema.".to_string());
    }
    if !matches!(audit.status.as_str(), "complete" | "partial") {
        return Err("Quality Runner custom-gate audit status is invalid.".to_string());
    }
    if audit.policy.authority != "recommendation_only" || audit.policy.implementation_allowed {
        return Err(
            "Custom-gate audit attempted to exceed recommendation-only authority.".to_string(),
        );
    }
    if audit.repository.branch.as_deref() != target_branch
        || audit.repository.head_sha.as_deref() != target_head
    {
        return Err(
            "Custom-gate audit does not match the repository target branch and commit.".to_string(),
        );
    }
    if audit.candidate_count != audit.candidates.len() || audit.candidates.len() > 16 {
        return Err("Custom-gate audit candidate count is invalid.".to_string());
    }
    let Some(expected_hash) = raw.get("provenance_hash").and_then(Value::as_str) else {
        return Err("Custom-gate audit provenance hash is missing.".to_string());
    };
    let mut hashable = raw.clone();
    hashable.remove("provenance_hash");
    let payload = serde_json::to_string(&Value::Object(hashable))
        .map_err(|_| "Custom-gate audit provenance cannot be serialized.".to_string())?;
    let actual_hash = format!("{:x}", Sha256::digest(payload.as_bytes()));
    if expected_hash.len() != 64 || actual_hash != expected_hash {
        return Err("Custom-gate audit provenance hash does not match its content.".to_string());
    }
    let mut ids = HashSet::new();
    for candidate in &audit.candidates {
        if !valid_custom_gate_id(&candidate.id) || !ids.insert(candidate.id.as_str()) {
            return Err("Custom-gate audit contains an invalid or duplicate gate ID.".to_string());
        }
        if !matches!(
            candidate.recommendation.as_str(),
            "required_candidate" | "optional_candidate" | "review_required"
        ) || !matches!(candidate.confidence.as_str(), "high" | "medium")
            || !matches!(
                candidate.admission.state.as_str(),
                "proposal_only" | "implementation_detected"
            )
            || candidate.admission.blockers.is_empty()
            || candidate.invariant.is_empty()
            || candidate.failure_mode.is_empty()
            || candidate.evidence.is_empty()
        {
            return Err("Custom-gate audit candidate semantics are incomplete.".to_string());
        }
        if candidate
            .evidence
            .iter()
            .chain(candidate.negative_controls.iter())
            .any(|evidence| !safe_ci_gate_evidence_path(&evidence.path))
            || candidate
                .existing_check
                .contexts
                .iter()
                .any(|context| !safe_ci_gate_evidence_path(&context.path))
        {
            return Err("Custom-gate audit contains an unsafe evidence path.".to_string());
        }
    }
    Ok(())
}

fn valid_custom_gate_id(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("custom:") else {
        return false;
    };
    let mut characters = suffix.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn safe_ci_gate_evidence_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_maturity_feed(feed: &Value) -> bool {
    let Some(feed) = feed.as_object() else {
        return false;
    };
    let feed_schema = feed.get("schema").and_then(Value::as_str);
    if !feed_schema.is_some_and(|schema| MATURITY_FEED_SCHEMAS.contains(&schema))
        || !feed
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| MATURITY_FEED_STATUS.contains(&status))
        || feed.get("feed_timestamp").and_then(Value::as_str).is_none()
    {
        return false;
    }
    let Some(source) = feed.get("source").and_then(Value::as_object) else {
        return false;
    };
    if !has_non_empty_string(source, "audit_id")
        || !has_non_empty_string(source, "as_of")
        || !has_non_empty_string(source, "projects_root")
    {
        return false;
    }
    let Some(replay) = feed.get("replay").and_then(Value::as_object) else {
        return false;
    };
    if replay.get("status").and_then(Value::as_str) != Some("passed")
        || replay.get("deterministic") != Some(&Value::Bool(true))
        || replay.get("source_summary_hash") != source.get("summary_hash")
        || replay.get("replayed_summary_hash") != source.get("summary_hash")
    {
        return false;
    }
    if !valid_measurement_confidence(feed.get("measurement_confidence")) {
        return false;
    }
    let Some(repositories) = feed.get("repositories").and_then(Value::as_array) else {
        return false;
    };
    if repositories.is_empty()
        || feed.get("repository_count").and_then(Value::as_u64) != Some(repositories.len() as u64)
    {
        return false;
    }
    let mut repository_ids = HashSet::new();
    for repository in repositories {
        let Some(repo_id) = repository.get("repo_id").and_then(Value::as_str) else {
            return false;
        };
        if repo_id.is_empty() || !repository_ids.insert(repo_id) {
            return false;
        }
        if feed_schema == Some("quality-runner-maturity-feed/v2")
            && !valid_repository_maturity_projection(repository)
        {
            return false;
        }
    }
    let Some(provenance_hash) = feed.get("provenance_hash").and_then(Value::as_str) else {
        return false;
    };
    if provenance_hash.len() != 64
        || !provenance_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        || maturity_feed_hash(&Value::Object(feed.clone())).as_deref() != Some(provenance_hash)
    {
        return false;
    }
    let Some(privacy) = feed.get("privacy").and_then(Value::as_object) else {
        return false;
    };
    if privacy.get("private_local_feed") != Some(&Value::Bool(true))
        || [
            "raw_paths",
            "raw_prompts",
            "raw_code",
            "raw_diffs",
            "raw_transcripts",
            "credentials",
        ]
        .iter()
        .any(|key| privacy.get(*key) != Some(&Value::Bool(false)))
    {
        return false;
    }
    feed_tree_is_safe(&Value::Object(feed.clone()), None, None)
}
