#[allow(unused_variables)]
fn run_cli_arm_38(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &[]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let (force_refresh, query) = match positionals.as_slice() {
        [query] => (false, query.as_str()),
        [operation, query] if operation == "refresh" => (true, query.as_str()),
        _ => {
            eprintln!("Usage: pronto telescope <repository> [--json] | pronto telescope refresh <repository> [--json]");
            std::process::exit(2);
        }
    };
    let result = load_store_read_only(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            find_cli_repository(&snapshot, query).map(|repository| repository.id.clone())
        })
        .and_then(|repository_id| {
            repository_telescope_at(&path, &repository_id, force_refresh, None)
        });
    match result {
        Ok(projection) if json => println!(
            "{}",
            serde_json::to_string_pretty(&projection).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(projection) => {
            println!(
                "PRONTO TELESCOPE · {} · {} nodes · {} relationships · {} flows",
                projection.repository_name,
                projection.nodes.len(),
                projection.edges.len(),
                projection.flows.len()
            );
            println!(
                "Workspace: {} · {} · {}",
                projection.binding.branch,
                projection
                    .binding
                    .commit
                    .as_deref()
                    .unwrap_or("unknown commit"),
                if projection.binding.dirty { "dirty" } else { "clean" }
            );
            println!(
                "Coverage: {} supported · {} partial · confidence {} · cache {}",
                projection.coverage.supported_source_files,
                projection.coverage.partial_source_files,
                projection.coverage.confidence,
                projection.freshness.cache
            );
        }
        Err(error) => {
            if json {
                print_cli_json_error("telescope", &error);
            }
            eprintln!("Pronto could not build Telescope: {error}");
            std::process::exit(1);
        }
    }
}
