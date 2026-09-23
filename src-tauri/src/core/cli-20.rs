#[allow(unused_variables)]
fn run_cli_arm_20(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
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
                eprintln!(
                    "Pronto CLI error: --max-age must be a non-negative integer"
                );
                std::process::exit(2);
            });
            if parsed < 0 {
                eprintln!(
                    "Pronto CLI error: --max-age must be a non-negative integer"
                );
                std::process::exit(2);
            }
            if parsed > MAX_AGENT_DOCTOR_MAX_AGE_MINUTES {
                eprintln!(
                    "Pronto CLI error: --max-age must be {MAX_AGENT_DOCTOR_MAX_AGE_MINUTES} or less"
                );
                std::process::exit(2);
            }
            parsed
        })
        .unwrap_or(DEFAULT_AGENT_DOCTOR_MAX_AGE_MINUTES);
    let max_change_commits = cli_option(&arguments, "--max-change-commits")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| {
            let parsed = value.parse::<u64>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: --max-change-commits must be a positive integer");
                std::process::exit(2);
            });
            if parsed == 0 || parsed > MAX_AGENT_DOCTOR_MAX_CHANGE_COMMITS {
                eprintln!(
                    "Pronto CLI error: --max-change-commits must be between 1 and {MAX_AGENT_DOCTOR_MAX_CHANGE_COMMITS}"
                );
                std::process::exit(2);
            }
            parsed
        })
        .unwrap_or(DEFAULT_AGENT_DOCTOR_MAX_CHANGE_COMMITS);
    let max_changed_paths = cli_option(&arguments, "--max-changed-paths")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| {
            let parsed = value.parse::<u64>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: --max-changed-paths must be a positive integer");
                std::process::exit(2);
            });
            if parsed == 0 || parsed > MAX_AGENT_DOCTOR_MAX_CHANGED_PATHS {
                eprintln!(
                    "Pronto CLI error: --max-changed-paths must be between 1 and {MAX_AGENT_DOCTOR_MAX_CHANGED_PATHS}"
                );
                std::process::exit(2);
            }
            parsed
        })
        .unwrap_or(DEFAULT_AGENT_DOCTOR_MAX_CHANGED_PATHS);
    let positionals = cli_positionals(
        &arguments,
        &[
            "--max-age",
            "--max-change-commits",
            "--max-changed-paths",
            "--product",
            "--group",
        ],
    )
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto doctor [<repository>] [--product <name> | --group <name>] [--max-age <minutes>] [--max-change-commits <n>] [--max-changed-paths <n>] [--json]"
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
    let report = match load_store_read_only(&path) {
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
                Ok(snapshot) => {
                    agent_doctor_report_with_thresholds(
                        &snapshot,
                        &path,
                        max_age_minutes,
                        max_change_commits,
                        max_changed_paths,
                        &scope,
                    )
                }
                Err(error) => agent_doctor_error_report_with_thresholds(
                    &path,
                    max_age_minutes,
                    max_change_commits,
                    max_changed_paths,
                    &scope,
                    "scope",
                    error,
                ),
            }
        }
        Err(error) => {
            agent_doctor_error_report_with_thresholds(
                &path,
                max_age_minutes,
                max_change_commits,
                max_changed_paths,
                &scope,
                "storage",
                error,
            )
        }
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        print_human_doctor(&report);
    }
    if !report.ready {
        std::process::exit(1);
    }
}
