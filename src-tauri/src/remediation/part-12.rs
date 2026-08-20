pub fn export_run(run: &RemediationRun, output_dir: &Path) -> Result<RemediationExport, String> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("Could not create remediation export directory: {error}"))?;
    let mut files = Vec::new();
    let manifest_path = output_dir.join("remediation-run.json");
    write_json(&manifest_path, run)?;
    files.push(manifest_path.to_string_lossy().to_string());
    let markdown_path = output_dir.join("repository-remediation-order.md");
    fs::write(&markdown_path, render_active_queue_markdown(run)).map_err(|error| {
        format!(
            "Could not write remediation queue {}: {error}",
            markdown_path.display()
        )
    })?;
    files.push(markdown_path.to_string_lossy().to_string());
    for plan in &run.plans {
        let file_name = format!("repo-{}.json", safe_file_component(&plan.repository_id));
        let plan_path = output_dir.join(file_name);
        write_json(&plan_path, plan)?;
        files.push(plan_path.to_string_lossy().to_string());
    }
    if !run.closures.is_empty() {
        let closures_path = output_dir.join("remediation-closures.json");
        write_json(&closures_path, &run.closures)?;
        files.push(closures_path.to_string_lossy().to_string());
    }
    Ok(RemediationExport {
        run_id: run.id.clone(),
        output_path: output_dir.to_string_lossy().to_string(),
        files,
    })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Could not encode remediation export: {error}"))?;
    fs::write(path, contents).map_err(|error| {
        format!(
            "Could not write remediation export {}: {error}",
            path.display()
        )
    })
}

fn safe_file_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}
