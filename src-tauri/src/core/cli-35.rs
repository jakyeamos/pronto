#[allow(unused_variables)]
fn run_cli_arm_35(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    let snapshot = load_store(&path)
        .map(|state| snapshot_from_store(&path, &state))
        .unwrap_or_else(|_| PortfolioSnapshot {
            roots: Vec::new(),
            repositories: Vec::new(),
            products: Vec::new(),
            groups: Vec::new(),
            events: Vec::new(),
            action_audits: Vec::new(),
            provider_identities: Vec::new(),
            remote_repositories: Vec::new(),
            provider_status: ProviderStatus::default(),
            quality: QualityPortfolioSnapshot::default(),
            remediation: remediation::empty_run(),
            showcase: ShowcasePortfolioSnapshot::default(),
            retention_days: DEFAULT_RETENTION_DAYS,
            generated_at: iso_now(),
            storage_path: path.to_string_lossy().to_string(),
        });
    let repository = find_repository_for_directory(
        &snapshot,
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );
    if let Err(error) = launch_desktop_focus(repository) {
        eprintln!("Pronto could not open the desktop app: {error}");
        std::process::exit(1);
    }
}
