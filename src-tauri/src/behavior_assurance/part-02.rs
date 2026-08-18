impl Default for BehaviorAssuranceRepositoryState {
    fn default() -> Self {
        Self {
            schema: ASSESSMENT_SCHEMA.to_string(),
            applicability: "unknown".to_string(),
            state: "unknown".to_string(),
            contract_status: "missing".to_string(),
            contract_schema: None,
            edge_profile_status: "missing".to_string(),
            result_status: "unknown".to_string(),
            freshness: "unknown".to_string(),
            release_ready: false,
            score: None,
            contract_path: ".pronto/behavior-assurance.json".to_string(),
            receipt_directory: ".quality-runner/behavior-assurance/receipts".to_string(),
            contract_digest: None,
            target_branch: None,
            target_commit: None,
            observed_at: None,
            required_scenario_count: 0,
            passed_scenario_count: 0,
            accepted_defect_count: 0,
            receipt_count: 0,
            verified: Vec::new(),
            coverage: BehaviorCoverage::default(),
            gaps: vec![BehaviorAssuranceGap {
                kind: "evidence_unavailable".to_string(),
                message: "No current Quality Runner behavior-assurance projection is available."
                    .to_string(),
                behavior_id: None,
                scenario_id: None,
            }],
            detail: None,
            next_step: "Run and publish a current Quality Runner fleet audit.".to_string(),
        }
    }
}

impl BehaviorAssuranceRepositoryState {
    pub fn normalize_state(&mut self) {
        if self.state != "unknown" && !self.state.is_empty() {
            return;
        }
        self.state = inferred_state(self).to_string();
    }

    pub fn project_to_target(&mut self, branch: &str, commit: &str) {
        if self.applicability == "not_applicable"
            || self.receipt_count == 0
            || self.target_branch.is_none()
            || self.target_commit.is_none()
            || (self.target_branch.as_deref() == Some(branch)
                && self.target_commit.as_deref() == Some(commit))
        {
            return;
        }
        self.release_ready = false;
        self.result_status = "unknown".to_string();
        self.freshness = "stale".to_string();
        self.state = "stale".to_string();
        mark_coverage_stale(&mut self.coverage);
        self.gaps.push(BehaviorAssuranceGap {
            kind: "target_mismatch".to_string(),
            message: format!(
                "Behavior receipts target {} @ {}, not {branch} @ {commit}.",
                self.target_branch.as_deref().unwrap_or("unknown"),
                self.target_commit.as_deref().unwrap_or("unknown")
            ),
            behavior_id: None,
            scenario_id: None,
        });
        self.next_step =
            "Publish Quality Runner behavior assurance for the configured target.".to_string();
    }
}

fn inferred_state(state: &BehaviorAssuranceRepositoryState) -> &'static str {
    if state.contract_status == "missing"
        && state.gaps.iter().any(|gap| gap.kind == "contract_missing")
    {
        return "missing_contract";
    }
    if state.contract_status == "invalid" || state.result_status == "blocked" {
        return "blocked";
    }
    if state.applicability == "not_applicable" {
        return "not_applicable";
    }
    if state.schema == "quality-runner-behavior-assurance/v1"
        || state.contract_schema.as_deref() == Some("pronto-behavior-assurance/v1")
        || state.edge_profile_status == "legacy"
    {
        return "legacy_v1";
    }
    if state.result_status == "failed" {
        return "failed";
    }
    if state.freshness == "stale" {
        return "stale";
    }
    if state.passed_scenario_count > 0
        && state.passed_scenario_count < state.required_scenario_count
    {
        return "partially_verified";
    }
    if matches!(
        state.edge_profile_status.as_str(),
        "missing" | "unprofiled" | "partially_profiled"
    ) {
        return "unprofiled";
    }
    if state.release_ready {
        return "current";
    }
    "unknown"
}

fn mark_coverage_stale(coverage: &mut BehaviorCoverage) {
    coverage.counts.stale += coverage.counts.verified;
    coverage.counts.verified = 0;
    for counts in coverage
        .per_tier
        .values_mut()
        .chain(coverage.per_edge_category.values_mut())
    {
        counts.stale += counts.verified;
        counts.verified = 0;
    }
    for scenario in &mut coverage.scenarios {
        if scenario.status == "verified" {
            scenario.status = "stale".to_string();
            scenario.freshness = "stale".to_string();
        }
    }
}
