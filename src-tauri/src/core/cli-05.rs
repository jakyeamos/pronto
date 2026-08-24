#[allow(unused_variables)]
fn run_cli_arm_05(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.first().map(String::as_str) == Some("open") && positionals.len() == 2 {
        match skills::open_source(&positionals[1]) {
            Ok(()) => println!("Opened skill source: {}", positionals[1]),
            Err(error) => {
                eprintln!("Pronto could not open skill source: {error}");
                std::process::exit(1);
            }
        }
    } else if positionals.len() > 1 {
        eprintln!("Usage: pronto skills [<skill-id>] [--json]");
        std::process::exit(2);
    }
    match skills::load(&path) {
        Ok(mut snapshot) if json => {
            if let Some(query) = positionals.first() {
                snapshot.skills.retain(|skill| {
                    skill.id == *query || skill.name.eq_ignore_ascii_case(query)
                });
                if snapshot.skills.is_empty() {
                    eprintln!("Pronto could not find skill: {query}");
                    std::process::exit(1);
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".into())
            )
        }
        Ok(snapshot) => {
            if let Some(query) = positionals.first() {
                let Some(skill) = snapshot.skills.iter().find(|skill| {
                    skill.id == *query || skill.name.eq_ignore_ascii_case(query)
                }) else {
                    eprintln!("Pronto could not find skill: {query}");
                    std::process::exit(1);
                };
                println!("PRONTO SKILL · {}", skill.name);
                println!("Description: {}", skill.description);
                println!("Category: {} · Family: {}", skill.category, skill.family);
                println!("Lifecycle: {}", skill.lifecycle);
                if skill.usage.state == "observed" {
                    println!(
                        "Usage: {} recent · {} all-time",
                        skill.usage.recent_count, skill.usage.all_time_count
                    );
                } else {
                    println!("Usage: unavailable");
                    println!("Usage evidence: {}", skill.usage.reason);
                }
            } else {
                println!(
                    "PRONTO SKILLS · {} skills · {}",
                    snapshot.skills.len(),
                    snapshot.freshness
                );
            }
        }
        Err(error) => {
            eprintln!("Pronto could not read skills: {error}");
            std::process::exit(1);
        }
    }
}
