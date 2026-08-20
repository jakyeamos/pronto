#[allow(unused_variables)]
fn run_cli_arm_08(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let operation = cli_option(&arguments, "--operation").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if operation
        .as_deref()
        .is_some_and(|value| !matches!(value, "add" | "change" | "remove"))
    {
        eprintln!("Pronto CLI error: --operation must be add, change, or remove");
        std::process::exit(2);
    }
    let positionals =
        cli_positionals(&arguments, &["--operation"]).unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if positionals.len() != 2 || !matches!(positionals[0].as_str(), "repo" | "skill") {
        eprintln!("Usage: pronto change-matrix repo <repository> [--operation <add|change|remove>] [--json] | pronto change-matrix skill <skill-id> [--operation <add|change|remove>] [--json]");
        std::process::exit(2);
    }
    let report = if positionals[0] == "repo" {
        let state = load_store_read_only(&path).unwrap_or_else(|error| {
            eprintln!("Pronto could not read repository state: {error}");
            std::process::exit(1);
        });
        let snapshot = snapshot_from_store(&path, &state);
        let repository =
            find_cli_repository(&snapshot, &positionals[1]).unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(1);
            });
        change_matrix::inspect_repository(
            Path::new(&repository.path),
            &repository.id,
            repository.remote_url.as_deref(),
            operation.as_deref(),
        )
    } else {
        let snapshot = skills::load(&path).unwrap_or_else(|error| {
            eprintln!("Pronto could not read skills: {error}");
            std::process::exit(1);
        });
        let matches = snapshot
            .skills
            .iter()
            .filter(|skill| {
                skill.id == positionals[1]
                    || skill.name.eq_ignore_ascii_case(&positionals[1])
            })
            .collect::<Vec<_>>();
        let skill = match matches.as_slice() {
            [skill] => *skill,
            [] => {
                eprintln!("Pronto could not find skill: {}", positionals[1]);
                std::process::exit(1);
            }
            _ => {
                eprintln!("Pronto skill query is ambiguous: {}", positionals[1]);
                std::process::exit(1);
            }
        };
        change_matrix::inspect_skill(skill, operation.as_deref())
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "CHANGE MATRIX · {} · {} · {}",
            report.subject_kind, report.subject_id, report.status
        );
        println!("{}", report.maturity_impact);
        println!("Expected: {}", report.expected_contract_location);
    }
}
