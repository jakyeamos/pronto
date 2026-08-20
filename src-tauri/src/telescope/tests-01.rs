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
