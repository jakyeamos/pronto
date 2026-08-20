#[allow(unused_variables)]
fn run_cli_arm_14(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let tool = cli_option(&arguments, "--tool")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .unwrap_or_else(|| {
            eprintln!("Usage: pronto workspace open <repository> <workspace> --tool <tool> [--json]");
            std::process::exit(2);
        });
    let positionals = cli_positionals(&arguments, &["--tool"]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() != 3 || positionals[0] != "open" {
        eprintln!(
            "Usage: pronto workspace open <repository> <workspace> --tool <tool> [--json]"
        );
        std::process::exit(2);
    }
    let result = load_store(&path).and_then(|state| {
        let snapshot = snapshot_from_store(&path, &state);
        let repository = find_cli_repository(&snapshot, &positionals[1])?;
        let repository_id = repository.id.clone();
        open_workspace_at(&path, &repository_id, &positionals[2], &tool)
    });
    match result {
        Ok(snapshot) if json => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(_) => println!("Opened workspace {}.", positionals[2]),
        Err(error) => {
            eprintln!("Pronto could not open the workspace: {error}");
            std::process::exit(1);
        }
    }
}
