    #[test]
    fn telescope_tasks_use_the_existing_remediation_lifecycle() {
        let nonce = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pronto-telescope-remediation-{}-{}",
            std::process::id(),
            nonce
        ));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/main.ts"),
            "export function search(query: string) { return query.trim(); }\n",
        )
        .unwrap();
        let mut repository = fixture_repository("telescope-readiness");
        repository.path = root.to_string_lossy().to_string();
        repository.workspace.path = repository.path.clone();
        let projection = crate::telescope::get_or_generate(
            &root.join("pronto.sqlite"),
            &crate::telescope::TelescopeRequest {
                repository_id: &repository.id,
                repository_name: &repository.name,
                workspace_id: &repository.workspace.id,
                workspace_path: &root,
                branch: &repository.workspace.branch,
                known_commit: repository.workspace.last_commit.as_deref(),
                known_dirty: false,
            },
            true,
        )
        .unwrap();
        assert!(!projection.knowledge_tasks.is_empty());

        let mut run = empty_run();
        assert!(sync_telescope_readiness(&mut run, &repository, &projection));
        let plan = run
            .plans
            .iter_mut()
            .find(|plan| plan.repository_id == repository.id)
            .unwrap();
        let action = plan
            .actions
            .iter_mut()
            .find(|action| action.domain == "telescope_readiness")
            .unwrap();
        let stable_key = action.stable_key.clone();
        action.status = "in_progress".to_string();

        assert!(sync_telescope_readiness(&mut run, &repository, &projection));
        let projected = run
            .plans
            .iter()
            .flat_map(|plan| plan.actions.iter())
            .find(|action| action.stable_key == stable_key)
            .unwrap();
        assert_eq!(projected.status, "in_progress");
        assert_eq!(projected.domain, "telescope_readiness");
        assert!(projected.evidence.iter().all(|item| item
            .report_path
            .as_deref()
            .is_none_or(|path| !Path::new(path).is_absolute())));

        std::fs::remove_dir_all(root).unwrap();
    }
