    #[test]
    fn authored_map_drift_and_invalid_flow_are_visible_without_mutation() {
        let root = fixture_root("authored-drift");
        fs::write(root.join("src/main.ts"), "export const ready = true;").unwrap();
        fs::create_dir_all(root.join(".pronto")).unwrap();
        fs::write(
            root.join(NARRATIVE_MANIFEST_PATH),
            r#"{
              "status":"reviewed",
              "topologyFingerprint":"topology-does-not-match",
              "groups":[
                {"id":"one","label":"One"},
                {"id":"two","label":"Two"},
                {"id":"three","label":"Three"},
                {"id":"four","label":"Four"}
              ],
              "nodes":[{"id":"broken","label":"Broken","files":["src/main.ts"]}],
              "flows":[{"id":"broken-flow","label":"Broken","nodeIds":["broken"],"edgeIds":["missing-edge"],"primary":true}]
            }"#,
        )
        .unwrap();
        let manifest_before = fs::read_to_string(root.join(NARRATIVE_MANIFEST_PATH)).unwrap();
        let projection = get_or_generate(
            &root.join("registry.db"),
            &TelescopeRequest {
                repository_id: "repo",
                repository_name: "Fixture",
                workspace_id: "workspace",
                workspace_path: &root,
                branch: "dev",
                known_commit: None,
                known_dirty: false,
            },
            true,
        )
        .unwrap();
        assert_eq!(projection.narrative.status, "stale");
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.code == "narrative-drift-detected"));
        assert!(projection
            .warnings
            .iter()
            .any(|warning| warning.code == "narrative-flow-unmapped"));
        assert!(projection
            .narrative
            .authored_flows
            .iter()
            .any(|flow| flow.id == "broken-flow" && flow.status == "partial"));
        assert_eq!(
            fs::read_to_string(root.join(NARRATIVE_MANIFEST_PATH)).unwrap(),
            manifest_before
        );
        let _ = fs::remove_dir_all(root);
    }
