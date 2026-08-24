#[allow(unused_variables)]
fn run_cli_arm_31(
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
        eprintln!("Usage: pronto refresh-github [repository|group|product] [--json]");
        std::process::exit(2);
    }
    let result = if let Some(target) = positionals.first() {
        load_store(&path).and_then(|state| {
            let current = snapshot_from_store(&path, &state);
            let (repository_ids, _) = resolve_refresh_target(&current, target)?;
            refresh_github_scoped_at(&path, &repository_ids).map(|snapshot| {
                filter_snapshot_to_repository_ids(snapshot, &repository_ids)
            })
        })
    } else {
        refresh_github_at(&path)
    };
    match result {
        Ok(snapshot) if json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            );
            if snapshot.provider_status.state != "Ready" {
                std::process::exit(1);
            }
        }
        Ok(snapshot) if snapshot.provider_status.state == "Ready" => {
            println!(
                "GitHub provider: {} · {}",
                snapshot.provider_status.state, snapshot.provider_status.message
            );
            print_human_status(&snapshot);
        }
        Ok(snapshot) => {
            eprintln!(
                "Pronto GitHub refresh unavailable: {}",
                snapshot.provider_status.message
            );
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("Pronto could not refresh GitHub: {error}");
            std::process::exit(1);
        }
    }
}
