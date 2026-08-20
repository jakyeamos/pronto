#[allow(unused_variables)]
fn run_cli_arm_09(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if !positionals.is_empty() {
        eprintln!("Usage: pronto refresh-skills [--json]");
        std::process::exit(2);
    }
    match skills::refresh(&path) {
        Ok(snapshot) if json => println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".into())
        ),
        Ok(snapshot) => {
            println!("PRONTO SKILLS · refreshed {} skills", snapshot.skills.len())
        }
        Err(error) => {
            eprintln!("Pronto could not refresh skills: {error}");
            std::process::exit(1);
        }
    }
}
