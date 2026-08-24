fn copy_target_evidence_tree_inner(
    source: &Path,
    destination: &Path,
    old_root: &str,
    new_root: &str,
) -> Result<usize, String> {
    let file_type = fs::symlink_metadata(source)
        .map_err(|error| {
            format!(
                "Could not inspect target evidence {}: {error}",
                source.display()
            )
        })?
        .file_type();
    if file_type.is_symlink() {
        return Ok(0);
    }
    if file_type.is_dir() {
        fs::create_dir_all(destination).map_err(|error| {
            format!(
                "Could not create copied target evidence directory {}: {error}",
                destination.display()
            )
        })?;
        let mut copied = 0;
        let entries = fs::read_dir(source).map_err(|error| {
            format!(
                "Could not read target evidence directory {}: {error}",
                source.display()
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "Could not enumerate target evidence directory {}: {error}",
                    source.display()
                )
            })?;
            copied += copy_target_evidence_tree_inner(
                &entry.path(),
                &destination.join(entry.file_name()),
                old_root,
                new_root,
            )?;
        }
        return Ok(copied);
    }
    if !file_type.is_file() {
        return Ok(0);
    }
    if destination.exists() {
        return Err(format!(
            "Target evidence destination already exists: {}",
            destination.display()
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Could not create copied target evidence parent {}: {error}",
                parent.display()
            )
        })?;
    }
    let bytes = fs::read(source).map_err(|error| {
        format!(
            "Could not read target evidence file {}: {error}",
            source.display()
        )
    })?;
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        fs::write(destination, text.replace(old_root, new_root)).map_err(|error| {
            format!(
                "Could not write copied target evidence file {}: {error}",
                destination.display()
            )
        })?;
    } else {
        fs::write(destination, bytes).map_err(|error| {
            format!(
                "Could not write copied target evidence file {}: {error}",
                destination.display()
            )
        })?;
    }
    Ok(1)
}

fn copy_target_evidence_tree(
    source: &Path,
    destination: &Path,
    old_root: &Path,
    new_root: &Path,
) -> Result<usize, String> {
    copy_target_evidence_tree_inner(
        source,
        destination,
        &old_root.to_string_lossy(),
        &new_root.to_string_lossy(),
    )
}

fn rewrite_target_qr_branch_provenance_value(
    value: &mut serde_json::Value,
    target_branch: &str,
) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            let mut rewritten = false;
            for (key, child) in object.iter_mut() {
                if key == "branch" && child.as_str() == Some("HEAD") {
                    *child = serde_json::Value::String(target_branch.to_string());
                    rewritten = true;
                } else if key == "ref" && child.as_str() == Some("refs/heads/HEAD") {
                    *child = serde_json::Value::String(format!("refs/heads/{target_branch}"));
                    rewritten = true;
                }
                rewritten |= rewrite_target_qr_branch_provenance_value(child, target_branch);
            }
            rewritten
        }
        serde_json::Value::Array(items) => items
            .iter_mut()
            .map(|item| rewrite_target_qr_branch_provenance_value(item, target_branch))
            .any(|rewritten| rewritten),
        _ => false,
    }
}

fn rewrite_target_qr_branch_provenance_inner(
    root: &Path,
    target_branch: &str,
) -> Result<usize, String> {
    let file_type = fs::symlink_metadata(root)
        .map_err(|error| {
            format!(
                "Could not inspect target QR artifact {}: {error}",
                root.display()
            )
        })?
        .file_type();
    if file_type.is_symlink() {
        return Ok(0);
    }
    if file_type.is_dir() {
        let mut rewritten = 0;
        for entry in fs::read_dir(root).map_err(|error| {
            format!(
                "Could not read target QR artifact directory {}: {error}",
                root.display()
            )
        })? {
            rewritten += rewrite_target_qr_branch_provenance_inner(
                &entry
                    .map_err(|error| format!("Could not enumerate target QR artifacts: {error}"))?
                    .path(),
                target_branch,
            )?;
        }
        return Ok(rewritten);
    }
    if !file_type.is_file() || root.extension().and_then(|value| value.to_str()) != Some("json") {
        return Ok(0);
    }
    let bytes = fs::read(root).map_err(|error| {
        format!(
            "Could not read target QR artifact {}: {error}",
            root.display()
        )
    })?;
    let Ok(mut payload) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Ok(0);
    };
    if !rewrite_target_qr_branch_provenance_value(&mut payload, target_branch) {
        return Ok(0);
    }
    let mut encoded = serde_json::to_vec_pretty(&payload).map_err(|error| {
        format!(
            "Could not encode target QR artifact {}: {error}",
            root.display()
        )
    })?;
    encoded.push(b'\n');
    fs::write(root, encoded).map_err(|error| {
        format!(
            "Could not rewrite target QR artifact {}: {error}",
            root.display()
        )
    })?;
    Ok(1)
}

fn rewrite_target_qr_branch_provenance(root: &Path, target_branch: &str) -> Result<usize, String> {
    rewrite_target_qr_branch_provenance_inner(root, target_branch)
}

fn copy_target_qr_runs(
    target_worktree: &Path,
    repository_path: &Path,
    run_id_prefix: &str,
    target_branch: &str,
) -> Result<usize, String> {
    let source_root = target_worktree.join(".quality-runner").join("runs");
    if !source_root.is_dir() {
        return Ok(0);
    }
    let destination_root = repository_path.join(".quality-runner").join("runs");
    fs::create_dir_all(&destination_root).map_err(|error| {
        format!(
            "Could not create Pronto target QR artifact directory {}: {error}",
            destination_root.display()
        )
    })?;
    let mut copied = 0;
    for entry in fs::read_dir(&source_root).map_err(|error| {
        format!(
            "Could not read target QR artifact directory {}: {error}",
            source_root.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "Could not enumerate target QR artifact directory {}: {error}",
                source_root.display()
            )
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(run_id_prefix)
            || !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
        {
            continue;
        }
        let destination = destination_root.join(name.as_ref());
        copied += copy_target_evidence_tree(
            &entry.path(),
            &destination,
            target_worktree,
            repository_path,
        )?;
        rewrite_target_qr_branch_provenance(&destination, target_branch)?;
    }
    Ok(copied)
}

fn rewrite_target_evidence_paths_inner(
    root: &Path,
    old_root: &str,
    new_root: &str,
) -> Result<usize, String> {
    let file_type = fs::symlink_metadata(root)
        .map_err(|error| {
            format!(
                "Could not inspect target audit artifact {}: {error}",
                root.display()
            )
        })?
        .file_type();
    if file_type.is_symlink() {
        return Ok(0);
    }
    if file_type.is_dir() {
        let mut rewritten = 0;
        for entry in fs::read_dir(root).map_err(|error| {
            format!(
                "Could not read target audit artifact directory {}: {error}",
                root.display()
            )
        })? {
            rewritten += rewrite_target_evidence_paths_inner(
                &entry
                    .map_err(|error| {
                        format!("Could not enumerate target audit artifacts: {error}")
                    })?
                    .path(),
                old_root,
                new_root,
            )?;
        }
        return Ok(rewritten);
    }
    if !file_type.is_file() {
        return Ok(0);
    }
    let bytes = fs::read(root).map_err(|error| {
        format!(
            "Could not read target audit artifact {}: {error}",
            root.display()
        )
    })?;
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(0);
    };
    if !text.contains(old_root) {
        return Ok(0);
    }
    fs::write(root, text.replace(old_root, new_root)).map_err(|error| {
        format!(
            "Could not rewrite target audit artifact {}: {error}",
            root.display()
        )
    })?;
    Ok(1)
}

fn rewrite_target_evidence_paths(
    root: &Path,
    old_root: &Path,
    new_root: &Path,
) -> Result<usize, String> {
    rewrite_target_evidence_paths_inner(
        root,
        &old_root.to_string_lossy(),
        &new_root.to_string_lossy(),
    )
}

fn bounded_target_artifact_root(base: &Path, payload: &serde_json::Value) -> Option<PathBuf> {
    let candidate = json_string(payload, &["artifact_root"])?;
    let candidate = canonical_path(Path::new(&candidate))?;
    let base = canonical_path(base)?;
    candidate
        .starts_with(&base)
        .then_some(candidate)
        .filter(|path| path.is_dir())
}

fn concise_target_command_error(error: &str) -> String {
    let detail = error
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(error)
        .trim();
    let mut concise = detail.chars().take(300).collect::<String>();
    if detail.chars().count() > 300 {
        concise.push_str("…");
    }
    concise
}

fn run_target_qr_refresh(
    qr_executable: &str,
    target_worktree: &Path,
    repository_path: &Path,
    run_id_prefix: &str,
    target_branch: &str,
) -> Result<String, String> {
    let arguments = vec![
        "refresh".to_string(),
        target_worktree.to_string_lossy().to_string(),
        "--run-id-prefix".to_string(),
        run_id_prefix.to_string(),
        "--execute-gates".to_string(),
        "--worktree-mode".to_string(),
        "disposable".to_string(),
        "--no-progress".to_string(),
        "--json".to_string(),
        "--timeout-seconds".to_string(),
        TARGET_EVIDENCE_GATE_TIMEOUT_SECONDS.to_string(),
        "--total-timeout-seconds".to_string(),
        TARGET_EVIDENCE_TOTAL_TIMEOUT_SECONDS.to_string(),
    ];
    let result = run_json_command_in_with_status(qr_executable, &arguments, Some(target_worktree));
    let copied = copy_target_qr_runs(
        target_worktree,
        repository_path,
        run_id_prefix,
        target_branch,
    )?;
    match result {
        Ok((payload, success, detail)) => {
            let status =
                json_string(&payload, &["status"]).unwrap_or_else(|| "completed".to_string());
            if success {
                Ok(format!(
                    "QR refresh {status}; copied {copied} artifact files"
                ))
            } else {
                Ok(format!(
                    "QR refresh {status}; retained failed gate evidence and copied {copied} artifact files{}",
                    detail
                        .as_deref()
                        .map(|value| format!(": {}", concise_target_command_error(value)))
                        .unwrap_or_default()
                ))
            }
        }
        Err(error) => Err(format!(
            "QR refresh failed after copying {copied} artifact files: {}",
            concise_target_command_error(&error)
        )),
    }
}

fn run_target_fleet_audit(
    qr_executable: &str,
    target_worktree: &Path,
    repository_path: &Path,
    projects_root: &Path,
    output_base: &Path,
    target_branch: &str,
    repository_id: &str,
) -> Result<(String, PathBuf), String> {
    fs::create_dir_all(output_base).map_err(|error| {
        format!(
            "Could not create target fleet audit output directory {}: {error}",
            output_base.display()
        )
    })?;
    let arguments = vec![
        "fleet".to_string(),
        "audit".to_string(),
        "run".to_string(),
        "--repo-path".to_string(),
        target_worktree.to_string_lossy().to_string(),
        "--projects-root".to_string(),
        projects_root.to_string_lossy().to_string(),
        "--output-dir".to_string(),
        output_base.to_string_lossy().to_string(),
        "--dynamic".to_string(),
        "--no-changed-only".to_string(),
        "--timeout-seconds".to_string(),
        TARGET_EVIDENCE_GATE_TIMEOUT_SECONDS.to_string(),
        "--target-override".to_string(),
        format!("{repository_id}={target_branch}"),
        "--json".to_string(),
    ];
    let payload = run_json_command_in(qr_executable, &arguments, Some(target_worktree))?;
    let artifact_root = bounded_target_artifact_root(output_base, &payload).ok_or_else(|| {
        format!(
            "Quality Runner fleet audit did not return a bounded artifact root under {}",
            output_base.display()
        )
    })?;
    let rewritten =
        rewrite_target_evidence_paths(&artifact_root, target_worktree, repository_path)?;
    let status = json_string(&payload, &["status"]).unwrap_or_else(|| "completed".to_string());
    Ok((
        format!(
            "fleet audit {status}; ingested {} rewritten artifact files",
            rewritten
        ),
        artifact_root,
    ))
}
