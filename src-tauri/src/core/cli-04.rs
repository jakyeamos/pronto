#[allow(unused_variables)]
fn run_cli_arm_04(
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
        eprintln!("Usage: pronto custody [<repository>] [--json]");
        std::process::exit(2);
    }
    let result = if let Some(query) = positionals.first() {
        let candidate = Path::new(query);
        if candidate.exists() {
            crate::custody::project(candidate)
        } else {
            load_store_read_only(&path)
                .map(|state| snapshot_from_store(&path, &state))
                .and_then(|snapshot| {
                    find_cli_repository(&snapshot, query)
                        .map(|repository| repository.path.clone())
                })
                .and_then(|repository| crate::custody::project(Path::new(&repository)))
        }
        .map(|snapshot| {
            serde_json::json!({
                "schema_version": "pronto-custody-cli/v1",
                "generated_at": iso_now(),
                "scope": "repository",
                "custody": snapshot
            })
        })
    } else {
        load_store_read_only(&path)
            .map(|state| snapshot_from_store(&path, &state))
            .and_then(|snapshot| {
                let mut repositories = Vec::new();
                for repository in snapshot.repositories {
                    repositories
                        .push(crate::custody::project(Path::new(&repository.path))?);
                }
                Ok(serde_json::json!({
                    "schema_version": "pronto-custody-cli/v1",
                    "generated_at": iso_now(),
                    "scope": "fleet",
                    "repositories": repositories
                }))
            })
    };
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => {
            if let Some(snapshot) = report.get("custody") {
                println!(
                    "PRONTO CUSTODY · {} · {}",
                    snapshot
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    snapshot
                        .get("repository")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                );
                println!(
                    "Lanes: {} · unleased worktrees: {} · overlaps: {}",
                    snapshot
                        .get("lanes")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len),
                    snapshot
                        .get("unleased_worktrees")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len),
                    snapshot
                        .get("overlaps")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len)
                );
                if let Some(next) = snapshot.get("next_safe_step").and_then(Value::as_str) {
                    println!("Next: {next}");
                }
            } else {
                println!("PRONTO CUSTODY · fleet projection");
                println!(
                    "Repositories: {}",
                    report
                        .get("repositories")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len)
                );
            }
        }
        Err(error) => {
            if json {
                print_cli_json_error("custody", &error);
            }
            eprintln!("Pronto could not project custody: {error}");
            std::process::exit(1);
        }
    }
}
