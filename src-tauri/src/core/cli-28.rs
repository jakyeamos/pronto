#[allow(unused_variables)]
fn run_cli_arm_28(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals_with_flags(
        &arguments,
        &["--qr-bin", "--notes", "--timeout-seconds", "--workspace"],
        &["--dynamic", "--no-changed-only", "--skip-provider"],
    )
    .unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    match positionals.first().map(String::as_str) {
        Some("gate") => {
            if positionals.len() != 2 {
                eprintln!(
                    "Usage: pronto remediation gate <repository> [--workspace <id>] [--json]"
                );
                std::process::exit(2);
            }
            let workspace_id =
                cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            match remediation_execution_gate_at(
                &path,
                &positionals[1],
                workspace_id.as_deref(),
            ) {
                Ok(gate) if json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&gate)
                            .unwrap_or_else(|_| "{}".to_string())
                    );
                    if !gate.ready {
                        std::process::exit(1);
                    }
                }
                Ok(gate) => {
                    print_human_remediation_execution_gate(&gate);
                    if !gate.ready {
                        std::process::exit(1);
                    }
                }
                Err(error) => {
                    eprintln!("Pronto could not evaluate remediation execution: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("handoff-check") => {
            if positionals.len() != 2 {
                eprintln!(
                    "Usage: pronto remediation handoff-check <repository> [--workspace <id>] [--json]"
                );
                std::process::exit(2);
            }
            let workspace_id =
                cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            match remediation_handoff_check_at(
                &path,
                &positionals[1],
                workspace_id.as_deref(),
            ) {
                Ok(check) if json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&check)
                            .unwrap_or_else(|_| "{}".to_string())
                    );
                    if !check.ready {
                        std::process::exit(1);
                    }
                }
                Ok(check) => {
                    print_human_remediation_handoff_check(&check);
                    if !check.ready {
                        std::process::exit(1);
                    }
                }
                Err(error) => {
                    eprintln!("Pronto could not check the remediation handoff: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("refresh") => {
            if positionals.len() != 1 {
                eprintln!("Usage: pronto remediation refresh [--qr-bin <path>] [--dynamic] [--no-changed-only] [--timeout-seconds <positive-integer>] [--skip-provider] [--json]");
                std::process::exit(2);
            }
            let qr_bin = cli_option(&arguments, "--qr-bin").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            let timeout_seconds = cli_positive_u64_option(&arguments, "--timeout-seconds")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .unwrap_or(DEFAULT_QR_AUDIT_TIMEOUT_SECONDS);
            let result = refresh_remediation_at(
                &path,
                qr_bin.as_deref(),
                arguments.iter().any(|argument| argument == "--dynamic"),
                !arguments
                    .iter()
                    .any(|argument| argument == "--no-changed-only"),
                arguments
                    .iter()
                    .any(|argument| argument == "--skip-provider"),
                timeout_seconds,
            );
            match result {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot.remediation)
                        .unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(snapshot) => print_human_remediation(&snapshot.remediation),
                Err(error) => {
                    eprintln!("Pronto could not refresh remediation evidence: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("export") => {
            if positionals.len() > 2 {
                eprintln!("Usage: pronto remediation export [output-dir] [--json]");
                std::process::exit(2);
            }
            let output_dir = positionals.get(1).cloned();
            match export_remediation(output_dir) {
                Ok(export) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&export)
                        .unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(export) => println!(
                    "Remediation export: {} · {} files",
                    export.output_path,
                    export.files.len()
                ),
                Err(error) => {
                    eprintln!("Pronto could not export remediation plans: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("set-status") => {
            if positionals.len() != 3 {
                eprintln!("Usage: pronto remediation set-status <action-id> <status> [--notes <text>] [--json]");
                std::process::exit(2);
            }
            let notes = cli_option(&arguments, "--notes").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
            match set_remediation_action_status(
                positionals[1].clone(),
                positionals[2].clone(),
                notes,
            ) {
                Ok(snapshot) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot.remediation)
                        .unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(snapshot) => print_human_remediation(&snapshot.remediation),
                Err(error) => {
                    eprintln!("Pronto could not update remediation status: {error}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            if positionals.len() > 1 {
                eprintln!("Usage: pronto remediation [<repository>] [--json]");
                std::process::exit(2);
            }
            let result = load_store_read_only(&path).and_then(|state| {
                let snapshot = snapshot_from_store(&path, &state);
                if let Some(query) = positionals.first() {
                    let plan = snapshot
                        .remediation
                        .plans
                        .iter()
                        .find(|plan| {
                            plan.repository_id == *query
                                || plan.repository_name.eq_ignore_ascii_case(query)
                                || plan.repository_path == *query
                        })
                        .cloned();
                    let closures = snapshot
                        .remediation
                        .closures
                        .iter()
                        .filter(|closure| {
                            closure.repository_id == *query
                                || closure.repository_name.eq_ignore_ascii_case(query)
                                || closure.repository_path == *query
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    if plan.is_none() && closures.is_empty() {
                        return Err(format!(
                            "No active remediation plan or resolved remediation history found for repository '{query}'."
                        ));
                    }
                    let mut run = snapshot.remediation;
                    run.plans = plan.into_iter().collect();
                    run.closures = closures;
                    Ok(run)
                } else {
                    Ok(snapshot.remediation)
                }
            });
            match result {
                Ok(run) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&run).unwrap_or_else(|_| "{}".to_string())
                ),
                Ok(run) => print_human_remediation(&run),
                Err(error) => {
                    eprintln!("Pronto could not read remediation plans: {error}");
                    std::process::exit(1);
                }
            }
        }
    }
}
