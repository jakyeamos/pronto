use std::fs as radar_fs;
use std::sync::Mutex;

static RADAR_ENV_LOCK: Mutex<()> = Mutex::new(());

fn radar_repository(name: &str, remote_url: Option<&str>) -> RepositorySnapshot {
    let now = Utc::now().to_rfc3339();
    RepositorySnapshot {
        id: format!("repo-{name}"),
        name: name.to_string(),
        path: format!("/tmp/{name}"),
        locality: "Local".to_string(),
        lifecycle: "Active".to_string(),
        lifecycle_candidate: "Active".to_string(),
        remote_url: remote_url.map(str::to_string),
        provider_state: "GitHub connected".to_string(),
        branch: "main".to_string(),
        default_branch: Some("main".to_string()),
        target_branch: Some("main".to_string()),
        target_branch_configured: true,
        workspace: WorkspaceSummary {
            id: format!("workspace-{name}"),
            path: format!("/tmp/{name}"),
            is_primary: true,
            branch: "main".to_string(),
            status_available: true,
            status_error: None,
            dirty: false,
            added: 0,
            removed: 0,
            line_totals_partial: false,
            sync_state: "Synced".to_string(),
            remote_freshness: "Fresh".to_string(),
            ahead: 0,
            behind: 0,
            upstream: Some("origin/main".to_string()),
            operation: None,
            last_commit: Some("abc123".to_string()),
            last_commit_at: Some(now.clone()),
            last_activity_at: None,
            integration_state: "Ready".to_string(),
            target_branch: Some("main".to_string()),
            target_confidence: "High".to_string(),
            role: "Primary".to_string(),
            role_confidence: "High".to_string(),
            activity: WorkspaceActivity::default(),
            provenance: WorkspaceProvenance::default(),
            sync_detail: None,
        },
        workspaces: Vec::new(),
        branches: Vec::new(),
        branch_lifecycle: Default::default(),
        submodules: Vec::new(),
        pull_requests: Vec::new(),
        releases: Vec::new(),
        quality: Default::default(),
        project_compass: Default::default(),
        custody: Default::default(),
        release_rule: None,
        release_recipe: None,
        confirmed_release_version: None,
        ai_permission: "Blocked".to_string(),
        conditions: Vec::new(),
        last_scan_at: now,
        last_fetch_at: None,
        last_activity_at: None,
    }
}

fn radar_remote(
    id: &str,
    full_name: &str,
    visibility: Option<&str>,
    last_refreshed_at: &str,
) -> RemoteRepositorySnapshot {
    let name = full_name.rsplit('/').next().unwrap_or(full_name);
    let owner = full_name.split('/').next().unwrap_or("unknown");
    RemoteRepositorySnapshot {
        id: id.to_string(),
        provider: "github".to_string(),
        full_name: full_name.to_string(),
        name: name.to_string(),
        owner: owner.to_string(),
        html_url: format!("https://github.com/{full_name}"),
        default_branch: Some("main".to_string()),
        archived: false,
        visibility: visibility.map(str::to_string),
        viewer_can_administer: Some(true),
        viewer_permission: Some("admin".to_string()),
        locality: "Remote only".to_string(),
        identity_id: "github:jakyeamos".to_string(),
        last_refreshed_at: last_refreshed_at.to_string(),
        pull_requests: Vec::new(),
        releases: Vec::new(),
        ci_checks: Vec::new(),
        ci_branch: None,
        ci_commit: None,
        ci_runs: Vec::new(),
    }
}

fn radar_showcase_project(
    name: &str,
    product_status: &str,
    product_score: Option<f64>,
    demo_status: &str,
    demo_score: Option<f64>,
    public_eligibility: &str,
    missing_materials: Vec<&str>,
) -> showcase::ShowcaseProjectSnapshot {
    showcase::ShowcaseProjectSnapshot {
        repository_name: name.to_string(),
        display_name: name.to_string(),
        repository_id: Some(format!("repo-{name}")),
        repository_path: Some(format!("/tmp/{name}")),
        registration_status: "registered".to_string(),
        public_eligibility: public_eligibility.to_string(),
        disposition_source: "fixture".to_string(),
        work_disposition: "active".to_string(),
        work_disposition_summary: "Fixture evidence".to_string(),
        next_step_category: "evidence".to_string(),
        product_readiness: showcase::ShowcaseDimension {
            status: product_status.to_string(),
            score: product_score,
            evidence: "fixture".to_string(),
        },
        demo_materials: showcase::ShowcaseDimension {
            status: demo_status.to_string(),
            score: demo_score,
            evidence: "fixture".to_string(),
        },
        career_signal: showcase::ShowcaseDimension {
            status: "unknown".to_string(),
            score: None,
            evidence: "fixture".to_string(),
        },
        showcase_score: None,
        priority_score: None,
        lane: "fixture".to_string(),
        publishable: false,
        blockers: Vec::new(),
        missing_materials: missing_materials.into_iter().map(str::to_string).collect(),
        next_step: "Review fixture evidence".to_string(),
    }
}

fn write_radar_profile_catalog(root: &Path) {
    let entries = serde_json::json!([
        {
            "name": "public-app",
            "repository_id": "repo-public-app",
            "provider_identity": "github:acme/public-app",
            "provider_visibility": "public",
            "profile_visibility": "public",
            "readme_disposition": "include",
            "public_url": "https://github.com/acme/public-app"
        },
        {
            "name": "private-app",
            "repository_id": "repo-private-app",
            "provider_identity": "github:acme/private-app",
            "provider_visibility": "private",
            "profile_visibility": "public",
            "readme_disposition": "include",
            "public_url": "https://github.com/acme/private-app"
        },
        {
            "name": "AIOS",
            "repository_id": "repo-AIOS",
            "provider_identity": "github:acme/AIOS",
            "provider_visibility": "public",
            "profile_visibility": "public",
            "readme_disposition": "include",
            "public_url": "https://github.com/acme/AIOS"
        }
    ]);
    radar_fs::write(
        root.join("profile-catalog.json"),
        serde_json::json!({"entries": entries}).to_string(),
    )
    .expect("radar profile catalog fixture should be writable");
}

#[test]
fn release_radar_unions_and_deduplicates_local_and_github_repositories_without_inventing_readiness() {
    let _guard = RADAR_ENV_LOCK.lock().expect("radar environment lock should be available");
    let profile_root = fixture_root();
    write_radar_profile_catalog(&profile_root);
    std::env::set_var("PRONTO_PROFILE_ROOT", &profile_root);

    let now = Utc::now();
    let fresh = now.to_rfc3339();
    let older = (now - chrono::Duration::hours(1)).to_rfc3339();
    let repositories = vec![
        radar_repository("public-app", Some("https://github.com/acme/public-app.git")),
        radar_repository("private-app", Some("git@github.com:acme/private-app.git")),
        radar_repository("local-only", None),
        radar_repository("AIOS", Some("https://github.com/acme/AIOS")),
    ];
    let state = StoreState {
        provider_status: ProviderStatus {
            provider: "GitHub".to_string(),
            state: "Ready".to_string(),
            message: "fresh fixture".to_string(),
            last_refresh_at: Some(fresh.clone()),
            identity_count: 1,
            repository_count: 5,
        },
        remote_repositories: vec![
            radar_remote("github:old", "acme/public-app", Some("public"), &older),
            radar_remote("github:public", "acme/public-app", Some("public"), &fresh),
            radar_remote("github:private", "acme/private-app", Some("private"), &fresh),
            radar_remote("github:unknown", "acme/github-only", None, &fresh),
            radar_remote("github:aios", "acme/AIOS", Some("public"), &fresh),
        ],
        ..StoreState::default()
    };
    let showcase = showcase::ShowcasePortfolioSnapshot {
        projects: vec![
            radar_showcase_project(
                "public-app",
                "assessed",
                Some(4.0),
                "assessed",
                Some(3.0),
                "eligible",
                Vec::new(),
            ),
            radar_showcase_project(
                "private-app",
                "assessed",
                Some(3.5),
                "assessed",
                Some(2.5),
                "eligible",
                Vec::new(),
            ),
            radar_showcase_project(
                "AIOS",
                "assessed",
                Some(3.2),
                "assessed",
                Some(1.2),
                "not_applicable",
                vec!["demo brief", "walkthrough"],
            ),
        ],
        ..showcase::ShowcasePortfolioSnapshot::default()
    };

    let radar = build_release_radar(&state, &repositories, &showcase);
    assert_eq!(radar.schema_version, "pronto-release-radar/v1");
    assert_eq!(radar.rows.len(), 5, "local and remote identities should be deduplicated");
    assert_eq!(radar.counts.total, 5);
    assert_eq!(radar.counts.local_only, 1);
    assert_eq!(radar.counts.github_only, 1);
    assert_eq!(radar.counts.local_and_remote, 3);
    assert_eq!(radar.counts.public_fresh, 2);
    assert_eq!(radar.counts.private_or_internal_fresh, 1);
    assert_eq!(radar.counts.unknown_or_blocked, 2);
    assert_eq!(radar.counts.public_framing_eligible, 2);
    assert_eq!(radar.counts.profile_needs_sync, 1);

    let public = radar.rows.iter().find(|row| row.name == "public-app").unwrap();
    assert_eq!(public.locality, "Local and remote");
    assert_eq!(public.visibility, "public");
    assert_eq!(public.visibility_freshness, "fresh");
    assert_eq!(public.public_framing, "eligible");
    assert_eq!(public.demo_materials, "assessed (3.0/5)");

    let private = radar.rows.iter().find(|row| row.name == "private-app").unwrap();
    assert_eq!(private.public_framing, "suppressed");
    assert_eq!(private.profile_link_sync, "needs_sync");
    assert_eq!(private.demo_materials, "assessed (2.5/5)");

    let aios = radar.rows.iter().find(|row| row.name == "AIOS").unwrap();
    assert_eq!(aios.public_framing, "eligible");
    assert_eq!(aios.product_readiness, "assessed (3.2/5)");
    assert_eq!(aios.demo_materials, "missing evidence");
    assert_ne!(aios.demo_materials, "created");

    let remote_only = radar.rows.iter().find(|row| row.name == "github-only").unwrap();
    assert_eq!(remote_only.locality, "GitHub only");
    assert_eq!(remote_only.visibility, "unknown");
    assert_eq!(remote_only.public_framing, "blocked");
    assert!(remote_only.blocker.is_some());

    let local_only = radar.rows.iter().find(|row| row.name == "local-only").unwrap();
    assert_eq!(local_only.locality, "Local only");
    assert_eq!(local_only.visibility_freshness, "unknown");
    assert_eq!(local_only.profile_link_sync, "not_applicable");

    std::env::remove_var("PRONTO_PROFILE_ROOT");
    radar_fs::remove_dir_all(profile_root).expect("radar profile fixture should be removable");
}

#[test]
fn github_visibility_normalization_parsing_gates_and_readback_are_fail_closed() {
    assert_eq!(
        normalized_github_repository_name("git@github.com:Acme/Release-Radar.git"),
        Some("acme/release-radar".to_string())
    );
    assert_eq!(
        normalized_github_repository_name("https://github.com/Acme/Release-Radar"),
        Some("acme/release-radar".to_string())
    );
    assert!(normalized_github_repository_name("acme").is_none());
    assert_eq!(normalize_github_visibility(" PUBLIC ").unwrap(), "public");
    assert!(normalize_github_visibility("secret").is_err());

    let control = parse_github_repository_control(&serde_json::json!({
        "full_name": "Acme/Release-Radar",
        "visibility": "PRIVATE",
        "archived": false,
        "permissions": {"admin": true},
        "permission": "admin"
    }))
    .expect("GitHub control payload should parse");
    assert_eq!(control.full_name, "acme/release-radar");
    assert_eq!(control.visibility, "private");
    assert_eq!(control.viewer_can_administer, Some(true));
    assert_eq!(control.viewer_permission.as_deref(), Some("admin"));
    validate_github_visibility_preflight(&control, "acme/release-radar", "public")
        .expect("admin control should pass the visibility preflight");

    let mut not_admin = control.clone();
    not_admin.viewer_can_administer = Some(false);
    assert!(validate_github_visibility_preflight(&not_admin, "acme/release-radar", "public")
        .unwrap_err()
        .contains("permissions.admin=true"));

    let mut archived = control.clone();
    archived.archived = true;
    assert!(validate_github_visibility_preflight(&archived, "acme/release-radar", "public")
        .unwrap_err()
        .contains("Archived"));

    assert!(validate_github_visibility_preflight(&control, "acme/release-radar", "private")
        .unwrap_err()
        .contains("no provider mutation"));
    assert!(validate_github_visibility_preflight(&control, "other/release-radar", "public")
        .unwrap_err()
        .contains("identity mismatch"));

    let mut readback = control.clone();
    readback.visibility = "public".to_string();
    github_visibility_readback_matches(&readback, "acme/release-radar", "public")
        .expect("matching post-write readback should pass");
    assert!(github_visibility_readback_matches(&control, "acme/release-radar", "public")
        .unwrap_err()
        .contains("post-change readback mismatch"));
}

#[test]
fn github_visibility_edit_includes_explicit_consequence_acknowledgement() {
    let root = fixture_root();
    let args_file = root.join("args");
    let executable = root.join("gh");
    radar_fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            args_file.display()
        ),
    )
    .expect("GitHub CLI fixture should be writable");
    let mut permissions = radar_fs::metadata(&executable)
        .expect("GitHub CLI fixture metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    radar_fs::set_permissions(&executable, permissions)
        .expect("GitHub CLI fixture should be executable");

    let adapter = GitHubCliAdapter {
        executable: executable.to_string_lossy().into_owned(),
        target_repository_names: None,
    };
    adapter
        .edit_repository_visibility("acme/release-radar", "private")
        .expect("the guarded GitHub edit should invoke the fixture CLI");
    let args = radar_fs::read_to_string(args_file).expect("fixture CLI should record arguments");
    assert!(args.contains("repo\n"));
    assert!(args.contains("edit\n"));
    assert!(args.contains("acme/release-radar\n"));
    assert!(args.contains("--visibility\n"));
    assert!(args.contains("private\n"));
    assert!(args.contains("--accept-visibility-change-consequences\n"));

    radar_fs::remove_dir_all(root).expect("GitHub CLI fixture should be removable");
}
