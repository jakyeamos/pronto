const RELEASE_RADAR_SCHEMA: &str = "pronto-release-radar/v1";
const RELEASE_RADAR_FRESHNESS_MINUTES: i64 = 1_440;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseRadarCounts {
    pub total: usize,
    pub local_only: usize,
    pub github_only: usize,
    pub local_and_remote: usize,
    pub public_fresh: usize,
    pub private_or_internal_fresh: usize,
    pub unknown_or_blocked: usize,
    pub public_framing_eligible: usize,
    pub profile_needs_sync: usize,
}

impl Default for ReleaseRadarCounts {
    fn default() -> Self {
        Self {
            total: 0,
            local_only: 0,
            github_only: 0,
            local_and_remote: 0,
            public_fresh: 0,
            private_or_internal_fresh: 0,
            unknown_or_blocked: 0,
            public_framing_eligible: 0,
            profile_needs_sync: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseRadarRow {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub locality: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_url: Option<String>,
    pub visibility: String,
    pub visibility_freshness: String,
    pub public_framing: String,
    pub profile_link_sync: String,
    pub product_readiness: String,
    pub demo_materials: String,
    pub release_readiness: String,
    pub next_action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_repository_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReleaseRadarSnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub provider_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_last_refresh_at: Option<String>,
    pub profile_state: String,
    pub counts: ReleaseRadarCounts,
    pub rows: Vec<ReleaseRadarRow>,
}

impl Default for ReleaseRadarSnapshot {
    fn default() -> Self {
        Self {
            schema_version: RELEASE_RADAR_SCHEMA.to_string(),
            generated_at: iso_now(),
            provider_state: "Not connected".to_string(),
            provider_last_refresh_at: None,
            profile_state: "Unknown".to_string(),
            counts: ReleaseRadarCounts::default(),
            rows: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RadarProfileEntry {
    state: String,
}

fn profile_catalog_path() -> PathBuf {
    if let Ok(root) = std::env::var("PRONTO_PROFILE_ROOT") {
        return PathBuf::from(root).join("profile-catalog.json");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("projects")
        .join("jakyeamos-profile")
        .join("profile-catalog.json")
}

fn normalized_github_repository_name(value: &str) -> Option<String> {
    let mut normalized = value
        .trim()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_ascii_lowercase();
    if let Some(value) = normalized.strip_prefix("git@github.com:") {
        normalized = value.to_string();
    } else {
        for prefix in ["https://github.com/", "http://github.com/", "github.com/"] {
            if let Some(value) = normalized.strip_prefix(prefix) {
                normalized = value.to_string();
                break;
            }
        }
    }
    let mut parts = normalized.split('/');
    let owner = parts.next()?.trim();
    let name = parts.next()?.trim();
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(format!("{owner}/{name}"))
}

fn profile_entry_state(
    repository_name: &str,
    full_name: Option<&str>,
    visibility: &str,
) -> RadarProfileEntry {
    let path = profile_catalog_path();
    let Ok(text) = fs::read_to_string(path) else {
        return RadarProfileEntry {
            state: "unknown".to_string(),
        };
    };
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) else {
        return RadarProfileEntry {
            state: "unknown".to_string(),
        };
    };
    let expected_identity = full_name
        .and_then(normalized_github_repository_name)
        .map(|name| format!("github:{name}"));
    let Some(entries) = payload.get("entries").and_then(serde_json::Value::as_array) else {
        return RadarProfileEntry {
            state: "unknown".to_string(),
        };
    };
    let entry = entries.iter().find(|entry| {
        entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|name| name.eq_ignore_ascii_case(repository_name))
            || expected_identity.as_deref().is_some_and(|identity| {
                entry
                    .get("provider_identity")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(identity))
            })
    });
    let Some(entry) = entry else {
        return RadarProfileEntry {
            state: "unknown".to_string(),
        };
    };
    let provider_visibility = entry
        .get("provider_visibility")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let profile_visibility = entry
        .get("profile_visibility")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let disposition = entry
        .get("readme_disposition")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("defer");
    let public_url = entry
        .get("public_url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let state = if visibility == "public" {
        if provider_visibility == "public"
            && profile_visibility == "public"
            && disposition == "include"
            && public_url.starts_with("https://github.com/")
        {
            "in_sync"
        } else {
            "needs_sync"
        }
    } else if provider_visibility == "public"
        || profile_visibility == "public"
        || !public_url.is_empty()
    {
        "needs_sync"
    } else {
        "suppressed"
    };
    RadarProfileEntry {
        state: state.to_string(),
    }
}

fn radar_visibility(remote: Option<&RemoteRepositorySnapshot>) -> String {
    remote
        .and_then(|remote| remote.visibility.as_deref())
        .unwrap_or("unknown")
        .to_ascii_lowercase()
}

fn radar_visibility_freshness(
    provider_status: &ProviderStatus,
    remote: Option<&RemoteRepositorySnapshot>,
) -> String {
    let Some(remote) = remote else {
        return "unknown".to_string();
    };
    if provider_status.provider != "GitHub" || provider_status.state != "Ready" {
        return "blocked".to_string();
    }
    let Ok(observed) = DateTime::parse_from_rfc3339(&remote.last_refreshed_at) else {
        return "unknown".to_string();
    };
    let age = Utc::now().signed_duration_since(observed.with_timezone(&Utc));
    if age.num_minutes() <= RELEASE_RADAR_FRESHNESS_MINUTES {
        "fresh".to_string()
    } else {
        "stale".to_string()
    }
}

fn radar_dimension_status(
    project: Option<&showcase::ShowcaseProjectSnapshot>,
    product: bool,
) -> String {
    let Some(project) = project else {
        return "unknown".to_string();
    };
    if !product && !project.missing_materials.is_empty() {
        return "missing evidence".to_string();
    }
    let dimension = if product {
        &project.product_readiness
    } else {
        &project.demo_materials
    };
    match dimension.status.as_str() {
        "assessed" => dimension
            .score
            .map(|score| format!("assessed ({score:.1}/5)"))
            .unwrap_or_else(|| "assessed".to_string()),
        status => status.to_string(),
    }
}

fn radar_release_status(
    repository: Option<&RepositorySnapshot>,
    remote: Option<&RemoteRepositorySnapshot>,
) -> String {
    if let Some(repository) = repository {
        if repository.confirmed_release_version.is_some() {
            return "confirmed".to_string();
        }
        if repository.release_rule.is_some() {
            return "configured".to_string();
        }
    }
    if remote.is_some_and(|remote| !remote.releases.is_empty()) {
        return "provider evidence".to_string();
    }
    "unknown".to_string()
}

fn radar_row(
    provider_status: &ProviderStatus,
    repository: Option<&RepositorySnapshot>,
    remote: Option<&RemoteRepositorySnapshot>,
    showcase: &ShowcasePortfolioSnapshot,
) -> ReleaseRadarRow {
    let repository_name = repository
        .map(|repository| repository.name.clone())
        .or_else(|| remote.map(|remote| remote.name.clone()))
        .unwrap_or_else(|| "Unknown repository".to_string());
    let full_name = remote.map(|remote| remote.full_name.clone());
    let locality = match (repository, remote) {
        (Some(_), Some(_)) => "Local and remote",
        (Some(_), None) => "Local only",
        (None, Some(_)) => "GitHub only",
        (None, None) => "Unknown",
    }
    .to_string();
    let visibility = radar_visibility(remote);
    let freshness = radar_visibility_freshness(provider_status, remote);
    let project = showcase.projects.iter().find(|project| {
        project.repository_id.as_deref() == repository.map(|item| item.id.as_str())
            || project.repository_name.eq_ignore_ascii_case(&repository_name)
    });
    let profile_entry = profile_entry_state(&repository_name, full_name.as_deref(), &visibility);
    let profile_link_sync = if remote.is_none() {
        "not_applicable".to_string()
    } else if freshness != "fresh" {
        "blocked".to_string()
    } else if visibility == "public" {
        profile_entry.state.clone()
    } else if matches!(visibility.as_str(), "private" | "internal") {
        if profile_entry.state == "needs_sync" {
            "needs_sync".to_string()
        } else {
            "suppressed".to_string()
        }
    } else {
        "blocked".to_string()
    };
    let public_framing = if remote.is_none() {
        "unknown".to_string()
    } else if freshness == "stale" {
        "stale".to_string()
    } else if freshness != "fresh" {
        "blocked".to_string()
    } else if visibility == "public" && profile_link_sync == "in_sync" {
        "eligible".to_string()
    } else if matches!(visibility.as_str(), "private" | "internal") {
        "suppressed".to_string()
    } else {
        "blocked".to_string()
    };
    let product_readiness = radar_dimension_status(project, true);
    let demo_materials = radar_dimension_status(project, false);
    let release_readiness = radar_release_status(repository, remote);
    let archived = remote.is_some_and(|remote| remote.archived);
    let (next_action, blocker) = if archived {
        (
            "Review archived repository disposition".to_string(),
            Some("Archived repositories cannot be promoted through the public profile.".to_string()),
        )
    } else if remote.is_none() {
        (
            "Connect a confirmed GitHub remote".to_string(),
            Some("No provider visibility readback exists for this local repository.".to_string()),
        )
    } else if freshness == "stale" {
        (
            "Refresh GitHub visibility".to_string(),
            Some("The provider readback is outside the Release Radar freshness window.".to_string()),
        )
    } else if freshness != "fresh" {
        (
            "Resolve GitHub provider evidence".to_string(),
            Some("Visibility is unknown or the provider refresh is blocked.".to_string()),
        )
    } else if visibility == "public" && profile_link_sync == "needs_sync" {
        (
            "Sync the generated profile README".to_string(),
            Some("Public provider state is fresh, but the profile catalog or README has drifted.".to_string()),
        )
    } else if matches!(visibility.as_str(), "private" | "internal")
        && profile_link_sync == "needs_sync"
    {
        (
            "Remove the public profile link".to_string(),
            Some("Private-facing profile output still contains a public URL.".to_string()),
        )
    } else if public_framing == "eligible" {
        (
            "Review independent product, demo, and release evidence".to_string(),
            None,
        )
    } else {
        (
            "Review the blocking evidence".to_string(),
            Some("The row is not eligible for public framing.".to_string()),
        )
    };
    ReleaseRadarRow {
        id: repository
            .map(|repository| repository.id.clone())
            .or_else(|| remote.map(|remote| remote.id.clone()))
            .unwrap_or_else(|| format!("radar:{repository_name}")),
        name: repository_name,
        full_name,
        locality,
        github_url: remote.map(|remote| remote.html_url.clone()),
        visibility,
        visibility_freshness: freshness,
        public_framing,
        profile_link_sync,
        product_readiness,
        demo_materials,
        release_readiness,
        next_action,
        blocker,
        repository_id: repository.map(|repository| repository.id.clone()),
        remote_repository_id: remote.map(|remote| remote.id.clone()),
    }
}

fn build_release_radar(
    state: &StoreState,
    repositories: &[RepositorySnapshot],
    showcase: &ShowcasePortfolioSnapshot,
) -> ReleaseRadarSnapshot {
    let mut by_identity = BTreeMap::<String, (Option<usize>, Option<usize>)>::new();
    for (index, repository) in repositories.iter().enumerate() {
        let key = repository
            .remote_url
            .as_deref()
            .and_then(normalize_remote_name)
            .unwrap_or_else(|| format!("local:{}", repository.id));
        by_identity.entry(key).or_default().0 = Some(index);
    }
    for (index, remote) in state.remote_repositories.iter().enumerate() {
        let key = normalize_remote_name(&remote.full_name)
            .unwrap_or_else(|| format!("remote:{}", remote.id));
        let entry = by_identity.entry(key).or_default();
        if entry.1.is_none_or(|current| {
            state.remote_repositories[current].last_refreshed_at < remote.last_refreshed_at
        }) {
            entry.1 = Some(index);
        }
    }
    let mut rows = by_identity
        .into_values()
        .map(|(repository_index, remote_index)| {
            radar_row(
                &state.provider_status,
                repository_index.and_then(|index| repositories.get(index)),
                remote_index.and_then(|index| state.remote_repositories.get(index)),
                showcase,
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.name.to_ascii_lowercase());
    let mut counts = ReleaseRadarCounts {
        total: rows.len(),
        ..ReleaseRadarCounts::default()
    };
    for row in &rows {
        match row.locality.as_str() {
            "Local only" => counts.local_only += 1,
            "GitHub only" => counts.github_only += 1,
            "Local and remote" => counts.local_and_remote += 1,
            _ => {}
        }
        if row.visibility_freshness == "fresh" && row.visibility == "public" {
            counts.public_fresh += 1;
        }
        if row.visibility_freshness == "fresh"
            && matches!(row.visibility.as_str(), "private" | "internal")
        {
            counts.private_or_internal_fresh += 1;
        }
        if matches!(row.visibility_freshness.as_str(), "unknown" | "blocked")
            || row.visibility == "unknown"
        {
            counts.unknown_or_blocked += 1;
        }
        if row.public_framing == "eligible" {
            counts.public_framing_eligible += 1;
        }
        if row.profile_link_sync == "needs_sync" {
            counts.profile_needs_sync += 1;
        }
    }
    ReleaseRadarSnapshot {
        schema_version: RELEASE_RADAR_SCHEMA.to_string(),
        generated_at: iso_now(),
        provider_state: state.provider_status.state.clone(),
        provider_last_refresh_at: state.provider_status.last_refresh_at.clone(),
        profile_state: if profile_catalog_path().is_file() {
            "Available".to_string()
        } else {
            "Unknown".to_string()
        },
        counts,
        rows,
    }
}
