#[test]
    fn narrative_validation_has_no_arbitrary_district_quota() {
        let manifest = TelescopeManifest {
            groups: (0..12)
                .map(|index| TelescopeNarrativeGroup {
                    id: format!("district-{index}"),
                    label: format!("District {index}"),
                    status: "draft".to_string(),
                    ..TelescopeNarrativeGroup::default()
                })
                .collect(),
            ..TelescopeManifest::default()
        };
        let mut warnings = Vec::new();
        let mut drift_warnings = Vec::new();
        validate_manifest_shape(&manifest, &[], &mut warnings, &mut drift_warnings);

        assert!(!warnings
            .iter()
            .any(|warning| warning.code == "narrative-group-range"));
    }
    #[test]
    fn applicability_requires_a_reason_and_unknown_remains_unknown() {
        let mut narrative = TelescopeNarrative::default();
        narrative.status = "draft".to_string();
        narrative.applicability = vec![
            TelescopeApplicabilityDecision {
                requirement: "constraints".to_string(),
                state: "not_applicable".to_string(),
                reason: String::new(),
                status: "draft".to_string(),
            },
            TelescopeApplicabilityDecision {
                requirement: "movement".to_string(),
                state: "unknown".to_string(),
                reason: "The payload boundary is still being investigated.".to_string(),
                status: "draft".to_string(),
            },
        ];
        let (readiness, _, _, tasks) = assess_map_readiness(
            &narrative,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            "fingerprint",
            false,
        );
        assert_eq!(readiness.state, "needs_information");
        assert!(readiness.requirements.iter().any(|requirement| {
            requirement.key == "constraints" && requirement.status == "missing"
        }));
        assert!(readiness.requirements.iter().any(|requirement| {
            requirement.key == "movement" && requirement.status == "missing"
        }));
        assert!(tasks.iter().any(|task| {
            task.stable_gap_key == "telescope-readiness:movement"
                && task.allowed_responses.contains(&"unknown".to_string())
        }));

        narrative.applicability[0].reason =
            "This static library has no operational failure boundary at this level.".to_string();
        let (readiness, _, _, _) = assess_map_readiness(
            &narrative,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            "fingerprint",
            false,
        );
        assert!(readiness.requirements.iter().any(|requirement| {
            requirement.key == "constraints"
                && requirement.status == "not_applicable"
                && !requirement.reason.is_empty()
        }));
    }
