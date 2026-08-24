#[allow(unused_variables)]
fn run_cli_arm_12(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(&arguments, &["--range-days", "--config-json"])
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if positionals.first().map(String::as_str) == Some("view") {
        let result = match positionals.get(1).map(String::as_str) {
            Some("list") if positionals.len() == 2 => open_store(&path).and_then(|connection| load_analytics_views(&connection, ANALYTICS_RANGE_DAYS)),
            Some("save") if positionals.len() == 2 => cli_json_option::<AnalyticsView>(&arguments, "--config-json").and_then(|view| view.ok_or_else(|| "analytics view save requires --config-json <json|@file>".to_string())).and_then(|view| save_analytics_view_at(&path, view)),
            Some("delete") if positionals.len() == 3 && positionals[2] != "curated" => open_store(&path).and_then(|connection| { connection.execute("DELETE FROM analytics_views WHERE id = ?1", params![positionals[2]]).map_err(|error| format!("Could not delete analytics view: {error}"))?; load_analytics_views(&connection, ANALYTICS_RANGE_DAYS) }),
            Some("default") if positionals.len() == 3 => set_default_analytics_view_at(&path, &positionals[2]),
            _ => Err("Usage: pronto analytics view list|save --config-json <json|@file>|delete <id>|default <id> [--json]".to_string()),
        };
        match result {
            Ok(views) if json => println!(
                "{}",
                serde_json::to_string_pretty(&views).unwrap_or_else(|_| "[]".to_string())
            ),
            Ok(views) => {
                for view in views {
                    println!(
                        "{} · {}{}",
                        view.id,
                        view.name,
                        if view.is_default { " · default" } else { "" }
                    );
                }
            }
            Err(error) => {
                eprintln!("Pronto could not manage analytics views: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if !positionals.is_empty() {
        eprintln!("Usage: pronto analytics [--range-days <days>] [--json]");
        std::process::exit(2);
    }
    let range_days = cli_positive_u64_option(&arguments, "--range-days")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .map(|value| value as i64);
    match load_analytics_at(&path, range_days) {
        Ok(analytics) if json => println!(
            "{}",
            serde_json::to_string_pretty(&analytics).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(analytics) => println!(
            "PRONTO ANALYTICS · {} portfolio samples · {} repository series · {}",
            analytics.portfolio_samples.len(),
            analytics.repositories.len(),
            analytics.freshness
        ),
        Err(error) => {
            eprintln!("Pronto could not read analytics: {error}");
            std::process::exit(1);
        }
    }
}
