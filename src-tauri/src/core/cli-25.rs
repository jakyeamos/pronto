#[allow(unused_variables)]
fn run_cli_arm_25(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let limit = cli_option(&arguments, "--limit")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| {
            value.parse::<usize>().unwrap_or_else(|_| {
                eprintln!("Pronto CLI error: --limit must be a non-negative integer");
                std::process::exit(2);
            })
        })
        .unwrap_or(24);
    let positionals = cli_positionals(&arguments, &["--limit"]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() > 1 {
        eprintln!("Usage: pronto activity [<repository>] [--limit <n>] [--json]");
        std::process::exit(2);
    }
    let query = positionals.first().map(String::as_str);
    let result = load_store_read_only(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| agent_activity_report(&snapshot, query, limit));
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => print_human_activity(&report),
        Err(error) => {
            eprintln!("Pronto could not read activity: {error}");
            std::process::exit(1);
        }
    }
}
