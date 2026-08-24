#[test]
fn agent_attention_exposes_project_compass_afflictions_and_omits_healthy_contracts() {
    let root = fixture_root();
    fixture_repository(&root);
    let store = root.join("registry.db");
    let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
        .expect("fixture portfolio should scan");
    let missing = agent_attention_report(&snapshot)
        .items
        .into_iter()
        .find(|item| item.category == "project_compass")
        .expect("missing contract should require attention");

    assert_eq!(missing.category, "project_compass");
    assert_eq!(missing.status, "Missing");
    assert!(missing.summary.contains("contract is missing"));
    assert!(missing.evidence.iter().any(|item| {
        item.label == "Contract status"
            && item.status.as_deref() == Some("Missing")
            && item
                .report_path
                .as_deref()
                .is_some_and(|path| path.ends_with(".project-compass/contract.json"))
    }));

    let repository = snapshot
        .repositories
        .first_mut()
        .expect("fixture repository should exist");
    repository.project_compass.status = "Invalid".to_string();
    repository.project_compass.error = Some("fixture contract error".to_string());
    let invalid = agent_project_compass_attention(repository)
        .expect("invalid contract should require attention");
    assert_eq!(invalid.status, "Invalid");
    assert!(invalid.evidence.iter().any(|item| {
        item.label == "Contract status"
            && item.value.as_deref() == Some("fixture contract error")
    }));

    repository.project_compass.status = "Ready".to_string();
    repository.project_compass.error = None;
    repository.project_compass.mvp.total_pillar_count = 3;
    repository.project_compass.mvp.covered_pillar_count = 2;
    repository.project_compass.mvp.scored_outcome_count = 4;
    let incomplete = agent_project_compass_attention(repository)
        .expect("incomplete coverage should require attention");
    assert_eq!(incomplete.status, "Attention");
    assert!(incomplete.summary.contains("incomplete (2/3)"));
    assert!(incomplete.evidence.iter().any(|item| {
        item.label == "MVP pillar coverage" && item.status.as_deref() == Some("Incomplete")
    }));
    let action = agent_next_action(&incomplete);
    assert_eq!(action.recommended_projection, "repo");
    assert!(action.next_safe_step.contains("Project Compass"));

    repository.project_compass.mvp.covered_pillar_count = 3;
    repository.project_compass.open_blockers = 1;
    repository.project_compass.open_drift = 2;
    let open_items = agent_project_compass_attention(repository)
        .expect("open Compass items should require attention");
    assert!(open_items.summary.contains("1 open blocker(s)"));
    assert!(open_items.summary.contains("2 open drift item(s)"));
    assert!(open_items
        .evidence
        .iter()
        .any(|item| item.label == "Open blockers"));
    assert!(open_items
        .evidence
        .iter()
        .any(|item| item.label == "Open drift"));

    repository.project_compass.open_blockers = 0;
    repository.project_compass.open_drift = 0;
    assert!(agent_project_compass_attention(repository).is_none());
    assert!(!agent_attention_report(&snapshot)
        .items
        .iter()
        .any(|item| item.category == "project_compass"));

    fs::remove_dir_all(root).expect("Compass fixture should be removable");
}

#[test]
fn agent_attention_can_be_scoped_to_one_repository() {
    let root = fixture_root();
    fixture_repository(&root);
    let store = root.join("registry.db");
    let mut snapshot = register_root_and_scan(&store, &root.to_string_lossy())
        .expect("fixture portfolio should scan");
    let selected_id = snapshot.repositories[0].id.clone();
    let mut second = snapshot.repositories[0].clone();
    second.id = "repository:second".to_string();
    second.name = "second".to_string();
    second.path = root.join("second").to_string_lossy().to_string();
    snapshot.repositories.push(second);

    let report = agent_attention_report_for_query(&snapshot, Some(&selected_id))
        .expect("selected repository should resolve");
    assert!(!report.items.is_empty());
    assert!(report
        .items
        .iter()
        .all(|item| item.repository_id == selected_id));
    assert!(agent_attention_report_for_query(&snapshot, Some("missing-repository")).is_err());

    fs::remove_dir_all(root).expect("scoped attention fixture should be removable");
}
