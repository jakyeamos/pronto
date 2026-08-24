#[allow(unused_variables)]
fn run_cli_arm_19(
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
    let positionals = cli_positionals(&arguments, &["--product", "--group"])
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if !positionals.is_empty() {
        eprintln!("Usage: pronto summary [--product <name> | --group <name>] [--json]");
        std::process::exit(2);
    }
    let scope = product_name
        .as_deref()
        .map(|value| format!("product:{value}"))
        .or_else(|| group_name.as_deref().map(|value| format!("group:{value}")))
        .unwrap_or_else(|| "fleet".to_string());
    let result = load_store_read_only(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            filter_snapshot_by_collection(
                snapshot,
                product_name.as_deref(),
                group_name.as_deref(),
            )
        })
        .map(|snapshot| agent_summary(&snapshot, &scope));
    match result {
        Ok(summary) if json => println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(summary) => print_human_summary(&summary),
        Err(error) => {
            eprintln!("Pronto could not read local state: {error}");
            std::process::exit(1);
        }
    }
}
