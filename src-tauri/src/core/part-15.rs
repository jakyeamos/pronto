fn json_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    if let Some(object) = value.as_object() {
        for key in keys {
            if let Some(item) = object.get(*key).and_then(serde_json::Value::as_str) {
                return Some(item.to_string());
            }
        }
        for child in object.values() {
            if let Some(found) = json_string(child, keys) {
                return Some(found);
            }
        }
    } else if let Some(items) = value.as_array() {
        for child in items {
            if let Some(found) = json_string(child, keys) {
                return Some(found);
            }
        }
    }
    None
}

fn resolve_qr_executable(requested: Option<&str>) -> String {
    if let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        return requested.to_string();
    }
    if let Some(from_environment) = std::env::var_os("PRONTO_QR_BIN") {
        if !from_environment.is_empty() {
            return from_environment.to_string_lossy().to_string();
        }
    }
    [
        "/Users/jakyeamos/projects/quality-runner/.venv/bin/qr",
        "qr",
        "quality-runner",
    ]
    .iter()
    .find(|candidate| !candidate.contains('/') || Path::new(candidate).is_file())
    .unwrap_or(&"qr")
    .to_string()
}

fn resolve_ci_command(command_name: &str, environment_key: &str) -> Option<PathBuf> {
    if let Some(requested) = std::env::var_os(environment_key) {
        if requested.to_string_lossy().contains('/') {
            let path = PathBuf::from(requested);
            if path.is_file() {
                return Some(path);
            }
        } else if let Some(path) = std::env::var_os("PATH").as_deref().and_then(|path| {
            std::env::split_paths(path).find(|directory| directory.join(&requested).is_file())
        }) {
            return Some(path.join(requested));
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(command_name))
        .find(|candidate| candidate.is_file())
}

fn ci_child_path(node: &Path, codex: &Path) -> Option<OsString> {
    let mut directories = vec![node.parent()?.to_path_buf(), codex.parent()?.to_path_buf()];
    if let Some(path) = std::env::var_os("PATH") {
        directories.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(directories).ok()
}

fn resolve_ci_bridge() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("PRONTO_CI_BRIDGE") {
        candidates.push(PathBuf::from(path));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("projects/ci-incident-router"));
        candidates.push(home.join("Documents/ci-incident-router"));
    }
    candidates
        .into_iter()
        .find(|path| path.join("bin/codex-ci.mjs").is_file())
        .ok_or_else(|| {
            "CI bridge is unavailable; set PRONTO_CI_BRIDGE to the ci-incident-router checkout."
                .to_string()
        })
}

fn ci_handoff_slug(repository: &str) -> String {
    repository
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn ci_process_failure(prefix: &str, output: &std::process::Output) -> String {
    let status = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string());
    format!("{prefix}; bridge exited with status {status}")
}

fn start_ci_codex_handoff_at(
    path: &Path,
    repository_name: &str,
    run_id: u64,
    run_attempt: u64,
) -> Result<CiCodexHandoffReceipt, String> {
    let normalized_repository = normalize_remote_name(repository_name)
        .ok_or_else(|| "CI handoff requires a GitHub repository such as owner/name.".to_string())?;
    let state = load_store(path)?;
    let remote = state
        .remote_repositories
        .iter()
        .find(|repository| {
            normalize_remote_name(&repository.full_name).as_deref() == Some(&normalized_repository)
        })
        .ok_or_else(|| {
            "That repository is not present in the current GitHub snapshot.".to_string()
        })?;
    let run = remote
        .ci_runs
        .iter()
        .find(|run| run.id == run_id && run.run_attempt == run_attempt)
        .ok_or_else(|| "That CI run is no longer present in the current snapshot; refresh GitHub and try again.".to_string())?;
    if !ci_run_needs_artifact(run) {
        return Err("Codex handoff is available only for failed or cancelled CI runs.".to_string());
    }
    if run.prompt_artifact.is_none() {
        return Err(
            "This CI run has no downloadable Codex prompt artifact. Verify the bridge workflow is installed and rerun GitHub refresh."
                .to_string(),
        );
    }
    let checkout = state
        .repositories
        .iter()
        .find(|repository| {
            repository
                .remote_url
                .as_deref()
                .and_then(normalize_remote_name)
                .as_deref()
                == Some(&normalized_repository)
        })
        .map(|repository| PathBuf::from(&repository.path))
        .ok_or_else(|| {
            "Codex handoff needs a registered local checkout for this repository; remote-only repositories stay diagnosis-only in the tracker."
                .to_string()
        })?;
    let checkout = canonical_path(&checkout)
        .filter(|checkout| checkout.is_dir())
        .ok_or_else(|| "The registered local checkout is unavailable.".to_string())?;
    let bridge = resolve_ci_bridge()?;
    let node = resolve_ci_command("node", "PRONTO_NODE_BIN").ok_or_else(|| {
        "Node.js is unavailable; install or expose node before starting Codex.".to_string()
    })?;
    let codex = resolve_ci_command("codex", "PRONTO_CODEX_BIN").ok_or_else(|| {
        "Codex is unavailable; expose the codex command before starting a CI handoff.".to_string()
    })?;
    let output_directory = std::env::temp_dir().join(format!(
        "pronto-ci-handoff-{}-{}-{}",
        ci_handoff_slug(&normalized_repository),
        run_id,
        run_attempt
    ));
    fs::create_dir_all(&output_directory).map_err(|error| {
        format!(
            "Could not create the temporary CI prompt directory {}: {error}",
            output_directory.display()
        )
    })?;
    let child_path = ci_child_path(&node, &codex)
        .or_else(|| std::env::var_os("PATH"))
        .unwrap_or_default();
    let run_id_text = run_id.to_string();
    let checkout_text = checkout.to_string_lossy().to_string();
    let output_text = output_directory.to_string_lossy().to_string();
    let download = Command::new(&node)
        .current_dir(&bridge)
        .args([
            "./bin/codex-ci.mjs",
            "download",
            "--run",
            run_id_text.as_str(),
            "--repo",
            normalized_repository.as_str(),
            "--repo-path",
            checkout_text.as_str(),
            "--output-dir",
            output_text.as_str(),
        ])
        .env("PATH", &child_path)
        .output()
        .map_err(|error| format!("Could not run the CI bridge download: {error}"))?;
    if !download.status.success() {
        return Err(ci_process_failure(
            "The CI prompt artifact could not be downloaded",
            &download,
        ));
    }
    let prompt_path = output_directory.join("codex-ci-prompt.md");
    if !prompt_path.is_file() {
        return Err("The CI bridge completed without producing codex-ci-prompt.md.".to_string());
    }
    let prompt_text = prompt_path.to_string_lossy().to_string();
    let codex_text = codex.to_string_lossy().to_string();
    let handoff = Command::new(&node)
        .current_dir(&bridge)
        .args([
            "./bin/codex-ci.mjs",
            "handoff",
            "--prompt",
            prompt_text.as_str(),
            "--repo",
            checkout_text.as_str(),
            "--codex",
            codex_text.as_str(),
            "--start",
        ])
        .env("PATH", &child_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the Codex CI handoff: {error}"))?;
    let _ = handoff.id();
    Ok(CiCodexHandoffReceipt {
        schema_version: "pronto-ci-codex-handoff/v1".to_string(),
        status: "started".to_string(),
        repository: normalized_repository,
        run_id,
        run_attempt,
        failure_signature: run.failure_signature.clone(),
        prompt_directory: output_directory.to_string_lossy().to_string(),
        started: true,
        message: "Codex was started with a read-only CI diagnosis prompt. Review its suggestions before making changes.".to_string(),
    })
}

fn qr_projects_root(repository_paths: &[String]) -> Result<PathBuf, String> {
    let home_projects = dirs::home_dir().map(|home| home.join("projects"));
    if let Some(root) = home_projects.filter(|root| {
        root.is_dir()
            && repository_paths
                .iter()
                .all(|path| Path::new(path).starts_with(root))
    }) {
        return Ok(root);
    }
    let mut common = PathBuf::from(
        repository_paths
            .first()
            .ok_or_else(|| "No eligible repositories are registered for QR.".to_string())?,
    );
    for path in repository_paths.iter().skip(1) {
        let candidate = Path::new(path);
        while !candidate.starts_with(&common) {
            if !common.pop() {
                return Err("Could not derive a bounded Quality Runner projects root.".to_string());
            }
        }
    }
    if common == Path::new("/") || !common.is_dir() {
        return Err(format!(
            "Quality Runner scope root is not bounded to a usable directory: {}",
            common.display()
        ));
    }
    Ok(common)
}

fn process_qr_audit_lifecycle(
    path: &Path,
    refresh_id: &str,
    steps: &mut [remediation::RemediationRefreshStep],
    qr: &str,
    audit_id: &str,
    artifact_root: Option<String>,
    publish_feed: bool,
) -> Result<bool, (String, String)> {
    let mut qr_audit_commands = vec![
        (
            "qr_replay",
            "replay",
            "Replay passed and the fleet artifacts are deterministic.",
        ),
        (
            "qr_report",
            "report",
            "Aggregate QR report written for review.",
        ),
    ];
    if publish_feed {
        qr_audit_commands.push((
            "qr_feed",
            "feed",
            "Canonical maturity feed published from the replay-validated audit.",
        ));
    }
    for (step_id, action, success_detail) in qr_audit_commands {
        set_remediation_refresh_step(
            steps,
            step_id,
            "in_progress",
            format!("Running qr fleet audit {action} for {audit_id}."),
            None,
        );
        if let Err(error) =
            persist_remediation_refresh(path, refresh_id, "in_progress", None, steps)
        {
            return Err((step_id.to_string(), error));
        }
        let arguments = vec![
            "fleet".to_string(),
            "audit".to_string(),
            action.to_string(),
            "--audit-id".to_string(),
            audit_id.to_string(),
            "--json".to_string(),
        ];
        match run_json_command(qr, &arguments) {
            Ok(payload) => {
                let status = json_string(&payload, &["status"]).unwrap_or_default();
                let valid = match step_id {
                    "qr_replay" => {
                        status == "passed"
                            && payload
                                .get("deterministic")
                                .and_then(serde_json::Value::as_bool)
                                == Some(true)
                    }
                    "qr_feed" => status == "published",
                    _ => status == "review_required",
                };
                if !valid {
                    let detail = format!("Quality Runner {action} returned status {status}.");
                    if step_id == "qr_feed" {
                        set_remediation_refresh_step(steps, step_id, "blocked", detail, None);
                        if let Err(error) = persist_remediation_refresh(
                            path,
                            refresh_id,
                            "partial",
                            Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                            steps,
                        ) {
                            return Err((step_id.to_string(), error));
                        }
                        return Ok(false);
                    }
                    return Err((step_id.to_string(), detail));
                }
                let evidence_path = json_string(&payload, &["feed_path", "artifact_root"])
                    .or_else(|| artifact_root.clone());
                set_remediation_refresh_step(
                    steps,
                    step_id,
                    "completed",
                    success_detail,
                    evidence_path,
                );
            }
            Err(error) if step_id == "qr_feed" => {
                set_remediation_refresh_step(steps, step_id, "blocked", error, None);
                if let Err(error) = persist_remediation_refresh(
                    path,
                    refresh_id,
                    "partial",
                    Some("QR maturity feed publication was blocked; prior maturity evidence was retained.".to_string()),
                    steps,
                ) {
                    return Err((step_id.to_string(), error));
                }
                return Ok(false);
            }
            Err(error) => return Err((step_id.to_string(), error)),
        }
        if let Err(error) =
            persist_remediation_refresh(path, refresh_id, "in_progress", None, steps)
        {
            return Err((step_id.to_string(), error));
        }
    }
    if !publish_feed {
        set_remediation_refresh_step(
            steps,
            "qr_feed",
            "skipped",
            "A repository-scoped refresh retains the canonical fleet feed and imports replay-validated audit evidence directly.",
            artifact_root,
        );
        if let Err(error) =
            persist_remediation_refresh(path, refresh_id, "in_progress", None, steps)
        {
            return Err(("qr_feed".to_string(), error));
        }
    }
    Ok(publish_feed)
}
