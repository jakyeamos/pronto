#[allow(unused_variables)]
fn run_cli_arm_32(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let positionals = cli_positionals(
        &arguments,
        &[
            "--reason",
            "--reviewer",
            "--evidence",
            "--expires-at",
            "--qr-bin",
            "--timeout-seconds",
            "--agent-review-mode",
        ],
    )
    .unwrap_or_else(|error| {
        eprintln!("Pronto CLI error: {error}");
        std::process::exit(2);
    });
    if positionals.first().map(String::as_str) == Some("refresh") && positionals.len() == 1
    {
        match refresh_quality_at(&path) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(snapshot) => print_human_status(&snapshot),
            Err(error) => {
                if json {
                    print_cli_json_error("quality refresh", &error);
                }
                eprintln!("Pronto could not refresh quality evidence: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    } else if positionals.first().map(String::as_str) == Some("detector-refresh")
        && positionals.len() == 1
    {
        let qr_bin = cli_option(&arguments, "--qr-bin").unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
        let timeout_seconds = cli_positive_u64_option(&arguments, "--timeout-seconds")
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            })
            .unwrap_or(DEFAULT_FLEET_DETECTOR_TIMEOUT_SECONDS);
        let agent_review_mode = cli_option(&arguments, "--agent-review-mode")
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            })
            .unwrap_or_else(|| "off".to_string());
        let mut exit_code = 0;
        match refresh_quality_detectors_at(
            &path,
            qr_bin.as_deref(),
            timeout_seconds,
            &agent_review_mode,
        ) {
            Ok(report) => {
                if report.rejected_published_repositories > 0 {
                    exit_code = 1;
                }
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!(
                        "Detector refresh: {} · {} published reports ingested · {} rejected · {} of {} applicable repositories with findings evidence · {} missing · {} unsupported excluded",
                        report.status,
                        report.ingested_published_repositories,
                        report.rejected_published_repositories,
                        report.applicable_findings_evidence_repositories,
                        report.detector_applicable_repositories,
                        report.missing_findings_evidence_repositories,
                        report.detector_excluded_repositories
                    );
                }
            }
            Err(error) => {
                if json {
                    print_cli_json_error("quality detector-refresh", &error);
                }
                eprintln!("Pronto could not refresh detector evidence: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(exit_code);
    } else if positionals.first().map(String::as_str) == Some("set-audit-root")
        && positionals.len() == 2
    {
        match set_maturity_audit_root_at(&path, Some(&positionals[1])) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(snapshot) => print_human_status(&snapshot),
            Err(error) => {
                eprintln!("Pronto could not set the maturity audit root: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    } else if positionals.first().map(String::as_str) == Some("open-report")
        && positionals.len() == 2
    {
        match open_quality_report_at(&path, &positionals[1]) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(_) => println!("Opened quality report: {}", positionals[1]),
            Err(error) => {
                eprintln!("Pronto could not open the quality report: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    } else if positionals.first().map(String::as_str) == Some("disposition") {
        if positionals.get(1).map(String::as_str) != Some("set") || positionals.len() != 5 {
            eprintln!(
                "Usage: pronto quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] [--json]"
            );
            std::process::exit(2);
        }
        let reason = cli_option(&arguments, "--reason")
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            })
            .unwrap_or_else(|| {
                eprintln!("Pronto CLI error: --reason is required");
                std::process::exit(2);
            });
        let reviewer = cli_option(&arguments, "--reviewer")
            .unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            })
            .unwrap_or_else(|| {
                eprintln!("Pronto CLI error: --reviewer is required");
                std::process::exit(2);
            });
        let evidence =
            cli_repeated_option(&arguments, "--evidence").unwrap_or_else(|error| {
                eprintln!("Pronto CLI error: {error}");
                std::process::exit(2);
            });
        let expires_at = cli_option(&arguments, "--expires-at").unwrap_or_else(|error| {
            eprintln!("Pronto CLI error: {error}");
            std::process::exit(2);
        });
        let query = &positionals[2];
        let result = load_store(&path)
            .map(|state| snapshot_from_store(&path, &state))
            .and_then(|snapshot| {
                let repository = find_cli_repository(&snapshot, query)?;
                quality::set_finding_disposition(
                    Path::new(&repository.path),
                    &positionals[3],
                    &positionals[4],
                    &reason,
                    &reviewer,
                    evidence,
                    expires_at,
                )?;
                load_store_with_quality(&path)
                    .map(|state| snapshot_from_store(&path, &state))
                    .and_then(|snapshot| agent_quality_report(&snapshot, Some(query)))
            });
        match result {
            Ok(report) if json => println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(report) => print_human_quality(&report),
            Err(error) => {
                eprintln!("Pronto could not update the finding disposition: {error}");
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    let is_feed_command =
        positionals.len() == 1 && matches!(positionals[0].as_str(), "feed" | "audit-root");
    if is_feed_command {
        match load_store_read_only(&path).map(|state| snapshot_from_store(&path, &state)) {
            Ok(snapshot) if json => println!(
                "{}",
                serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(snapshot) => {
                println!(
                    "Maturity feed: {}",
                    snapshot
                        .quality
                        .audit_root
                        .as_deref()
                        .unwrap_or("Unavailable")
                );
                println!(
                    "Maturity feed: {} · {} matched · fleet mean {}",
                    snapshot.quality.audit_status,
                    snapshot.quality.matched_repository_count,
                    snapshot
                        .quality
                        .maturity_score_display
                        .as_deref()
                        .map(|value| format!("{value} / 4"))
                        .unwrap_or_else(|| "Not scored".to_string())
                );
            }
            Err(error) => {
                eprintln!("Pronto could not read the maturity feed: {error}");
                std::process::exit(1);
            }
        }
    } else if positionals.len() <= 1 {
        let query = positionals.first().map(String::as_str);
        let result = load_store_read_only(&path)
            .map(|state| snapshot_from_store(&path, &state))
            .and_then(|snapshot| agent_quality_report(&snapshot, query));
        match result {
            Ok(report) if json => println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            ),
            Ok(report) => print_human_quality(&report),
            Err(error) => {
                eprintln!("Pronto could not read quality state: {error}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!(
            "Usage: pronto quality [<repository>] [--json] | pronto quality refresh [--json] | pronto quality detector-refresh [--qr-bin <path>] [--timeout-seconds <positive-integer>] [--agent-review-mode <off|auto|parallel|required>] [--json] | pronto quality feed [--json] | pronto quality disposition set <repository> <fingerprint> <status> --reason <text> --reviewer <name> [--evidence <reference>]... [--expires-at <timestamp>] [--json] (Quality Runner owns detector evidence; Pronto owns the disposition overlay)"
        );
        std::process::exit(2);
    }
}
