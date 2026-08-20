#[allow(unused_variables)]
fn run_cli_arm_13(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &["--repo", "--release-mode"])
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    match positionals.first().map(String::as_str) {
        Some("list") if positionals.len() == 1 => match load_store(&path) {
            Ok(state) if json => println!(
                "{}",
                serde_json::to_string_pretty(&state.products)
                    .unwrap_or_else(|_| "[]".to_string())
            ),
            Ok(state) => print_human_products(&state.products),
            Err(error) => {
                eprintln!("Pronto could not read products: {error}");
                std::process::exit(1);
            }
        },
        Some("create") if positionals.len() == 2 => {
            let release_mode = cli_option(&arguments, "--release-mode")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .unwrap_or_else(|| {
                    eprintln!("Usage: pronto product create <name> --release-mode <mode> [--repo <id>]... [--json]");
                    std::process::exit(2);
                });
            let repository_ids =
                cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            match upsert_product_at(
                &path,
                None,
                &positionals[1],
                repository_ids,
                &release_mode,
            )
            .map(|snapshot| snapshot.products)
            {
                Ok(products) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&products)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(products) => print_human_products(&products),
                Err(error) => {
                    eprintln!("Pronto could not create product: {error}");
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
                eprintln!("Usage: pronto product append <product> --repo <id>... [--json]");
                std::process::exit(2);
            }
            let result = load_store(&path).and_then(|state| {
                let product = find_cli_product(&state, &positionals[1])?;
                let repository_ids =
                    merge_repository_ids(&product.repository_ids, repository_ids);
                upsert_product_at(
                    &path,
                    Some(&product.id),
                    &product.name,
                    repository_ids,
                    &product.release_mode,
                )
            });
            match result.map(|snapshot| snapshot.products) {
                Ok(products) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&products)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(products) => print_human_products(&products),
                Err(error) => {
                    eprintln!("Pronto could not append to product: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("update") if positionals.len() == 3 => {
            let release_mode = cli_option(&arguments, "--release-mode")
                .unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                })
                .unwrap_or_else(|| {
                    eprintln!("Usage: pronto product update <product> <name> --release-mode <mode> [--repo <id>]... [--json]");
                    std::process::exit(2);
                });
            let repository_ids =
                cli_repeated_option(&arguments, "--repo").unwrap_or_else(|error| {
                    eprintln!("Pronto CLI error: {error}");
                    std::process::exit(2);
                });
            let result = load_store(&path).and_then(|state| {
                let product = find_cli_product(&state, &positionals[1])?;
                upsert_product_at(
                    &path,
                    Some(&product.id),
                    &positionals[2],
                    repository_ids,
                    &release_mode,
                )
            });
            match result.map(|snapshot| snapshot.products) {
                Ok(products) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&products)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(products) => print_human_products(&products),
                Err(error) => {
                    eprintln!("Pronto could not update product: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("delete") if positionals.len() == 2 => {
            let result = load_store(&path)
                .and_then(|state| {
                    find_cli_product(&state, &positionals[1])
                        .map(|product| product.id.clone())
                })
                .and_then(|product_id| delete_product_at(&path, &product_id))
                .map(|snapshot| snapshot.products);
            match result {
                Ok(products) if json => println!(
                    "{}",
                    serde_json::to_string_pretty(&products)
                        .unwrap_or_else(|_| "[]".to_string())
                ),
                Ok(products) => print_human_products(&products),
                Err(error) => {
                    eprintln!("Pronto could not delete product: {error}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: pronto product list [--json] | pronto product create <name> --release-mode <mode> [--repo <id>]... [--json] | pronto product append <product> --repo <id>... [--json] | pronto product update <product> <name> --release-mode <mode> [--repo <id>]... [--json] | pronto product delete <product> [--json]");
            std::process::exit(2);
        }
    }
}
