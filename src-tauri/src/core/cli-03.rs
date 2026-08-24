#[allow(unused_variables)]
fn run_cli_arm_03(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let result = (|| -> Result<Value, String> {
        let _ = cli_positionals(&arguments, &["--role-map"])?;
        let role_map_path = cli_option(&arguments, "--role-map")?.ok_or_else(|| {
            "workspace-manifest requires --role-map <path|@json>".to_string()
        })?;
        let source = role_map_path.strip_prefix('@').unwrap_or(&role_map_path);
        let content = fs::read_to_string(source)
            .map_err(|error| format!("Could not read --role-map file: {error}"))?;
        let role_map: Value = serde_json::from_str(&content)
            .map_err(|error| format!("--role-map must contain valid JSON: {error}"))?;
        let state = load_store_read_only(&path)?;
        workspace_fleet_manifest(&state, &role_map)
    })();
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => {
            let repositories = report
                .get("repositories")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let production = repositories
                .iter()
                .filter(|entry| {
                    entry
                        .get("policy")
                        .and_then(|policy| policy.get("repository_role"))
                        .and_then(Value::as_str)
                        == Some("production_product")
                })
                .count();
            let supporting = repositories
                .iter()
                .filter(|entry| {
                    entry
                        .get("policy")
                        .and_then(|policy| policy.get("repository_role"))
                        .and_then(Value::as_str)
                        == Some("supporting_project")
                })
                .count();
            let unresolved = repositories.len().saturating_sub(production + supporting);
            println!(
                "PRONTO WORKSPACE MANIFEST · {} repositories · {} production · {} supporting · {} unresolved",
                repositories.len(), production, supporting, unresolved
            );
            println!("Baseline: {}P + {}N", production, supporting);
            println!("Use --json for the workspace-fleet-manifest/v1 payload.");
        }
        Err(error) => {
            if json {
                print_cli_json_error("workspace-manifest", &error);
            }
            eprintln!("Pronto could not generate workspace manifest: {error}");
            std::process::exit(1);
        }
    }
}
