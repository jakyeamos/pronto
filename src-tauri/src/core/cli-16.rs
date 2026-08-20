#[allow(unused_variables)]
fn run_cli_arm_16(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() != 3 || !matches!(positionals[0].as_str(), "expect" | "clear") {
        eprintln!("Usage: pronto condition expect|clear <repository> <condition> [--json]");
        std::process::exit(2);
    }
    let result = load_store(&path).and_then(|state| {
        let snapshot = snapshot_from_store(&path, &state);
        let repository = find_cli_repository(&snapshot, &positionals[1])?;
        if positionals[0] == "expect" {
            mutate_expected(&path, &repository.id, &positionals[2], true)
        } else {
            mutate_expected(&path, &repository.id, &positionals[2], false)
        }
    });
    match result {
        Ok(snapshot) if json => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(_) => println!(
            "Condition {}.",
            if positionals[0] == "expect" {
                "marked expected"
            } else {
                "cleared"
            }
        ),
        Err(error) => {
            eprintln!("Pronto could not update the condition: {error}");
            std::process::exit(1);
        }
    }
}
