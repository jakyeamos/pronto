#[allow(unused_variables)]
fn run_cli_arm_07(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let fresh = arguments.iter().any(|argument| argument == "--fresh");
    let filter = cli_option(&arguments, "--filter").unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if filter
        .as_deref()
        .is_some_and(|value| !crate::behavior_assurance::AUDIT_FILTERS.contains(&value))
    {
        eprintln!(
            "Pronto CLI error: --filter must be one of {}",
            crate::behavior_assurance::AUDIT_FILTERS.join(", ")
        );
        std::process::exit(2);
    }
    let positionals = cli_positionals_with_flags(&arguments, &["--filter"], &["--fresh"])
        .unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
    if positionals.len() > 1 {
        eprintln!(
            "Usage: pronto behavior [<repository>] [--filter <kind>] [--fresh] [--json]"
        );
        std::process::exit(2);
    }
    let state_result = if fresh {
        load_store_read_only_with_quality_bounded(&path)
    } else {
        load_store_read_only(&path)
    };
    let report = state_result
        .map(|state| snapshot_from_store(&path, &state))
        .and_then(|snapshot| {
            let repositories = if let Some(query) = positionals.first() {
                vec![find_cli_repository(&snapshot, query)?.clone()]
            } else {
                snapshot.repositories
            };
            Ok(crate::behavior_assurance::audit_report(
                &repositories,
                filter.as_deref(),
            ))
        })
        .unwrap_or_else(|error| {
            if json {
                print_cli_json_error("behavior", &error);
            }
            eprintln!("Pronto could not audit behavior assurance: {error}");
            std::process::exit(1);
        });
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "PRONTO BEHAVIOR ASSURANCE · {} · {}/{} selected repositories ready · {} gaps",
            report.status,
            report.ready_repository_count,
            report.repository_count,
            report.gap_count
        );
        println!(
            "Edge durability: {}/{} verified · {}/{} profiled · {} stale · {} failed · {} blocked · {} unknown",
            report.coverage.verified,
            report.coverage.total,
            report.coverage.profiled,
            report.coverage.total,
            report.coverage.stale,
            report.coverage.failed,
            report.coverage.blocked,
            report.coverage.unknown
        );
        for repository in &report.repositories {
            println!(
                "  {} · state {} · release {} · {}/{} Tier-0 · edge {}/{} verified · {}/{} profiled",
                repository.repository_name,
                repository.assurance.state,
                if repository.assurance.release_ready {
                    "Ready"
                } else {
                    "Gaps present"
                },
                repository.assurance.passed_scenario_count,
                repository.assurance.required_scenario_count,
                repository.assurance.coverage.counts.verified,
                repository.assurance.coverage.counts.total,
                repository.assurance.coverage.counts.profiled,
                repository.assurance.coverage.counts.total
            );
            for gap in repository.assurance.gaps.iter().take(5) {
                println!("    {} · {}", gap.kind, gap.message);
            }
        }
        println!("Next: {}", report.next_safe_step);
    }
    if !report.ready {
        std::process::exit(1);
    }
}
