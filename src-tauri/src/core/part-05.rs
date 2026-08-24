fn load_store_from_connection(connection: &SqliteConnection) -> Result<StoreState, String> {
    let version = metadata_value(connection, "store_version")?
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(STORE_VERSION)
        .max(STORE_VERSION);
    let retention_days = metadata_value(connection, "retention_days")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS);
    let mut provider_status = match metadata_value(connection, "provider_status_json")? {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("Could not decode provider status: {error}"))?,
        None => ProviderStatus::default(),
    };
    let quality = match metadata_value(connection, "quality_summary_json")? {
        Some(payload) => serde_json::from_str(&payload)
            .map_err(|error| format!("Could not decode quality summary: {error}"))?,
        None => QualityPortfolioSnapshot::default(),
    };

    let root_rows = connection
        .prepare(
            "SELECT id, path, label, ignore_patterns_json, refresh_policy,
                    background_monitoring, registered_at
             FROM roots ORDER BY id",
        )
        .map_err(|error| format!("Could not prepare Pronto roots query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)? != 0,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto roots: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto roots: {error}"))?;
    let roots = root_rows
        .into_iter()
        .map(
            |(
                id,
                path,
                label,
                ignore_patterns_json,
                refresh_policy,
                background_monitoring,
                registered_at,
            )| {
                let ignore_patterns = serde_json::from_str(&ignore_patterns_json)
                    .map_err(|error| format!("Could not decode root ignore patterns: {error}"))?;
                Ok(RootConfig {
                    id,
                    path,
                    label,
                    ignore_patterns,
                    refresh_policy,
                    background_monitoring,
                    registered_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let product_rows = connection
        .prepare(
            "SELECT id, name, repository_ids_json, release_mode, created_at, updated_at
             FROM products ORDER BY name, id",
        )
        .map_err(|error| format!("Could not prepare Pronto products query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto products: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto products: {error}"))?;
    let products = product_rows
        .into_iter()
        .map(
            |(id, name, repository_ids_json, release_mode, created_at, updated_at)| {
                let repository_ids = serde_json::from_str(&repository_ids_json)
                    .map_err(|error| format!("Could not decode product repositories: {error}"))?;
                Ok(ProductConfig {
                    id,
                    name,
                    repository_ids,
                    release_mode,
                    created_at,
                    updated_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let group_rows = connection
        .prepare(
            "SELECT id, name, repository_ids_json, created_at, updated_at
             FROM groups_config ORDER BY name, id",
        )
        .map_err(|error| format!("Could not prepare Pronto groups query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto groups: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto groups: {error}"))?;
    let groups = group_rows
        .into_iter()
        .map(|(id, name, repository_ids_json, created_at, updated_at)| {
            let repository_ids = serde_json::from_str(&repository_ids_json)
                .map_err(|error| format!("Could not decode group repositories: {error}"))?;
            Ok(GroupConfig {
                id,
                name,
                repository_ids,
                created_at,
                updated_at,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let provider_identity_payloads = connection
        .prepare("SELECT payload_json FROM provider_identities ORDER BY id")
        .map_err(|error| format!("Could not prepare provider identities query: {error}"))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read provider identities: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode provider identities: {error}"))?;
    let provider_identities = provider_identity_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode provider identity: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let remote_repository_payloads = connection
        .prepare("SELECT payload_json FROM remote_repositories ORDER BY id")
        .map_err(|error| format!("Could not prepare remote repositories query: {error}"))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read remote repositories: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode remote repositories: {error}"))?;
    let remote_repositories = remote_repository_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode remote repository: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let repository_payloads = connection
        .prepare("SELECT payload_json FROM repositories ORDER BY id")
        .map_err(|error| format!("Could not prepare Pronto repositories query: {error}"))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not read Pronto repositories: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto repositories: {error}"))?;
    let mut repositories = repository_payloads
        .into_iter()
        .map(|payload| {
            serde_json::from_str(&payload)
                .map_err(|error| format!("Could not decode repository snapshot: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    sort_repositories_by_name(&mut repositories);
    let remote_repositories = classify_remote_repositories(&repositories, remote_repositories);
    provider_status.repository_count = remote_repositories.len();

    let expected_conditions = connection
        .prepare(
            "SELECT repository_id, condition_id, fingerprint, marked_at
             FROM expected_conditions ORDER BY repository_id, condition_id",
        )
        .map_err(|error| format!("Could not prepare Pronto expected conditions query: {error}"))?
        .query_map([], |row| {
            Ok(ExpectedCondition {
                repository_id: row.get(0)?,
                condition_id: row.get(1)?,
                fingerprint: row.get(2)?,
                marked_at: row.get(3)?,
            })
        })
        .map_err(|error| format!("Could not read Pronto expected conditions: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto expected conditions: {error}"))?;

    let events = connection
        .prepare(
            "SELECT id, repository_id, kind, summary, fingerprint, created_at
             FROM events ORDER BY created_at, id",
        )
        .map_err(|error| format!("Could not prepare Pronto events query: {error}"))?
        .query_map([], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                kind: row.get(2)?,
                summary: row.get(3)?,
                fingerprint: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|error| format!("Could not read Pronto events: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto events: {error}"))?;

    let action_audit_rows = connection
        .prepare(
            "SELECT id, action, target_ids_json, risk, status, summary, created_at, completed_at
             FROM action_audits ORDER BY created_at, rowid",
        )
        .map_err(|error| format!("Could not prepare Pronto action audits query: {error}"))?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|error| format!("Could not read Pronto action audits: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Pronto action audits: {error}"))?;
    let action_audits = action_audit_rows
        .into_iter()
        .map(
            |(id, action, target_ids_json, risk, status, summary, created_at, completed_at)| {
                let target_ids = serde_json::from_str(&target_ids_json)
                    .map_err(|error| format!("Could not decode action audit targets: {error}"))?;
                Ok(ActionAudit {
                    id,
                    action,
                    target_ids,
                    risk,
                    status,
                    summary,
                    created_at,
                    completed_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;

    let remediation = connection
        .query_row(
            "SELECT payload_json FROM remediation_runs ORDER BY generated_at DESC, id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Could not read Pronto remediation run: {error}"))?
        .map(|payload| {
            serde_json::from_str::<RemediationRun>(&payload)
                .map_err(|error| format!("Could not decode Pronto remediation run: {error}"))
        })
        .transpose()?
        .unwrap_or_else(remediation::empty_run);

    Ok(StoreState {
        version,
        roots,
        repositories,
        products,
        groups,
        expected_conditions,
        events,
        action_audits,
        provider_identities,
        remote_repositories,
        provider_status,
        quality,
        remediation,
        retention_days,
    })
}

fn load_store_with_quality(path: &Path) -> Result<StoreState, String> {
    let mut state = load_store(path)?;
    apply_quality_evidence_scoped(&mut state, None, None);
    Ok(state)
}

fn load_store_read_only_with_quality(path: &Path) -> Result<StoreState, String> {
    let mut state = load_store_read_only(path)?;
    apply_quality_evidence_scoped(&mut state, None, None);
    Ok(state)
}

fn load_store_read_only_with_quality_bounded(path: &Path) -> Result<StoreState, String> {
    let path = path.to_path_buf();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(load_store_read_only_with_quality(&path));
    });
    match receiver.recv_timeout(StdDuration::from_secs(QUALITY_READ_TIMEOUT_SECONDS)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "Fresh quality projection exceeded the {} second deadline; rerun without --fresh for the cached snapshot or run `pronto quality refresh` separately.",
            QUALITY_READ_TIMEOUT_SECONDS
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(
            "Fresh quality projection stopped before it returned a result; rerun without --fresh for the cached snapshot or run `pronto quality refresh` separately.".to_string(),
        ),
    }
}
