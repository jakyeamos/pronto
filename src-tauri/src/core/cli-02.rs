#[allow(unused_variables)]
fn run_cli_arm_02(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let write = arguments.iter().any(|argument| argument == "--write");
    let replace = arguments.iter().any(|argument| argument == "--replace");
    let result = (|| -> Result<(Value, bool), String> {
        let positionals = cli_positionals_with_flags(
            &arguments,
            &["--role-map", "--repository"],
            &["--write", "--replace"],
        )?;
        if positionals.len() != 1 || positionals[0] != "generate" {
            return Err(
                "Usage: pronto workspace-policy generate --role-map <path|@json> [--repository <id|path|name>] [--write] [--replace] [--json]".to_string(),
            );
        }
        let role_map_path = cli_option(&arguments, "--role-map")?.ok_or_else(|| {
            "workspace-policy generate requires --role-map <path|@json>".to_string()
        })?;
        let source = role_map_path.strip_prefix('@').unwrap_or(&role_map_path);
        let content = fs::read_to_string(source)
            .map_err(|error| format!("Could not read --role-map file: {error}"))?;
        let role_map: Value = serde_json::from_str(&content)
            .map_err(|error| format!("--role-map must contain valid JSON: {error}"))?;
        let repository_query = cli_option(&arguments, "--repository")?;
        let state = load_store_read_only(&path)?;
        workspace_policy_generation(
            &state,
            &role_map,
            repository_query.as_deref(),
            write,
            replace,
        )
    })();
    match result {
        Ok((report, blocked)) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "PRONTO WORKSPACE POLICY · {} · {} repositories",
                    report
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    report
                        .get("repository_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                );
                if let Some(counts) = report.get("counts").and_then(Value::as_object) {
                    for (status, count) in counts {
                        println!("  {status}: {}", count.as_u64().unwrap_or_default());
                    }
                }
                if let Some(next) = report.get("next_safe_step").and_then(Value::as_str) {
                    println!("Next: {next}");
                }
            }
            if blocked {
                std::process::exit(1);
            }
        }
        Err(error) => {
            if json {
                print_cli_json_error("workspace-policy generate", &error);
            }
            eprintln!("Pronto could not generate workspace policies: {error}");
            std::process::exit(1);
        }
    }
}
