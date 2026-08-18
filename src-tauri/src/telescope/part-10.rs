fn ensure_cache_table(store_path: &Path) -> Result<(), String> {
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create Pronto data directory: {error}"))?;
    }
    let connection = Connection::open(store_path)
        .map_err(|error| format!("Could not open Telescope cache: {error}"))?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS telescope_cache (
            repository_id TEXT NOT NULL,
            schema_version TEXT NOT NULL,
            workspace_fingerprint TEXT NOT NULL,
            generated_at TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            PRIMARY KEY(repository_id, schema_version, workspace_fingerprint)
        );
        DELETE FROM telescope_cache
        WHERE instr(repository_id, '/') > 0
           OR instr(repository_id, char(92)) > 0
           OR instr(COALESCE(json_extract(payload_json, '$.repository_id'), ''), '/') > 0
           OR instr(COALESCE(json_extract(payload_json, '$.binding.workspace_id'), ''), '/') > 0
           OR instr(COALESCE(json_extract(payload_json, '$.repository_id'), ''), char(92)) > 0
           OR instr(COALESCE(json_extract(payload_json, '$.binding.workspace_id'), ''), char(92)) > 0;",
        )
        .map_err(|error| format!("Could not initialize Telescope cache: {error}"))
}

fn load_cached(
    store_path: &Path,
    repository_id: &str,
    workspace_fingerprint: &str,
) -> Result<Option<TelescopeProjection>, String> {
    let connection = Connection::open(store_path)
        .map_err(|error| format!("Could not open Telescope cache: {error}"))?;
    let result = connection.query_row(
        "SELECT payload_json FROM telescope_cache WHERE repository_id = ?1 AND schema_version = ?2 AND workspace_fingerprint = ?3",
        params![repository_id, SCHEMA_VERSION, workspace_fingerprint],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(payload) => serde_json::from_str(&payload)
            .map(Some)
            .map_err(|error| format!("Could not decode Telescope cache: {error}")),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("Could not read Telescope cache: {error}")),
    }
}

fn save_cached(store_path: &Path, projection: &TelescopeProjection) -> Result<(), String> {
    let connection = Connection::open(store_path)
        .map_err(|error| format!("Could not open Telescope cache: {error}"))?;
    let payload = serde_json::to_string(projection)
        .map_err(|error| format!("Could not encode Telescope projection: {error}"))?;
    connection.execute(
        "INSERT INTO telescope_cache (repository_id, schema_version, workspace_fingerprint, generated_at, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(repository_id, schema_version, workspace_fingerprint) DO UPDATE SET generated_at=excluded.generated_at, payload_json=excluded.payload_json",
        params![projection.repository_id, projection.schema_version, projection.binding.workspace_fingerprint, projection.binding.generated_at, payload],
    ).map_err(|error| format!("Could not cache Telescope projection: {error}"))?;
    connection
        .execute(
            "DELETE FROM telescope_cache
         WHERE repository_id = ?1 AND schema_version = ?2 AND workspace_fingerprint NOT IN (
             SELECT workspace_fingerprint FROM telescope_cache
             WHERE repository_id = ?1 AND schema_version = ?2
             ORDER BY generated_at DESC LIMIT 4
         )",
            params![projection.repository_id, projection.schema_version],
        )
        .map_err(|error| format!("Could not prune Telescope cache: {error}"))?;
    Ok(())
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = git_output_bytes(root, arguments)?;
    Ok(String::from_utf8_lossy(&output).trim().to_string())
}

fn git_output_bytes(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("Could not run Git for Telescope: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(output.stdout)
}

fn dirty_state_fingerprint(root: &Path, status: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(status.as_bytes());
    for arguments in [
        ["diff", "--no-ext-diff", "--binary"].as_slice(),
        ["diff", "--no-ext-diff", "--binary", "--cached"].as_slice(),
    ] {
        if let Ok(output) = git_output_bytes(root, arguments) {
            digest.update(output);
        }
    }
    if let Ok(untracked) =
        git_output_bytes(root, &["ls-files", "--others", "--exclude-standard", "-z"])
    {
        for path_bytes in untracked.split(|byte| *byte == 0).take(MAX_SOURCE_FILES) {
            if path_bytes.is_empty() {
                continue;
            }
            let relative = String::from_utf8_lossy(path_bytes);
            let path = root.join(relative.as_ref());
            digest.update(path_bytes);
            if let Ok(metadata) = fs::metadata(&path) {
                digest.update(metadata.len().to_le_bytes());
                if metadata.len() <= MAX_SOURCE_BYTES {
                    if let Ok(content) = fs::read(path) {
                        digest.update(content);
                    }
                } else if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.duration_since(std::time::UNIX_EPOCH) {
                        digest.update(elapsed.as_nanos().to_le_bytes());
                    }
                }
            }
        }
    }
    format!(
        "dirty-{}",
        digest
            .finalize()
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn stable_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    format!(
        "{prefix}-{}",
        digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn public_identifier(kind: &str, value: &str) -> String {
    if Path::new(value).is_absolute() || value.contains('/') || value.contains('\\') {
        stable_id(kind, value)
    } else {
        value.to_string()
    }
}
