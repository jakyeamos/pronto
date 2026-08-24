#[allow(unused_variables)]
fn run_cli_arm_36(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let Some(query) = positionals.first() else {
        eprintln!("Usage: pronto open <repository>");
        std::process::exit(2);
    };
    if positionals.len() > 1 {
        eprintln!("Usage: pronto open <repository>");
        std::process::exit(2);
    }
    let result = load_store(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            let repository = find_cli_repository(&snapshot, query)?;
            launch_desktop_focus(Some(repository))
        });
    if let Err(error) = result {
        eprintln!("Pronto could not open the desktop app: {error}");
        std::process::exit(1);
    }
}
