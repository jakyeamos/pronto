#[allow(unused_variables)]
fn run_cli_arm_10(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let fresh = arguments.iter().any(|argument| argument == "--fresh");
    let product_name = cli_option(&arguments, "--product").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let group_name = cli_option(&arguments, "--group").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let max_age_minutes = cli_option(&arguments, "--max-age")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| {
            let parsed = value.parse::<i64>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: --max-age must be a non-negative integer");
                std::process::exit(2);
            });
            if parsed < 0 || parsed > MAX_AGENT_DOCTOR_MAX_AGE_MINUTES {
                eprintln!(
                    "Pronto CLI error: --max-age must be between 0 and {MAX_AGENT_DOCTOR_MAX_AGE_MINUTES}"
                );
                std::process::exit(2);
            }
            parsed
        })
        .unwrap_or(DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES);
    let limit = cli_option(&arguments, "--limit")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| {
            let parsed = value.parse::<usize>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                std::process::exit(2);
            });
            if parsed > MAX_AGENT_NEXT_LIMIT {
                eprintln!(
                    "Pronto CLI error: --limit must be {MAX_AGENT_NEXT_LIMIT} or less"
                );
                std::process::exit(2);
            }
            parsed
        })
        .unwrap_or(DEFAULT_AGENT_NEXT_LIMIT);
    let positionals = cli_positionals_with_flags(
        &arguments,
        &["--product", "--group", "--max-age", "--limit"],
        &["--fresh"],
    )
    .unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto route [<repository>] [--product <name> | --group <name>] [--max-age <minutes>] [--limit <n>] [--fresh] [--json]"
        );
        std::process::exit(2);
    }
    let query = positionals.first().map(String::as_str);
    let base_scope = product_name
        .as_deref()
        .map(|value| format!("product:{value}"))
        .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
        .unwrap_or_else(|| "fleet".to_string());
    let scope = query
        .map(|value| format!("{base_scope}; current_repository:{value}"))
        .unwrap_or(base_scope);
    let state_result = if fresh {
        load_store_read_only_with_quality_bounded(&path)
    } else {
        load_store_read_only(&path)
    };
    let report = match state_result {
        Ok(state) => {
            let snapshot = snapshot_from_store(&path, &state);
            let scoped_snapshot = filter_snapshot_by_collection(
                snapshot,
                product_name.as_deref(),
                group_name.as_deref(),
            )
            .and_then(|snapshot| {
                if let Some(query) = query {
                    let repository_id = find_cli_repository(&snapshot, query)?.id.clone();
                    let repository_ids = [repository_id].into_iter().collect();
                    Ok(filter_snapshot_to_repository_ids(snapshot, &repository_ids))
                } else {
                    Ok(snapshot)
                }
            });
            match scoped_snapshot {
                Ok(snapshot) => agent_route_report(
                    &snapshot,
                    &path,
                    max_age_minutes,
                    &scope,
                    query,
                    limit,
                )
                .unwrap_or_else(|error| {
                    agent_route_error_report(
                        &path,
                        max_age_minutes,
                        &scope,
                        "projection",
                        error,
                    )
                }),
                Err(error) => {
                    agent_route_error_report(&path, max_age_minutes, &scope, "scope", error)
                }
            }
        }
        Err(error) => {
            agent_route_error_report(&path, max_age_minutes, &scope, "storage", error)
        }
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        print_human_route(&report);
    }
    if !report.ready {
        std::process::exit(1);
    }
}
