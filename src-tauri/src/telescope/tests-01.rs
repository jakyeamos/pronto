    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pronto-telescope-{name}-{nonce}"));
        fs::create_dir_all(root.join("src/services")).unwrap();
        root
    }

    fn initialize_git_index(root: &Path) {
        for arguments in [vec!["init"], vec!["add", "."]] {
            let status = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(arguments)
                .status()
                .unwrap();
            assert!(status.success());
        }
    }

    #[test]
    fn extracts_typescript_topology_without_source_bodies_or_absolute_paths() {
        let root = fixture_root("typescript");
        fs::write(root.join("src/main.ts"), "import { load } from './services/load';\nexport interface RequestShape { id: string }\nexport function main() { return load(); }").unwrap();
        fs::write(
            root.join("src/services/load.ts"),
            "export function load() { return true; }",
        )
        .unwrap();
        let database = root.join("registry.db");
        let repository_id = format!("repository:{}", root.display());
        let workspace_id = format!("workspace:{}", root.display());
        let request = TelescopeRequest {
            repository_id: &repository_id,
            repository_name: "Fixture",
            workspace_id: &workspace_id,
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };
        let projection = get_or_generate(&database, &request, true).unwrap();
        assert_eq!(projection.schema_version, SCHEMA_VERSION);
        assert_eq!(projection.coverage.supported_source_files, 2);
        assert_eq!(projection.edges.len(), 1);
        assert!(projection
            .nodes
            .iter()
            .any(|node| node.data_shapes.contains(&"RequestShape".to_string())));
        let json = serde_json::to_string(&projection).unwrap();
        assert!(!json.contains(root.to_string_lossy().as_ref()));
        assert!(!json.contains("return true"));
        let connection = Connection::open(&database).unwrap();
        let cached: (String, String) = connection
            .query_row(
                "SELECT repository_id, payload_json FROM telescope_cache LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(!cached.0.contains('/'));
        assert!(!cached.1.contains(root.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unsupported_languages_are_visible_as_partial_generic_topology() {
        let root = fixture_root("partial");
        fs::write(root.join("src/main.py"), "print('hello')").unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: None,
            known_dirty: false,
        };
        let projection = get_or_generate(&database, &request, true).unwrap();
        assert_eq!(projection.coverage.partial_source_files, 1);
        assert_eq!(projection.coverage.confidence, "partial");
        assert_eq!(projection.nodes[0].summary_status, "derived");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn matching_workspace_fingerprint_reuses_cache() {
        let root = fixture_root("cache");
        fs::write(root.join("src/main.ts"), "export const ready = true;").unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };
        let first = get_or_generate(&database, &request, true).unwrap();
        let second = get_or_generate(&database, &request, false).unwrap();
        assert_eq!(
            first.binding.workspace_fingerprint,
            second.binding.workspace_fingerprint
        );
        assert_eq!(second.freshness.cache, "hit");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dynamic_double_quoted_imports_and_cycles_remain_inspectable() {
        let root = fixture_root("dynamic-cycle");
        fs::write(
            root.join("src/main.ts"),
            "export async function main() { return import(\"./services/load\"); }",
        )
        .unwrap();
        fs::write(
            root.join("src/services/load.ts"),
            "import { main } from \"../main\";\nexport const load = main;",
        )
        .unwrap();
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
        assert_eq!(projection.edges.len(), 2);
        assert!(projection.edges.iter().any(|edge| edge.kind == "dynamic"));
        assert!(projection.flows.iter().all(|flow| flow.node_ids.len() <= 6));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn identifiers_are_deterministic_across_forced_regeneration() {
        let root = fixture_root("deterministic");
        fs::create_dir_all(root.join("packages/client/src")).unwrap();
        fs::write(
            root.join("packages/client/src/index.ts"),
            "export const client = true;",
        )
        .unwrap();
        fs::write(root.join("src/main.py"), "print('partial')").unwrap();
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };
        let first = get_or_generate(&root.join("registry.db"), &request, true).unwrap();
        let second = get_or_generate(&root.join("registry.db"), &request, true).unwrap();
        assert_eq!(
            first.nodes.iter().map(|node| &node.id).collect::<Vec<_>>(),
            second.nodes.iter().map(|node| &node.id).collect::<Vec<_>>()
        );
        assert_eq!(
            first
                .groups
                .iter()
                .map(|group| &group.id)
                .collect::<Vec<_>>(),
            second
                .groups
                .iter()
                .map(|group| &group.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(first.coverage.partial_source_files, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dirty_content_changes_invalidate_a_matching_status_shape() {
        let root = fixture_root("dirty-cache");
        fs::write(root.join("src/main.ts"), "export const value = 1;").unwrap();
        initialize_git_index(&root);
        fs::write(root.join("src/main.ts"), "export const value = 2;").unwrap();
        let database = std::env::temp_dir().join(format!(
            "pronto-telescope-cache-{}.db",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: None,
            known_dirty: true,
        };
        let first = get_or_generate(&database, &request, false).unwrap();
        fs::write(root.join("src/main.ts"), "export const value = 3;").unwrap();
        let second = get_or_generate(&database, &request, false).unwrap();
        assert_ne!(
            first.binding.dirty_state_fingerprint,
            second.binding.dirty_state_fingerprint
        );
        assert_eq!(second.freshness.cache, "miss");
        let _ = fs::remove_file(database);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cancelled_refresh_does_not_generate_or_cache_a_partial_projection() {
        let root = fixture_root("cancelled-refresh");
        fs::write(root.join("src/main.ts"), "export const value = 1;").unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: None,
            known_dirty: false,
        };
        let cancellation = AtomicBool::new(true);

        let error = get_or_generate_cancellable(&database, &request, true, Some(&cancellation))
            .unwrap_err();

        assert_eq!(error, "Telescope refresh cancelled.");
        assert!(!database.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn authored_manifest_overlays_meaning_and_survives_measured_refresh() {
        let root = fixture_root("authored-map");
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
            root.join(NARRATIVE_MANIFEST_PATH),
            r#"{
              "schemaVersion": "pronto-telescope-map/v1",
              "status": "reviewed",
              "visualModelVersion": "pronto-telescope-city/v1",
              "groups": [
                {"id":"surface","label":"Surface","pathPrefixes":["src"],"visualArchetype":"district","status":"reviewed"},
                {"id":"runtime","label":"Runtime","pathPrefixes":["runtime"],"visualArchetype":"district","status":"reviewed"},
                {"id":"data","label":"Data","pathPrefixes":["data"],"visualArchetype":"district","status":"reviewed"},
                {"id":"delivery","label":"Delivery","pathPrefixes":["delivery"],"visualArchetype":"district","status":"reviewed"}
              ],
              "nodes": [
                {
                  "id":"request-story",
                  "label":"Request gateway",
                  "whatItDoes":"Receives a user request and starts the loading story.",
                  "howItsBuilt":"A thin route module delegates to the loader through an explicit import.",
                  "files":["src/main.ts"],
                  "visualArchetype":"tower",
                  "status":"reviewed"
                },
                {
                  "id":"loader-story",
                  "label":"Loader",
                  "whatItDoes":"Loads the requested resource for the active story.",
                  "howItsBuilt":"A focused TypeScript module keeps the loading boundary small.",
                  "files":["src/services/load.ts"],
                  "visualArchetype":"cube",
                  "status":"reviewed"
                }
              ],
              "edges": [
                {
                  "id":"request-to-loader",
                  "sourceFile":"src/main.ts",
                  "targetFile":"src/services/load.ts",
                  "kind":"data",
                  "label":"request",
                  "railKind":"data",
                  "status":"reviewed"
                }
              ],
              "flows": [
                {
                  "id":"primary-request",
                  "label":"User request",
                  "kind":"request",
                  "nodeIds":["request-story","loader-story"],
                  "edgeIds":["request-to-loader"],
                  "dataShape":"request",
                  "status":"reviewed",
                  "primary":true
                }
              ],
              "primaryFlowId":"primary-request"
            }"#,
        )
        .unwrap();
        let database = root.join("registry.db");
        let request = TelescopeRequest {
            repository_id: "repo",
            repository_name: "Fixture",
            workspace_id: "workspace",
            workspace_path: &root,
            branch: "dev",
            known_commit: Some("abc123"),
            known_dirty: false,
        };

        let first = get_or_generate(&database, &request, true).unwrap();
        assert_eq!(first.narrative.status, "reviewed");
        assert_eq!(first.narrative.coverage.coverage_percent, 100);
        assert_eq!(
            first.narrative.primary_flow_id.as_deref(),
            Some("primary-request")
        );
        assert!(first
            .groups
            .iter()
            .any(|group| group.id == "surface" && group.label == "Surface"));
        assert!(first
            .nodes
            .iter()
            .filter(|node| node
                .source_paths
                .iter()
                .any(|path| path == "src/main.ts" || path == "src/services/load.ts"))
            .all(|node| node.group_id == "surface"));
        assert!(first
            .warnings
            .iter()
            .all(|warning| warning.code != "narrative-flow-unmapped"));
        let request_node = first
            .nodes
            .iter()
            .find(|node| node.visual_building_id.as_deref() == Some("request-story"))
            .unwrap();
        assert_eq!(
            request_node.semantic_summary,
            "Receives a user request and starts the loading story."
        );
        assert_eq!(request_node.summary_status, "reviewed");
        assert_eq!(request_node.visual_archetype, "tower");
        assert!(first
            .edges
            .iter()
            .any(|edge| edge.label == "request" && edge.rail_kind == "data"));
        assert!(first
            .flows
            .iter()
            .any(|flow| flow.id == "primary-request" && flow.primary));

        fs::write(
            root.join("src/services/load.ts"),
            "export function load() { return true; }\nexport const refreshed = true;",
        )
        .unwrap();
        let refreshed = get_or_generate(&database, &request, true).unwrap();
        let refreshed_request_node = refreshed
            .nodes
            .iter()
            .find(|node| node.visual_building_id.as_deref() == Some("request-story"))
            .unwrap();
        assert_eq!(
            refreshed_request_node.semantic_summary,
            request_node.semantic_summary
        );
        assert!(refreshed
            .nodes
            .iter()
            .any(|node| node.measured_lines == 2
                && node.source_paths == vec!["src/services/load.ts"]));
        let json = serde_json::to_string(&refreshed).unwrap();
        assert!(!json.contains("return true"));
        assert!(!json.contains(root.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(root);
    }
