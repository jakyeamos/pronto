// Quality Runner owns model-audit evidence. This bridge neither invokes models
// nor copies findings into a second remediation store.
fn run_model_audit_cli(arguments: &[String]) -> Result<(serde_json::Value, i32), String> {
    let positionals = cli_positionals(arguments, &["--run-id", "--selection", "--qr-bin"])?;
    let action = positionals.first().map(String::as_str).unwrap_or("");
    let mut qr_arguments = vec!["model-audit".to_string(), action.to_string()];
    let expected_schema;
    let mut expected_repository = None;
    let mut expected_run_id = None;
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
    let partial = payload.get("status").and_then(serde_json::Value::as_str) == Some("partial");
    if !success && !partial {
        return Err("Quality Runner failed; no model-audit report was accepted".to_string());
    }
    Ok((payload, if partial { 1 } else { 0 }))
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
