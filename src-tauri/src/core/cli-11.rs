#[allow(unused_variables)]
fn run_cli_arm_11(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &["--repo"]).unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    match positionals.first().map(String::as_str) {
        Some("list") if positionals.len() == 1 => match load_store(&path) {
            Ok(state) if json => println!(
                "{}",
                serde_json::to_string_pretty(&state.groups)
                    .unwrap_or_else(|_| "[]".to_string())
            ),
            Ok(state) => print_human_groups(&state.groups),
            Err(error) => {
                eprintln!("Pronto could not read groups: {error}");
                std::process::exit(1);
            }
        },
        Some("create") if positionals.len() == 2 => {
            let repository_ids =
                cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            match upsert_group_at(&path, None, &positionals[1], repository_ids)
                .map(|snapshot| snapshot.groups)
            {
                Ok(groups) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&groups)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(groups) => print_human_groups(&groups),
                Err(error) => {
                    eprintln!("Pronto could not create group: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("append") if positionals.len() == 2 => {
            let repository_ids =
                cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            if repository_ids.is_empty() {
                eprintln!("Usage: pronto group append <group> --repo <id>... [--json]");
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|state| {
                let group = find_cli_group(&state, &positionals[1])?;
                let repository_ids =
                    merge_repository_ids(&group.repository_ids, repository_ids);
                upsert_group_at(&path, Some(&group.id), &group.name, repository_ids)
            });
            match result.map(|snapshot| snapshot.groups) {
                Ok(groups) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&groups)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(groups) => print_human_groups(&groups),
                Err(error) => {
                    eprintln!("Pronto could not append to group: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("update") if positionals.len() == 3 => {
            let repository_ids =
                cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            let result = load_store(&path).and_then(|state| {
                let group = find_cli_group(&state, &positionals[1])?;
                upsert_group_at(&path, Some(&group.id), &positionals[2], repository_ids)
            });
            match result.map(|snapshot| snapshot.groups) {
                Ok(groups) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&groups)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(groups) => print_human_groups(&groups),
                Err(error) => {
                    eprintln!("Pronto could not update group: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("delete") if positionals.len() == 2 => {
            let result = load_store(&path)
                .and_then(|state| {
                    find_cli_group(&state, &positionals[1]).map(|group| group.id.clone())
                })
                .and_then(|group_id| delete_group_at(&path, &group_id))
                .map(|snapshot| snapshot.groups);
            match result {
                Ok(groups) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&groups)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(groups) => print_human_groups(&groups),
                Err(error) => {
                    eprintln!("Pronto could not delete group: {error}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: pronto group list [--json] | pronto group create <name> [--repo <id>]... [--json] | pronto group append <group> --repo <id>... [--json] | pronto group update <group> <name> [--repo <id>]... [--json] | pronto group delete <group> [--json]");
            std::process::exit(2);
        }
    }
}
