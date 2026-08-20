fn load_coordinated_maturity_checkpoint(
    checkpoint_path: &Path,
    repositories: &[RepositorySnapshot],
    checkpoint: &Value,
) -> Result<CoordinatedMaturityImport, String> {
    if checkpoint.get("schema").and_then(Value::as_str) != Some(MATURITY_CHECKPOINT_SCHEMA) {
        return Err(format!(
            "The maturity checkpoint schema must be {MATURITY_CHECKPOINT_SCHEMA}."
        ));
    }
    if checkpoint.get("status").and_then(Value::as_str) != Some("complete")
        || checkpoint.get("publication_status").and_then(Value::as_str) != Some("ready")
    {
        return Err("The maturity checkpoint is not a complete published snapshot.".to_string());
    }
    let checkpoint_id = checkpoint_string(checkpoint, "checkpoint_id")?;
    let observed_at = checkpoint_string(checkpoint, "observed_at")?;
    let quality_status = checkpoint_string(checkpoint, "quality_status")?;
    let components = checkpoint_object(checkpoint, "components")?;
    let qr_component = checkpoint_object_from(components, "qr_maturity")?;
    let mac_component = checkpoint_object_from(components, "mac_control")?;
    let qr_audit_id = checkpoint_string_from(qr_component, "audit_id")?;
    let mac_control_audit_id = checkpoint_string_from(mac_component, "audit_id")?;
    let qr_as_of = checkpoint_string_from(qr_component, "as_of")?;
    let mac_control_as_of = checkpoint_string_from(mac_component, "as_of")?;
    if qr_as_of != observed_at || mac_control_as_of != observed_at {
        return Err("The maturity checkpoint component timestamps do not match.".to_string());
    }

    let qr_path = resolve_checkpoint_component(
        checkpoint_path,
        checkpoint_string_from(qr_component, "path")?,
        "QR maturity feed",
    )?;
    verify_checkpoint_component_hash(&qr_path, qr_component, "QR maturity feed")?;
    let mac_control_path = resolve_checkpoint_component(
        checkpoint_path,
        checkpoint_string_from(mac_component, "path")?,
        "Mac Control report",
    )?;
    verify_checkpoint_component_hash(&mac_control_path, mac_component, "Mac Control report")?;

    let feed = read_json(&qr_path)
        .ok_or_else(|| "The checkpoint QR maturity component is invalid JSON.".to_string())?;
    if !validate_maturity_feed(&feed) {
        return Err("The checkpoint QR maturity component failed feed validation.".to_string());
    }
    let feed_source = feed
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| "The checkpoint QR maturity source is missing.".to_string())?;
    if feed_source.get("audit_id").and_then(Value::as_str) != Some(qr_audit_id.as_str())
        || feed_source.get("as_of").and_then(Value::as_str) != Some(observed_at.as_str())
    {
        return Err("The checkpoint QR maturity source does not match its pointer.".to_string());
    }
    validate_checkpoint_target(checkpoint, &feed, repositories)?;

    let mac_report = read_json(&mac_control_path)
        .ok_or_else(|| "The checkpoint Mac Control component is invalid JSON.".to_string())?;
    if mac_report.get("schema_version").and_then(Value::as_str)
        != Some(crate::mac_control_maturity::MAC_CONTROL_SCHEMA)
        || mac_report.get("producer").and_then(Value::as_str) != Some("mac-control")
        || mac_report.get("run_id").and_then(Value::as_str) != Some(mac_control_audit_id.as_str())
        || mac_report.get("observed_at").and_then(Value::as_str) != Some(observed_at.as_str())
    {
        return Err("The checkpoint Mac Control component does not match its pointer.".to_string());
    }

    let audit = maturity_feed_import(Some(&qr_path), repositories);
    if audit.portfolio.audit_status == "Unavailable" {
        return Err("The checkpoint QR maturity component could not be imported.".to_string());
    }
    let mac_control =
        crate::mac_control_maturity::evaluate_report_at(&mac_control_path, repositories);
    if mac_control.portfolio.run_id.as_deref() != Some(mac_control_audit_id.as_str())
        || mac_control.portfolio.observed_at.as_deref() != Some(observed_at.as_str())
    {
        return Err("The imported Mac Control report does not match its pointer.".to_string());
    }

    let checkpoint_freshness = evaluate_audit_freshness_at(Some(&observed_at), Utc::now());
    let mut audit = audit;
    audit.portfolio.maturity_checkpoint = MaturityCheckpointSnapshot {
        status: "Coordinated".to_string(),
        publication_status: "Published".to_string(),
        quality_status,
        freshness: checkpoint_freshness.clone(),
        checkpoint_id: Some(checkpoint_id),
        observed_at: Some(observed_at),
        qr_audit_id: Some(qr_audit_id),
        mac_control_audit_id: Some(mac_control_audit_id),
        path: Some(checkpoint_path.to_string_lossy().to_string()),
        reason: checkpoint
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    Ok(CoordinatedMaturityImport {
        audit,
        mac_control,
        checkpoint: MaturityCheckpointSnapshot {
            status: "Coordinated".to_string(),
            publication_status: "Published".to_string(),
            quality_status: checkpoint_string(checkpoint, "quality_status")?,
            freshness: checkpoint_freshness,
            checkpoint_id: checkpoint
                .get("checkpoint_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            observed_at: checkpoint
                .get("observed_at")
                .and_then(Value::as_str)
                .map(str::to_string),
            qr_audit_id: Some(checkpoint_string_from(qr_component, "audit_id")?),
            mac_control_audit_id: Some(checkpoint_string_from(mac_component, "audit_id")?),
            path: Some(checkpoint_path.to_string_lossy().to_string()),
            reason: checkpoint
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
    })
}

fn blocked_maturity_checkpoint_import(
    checkpoint_path: &Path,
    repositories: &[RepositorySnapshot],
    reason: &str,
) -> CoordinatedMaturityImport {
    let path = checkpoint_path.to_string_lossy().to_string();
    let mut audit = AuditImport::default();
    audit.portfolio.audit_root = Some(path.clone());
    audit.portfolio.latest_audit_path = Some(path.clone());
    audit.portfolio.audit_status = "Blocked".to_string();
    let checkpoint = MaturityCheckpointSnapshot {
        status: "Blocked".to_string(),
        publication_status: "Invalid".to_string(),
        quality_status: "Blocked".to_string(),
        freshness: QualityFreshness::Unknown,
        path: Some(path.clone()),
        reason: Some(reason.to_string()),
        ..MaturityCheckpointSnapshot::default()
    };
    audit.portfolio.maturity_checkpoint = checkpoint.clone();
    let mac_control =
        crate::mac_control_maturity::blocked_for_checkpoint(repositories, Some(path), reason);
    CoordinatedMaturityImport {
        audit,
        mac_control,
        checkpoint,
    }
}

fn validate_checkpoint_target(
    checkpoint: &Value,
    feed: &Value,
    repositories: &[RepositorySnapshot],
) -> Result<(), String> {
    let target = checkpoint_object(checkpoint, "target")?;
    let target_count = target
        .get("repository_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| "The maturity checkpoint target count is missing.".to_string())?
        as usize;
    let target_repositories = target
        .get("repositories")
        .and_then(Value::as_array)
        .ok_or_else(|| "The maturity checkpoint target repositories are missing.".to_string())?;
    if target_repositories.len() != target_count {
        return Err("The maturity checkpoint target count is inconsistent.".to_string());
    }
    let mut commits = HashMap::new();
    for repository in target_repositories {
        let repository_id = checkpoint_string(repository, "repo_id")?;
        let observed_commit = checkpoint_string(repository, "observed_commit")?;
        if commits.insert(repository_id, observed_commit).is_some() {
            return Err(
                "The maturity checkpoint target contains duplicate repositories.".to_string(),
            );
        }
    }
    let projections = feed
        .get("repositories")
        .and_then(Value::as_array)
        .ok_or_else(|| "The checkpoint QR maturity repositories are missing.".to_string())?;
    if projections.len() != target_count {
        return Err("The checkpoint target and QR maturity populations do not match.".to_string());
    }
    for projection in projections {
        let repository_id = checkpoint_string(projection, "repo_id")?;
        let target_commit = commits.get(&repository_id).ok_or_else(|| {
            "The checkpoint target and QR maturity repositories do not match.".to_string()
        })?;
        if projection.get("target_head").and_then(Value::as_str) != Some(target_commit.as_str()) {
            return Err(format!(
                "The checkpoint target commit does not match QR maturity for {repository_id}."
            ));
        }
    }
    for repository in repositories {
        let repository_id = repository_feed_id(repository);
        if !commits.contains_key(&repository_id) {
            return Err(format!(
                "The checkpoint target is missing Pronto repository {}.",
                repository.name
            ));
        }
    }
    Ok(())
}

fn resolve_checkpoint_component(
    checkpoint_path: &Path,
    raw_path: String,
    label: &str,
) -> Result<PathBuf, String> {
    let relative = Path::new(&raw_path);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err(format!("{label} path must be a safe relative path."));
    }
    let root = checkpoint_path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "The maturity checkpoint root is unavailable.".to_string())?;
    let root = fs::canonicalize(root)
        .map_err(|error| format!("The maturity checkpoint root could not be resolved: {error}"))?;
    let candidate = root.join(relative);
    if candidate.is_symlink() {
        return Err(format!("{label} must not be a symlink."));
    }
    let resolved = fs::canonicalize(&candidate)
        .map_err(|error| format!("{label} could not be resolved: {error}"))?;
    if !resolved.starts_with(&root) {
        return Err(format!("{label} resolves outside the checkpoint root."));
    }
    Ok(candidate)
}

fn verify_checkpoint_component_hash(
    path: &Path,
    component: &serde_json::Map<String, Value>,
    label: &str,
) -> Result<(), String> {
    let expected = checkpoint_string_from(component, "sha256")?;
    let actual = Sha256::digest(
        fs::read(path).map_err(|error| format!("{label} could not be read: {error}"))?,
    )
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect::<String>();
    if actual != expected {
        return Err(format!(
            "{label} hash does not match its checkpoint pointer."
        ));
    }
    Ok(())
}

fn checkpoint_object<'a>(
    value: &'a Value,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("The maturity checkpoint field {key} is missing."))
}

fn checkpoint_object_from<'a>(
    value: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("The maturity checkpoint component {key} is missing."))
}

fn checkpoint_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|candidate| !candidate.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("The maturity checkpoint field {key} is missing."))
}

fn checkpoint_string_from(
    value: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|candidate| !candidate.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("The maturity checkpoint component field {key} is missing."))
}

#[derive(Debug, Clone)]
pub struct FleetAuditEvidence {
    pub maturity: QualityMaturity,
    pub findings: QualityFindings,
}

pub fn target_provenance_matches(
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
    target_branch: &str,
    target_commit: &str,
) -> bool {
    scanned_branch == Some(target_branch) && scanned_commit == Some(target_commit)
}

pub fn evaluate_target_freshness(
    scanned_branch: Option<&str>,
    scanned_commit: Option<&str>,
    target_branch: &str,
    target_commit: &str,
) -> QualityFreshness {
    if target_provenance_matches(scanned_branch, scanned_commit, target_branch, target_commit) {
        return QualityFreshness::Fresh;
    }
    if scanned_branch.is_none() || scanned_commit.is_none() {
        QualityFreshness::Unknown
    } else {
        QualityFreshness::Stale
    }
}

pub fn target_evidence_is_current(
    snapshot: &QualitySnapshot,
    target_branch: &str,
    target_commit: &str,
) -> bool {
    snapshot.gates.iter().any(|gate| {
        gate.evidence.iter().any(|evidence| {
            target_provenance_matches(
                evidence.scanned_branch.as_deref(),
                evidence.scanned_commit.as_deref(),
                target_branch,
                target_commit,
            )
        })
    }) || target_provenance_matches(
        snapshot.findings.scanned_branch.as_deref(),
        snapshot.findings.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    ) || target_provenance_matches(
        snapshot.maturity.scanned_branch.as_deref(),
        snapshot.maturity.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    )
}

pub fn scope_fleet_audit_evidence_to_target(
    evidence: &mut FleetAuditEvidence,
    target_branch: &str,
    target_commit: &str,
) {
    evidence.maturity.scanned_branch = Some(target_branch.to_string());
    evidence.maturity.scanned_commit = Some(target_commit.to_string());
    evidence.maturity.freshness = evaluate_target_freshness(
        evidence.maturity.scanned_branch.as_deref(),
        evidence.maturity.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
    evidence.findings.scanned_branch = Some(target_branch.to_string());
    evidence.findings.scanned_commit = Some(target_commit.to_string());
    evidence.findings.freshness = evaluate_target_freshness(
        evidence.findings.scanned_branch.as_deref(),
        evidence.findings.scanned_commit.as_deref(),
        target_branch,
        target_commit,
    );
}

#[derive(Debug, Clone, Default)]
pub struct FleetAuditImport {
    pub audit_id: Option<String>,
    pub observed_at: Option<String>,
    pub evidence: HashMap<String, FleetAuditEvidence>,
}

pub fn canonical_maturity_feed_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(CANONICAL_MATURITY_FEED_RELATIVE_PATH))
}

pub fn canonical_maturity_checkpoint_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(CANONICAL_MATURITY_CHECKPOINT_RELATIVE_PATH))
}

pub fn is_stable_detector_report(report_path: Option<&str>) -> bool {
    report_path
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "code-quality-scan.json")
}

fn sync_detector_counts(findings: &mut QualityFindings) {
    findings.detector_findings_total = findings.total;
    findings.detector_actionable_total = findings.actionable_total;
    findings.detector_unreviewed_total = findings.unreviewed_total;
}
