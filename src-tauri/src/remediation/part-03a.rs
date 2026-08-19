fn closure_from_transition(
    repository: &RepositorySnapshot,
    previous: &RemediationPlan,
    current: &RemediationPlan,
    closed_at: &str,
    source_refresh_id: Option<&str>,
) -> RemediationClosure {
    let mut closure = closure_from_plan(current, closed_at, source_refresh_id);
    closure.resolved_action_count = previous.actions.len();
    closure.last_evidence_at =
        latest_plan_evidence_at(current).or_else(|| Some(repository.last_scan_at.clone()));
    if current.actions.is_empty() {
        closure.summary = format!(
            "Fresh evidence removed {} prior action(s) from the active remediation queue.",
            previous.actions.len()
        );
    }
    closure
}

fn deduplicate_closures(closures: &mut Vec<RemediationClosure>) {
    closures.sort_by(|left, right| {
        right
            .closed_at
            .cmp(&left.closed_at)
            .then_with(|| left.repository_name.cmp(&right.repository_name))
    });
    let mut seen = std::collections::HashSet::new();
    closures.retain(|closure| seen.insert(closure.id.clone()));
}
