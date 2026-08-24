#[allow(unused_variables)]
fn run_cli_arm_23(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let fresh = arguments.iter().any(|argument| argument == "--fresh");
    let positionals = cli_positionals_with_flags(
        &arguments,
        &["--rule-json", "--recipe-json", "--workspace"],
        &["--clear", "--fresh"],
    )
    .unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.first().map(String::as_str) == Some("set-target") {
        if positionals.len() != 3 {
            eprintln!("Usage: pronto repo set-target <repository> <branch> [--json]");
            std::process::exit(2);
        }
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            set_repository_target_branch_at(&path, &repository.id, &positionals[2])
        });
        match result {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("Repository target branch updated."),
            Err(error) => {
                eprintln!("Pronto could not update the repository target branch: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if positionals.first().map(String::as_str) == Some("set-lifecycle")
        && positionals.len() == 3
    {
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            set_repository_lifecycle_at(&path, &repository.id, &positionals[2])
        });
        match result {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("Repository lifecycle updated."),
            Err(error) => {
                eprintln!("Pronto could not update repository lifecycle: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if positionals.first().map(String::as_str) == Some("set-release-rule")
        && positionals.len() == 2
    {
        let release_rule = if arguments.iter().any(|argument| argument == "--clear") {
            None
        } else {
            cli_json_option::<ReleaseRuleConfig>(&arguments, "--rule-json").unwrap_or_else(
                |error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                },
            )
        };
        if release_rule.is_none() && !arguments.iter().any(|argument| argument == "--clear")
        {
            eprintln!("Usage: pronto repo set-release-rule <repository> --rule-json <json|@file> [--json]");
            std::process::exit(2);
        }
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            set_release_rule_at(&path, &repository.id, release_rule)
        });
        match result {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("Release rule updated."),
            Err(error) => {
                eprintln!("Pronto could not update the release rule: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if positionals.first().map(String::as_str) == Some("set-release-recipe")
        && positionals.len() == 2
    {
        let release_recipe = if arguments.iter().any(|argument| argument == "--clear") {
            None
        } else {
            cli_json_option::<ReleaseRecipeConfig>(&arguments, "--recipe-json")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
        };
        if release_recipe.is_none()
            && !arguments.iter().any(|argument| argument == "--clear")
        {
            eprintln!("Usage: pronto repo set-release-recipe <repository> --recipe-json <json|@file> [--json]");
            std::process::exit(2);
        }
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            set_release_recipe_at(&path, &repository.id, release_recipe)
        });
        match result {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("Release recipe updated."),
            Err(error) => {
                eprintln!("Pronto could not update the release recipe: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if positionals.first().map(String::as_str) == Some("set-release-version")
        && (positionals.len() == 2 || positionals.len() == 3)
    {
        let release_version = if arguments.iter().any(|argument| argument == "--clear") {
            None
        } else {
            positionals.get(2).cloned()
        };
        if release_version.is_none()
            && !arguments.iter().any(|argument| argument == "--clear")
        {
            eprintln!("Usage: pronto repo set-release-version <repository> <version> [--json] | ... <repository> --clear [--json]");
            std::process::exit(2);
        }
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            set_release_version_at(&path, &repository.id, release_version)
        });
        match result {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("Release version updated."),
            Err(error) => {
                eprintln!("Pronto could not update the release version: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if positionals.first().map(String::as_str) == Some("set-ai-permission")
        && positionals.len() == 3
    {
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            set_ai_permission_at(&path, &repository.id, &positionals[2])
        });
        match result {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("AI permission updated."),
            Err(error) => {
                eprintln!("Pronto could not update AI permission: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if positionals.first().map(String::as_str) == Some("preview-ai-summary")
        && positionals.len() == 2
    {
        let workspace_id = cli_option(&arguments, "--workspace").unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
        let result = load_store(&path).and_then(|state| {
            let snapshot = snapshot_from_store(&path, &state);
            let repository = find_cli_repository(&snapshot, &positionals[1])?;
            preview_ai_summary_at(&path, &repository.id, workspace_id.as_deref())
        });
        match result {
            Ok(preview) if json => println!(
                "{}",
                serde_json::to_string_pretty(&preview).unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(preview) => {
                println!("AI summary: {} · {}", preview.status, preview.payload_bytes)
            }
            Err(error) => {
                eprintln!("Pronto could not preview the AI summary: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    let Some(query) = positionals.first() else {
        eprintln!("Usage: pronto repo <repository> [--fresh] [--json] | pronto repo set-target <repository> <branch> [--json]");
        std::process::exit(2);
    };
    if positionals.len() > 1 {
        eprintln!("Usage: pronto repo <repository> [--fresh] [--json] | pronto repo set-target <repository> <branch> [--json]");
        std::process::exit(2);
    }
    let state_result = if fresh {
        load_store_read_only_with_quality_bounded(&path)
    } else {
        load_store_read_only(&path)
    };
    let result = state_result
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            let repository = find_cli_repository(&snapshot, query)?;
            Ok(agent_repository_detail(&snapshot, repository))
        });
    match result {
        Ok(detail) if json => println!(
            "{}",
            serde_json::to_string_pretty(&detail).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(detail) => print_human_repository(&detail),
        Err(error) => {
            if json {
                print_cli_json_error("repo", &error);
            }
            eprintln!("Pronto could not read repository state: {error}");
            std::process::exit(1);
        }
    }
}
