// Quality Runner owns model-audit evidence. This bridge neither invokes models
// nor copies findings into a second remediation store.
fn run_model_audit_cli(arguments: &[String]) -> Result<(serde_json::Value, i32), String> {
    let positionals = cli_positionals(arguments, &["--run-id", "--selection", "--qr-bin"])?;
    let action = positionals.first().map(String::as_str).unwrap_or("");
    let mut qr_arguments = vec!["model-audit".to_string(), action.to_string()];
    let expected_schema;
    let mut expected_repository = None;
    let mut expected_run_id = None;
    let mut expected_fleet = None;
    match action {
        "report" if positionals.len() == 2 => {
            if cli_option(arguments, "--selection")?.is_some() {
                return Err("--selection belongs to model-audit fleet".to_string());
            }
            let repository = PathBuf::from(&positionals[1]);
            if !repository.is_absolute() {
                return Err("model-audit report requires an absolute repository path".to_string());
            }
            let repository = repository.canonicalize().map_err(|error| error.to_string())?;
            let run_id = cli_option(arguments, "--run-id")?
                .ok_or_else(|| "model-audit report requires --run-id".to_string())?;
            qr_arguments.extend([
                repository.to_string_lossy().to_string(),
                "--run-id".to_string(),
                run_id.clone(),
            ]);
            expected_repository = Some(repository.to_string_lossy().to_string());
            expected_run_id = Some(run_id);
            expected_schema = "quality-runner-model-audit-report/v1";
        }
        "fleet" if positionals.len() == 1 => {
            if cli_option(arguments, "--run-id")?.is_some() {
                return Err("fleet run IDs belong in the explicit selection file".to_string());
            }
            let selection = cli_option(arguments, "--selection")?
                .ok_or_else(|| "model-audit fleet requires --selection".to_string())?;
            if !Path::new(&selection).is_absolute() {
                return Err("model-audit selection must be an absolute path".to_string());
            }
            let metadata = std::fs::metadata(&selection).map_err(|error| error.to_string())?;
            if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 {
                return Err("model-audit selection must be a regular JSON file under 4 MiB".to_string());
            }
            let bytes = std::fs::read(&selection).map_err(|error| error.to_string())?;
            expected_fleet = Some(serde_json::from_slice::<serde_json::Value>(&bytes)
                .map_err(|error| error.to_string())?);
            qr_arguments.extend(["--selection".to_string(), selection]);
            expected_schema = "quality-runner-model-audit-fleet/v1";
        }
        _ => return Err("Usage: pronto model-audit report <absolute-repository> --run-id <id> [--qr-bin <path>] [--json], or model-audit fleet --selection <absolute-json> [--qr-bin <path>] [--json]".to_string()),
    }
    qr_arguments.push("--json".to_string());
    let requested = cli_option(arguments, "--qr-bin")?;
    let executable = resolve_qr_executable(requested.as_deref());
    let (payload, success, _) = run_json_command_in_with_status(&executable, &qr_arguments, None)?;
    validate_model_audit_projection(
        &payload,
        expected_schema,
        expected_repository.as_deref(),
        expected_run_id.as_deref(),
    )?;
    if let Some(selection) = expected_fleet {
        validate_model_audit_fleet_scope(&payload, &selection)?;
    }
    let partial = payload.get("status").and_then(serde_json::Value::as_str) == Some("partial");
    if !success && !partial {
        return Err("Quality Runner failed; no model-audit report was accepted".to_string());
    }
    Ok((payload, if partial { 1 } else { 0 }))
}

fn validate_model_audit_fleet_scope(
    payload: &serde_json::Value,
    selection: &serde_json::Value,
) -> Result<(), String> {
    let mismatch = "Model-audit fleet response does not match the requested selection and coverage";
    let selected = selection
        .get("repositories")
        .and_then(serde_json::Value::as_array)
        .filter(|rows| !rows.is_empty() && rows.len() <= 256)
        .ok_or(mismatch)?;
    if selection.get("schema").and_then(serde_json::Value::as_str)
        != Some("quality-runner-model-audit-selection/v1")
    {
        return Err(mismatch.to_string());
    }
    let mut expected = Vec::new();
    let mut identities = std::collections::BTreeSet::new();
    for item in selected {
        let path = item
            .get("repo_path")
            .and_then(serde_json::Value::as_str)
            .ok_or(mismatch)?;
        let run = item
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or(mismatch)?;
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(mismatch.to_string());
        }
        let canonical = path.canonicalize().ok();
        if !identities.insert(canonical.clone().unwrap_or_else(|| path.clone())) {
            return Err(mismatch.to_string());
        }
        expected.push((path, canonical, run));
    }
    let reports = payload
        .get("repositories")
        .and_then(serde_json::Value::as_array)
        .ok_or(mismatch)?;
    let unavailable = payload
        .get("unavailable")
        .and_then(serde_json::Value::as_array)
        .ok_or(mismatch)?;
    let mut seen = vec![false; expected.len()];
    for (rows, reported) in [(reports, true), (unavailable, false)] {
        for row in rows {
            let path = row
                .get(if reported { "repo_root" } else { "repo_path" })
                .and_then(serde_json::Value::as_str)
                .ok_or(mismatch)?;
            let run = row
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .ok_or(mismatch)?;
            let index = expected
                .iter()
                .position(|(original, canonical, expected_run)| {
                    let expected_path = if reported {
                        canonical.as_deref()
                    } else {
                        Some(original.as_path())
                    };
                    expected_path == Some(Path::new(path)) && *expected_run == run
                })
                .ok_or(mismatch)?;
            if seen[index] {
                return Err(mismatch.to_string());
            }
            seen[index] = true;
            if reported {
                validate_model_audit_projection(
                    row,
                    "quality-runner-model-audit-report/v1",
                    Some(path),
                    Some(run),
                )?;
            }
        }
    }
    if seen.iter().any(|present| !present)
        || payload["coverage"]
            != serde_json::json!({
                "selected": expected.len(), "reported": reports.len(), "unavailable": unavailable.len()
            })
        || payload.get("status").and_then(serde_json::Value::as_str)
            != Some(if unavailable.is_empty() {
                "reported"
            } else {
                "partial"
            })
    {
        return Err(mismatch.to_string());
    }
    Ok(())
}

fn validate_model_audit_projection(
    payload: &serde_json::Value,
    expected_schema: &str,
    expected_repository: Option<&str>,
    expected_run_id: Option<&str>,
) -> Result<(), String> {
    if payload.get("schema").and_then(serde_json::Value::as_str) != Some(expected_schema)
        || payload
            .get("read_only")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || payload
            .get("implementation_allowed")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
    {
        return Err("Unsupported or unsafe Quality Runner model-audit projection; use a compatible QR build".to_string());
    }
    let status = payload.get("status").and_then(serde_json::Value::as_str);
    if status != Some("reported") && !(expected_repository.is_none() && status == Some("partial")) {
        return Err("Model-audit projection has no valid reported/partial status".to_string());
    }
    if let Some(repository) = expected_repository {
        if payload.get("repo_root").and_then(serde_json::Value::as_str) != Some(repository)
            || payload.get("run_id").and_then(serde_json::Value::as_str) != expected_run_id
        {
            return Err(
                "Model-audit projection does not match the requested repository and run"
                    .to_string(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod model_audit_tests {
    use super::validate_model_audit_projection;
    use serde_json::json;

    #[test]
    fn model_audit_cli_binds_fleet_response_to_the_selection() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "pronto-audit-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let selection = root.join("selection.json");
        let producer = root.join("qr-fixture");
        let missing = root.join("missing");
        std::fs::write(
            &selection,
            json!({
                "schema":"quality-runner-model-audit-selection/v1",
                "repositories":[{"repo_path":root,"run_id":"a"},{"repo_path":missing,"run_id":"b"}]
            })
            .to_string(),
        )
        .unwrap();
        let good = json!({
            "schema":"quality-runner-model-audit-fleet/v1","read_only":true,
            "implementation_allowed":false,"status":"partial",
            "coverage":{"selected":2,"reported":1,"unavailable":1},
            "repositories":[{"schema":"quality-runner-model-audit-report/v1","read_only":true,
                "implementation_allowed":false,"status":"reported","repo_root":root,"run_id":"a"}],
            "unavailable":[{"repo_path":missing,"run_id":"b","error":"missing repository"}]
        });
        let arguments = vec![
            "model-audit".into(),
            "fleet".into(),
            "--selection".into(),
            selection.to_string_lossy().into(),
            "--qr-bin".into(),
            producer.to_string_lossy().into(),
        ];
        let mut cases = vec![(good.clone(), true)];
        let mut wrong_run = good.clone();
        wrong_run["repositories"][0]["run_id"] = json!("unrequested");
        cases.push((wrong_run, false));
        let mut wrong_missing = good.clone();
        wrong_missing["unavailable"][0]["repo_path"] = json!("/unrequested");
        cases.push((wrong_missing, false));
        let mut duplicate = good.clone();
        duplicate["repositories"]
            .as_array_mut()
            .unwrap()
            .push(good["repositories"][0].clone());
        cases.push((duplicate, false));
        let mut authority = good.clone();
        authority["repositories"][0]["implementation_allowed"] = json!(true);
        cases.push((authority, false));
        let mut coverage = good.clone();
        coverage["coverage"]["reported"] = json!(2);
        cases.push((coverage, false));
        let mut omitted = good.clone();
        omitted["unavailable"] = json!([]);
        cases.push((omitted, false));
        for (index, (payload, accepted)) in cases.into_iter().enumerate() {
            std::fs::write(
                &producer,
                format!("#!/bin/sh\ncat <<'AUDIT_JSON'\n{payload}\nAUDIT_JSON\nexit 1\n"),
            )
            .unwrap();
            std::fs::set_permissions(&producer, std::fs::Permissions::from_mode(0o700)).unwrap();
            let result = super::run_model_audit_cli(&arguments);
            assert_eq!(result.is_ok(), accepted, "fleet case {index}: {result:?}");
            if accepted {
                assert_eq!(result.unwrap(), (payload, 1));
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_audit_bridge_rejects_wrong_scope_schema_and_mutation_authority() {
        let schema = "quality-runner-model-audit-report/v1";
        let good = json!({"schema":schema,"read_only":true,"implementation_allowed":false,
                         "status":"reported","repo_root":"/repo","run_id":"a"});
        assert!(validate_model_audit_projection(&good, schema, Some("/repo"), Some("a")).is_ok());
        for (field, value) in [
            ("schema", json!("future")),
            ("repo_root", json!("/other")),
            ("run_id", json!("b")),
            ("read_only", json!(false)),
            ("implementation_allowed", json!(true)),
            ("status", json!("ready")),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(
                validate_model_audit_projection(&bad, schema, Some("/repo"), Some("a")).is_err()
            );
        }
    }

    #[test]
    fn model_audit_bridge_preserves_partial_fleet_coverage() {
        let schema = "quality-runner-model-audit-fleet/v1";
        let partial = json!({"schema":schema,"read_only":true,"implementation_allowed":false,
                            "status":"partial","coverage":{"selected":2,"reported":1,"unavailable":1}});
        assert!(validate_model_audit_projection(&partial, schema, None, None).is_ok());
    }
}
