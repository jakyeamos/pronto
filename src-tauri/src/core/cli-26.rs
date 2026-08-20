#[allow(unused_variables)]
fn run_cli_arm_26(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let fresh = arguments.iter().any(|argument| argument == "--fresh");
    let workspace_id = cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    let positionals =
        cli_positionals_with_flags(&arguments, &["--workspace"], &["--fresh"])
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
    let Some(query) = positionals.first() else {
        eprintln!(
            "Usage: pronto prepare <repository> [--workspace <id>] [--fresh] [--json]"
        );
        std::process::exit(2);
    };
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto prepare <repository> [--workspace <id>] [--fresh] [--json]"
        );
        std::process::exit(2);
    }
    let result =
        prepare_repository_by_query_at(&path, query, workspace_id.as_deref(), fresh).map(
            |preparation| AgentPreparationReport {
                schema_version: AGENT_PREPARATION_SCHEMA.to_string(),
                generated_at: preparation.generated_at.clone(),
                preparation,
            },
        );
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => print_human_preparation(&report),
        Err(error) => {
            eprintln!("Pronto could not prepare the repository: {error}");
            std::process::exit(1);
        }
    }
}
