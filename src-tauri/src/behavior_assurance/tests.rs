    fn normalizes_legacy_missing_and_not_applicable_states_without_collapsing_them() {
        let mut missing = BehaviorAssuranceRepositoryState {
            gaps: vec![BehaviorAssuranceGap {
                kind: "contract_missing".to_string(),
                ..BehaviorAssuranceGap::default()
            }],
            ..BehaviorAssuranceRepositoryState::default()
        };
        missing.normalize_state();
        assert_eq!(missing.state, "missing_contract");

        let mut legacy = BehaviorAssuranceRepositoryState {
            schema: "quality-runner-behavior-assurance/v1".to_string(),
            applicability: "applicable".to_string(),
            contract_status: "current".to_string(),
            freshness: "stale".to_string(),
            ..BehaviorAssuranceRepositoryState::default()
        };
        legacy.normalize_state();
        assert_eq!(legacy.state, "legacy_v1");

        let mut not_applicable = BehaviorAssuranceRepositoryState {
            applicability: "not_applicable".to_string(),
            contract_status: "current".to_string(),
            schema: "quality-runner-behavior-assurance/v1".to_string(),
            ..BehaviorAssuranceRepositoryState::default()
        };
        not_applicable.normalize_state();
        assert_eq!(not_applicable.state, "not_applicable");
    }

    #[test]
    fn target_mismatch_removes_release_and_edge_freshness_claims() {
        let mut state = BehaviorAssuranceRepositoryState {
            applicability: "applicable".to_string(),
            contract_status: "current".to_string(),
            result_status: "passed".to_string(),
            freshness: "current".to_string(),
            release_ready: true,
            target_branch: Some("dev".to_string()),
            target_commit: Some("abc".to_string()),
            receipt_count: 1,
            gaps: Vec::new(),
            coverage: BehaviorCoverage {
                counts: BehaviorCoverageCounts {
                    total: 1,
                    profiled: 1,
                    verified: 1,
                    ..BehaviorCoverageCounts::default()
                },
                scenarios: vec![BehaviorScenarioCoverage {
                    status: "verified".to_string(),
                    freshness: "current".to_string(),
                    ..BehaviorScenarioCoverage::default()
                }],
                ..BehaviorCoverage::default()
            },
            ..BehaviorAssuranceRepositoryState::default()
        };

        state.project_to_target("dev", "def");

        assert!(!state.release_ready);
        assert_eq!(state.freshness, "stale");
        assert_eq!(state.coverage.counts.verified, 0);
        assert_eq!(state.coverage.counts.stale, 1);
        assert_eq!(state.coverage.scenarios[0].status, "stale");
        assert_eq!(state.gaps[0].kind, "target_mismatch");
    }

    #[test]
    fn missing_provenance_is_not_misclassified_as_a_stale_receipt() {
        let mut state = BehaviorAssuranceRepositoryState::default();

        state.project_to_target("dev", "abc");

        assert_eq!(state.freshness, "unknown");
        assert_eq!(state.coverage.counts.stale, 0);
        assert_eq!(state.gaps[0].kind, "evidence_unavailable");
        assert!(!state.gaps.iter().any(|gap| gap.kind == "target_mismatch"));
        assert!(!matches_filter(&state, Some("stale")));
    }
