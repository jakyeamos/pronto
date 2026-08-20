#[allow(unused_variables)]
fn run_cli_arm_18(
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
    let positionals =
        cli_positionals_with_flags(&arguments, &["--product", "--group"], &["--fresh"])
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
    if !positionals.is_empty() {
        eprintln!(
            "Usage: pronto status [--product <name> | --group <name>] [--fresh] [--json]"
        );
        std::process::exit(2);
    }
    let state_result = if fresh {
        load_store_read_only_with_quality_bounded(&path)
    } else {
        load_store_read_only(&path)
    };
    let result = state_result
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            filter_snapshot_by_collection(
                snapshot,
                product_name.as_deref(),
                group_name.as_deref(),
            )
        });
    match result {
        Ok(snapshot) if json => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(snapshot) => print_human_status(&snapshot),
        Err(error) => {
            if json {
                print_cli_json_error("status", &error);
            }
            eprintln!("Pronto could not read local state: {error}");
            std::process::exit(1);
        }
    }
}
