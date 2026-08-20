#[allow(unused_variables)]
fn run_cli_arm_30(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let parallelism = cli_positive_usize_option(&arguments, "--parallelism")
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        })
        .unwrap_or(DEFAULT_REFRESH_BATCH_PARALLELISM);
    if parallelism > MAX_REFRESH_BATCH_PARALLELISM {
        eprintln!(
            "Pronto CLI error: --parallelism must be between 1 and {MAX_REFRESH_BATCH_PARALLELISM}"
        );
        std::process::exit(2);
    }
    let positionals =
        cli_positionals(&arguments, &["--parallelism"]).unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto refresh-batch [repository|group|product|repository-path] [--parallelism <positive-integer>] [--json]"
        );
        std::process::exit(2);
    }
    let result =
        refresh_batch_at(&path, positionals.first().map(String::as_str), parallelism);
    match result {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => print_human_refresh_batch(&report),
        Err(error) => {
            if json {
                print_cli_json_error("refresh-batch", &error);
            }
            eprintln!("Pronto could not run parallel refresh: {error}");
            std::process::exit(1);
        }
    }
}
