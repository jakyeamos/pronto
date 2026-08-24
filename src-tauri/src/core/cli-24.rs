#[allow(unused_variables)]
fn run_cli_arm_24(
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
        eprintln!("Usage: pronto attention [<repository>] [--json]");
        std::process::exit(2);
    }
    let query = positionals.first().map(String::as_str);
    match load_store_read_only(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| agent_attention_report_for_query(&snapshot, query))
    {
        Ok(report) if json => println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        ),
        Ok(report) => print_human_attention(&report),
        Err(error) => {
            eprintln!("Pronto could not read attention state: {error}");
            std::process::exit(1);
        }
    }
}
