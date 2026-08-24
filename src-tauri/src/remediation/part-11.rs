fn fleet_qr_run(repository_path: &Path, fleet_audit_root: Option<&Path>) -> Option<QrRunEvidence> {
    let root = fleet_audit_root?;
    let findings_dir = root.join("findings");
    let entries = fs::read_dir(&findings_dir).ok()?;
    let summary = read_json(&root.join("summary.json"));
    let summary_id = summary
        .as_ref()
        .and_then(|value| first_string(value, &[&["audit_id"]]));
    let summary_observed_at = summary
        .as_ref()
        .and_then(|value| first_string(value, &[&["as_of"]]));
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .filter_map(|entry| {
            let path = entry.path();
            let payload = read_json(&path)?;
            let repository_payload = payload.get("repository")?;
            let candidate_path = first_string(repository_payload, &[&["primary_path"], &["path"]])
                .or_else(|| {
                    repository_payload
                        .get("checkouts")
                        .and_then(Value::as_array)
                        .and_then(|checkouts| checkouts.first())
                        .and_then(|checkout| first_string(checkout, &[&["path"]]))
                });
            if !path_matches(candidate_path.as_deref(), repository_path) {
                return None;
            }
            let findings = parse_fleet_findings(&path, &payload);
            let observed_at = first_string(&payload, &[&["as_of"]]).or(summary_observed_at.clone());
            let target_branch = repository_payload
                .get("target_branch")
                .and_then(|value| first_string(value, &[&["branch"]]));
            let checkout = repository_payload
                .get("checkouts")
                .and_then(Value::as_array)
                .and_then(|checkouts| {
                    target_branch
                        .as_deref()
                        .and_then(|branch| {
                            checkouts.iter().find(|checkout| {
                                first_string(checkout, &[&["branch"]]).as_deref() == Some(branch)
                            })
                        })
                        .or_else(|| checkouts.first())
                });
            let scanned_branch = checkout.and_then(|value| first_string(value, &[&["branch"]]));
            let scanned_commit = checkout
                .and_then(|value| first_string(value, &[&["head"], &["fingerprint", "head"]]));
            let id = first_string(&payload, &[&["audit_id"], &["id"]])
                .or(summary_id.clone())
                .unwrap_or_else(|| path.display().to_string());
            Some(QrRunEvidence {
                id,
                run_dir: root.to_path_buf(),
                observed_at,
                scanned_branch,
                scanned_commit,
                findings,
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

fn path_matches(candidate: Option<&str>, repository_path: &Path) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    canonical_path(Path::new(candidate)) == canonical_path(repository_path)
}

fn canonical_path(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn parse_fleet_findings(path: &Path, payload: &Value) -> Vec<ParsedFinding> {
    let Some(items) = payload.get("findings").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| fleet_finding_requires_remediation(item))
        .map(|(index, item)| {
            let category =
                first_string(item, &[&["dimension"], &["category"], &["kind"], &["type"]])
                    .unwrap_or_else(|| "quality".to_string());
            let bucket = first_string(item, &[&["dimension"], &["status"], &["bucket"]])
                .unwrap_or_else(|| category.clone());
            let id = first_string(
                item,
                &[&["finding_id"], &["id"], &["fingerprint"], &["rule_id"]],
            )
            .unwrap_or_else(|| format!("fleet:{index}"));
            let pack = first_string(item, &[&["schema"], &["pack"], &["pack_id"]]);
            let severity = first_string(item, &[&["severity"], &["priority"], &["risk"]])
                .unwrap_or_else(|| "warning".to_string());
            let title = first_string(item, &[&["label"], &["title"], &["rule_id"]])
                .unwrap_or_else(|| format!("Resolve {category} finding"));
            let summary = first_string(
                item,
                &[
                    &["message"],
                    &["summary"],
                    &["description"],
                    &["recommended_fix"],
                ],
            )
            .unwrap_or_else(|| "Review the evidence and apply the recommended fix.".to_string());
            let file = first_string(item, &[&["file"], &["path"], &["file_path"]])
                .or_else(|| first_evidence_path(item));
            let verification = first_string(
                item,
                &[&["verification"], &["verification_command"], &["verify"]],
            )
            .or_else(|| first_array_string(item, "validation_commands"));
            ParsedFinding {
                id,
                fingerprint: first_string(item, &[&["fingerprint"]]),
                group_key: format!(
                    "{}|{}|{}",
                    category.to_ascii_lowercase(),
                    bucket.to_ascii_lowercase(),
                    pack.as_deref().unwrap_or("unknown").to_ascii_lowercase()
                ),
                category,
                pack,
                severity,
                title,
                summary,
                file,
                line: first_u64(item, &[&["line"], &["line_number"]]),
                verification,
                report_path: path.to_string_lossy().to_string(),
            }
        })
        .collect()
}

fn fleet_finding_requires_remediation(item: &Value) -> bool {
    if item.get("applicable").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    let is_maturity_record = quality::is_fleet_maturity_finding(item);
    if !is_maturity_record {
        return true;
    }
    if let Some(score) = item.get("score").and_then(Value::as_f64) {
        return score < MATURITY_CLOSURE_TARGET;
    }
    !first_string(item, &[&["status"], &["bucket"]]).is_some_and(|status| {
        matches!(
            status.to_ascii_lowercase().as_str(),
            "validated" | "maintained" | "not_applicable"
        )
    })
}

fn first_evidence_path(value: &Value) -> Option<String> {
    value
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|item| first_string(item, &[&["path"], &["file"], &["file_path"]]))
}

fn first_array_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(Value::as_str))
        .map(str::to_string)
}

fn parse_findings(run_dir: &Path) -> Vec<ParsedFinding> {
    let mut findings = Vec::new();
    for file_name in ["code-quality-scan.json", "quality-audit.json"] {
        let path = run_dir.join(file_name);
        let Some(payload) = read_json(&path) else {
            continue;
        };
        let Some(items) = payload.get("findings").and_then(Value::as_array) else {
            continue;
        };
        for (index, item) in items.iter().enumerate() {
            let id = first_string(
                item,
                &[&["id"], &["fingerprint"], &["finding_id"], &["rule_id"]],
            )
            .unwrap_or_else(|| format!("{file_name}:{index}"));
            let category = first_string(item, &[&["category"], &["kind"], &["type"]])
                .unwrap_or_else(|| "quality".to_string());
            let bucket = first_string(item, &[&["remediation_bucket"], &["bucket"], &["rule_id"]])
                .unwrap_or_else(|| category.clone());
            let pack = first_string(item, &[&["pack"], &["pack_id"], &["pack_name"]]);
            let severity = first_string(item, &[&["severity"], &["priority"], &["risk"]])
                .unwrap_or_else(|| "warning".to_string());
            let title = first_string(item, &[&["title"], &["summary"], &["rule"], &["rule_id"]])
                .unwrap_or_else(|| format!("Resolve {category} finding"));
            let summary = first_string(
                item,
                &[
                    &["summary"],
                    &["message"],
                    &["description"],
                    &["recommended_fix"],
                ],
            )
            .unwrap_or_else(|| "Review the evidence and apply the recommended fix.".to_string());
            let file = first_string(item, &[&["file"], &["path"], &["file_path"]]);
            let line = first_u64(item, &[&["line"], &["line_number"]]);
            let verification = first_string(
                item,
                &[&["verification"], &["verification_command"], &["verify"]],
            );
            findings.push(ParsedFinding {
                id,
                fingerprint: first_string(item, &[&["fingerprint"]]),
                group_key: format!(
                    "{}|{}|{}",
                    category.to_ascii_lowercase(),
                    bucket.to_ascii_lowercase(),
                    pack.as_deref().unwrap_or("unknown").to_ascii_lowercase()
                ),
                category,
                pack,
                severity,
                title,
                summary,
                file,
                line,
                verification,
                report_path: path.to_string_lossy().to_string(),
            });
        }
        if !findings.is_empty() {
            break;
        }
    }
    findings
}

fn read_json(path: &Path) -> Option<Value> {
    if !path.is_file() || path.is_symlink() {
        return None;
    }
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn first_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for segment in *path {
            current = current.get(*segment)?;
        }
        current.as_str().map(str::to_string)
    })
}

fn first_u64(value: &Value, paths: &[&[&str]]) -> Option<u64> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for segment in *path {
            current = current.get(*segment)?;
        }
        current.as_u64()
    })
}

fn markdown_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

fn first_active_action(plan: &RemediationPlan) -> Option<&RemediationAction> {
    plan.actions
        .iter()
        .filter(|action| !matches!(action.status.as_str(), "verified" | "deferred"))
        .min_by_key(|action| {
            (
                queue_domain_rank(&action.domain),
                queue_priority_rank(&action.priority),
                std::cmp::Reverse(action.weight),
                action.title.to_ascii_lowercase(),
            )
        })
}

pub fn render_active_queue_markdown(run: &RemediationRun) -> String {
    let mut output = format!(
        "# Repository remediation order\n\n\
Generated from `{}` at `{}`.\n\n\
This is the active remediation queue, ranked from current Pronto evidence and \
each repository's intended remediation outcome. Inferred goals remain active \
until a repository-owned goal contract confirms them. \
Each plan also classifies every repo-level surface tracked by the UI; unresolved \
coverage entries link to concrete remediation actions. \
Repositories leave the active table when the current evidence produces no \
actionable work or records an explicit deferral. That is a point-in-time queue \
transition, not a permanent repository state; a later refresh may reopen the \
same repository. Git, provider, publication, and pruning actions still require \
their own authorization.\n\n\
For maturity-applicable goals, **{MATURITY_CLOSURE_TARGET:.1}/4 is the minimum \
maturity threshold and {MATURITY_IDEAL_SCORE:.1}/4 is the evidence-backed ideal**. \
Continue only material improvements after the threshold, and never add superficial \
documentation, configuration, tests, or other artifacts solely to raise the \
score.\n\n\
Ranking preserves plan status, the earliest unresolved remediation domain, \
and action priority before fleet leverage. Pronto, AIOS, and Quality Runner \
receive explicit control-plane or evidence-provider precedence before the \
intended repository goal and raw action weight are used as tie-breakers.\n\n\
## Active queue\n\n\
Active repositories: **{}**. Resolved action history entries: **{}**. GitHub-only candidates: **{}**.\n\n\
<!-- prettier-ignore -->\n\
| Rank | Repository | Goal | Goal source | Status | Current stage | Remaining path | Leverage | Tracked gaps | Active actions | First safe action |\n\
| ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | --- |\n",
        run.schema_version,
        run.generated_at,
        run.plans.len(),
        run.closures.len(),
        run.github_only_candidates.len()
    );
    if run.plans.is_empty() {
        output.push_str("| — | _No active remediation remains_ | — | — | complete | complete | — | — | 0 | 0 | Refresh scoped evidence before treating this as current. |\n");
    } else {
        for (index, plan) in run.plans.iter().enumerate() {
            let active_action_count = plan
                .actions
                .iter()
                .filter(|action| !matches!(action.status.as_str(), "verified" | "deferred"))
                .count();
            let tracked_gap_count = plan
                .coverage
                .iter()
                .filter(|entry| matches!(entry.status.as_str(), "attention" | "blocked"))
                .count();
            let first_action = first_active_action(plan)
                .map(|action| action.title.as_str())
                .unwrap_or("Refresh scoped evidence and recheck the plan.");
            let remaining_path = plan
                .explanation
                .phases
                .iter()
                .map(|phase| phase.title.as_str())
                .collect::<Vec<_>>()
                .join(" → ");
            let leverage = queue_leverage(&plan.repository_name).1;
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                index + 1,
                markdown_cell(&plan.repository_name),
                markdown_cell(&plan.goal.label),
                markdown_cell(&plan.goal.source),
                markdown_cell(&plan.status),
                markdown_cell(&plan.current_stage),
                markdown_cell(&remaining_path),
                markdown_cell(leverage),
                tracked_gap_count,
                active_action_count,
                markdown_cell(first_action),
            ));
        }
    }
    output.push_str("\n## GitHub-only candidates\n\n");
    if run.github_only_candidates.is_empty() {
        output
            .push_str("No GitHub-only candidates are present in the current provider snapshot.\n");
    } else {
        output.push_str(
            "These provider-backed repositories have no matching local checkout; they remain counted without creating synthetic local plans. The terminal remediation task is **GitHub only**.\n\n\
<!-- prettier-ignore -->\n\
| Candidate | Label | Status | Last remediation task | Observed at |\n\
| --- | --- | --- | --- | --- |\n",
        );
        for candidate in &run.github_only_candidates {
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | `{}` |\n",
                markdown_cell(&candidate.full_name),
                markdown_cell(&candidate.label),
                markdown_cell(&candidate.status),
                markdown_cell(&candidate.last_remediation_task),
                markdown_cell(&candidate.observed_at),
            ));
        }
    }
    output.push_str("\n## Resolved action history\n\n");
    if run.closures.is_empty() {
        output.push_str("No resolved action history is present in this run.\n");
    } else {
        output.push_str(
            "<!-- prettier-ignore -->\n\
| Repository | Goal | Goal source | Disposition | Resolved at | Resolved actions | Evidence observed at | Summary |\n\
| --- | --- | --- | --- | --- | ---: | --- | --- |\n",
        );
        for closure in &run.closures {
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | `{}` | {} | {} | {} |\n",
                markdown_cell(&closure.repository_name),
                markdown_cell(&closure.target_state),
                markdown_cell(&closure.goal_source),
                markdown_cell(&closure.disposition),
                markdown_cell(&closure.closed_at),
                closure.resolved_action_count,
                closure
                    .last_evidence_at
                    .as_deref()
                    .map(markdown_cell)
                    .unwrap_or_else(|| "Not recorded".to_string()),
                markdown_cell(&closure.summary),
            ));
        }
    }
    output.push_str(
        "\nA later refresh may return a repository to the active queue when new or \
regressed evidence creates actionable work.\n",
    );
    output
}
