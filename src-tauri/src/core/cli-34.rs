#[allow(unused_variables)]
fn run_cli_arm_34(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let Some(remote) = positionals.first() else {
        eprintln!("Usage: pronto clone <owner/repository> [--json]");
        std::process::exit(2);
    };
    if positionals.len() > 1 {
        eprintln!("Usage: pronto clone <owner/repository> [--json]");
        std::process::exit(2);
    }
    let result = preflight_action_at(&path, "clone", None);
    match result {
        Ok(preflight) if json => println!(
            "{}",
            serde_json::to_string_pretty(&preflight)
                .unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(_) => println!(
            "Clone of {remote} is blocked by the current local-only action policy; use the desktop confirmation flow when provider access is enabled."
        ),
        Err(error) => {
            eprintln!("Pronto could not record the clone boundary: {error}");
            std::process::exit(1);
        }
    }
}
