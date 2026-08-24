fn unique_commits(path: &Path, branch: &str, target: Option<&str>) -> u64 {
    let Some(target) = target else {
        return 0;
    };
    git_owned(
        path,
        vec![
            "rev-list".to_string(),
            "--count".to_string(),
            format!("{target}..{branch}"),
        ],
    )
    .and_then(|value| value.parse::<u64>().ok())
    .unwrap_or(0)
}

fn release_commit_details(subject: &str) -> (String, Option<String>) {
    let header = subject.split(':').next().unwrap_or(subject).trim();
    let breaking = header.contains('!') || subject.contains("BREAKING CHANGE");
    let commit_type = header
        .trim_end_matches('!')
        .split('(')
        .next()
        .unwrap_or(header)
        .to_ascii_lowercase();
    let (category, bump) = if breaking {
        ("Breaking", Some("major"))
    } else {
        match commit_type.as_str() {
            "feat" => ("Features", Some("minor")),
            "fix" => ("Fixes", Some("patch")),
            "perf" => ("Performance", Some("patch")),
            _ => ("Other", None),
        }
    };
    (category.to_string(), bump.map(str::to_string))
}

fn release_commits(
    path: &Path,
    base: &str,
    head: &str,
) -> Result<Vec<ReleaseCommitSummary>, String> {
    let range = format!("{base}..{head}");
    let output = run_git_bounded(
        path,
        vec![
            "log".to_string(),
            range,
            format!("--max-count={RELEASE_COMMIT_LIMIT}"),
            "--format=%H%x09%s%x09%cI".to_string(),
        ],
        StdDuration::from_secs(RELEASE_GIT_TIMEOUT_SECONDS),
    )?;
    if !output.success {
        let detail = output.stderr.trim();
        return Err(if detail.is_empty() {
            format!("Git could not inspect the committed release range {base}..{head}")
        } else {
            format!("Git could not inspect the committed release range {base}..{head}: {detail}")
        });
    }
    Ok(output
        .stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let sha = fields.next()?.trim();
            let subject = fields.next()?.trim();
            let committed_at = fields.next()?.trim();
            if sha.is_empty() || subject.is_empty() || committed_at.is_empty() {
                return None;
            }
            let (category, bump) = release_commit_details(subject);
            Some(ReleaseCommitSummary {
                sha: sha.to_string(),
                subject: subject.to_string(),
                category,
                bump,
                committed_at: committed_at.to_string(),
            })
        })
        .collect())
}

fn git_ref_exists(path: &Path, reference: &str) -> bool {
    git_owned(
        path,
        vec![
            "rev-parse".to_string(),
            "--verify".to_string(),
            reference.to_string(),
        ],
    )
    .is_some()
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn committed_diff(path: &Path, base: &str, head: &str) -> Option<(String, bool)> {
    let range = format!("{base}..{head}");
    let result = run_git(
        path,
        vec![
            "diff".to_string(),
            "--no-ext-diff".to_string(),
            range,
            "--".to_string(),
        ],
    )
    .ok()?;
    if !result.success {
        return None;
    }
    let truncated = result.stdout.len() > MAX_AI_DIFF_BYTES;
    Some((bounded_text(&result.stdout, MAX_AI_DIFF_BYTES), truncated))
}

fn empty_ai_preview(repository_id: &str, workspace_id: &str, permission: &str) -> AiPayloadPreview {
    AiPayloadPreview {
        repository_id: repository_id.to_string(),
        workspace_id: workspace_id.to_string(),
        permission: permission.to_string(),
        provider: "None — local preview only".to_string(),
        model: None,
        status: "Preview unavailable".to_string(),
        reasons: Vec::new(),
        categories: Vec::new(),
        source_references: Vec::new(),
        payload_text: String::new(),
        payload_bytes: 0,
        uncommitted_included: false,
        request_performed: false,
        generated_at: iso_now(),
    }
}

fn preview_ai_summary_at(
    path: &Path,
    repository_id: &str,
    workspace_id: Option<&str>,
) -> Result<AiPayloadPreview, String> {
    let state = load_store(path)?;
    let repository = state
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = match workspace_id.filter(|value| !value.trim().is_empty()) {
        Some(workspace_id) => repository
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| "Workspace is not registered for this repository".to_string())?,
        None => &repository.workspace,
    };
    let permission = normalize_ai_permission(&repository.ai_permission)
        .unwrap_or_else(|_| default_ai_permission());
    let mut preview = empty_ai_preview(&repository.id, &workspace.id, &permission);
    if permission == "Disabled" {
        preview.status = "AI disabled by repository policy".to_string();
        preview
            .reasons
            .push("No external request was made.".to_string());
        return Ok(preview);
    }
    if !Path::new(&workspace.path).is_dir() {
        preview.status = "Workspace path unavailable".to_string();
        preview
            .reasons
            .push("The registered workspace path is not accessible.".to_string());
        return Ok(preview);
    }
    if !workspace.status_available {
        preview.status = "Git status unavailable".to_string();
        preview
            .reasons
            .push(workspace_status_unavailable_reason(workspace));
        return Ok(preview);
    }
    let base = latest_published_release(repository)
        .and_then(|release| release.target_commit)
        .or_else(|| workspace.target_branch.clone());
    let Some(base) = base.filter(|value| !value.trim().is_empty()) else {
        preview.status = "Committed evidence range unavailable".to_string();
        preview
            .reasons
            .push("A published baseline or verified target branch is required.".to_string());
        return Ok(preview);
    };
    let head = workspace.branch.trim();
    if head.is_empty()
        || !git_ref_exists(Path::new(&workspace.path), &base)
        || !git_ref_exists(Path::new(&workspace.path), head)
    {
        preview.status = "Committed evidence range unavailable".to_string();
        preview
            .reasons
            .push("The selected committed range could not be verified locally.".to_string());
        return Ok(preview);
    }

    let commits = match release_commits(Path::new(&workspace.path), &base, head) {
        Ok(commits) => commits,
        Err(error) => {
            preview.status = "Committed evidence range unavailable".to_string();
            preview.reasons.push(error);
            return Ok(preview);
        }
    };
    let metadata_payload = serde_json::json!({
        "repository_id": repository.id.clone(),
        "workspace_id": workspace.id.clone(),
        "commits": commits,
    });
    let metadata_text = serde_json::to_string_pretty(&metadata_payload)
        .map_err(|error| format!("Could not encode AI metadata preview: {error}"))?;
    preview.source_references = commits
        .iter()
        .map(|commit| AiSourceReference {
            sha: commit.sha.clone(),
            subject: commit.subject.clone(),
            committed_at: commit.committed_at.clone(),
            category: commit.category.clone(),
        })
        .collect();
    preview.categories.push(AiPayloadCategory {
        category: "Committed metadata".to_string(),
        included: true,
        item_count: commits.len(),
        byte_count: metadata_text.len(),
    });

    let mut payload = metadata_payload;
    if permission == "Committed diff allowed" {
        let Some((diff_text, truncated)) = committed_diff(Path::new(&workspace.path), &base, head)
        else {
            preview.status = "Committed diff preview unavailable".to_string();
            preview
                .reasons
                .push("Git could not produce the committed-only diff.".to_string());
            return Ok(preview);
        };
        if truncated {
            preview.reasons.push(format!(
                "Committed diff preview is capped at {} bytes.",
                MAX_AI_DIFF_BYTES
            ));
        }
        payload["committed_diff"] = serde_json::Value::String(diff_text.clone());
        preview.categories.push(AiPayloadCategory {
            category: "Committed diff".to_string(),
            included: true,
            item_count: usize::from(!diff_text.is_empty()),
            byte_count: diff_text.len(),
        });
    }
    preview.payload_text = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Could not encode AI payload preview: {error}"))?;
    preview.payload_bytes = preview.payload_text.len();
    preview.status = if commits.is_empty() {
        "No committed changes in selected range".to_string()
    } else {
        "Payload ready for user inspection".to_string()
    };
    preview
        .reasons
        .push("Preview only; no external AI request was made.".to_string());
    if workspace.dirty {
        preview
            .reasons
            .push("Uncommitted changes are excluded from this payload.".to_string());
    }
    Ok(preview)
}

fn latest_published_release(repository: &RepositorySnapshot) -> Option<ReleaseSnapshot> {
    repository
        .releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease && release.published_at.is_some())
        .max_by(|left, right| left.published_at.cmp(&right.published_at))
        .cloned()
}

fn parse_release_version(tag: &str) -> Option<(u64, u64, u64)> {
    let trimmed = tag.trim().trim_start_matches('v');
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn normalize_release_version(value: &str) -> Result<String, String> {
    let (major, minor, patch) = parse_release_version(value)
        .ok_or_else(|| "Release version must use the form vMAJOR.MINOR.PATCH".to_string())?;
    Ok(format!("v{major}.{minor}.{patch}"))
}

fn highest_release_bump(commits: &[ReleaseCommitSummary]) -> Option<String> {
    commits
        .iter()
        .filter_map(|commit| commit.bump.as_deref())
        .max_by_key(|bump| match *bump {
            "major" => 3,
            "minor" => 2,
            "patch" => 1,
            _ => 0,
        })
        .map(str::to_string)
}

fn candidate_version(release: &ReleaseSnapshot, bump: Option<&str>) -> Option<String> {
    let (mut major, mut minor, mut patch) = parse_release_version(&release.tag)?;
    match bump {
        Some("major") => {
            major += 1;
            minor = 0;
            patch = 0;
        }
        Some("minor") => {
            minor += 1;
            patch = 0;
        }
        Some("patch") => patch += 1,
        _ => return None,
    }
    Some(format!("v{major}.{minor}.{patch}"))
}

fn release_recommendation(
    baseline: Option<&ReleaseSnapshot>,
    commits: &[ReleaseCommitSummary],
    candidate_bump: Option<&str>,
    candidate_version: Option<&str>,
    rule_result: Option<ReleaseRuleResult>,
    blocked: bool,
) -> ReleaseRecommendation {
    let basis = baseline
        .map(|release| {
            format!(
                "{} commits since last published tag {}",
                commits.len(),
                release.tag
            )
        })
        .unwrap_or_else(|| "No published SemVer tag baseline is available".to_string());
    let mut recommendation = ReleaseRecommendation {
        disposition: "do_not_release_yet".to_string(),
        label: "Do not release yet".to_string(),
        suggested_bump: candidate_bump.map(str::to_string),
        suggested_version: candidate_version.map(str::to_string),
        basis,
        reasons: Vec::new(),
        advisory: true,
    };

    if blocked {
        recommendation
            .reasons
            .push("Required release evidence or readiness gates are not ready.".to_string());
        return recommendation;
    }
    if baseline.is_none() {
        recommendation.disposition = "review_required".to_string();
        recommendation.label = "Review first-release version".to_string();
        recommendation.reasons.push(
            "A published SemVer baseline is required for an automatic version increment."
                .to_string(),
        );
        return recommendation;
    }
    if commits.is_empty() {
        recommendation
            .reasons
            .push("No commits were found after the last published tag.".to_string());
        return recommendation;
    }
    let (Some(bump), Some(version)) = (candidate_bump, candidate_version) else {
        recommendation.disposition = "review_required".to_string();
        recommendation.label = "Review release impact".to_string();
        recommendation.reasons.push(
            "Commits exist, but none imply a deterministic conventional-commit SemVer bump."
                .to_string(),
        );
        return recommendation;
    };
    if rule_result != Some(ReleaseRuleResult::Passed) {
        recommendation.disposition = "review_required".to_string();
        recommendation.label = format!("Review {version} ({bump})");
        recommendation.reasons.push(
            "The change-based version candidate is available, but no configured release threshold has passed."
                .to_string(),
        );
        return recommendation;
    }

    recommendation.disposition = format!("release_{bump}");
    recommendation.label = format!("Release {version} ({bump})");
    recommendation.reasons.push(format!(
        "The configured release threshold passed; {bump} is the highest bump implied by commits since the last published tag."
    ));
    recommendation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseRuleResult {
    Passed,
    Failed,
    Blocked,
    Unknown,
}

fn combine_release_rule_results(
    operator: &str,
    results: &[ReleaseRuleResult],
) -> ReleaseRuleResult {
    if results.contains(&ReleaseRuleResult::Blocked) {
        return ReleaseRuleResult::Blocked;
    }
    if operator == "OR" {
        if results.contains(&ReleaseRuleResult::Passed) {
            ReleaseRuleResult::Passed
        } else if results
            .iter()
            .all(|result| *result == ReleaseRuleResult::Failed)
        {
            ReleaseRuleResult::Failed
        } else {
            ReleaseRuleResult::Unknown
        }
    } else if results.contains(&ReleaseRuleResult::Failed) {
        ReleaseRuleResult::Failed
    } else if results.contains(&ReleaseRuleResult::Unknown) {
        ReleaseRuleResult::Unknown
    } else {
        ReleaseRuleResult::Passed
    }
}

fn release_rule_status(result: ReleaseRuleResult) -> &'static str {
    match result {
        ReleaseRuleResult::Passed => "Passed",
        ReleaseRuleResult::Failed => "Failed",
        ReleaseRuleResult::Blocked => "Blocked",
        ReleaseRuleResult::Unknown => "Unknown",
    }
}

fn release_rule_needs_baseline(rule: &ReleaseRuleConfig) -> bool {
    rule.min_commits.is_some()
        || rule.min_elapsed_days.is_some()
        || !rule.required_commit_types.is_empty()
}
