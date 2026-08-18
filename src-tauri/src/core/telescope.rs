fn repository_telescope_at(
    path: &Path,
    repository_id: &str,
    force_refresh: bool,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<crate::telescope::TelescopeProjection, String> {
    let state = load_store_read_only(path)?;
    let snapshot = snapshot_from_store(path, &state);
    let repository = snapshot
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| "Repository is not registered".to_string())?;
    let workspace = &repository.workspace;
    crate::telescope::get_or_generate_cancellable(
        path,
        &crate::telescope::TelescopeRequest {
            repository_id: &repository.id,
            repository_name: &repository.name,
            workspace_id: &workspace.id,
            workspace_path: Path::new(&workspace.path),
            branch: &workspace.branch,
            known_commit: workspace.last_commit.as_deref(),
            known_dirty: workspace.dirty,
        },
        force_refresh,
        cancellation,
    )
}

#[tauri::command]
pub async fn get_repository_telescope(
    repository_id: String,
) -> Result<crate::telescope::TelescopeProjection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        repository_telescope_at(&store_path(), &repository_id, false, None)
    })
    .await
    .map_err(|error| format!("Telescope projection task failed: {error}"))?
}

#[tauri::command]
pub async fn refresh_repository_telescope(
    repository_id: String,
) -> Result<crate::telescope::TelescopeProjection, String> {
    let cancellation = crate::telescope::begin_refresh(&repository_id);
    let task_repository_id = repository_id.clone();
    let task_cancellation = cancellation.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        repository_telescope_at(
            &store_path(),
            &task_repository_id,
            true,
            Some(task_cancellation.as_ref()),
        )
    })
    .await
    .map_err(|error| format!("Telescope refresh task failed: {error}"))?;
    crate::telescope::finish_refresh(&repository_id, &cancellation);
    result
}

#[tauri::command]
pub fn cancel_repository_telescope_refresh(repository_id: String) -> bool {
    crate::telescope::cancel_refresh(&repository_id)
}
