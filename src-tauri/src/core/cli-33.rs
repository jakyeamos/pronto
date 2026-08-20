#[allow(unused_variables)]
fn run_cli_arm_33(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(
        &arguments,
        &["--ignore", "--refresh-policy", "--background-monitoring"],
    )
    .unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() == 2 && positionals[0] == "settings" {
        let refresh_policy = cli_option(&arguments, "--refresh-policy")
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            })
            .unwrap_or_else(|| {
                eprintln!("Usage: pronto root settings <root-id> [--ignore <pattern>]... --refresh-policy <policy> --background-monitoring <bool> [--json]");
                std::process::exit(2);
            });
        let background_monitoring = cli_bool_option(&arguments, "--background-monitoring")
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            })
            .unwrap_or_else(|| {
                eprintln!("Usage: pronto root settings <root-id> [--ignore <pattern>]... --refresh-policy <policy> --background-monitoring <bool> [--json]");
                std::process::exit(2);
            });
        let ignore_patterns =
            cli_repeated_option(&arguments, "--ignore").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
        match update_root_settings_at(
            &path,
            &positionals[1],
            ignore_patterns,
            &refresh_policy,
            background_monitoring,
        ) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(snapshot) => print_human_status(&snapshot),
            Err(error) => {
                eprintln!("Pronto could not update root settings: {error}");
                std::process::exit(1);
            }
        }
    } else if positionals.len() == 2 && positionals[0] == "add" {
        let root_path = &positionals[1];
        match register_root_and_scan(&path, root_path) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(snapshot) => {
                println!("Configured discovery root: {root_path}");
                print_human_status(&snapshot);
            }
            Err(error) => {
                eprintln!("Pronto could not configure the discovery root: {error}");
                std::process::exit(1);
            }
        }
    } else if positionals.len() >= 3 && positionals[0] == "exclude" {
        let root_path = &positionals[1];
        let patterns = positionals[2..].to_vec();
        match exclude_root_patterns_at(&path, root_path, patterns.clone()) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(snapshot) => {
                println!(
                    "Excluded {} from discovery under {}.",
                    patterns.join(", "),
                    root_path
                );
                print_human_status(&snapshot);
            }
            Err(error) => {
                eprintln!("Pronto could not exclude those discovery folders: {error}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!(
            "Usage: pronto root add <folder> [--json] | pronto root exclude <folder> <name>... [--json]"
        );
        std::process::exit(2);
    }
}
