#[allow(unused_variables)]
fn run_cli_arm_15(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if !(1..=2).contains(&positionals.len()) {
        eprintln!("Usage: pronto preflight <action> [<repository>] [--json]");
        std::process::exit(2);
    }
    let result = load_store(&path).and_then(|state| {
        let repository_id = positionals
            .get(1)
            .map(|query| {
                let snapshot = snapshot_from_store(&path, &state);
                find_cli_repository(&snapshot, query)
                    .map(|repository| repository.id.clone())
            })
            .transpose()?;
        preflight_action_at(&path, &positionals[0], repository_id.as_deref())
    });
    match result {
        Ok(preflight) if json => println!(
            "{}",
            serde_json::to_string_pretty(&preflight).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(preflight) => println!(
            "PRONTO PREFLIGHT · {} · {}",
            preflight.target_label,
            if preflight.allowed {
                "allowed"
            } else {
                "blocked"
            }
        ),
        Err(error) => {
            eprintln!("Pronto could not preflight the action: {error}");
            std::process::exit(1);
        }
    }
}
