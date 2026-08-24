    #[test]
    fn action_inventory_projects_behavior_contract_without_creating_a_second_source_of_truth() {
        let root = fixture_root("action-behavior-merge");
        fs::write(
            root.join("src/main.ts"),
            "import { load } from './services/load';\nexport function main() { return load(); }",
        )
        .unwrap();
        fs::write(
            root.join("src/services/load.ts"),
            "export function load() { return true; }",
        )
        .unwrap();
        fs::create_dir_all(root.join(".pronto")).unwrap();
        fs::write(
            root.join(".pronto/behavior-assurance.json"),
            r#"{
              "schema":"pronto-behavior-assurance/v2",
              "applicability":"applicable",
              "behaviors":[
                {
                  "id":"behavior-linked",
                  "title":"Loading remains conservative",
                  "tier":0,
                  "automation":"permanent",
                  "change_triggers":["src/main.ts"],
                  "invariants":["Missing evidence never becomes verified."],
                  "scenarios":[{"id":"linked-scenario","verification_level":"automated"}]
                },
                {
                  "id":"behavior-unmapped",
                  "title":"The loader preserves its boundary",
                  "tier":1,
                  "automation":"on_demand",
                  "change_triggers":["src/services/load.ts"],
                  "invariants":["The loader remains isolated."],
                  "scenarios":[{"id":"unmapped-scenario","verification_level":"direct_surface"}]
                }
              ]
            }"#,
        )
        .unwrap();
        fs::write(
            root.join(NARRATIVE_MANIFEST_PATH),
            r#"{
              "schemaVersion":"pronto-telescope-map/v1",
              "status":"reviewed",
              "groups":[
                {"id":"surface","label":"Surface","pathPrefixes":["src"],"status":"reviewed"},
                {"id":"runtime","label":"Runtime","pathPrefixes":["runtime"],"status":"reviewed"},
                {"id":"data","label":"Data","pathPrefixes":["data"],"status":"reviewed"},
                {"id":"delivery","label":"Delivery","pathPrefixes":["delivery"],"status":"reviewed"}
              ],
              "nodes":[
                {"id":"main-story","label":"Request","groupId":"surface","files":["src/main.ts"],"status":"reviewed"},
                {"id":"loader-story","label":"Loader","groupId":"surface","files":["src/services/load.ts"],"status":"reviewed"}
              ],
              "actions":[
                {
                  "id":"inspect-loading",
                  "label":"Inspect loading assurance",
                  "verb":"Inspect",
                  "category":"evidence",
                  "whatItDoes":"Focuses the city on the loading boundary.",
                  "howItsBuilt":"Maps the reviewed action to the canonical behavior contract.",
                  "files":["src/main.ts"],
                  "nodeIds":["main-story"],
                  "behaviorId":"behavior-linked",
                  "status":"reviewed",
                  "readOnly":true,
                  "guarded":true
                },
                {
                  "id":"inspect-unresolved",
                  "label":"Inspect unresolved assurance",
                  "verb":"Inspect",
                  "category":"evidence",
                  "whatItDoes":"Keeps an unresolved behavior link visible.",
                  "howItsBuilt":"The action cannot claim behavioral evidence until its behavior ID is declared.",
                  "files":["src/services/load.ts"],
                  "nodeIds":["loader-story"],
                  "behaviorId":"behavior-missing",
                  "status":"reviewed",
                  "readOnly":true,
                  "guarded":true
                }
              ]
            }"#,
        )
        .unwrap();

        let mut projection = get_or_generate(
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
        let linked = projection
            .actions
            .iter()
            .find(|action| action.id == "inspect-loading")
            .unwrap()
            .clone();
        assert_eq!(linked.behavior_id.as_deref(), Some("behavior-linked"));
        assert_eq!(linked.scenario_ids, vec!["linked-scenario"]);
        assert_eq!(linked.behavior_state, "declared");
        assert_eq!(linked.behavior_verification, "automated");
        let unresolved = projection
            .actions
            .iter()
            .find(|action| action.id == "inspect-unresolved")
            .unwrap();
        assert_eq!(unresolved.behavior_id.as_deref(), Some("behavior-missing"));
        assert_eq!(unresolved.behavior_state, "unresolved");
        assert_eq!(unresolved.behavior_verification, "behavior-not-found");
        assert_eq!(unresolved.status, "partial");
        assert!(projection
            .actions
            .iter()
            .any(|action| action.id == "behavior-unmapped"
                && action.provenance == "behavior-assurance-contract"));
        assert_eq!(projection.action_coverage.authored, 2);
        assert_eq!(projection.action_coverage.behavior_backed, 2);
        assert_eq!(projection.action_coverage.unprofiled, 2);
        assert!(projection.warnings.iter().any(|warning| {
            warning.code == "action-behavior-unresolved"
                && warning.message.contains("behavior-missing")
        }));
        let json = serde_json::to_string(&projection).unwrap();
        assert!(!json.contains(root.to_string_lossy().as_ref()));
        assert!(!json.contains("return true"));

        fs::write(
            root.join("src/services/load.ts"),
            "export function load() { return false; }",
        )
        .unwrap();
        projection = get_or_generate(
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
        let refreshed_linked = projection
            .actions
            .iter()
            .find(|action| action.id == "inspect-loading")
            .unwrap();
        assert_eq!(refreshed_linked.what_it_does, linked.what_it_does);
        assert_eq!(refreshed_linked.behavior_id, linked.behavior_id);
        let _ = fs::remove_dir_all(root);
    }
