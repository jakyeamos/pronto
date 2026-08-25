#[derive(Debug, Clone, PartialEq)]
struct GitHubRepositoryControl {
    full_name: String,
    visibility: String,
    archived: bool,
    viewer_can_administer: Option<bool>,
    viewer_permission: Option<String>,
}

fn normalize_github_visibility(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "public" | "private" | "internal") {
        Ok(normalized)
    } else {
        Err("GitHub visibility must be public, private, or internal.".to_string())
    }
}

fn github_admin_from_payload(payload: &serde_json::Value) -> Option<bool> {
    payload
        .get("viewerCanAdminister")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            payload
                .get("permissions")
                .and_then(|permissions| permissions.get("admin"))
                .and_then(serde_json::Value::as_bool)
        })
}

fn github_permission_from_payload(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("viewerPermission")
        .and_then(serde_json::Value::as_str)
        .or_else(|| payload.get("permission").and_then(serde_json::Value::as_str))
        .map(|permission| permission.to_ascii_lowercase())
        .or_else(|| {
            github_admin_from_payload(payload)
                .filter(|admin| *admin)
                .map(|_| "admin".to_string())
        })
}

fn repository_visibility_from_payload(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("visibility")
        .and_then(serde_json::Value::as_str)
        .and_then(|visibility| normalize_github_visibility(visibility).ok())
        .or_else(|| {
            payload
                .get("private")
                .and_then(serde_json::Value::as_bool)
                .map(|private| if private { "private" } else { "public" }.to_string())
        })
}

fn parse_github_repository_control(
    payload: &serde_json::Value,
) -> Result<GitHubRepositoryControl, String> {
    let full_name = payload
        .get("full_name")
        .and_then(serde_json::Value::as_str)
        .and_then(normalized_github_repository_name)
        .ok_or_else(|| "GitHub control readback did not include a valid full_name.".to_string())?;
    let visibility = repository_visibility_from_payload(payload).ok_or_else(|| {
        "GitHub control readback did not include a supported visibility.".to_string()
    })?;
    Ok(GitHubRepositoryControl {
        full_name,
        visibility,
        archived: payload
            .get("archived")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        viewer_can_administer: github_admin_from_payload(payload),
        viewer_permission: github_permission_from_payload(payload),
    })
}

fn validate_github_visibility_preflight(
    control: &GitHubRepositoryControl,
    expected_full_name: &str,
    target_visibility: &str,
) -> Result<(), String> {
    let expected_identity = normalized_github_repository_name(expected_full_name)
        .ok_or_else(|| "The stored GitHub repository identity is invalid.".to_string())?;
    if control.full_name != expected_identity {
        return Err(format!(
            "GitHub identity mismatch: expected {expected_identity}, read back {}.",
            control.full_name
        ));
    }
    if control.archived {
        return Err("Archived GitHub repositories cannot be changed through Release Radar.".to_string());
    }
    if control.viewer_can_administer != Some(true) {
        return Err(
            "GitHub visibility change requires permissions.admin=true on a fresh control readback."
                .to_string(),
        );
    }
    if control.visibility == target_visibility {
        return Err(format!(
            "GitHub repository {expected_full_name} is already {target_visibility}; no provider mutation was sent."
        ));
    }
    Ok(())
}

fn github_visibility_readback_matches(
    control: &GitHubRepositoryControl,
    expected_full_name: &str,
    target_visibility: &str,
) -> Result<(), String> {
    let expected_identity = normalized_github_repository_name(expected_full_name)
        .ok_or_else(|| "The stored GitHub repository identity is invalid.".to_string())?;
    if control.full_name != expected_identity || control.visibility != target_visibility {
        return Err(format!(
            "GitHub post-change readback mismatch for {expected_full_name}: expected {target_visibility}, read back {}.",
            control.visibility
        ));
    }
    Ok(())
}

impl GitHubCliAdapter {
    fn repository_control(&self, full_name: &str) -> Result<GitHubRepositoryControl, String> {
        let endpoint = format!("repos/{full_name}");
        let payload = self.json(&["api", endpoint.as_str()])?;
        parse_github_repository_control(&payload)
    }

    fn edit_repository_visibility(
        &self,
        full_name: &str,
        target_visibility: &str,
    ) -> Result<(), String> {
        let target_visibility = normalize_github_visibility(target_visibility)?;
        let output = self.run(&[
            "repo",
            "edit",
            full_name,
            "--visibility",
            target_visibility.as_str(),
            "--accept-visibility-change-consequences",
        ])?;
        if output.status.success() {
            Ok(())
        } else {
            Err(Self::failure_message(&output.stderr))
        }
    }
}

fn record_visibility_audit(
    path: &Path,
    audit_id: &str,
    status: &str,
    summary: String,
) -> Result<(), String> {
    let mut state = load_store(path)?;
    update_action_audit(&mut state, audit_id, status, summary)?;
    save_store(path, &state)
}

fn change_github_repository_visibility_at(
    path: &Path,
    repository_id: &str,
    target_visibility: &str,
    confirmation: &str,
) -> Result<PortfolioSnapshot, String> {
    let target_visibility = normalize_github_visibility(target_visibility)?;
    let state = load_store_read_only(path)?;
    let repository = state
        .remote_repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| {
            "GitHub repository is not present in the current provider snapshot.".to_string()
        })?;
    if repository.provider != "github" {
        return Err("Visibility changes are supported only for GitHub repositories.".to_string());
    }
    let expected_full_name = repository.full_name.clone();
    if confirmation != expected_full_name {
        return Err(format!(
            "Type the exact repository name '{expected_full_name}' to confirm this provider write."
        ));
    }
    let created_at = iso_now();
    let audit = ActionAudit {
        id: action_audit_id("github_visibility", &created_at),
        action: "github_visibility".to_string(),
        target_ids: vec![repository_id.to_string()],
        risk: "provider-write".to_string(),
        status: "Started".to_string(),
        summary: format!("Preparing GitHub visibility change for {expected_full_name}."),
        created_at,
        completed_at: None,
    };
    let audit_id = audit.id.clone();
    let mut state = load_store(path)?;
    state.action_audits.push(audit);
    prune_action_audits(&mut state);
    save_store(path, &state)?;

    let adapter = GitHubCliAdapter::default();
    let control = match adapter.repository_control(&expected_full_name) {
        Ok(control) => control,
        Err(error) => {
            let _ = record_visibility_audit(path, &audit_id, "Unknown", error.clone());
            return Err(error);
        }
    };
    if let Err(error) = validate_github_visibility_preflight(
        &control,
        &expected_full_name,
        &target_visibility,
    ) {
        let status = if error.contains("identity mismatch") {
            "Unknown"
        } else {
            "Rejected"
        };
        let _ = record_visibility_audit(path, &audit_id, status, error.clone());
        return Err(error);
    }
    record_visibility_audit(
        path,
        &audit_id,
        "Preflighted",
        format!(
            "Fresh GitHub readback confirmed {expected_full_name} is {} and administration is available.",
            control.visibility
        ),
    )?;
    if let Err(error) = adapter.edit_repository_visibility(&expected_full_name, &target_visibility)
    {
        let _ = record_visibility_audit(path, &audit_id, "Unknown", error.clone());
        return Err(error);
    }
    let readback = match adapter.repository_control(&expected_full_name) {
        Ok(readback) => readback,
        Err(error) => {
            let _ = record_visibility_audit(path, &audit_id, "Unknown", error.clone());
            return Err(format!(
                "GitHub visibility mutation was sent, but post-change readback failed: {error}"
            ));
        }
    };
    if let Err(error) = github_visibility_readback_matches(
        &readback,
        &expected_full_name,
        &target_visibility,
    ) {
        let _ = record_visibility_audit(path, &audit_id, "Unknown", error.clone());
        return Err(error);
    }
    let mut state = load_store(path)?;
    let remote = state
        .remote_repositories
        .iter_mut()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| {
            "The GitHub repository disappeared before local readback projection.".to_string()
        })?;
    remote.visibility = Some(readback.visibility.clone());
    remote.archived = readback.archived;
    remote.viewer_can_administer = readback.viewer_can_administer;
    remote.viewer_permission = readback.viewer_permission;
    remote.last_refreshed_at = iso_now();
    update_action_audit(
        &mut state,
        &audit_id,
        "Completed",
        format!(
            "GitHub visibility changed to {} and read back for {expected_full_name}.",
            readback.visibility
        ),
    )?;
    save_store(path, &state)?;
    Ok(snapshot_from_store(path, &state))
}

#[tauri::command]
pub fn change_github_repository_visibility(
    repository_id: String,
    target_visibility: String,
    confirmation: String,
) -> Result<PortfolioSnapshot, String> {
    change_github_repository_visibility_at(
        &store_path(),
        &repository_id,
        &target_visibility,
        &confirmation,
    )
}
