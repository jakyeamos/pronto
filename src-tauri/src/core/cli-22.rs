#[allow(unused_variables)]
fn run_cli_arm_22(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    if arguments.get(1).map(String::as_str) != Some("preview") {
        eprintln!(
            "Usage: pronto fold preview [<repository>] [--target <branch>] [--product <name> | --group <name>] [--limit <n>] [--cursor <token>] [--json]"
        );
        std::process::exit(2);
    }
    let command_arguments = &arguments[1..];
    let target = cli_option(command_arguments, "--target").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if target
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        eprintln!("Pronto CLI error: --target requires a non-empty branch name");
        std::process::exit(2);
    }
    let product_name = cli_option(command_arguments, "--product").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let group_name = cli_option(command_arguments, "--group").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let limit = cli_option(command_arguments, "--limit")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| {
            let parsed = value.parse::<usize>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                std::process::exit(2);
            });
            if parsed > MAX_AGENT_FOLD_PREVIEW_LIMIT {
                eprintln!(
                    "Pronto CLI error: --limit must be {MAX_AGENT_FOLD_PREVIEW_LIMIT} or less"
                );
                std::process::exit(2);
            }
            if parsed == 0 {
                eprintln!("Pronto CLI error: --limit must be greater than zero");
                std::process::exit(2);
            }
            parsed
        })
        .unwrap_or(DEFAULT_AGENT_FOLD_PREVIEW_LIMIT);
    let cursor = cli_option(command_arguments, "--cursor").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if cursor
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        eprintln!("Pronto CLI error: --cursor requires a non-empty token");
        std::process::exit(2);
    }
    let positionals = cli_positionals(
        command_arguments,
        &["--target", "--product", "--group", "--limit", "--cursor"],
    )
    .unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto fold preview [<repository>] [--target <branch>] [--product <name> | --group <name>] [--limit <n>] [--cursor <token>] [--json]"
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
    let result = load_store_read_only(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            filter_snapshot_by_collection(
                snapshot,
                product_name.as_deref(),
                group_name.as_deref(),
            )
        })
        .and_then(|snapshot| {
            agent_fold_preview_report_with_cursor_and_merge_preview(
                &snapshot,
                query,
                target.as_deref(),
                &scope,
                limit,
                cursor.as_deref(),
                true,
            )
        });
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => print_human_fold_preview(&report),
        Err(error) => {
            eprintln!("Pronto could not read fold preview state: {error}");
            std::process::exit(1);
        }
    }
}
