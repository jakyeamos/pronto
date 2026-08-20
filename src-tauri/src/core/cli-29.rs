#[allow(unused_variables)]
fn run_cli_arm_29(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto refresh [repository|group|product|repository-path] [--json]"
        );
        std::process::exit(2);
    }
    let result = positionals
        .first()
        .map(|target| refresh_target_at(&path, target))
        .unwrap_or_else(|| refresh_at(&path));
    match result {
        Ok(snapshot) if json => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(snapshot) => print_human_status(&snapshot),
        Err(error) => {
            eprintln!("Pronto could not refresh local state: {error}");
            std::process::exit(1);
        }
    }
}
