#[allow(unused_variables)]
fn run_cli_arm_21(
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
    let positionals = cli_positionals(&arguments, &["--product", "--group", "--limit"])
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto next [<repository>] [--product <name> | --group <name>] [--limit <n>] [--json]"
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
        .and_then(|snapshot| agent_next_report(&snapshot, query, &scope, limit));
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => print_human_next(&report),
        Err(error) => {
            eprintln!("Pronto could not read next-step state: {error}");
            std::process::exit(1);
        }
    }
}
