#[allow(unused_variables)]
fn run_cli_arm_17(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() != 2 || positionals[0] != "retention" {
        eprintln!("Usage: pronto settings retention <days> [--json]");
        std::process::exit(2);
    }
    let retention_days = positionals[1].parse::<i64>().unwrap_or_else(|_| {
        eprintln!("Pronto CLI error: retention days must be an integer");
        std::process::exit(2);
    });
    match set_retention_days_at(&path, retention_days) {
        Ok(snapshot) if json => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(snapshot) => println!("Retention: {} days", snapshot.retention_days),
        Err(error) => {
            eprintln!("Pronto could not update settings: {error}");
            std::process::exit(1);
        }
    }
}
