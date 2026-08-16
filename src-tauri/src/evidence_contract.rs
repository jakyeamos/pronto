use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const EVIDENCE_CONTRACT_STATUS_CURRENT: &str = "current";
pub const EVIDENCE_CONTRACT_STATUS_AUDIT_REQUIRED: &str = "audit_required";
pub const EVIDENCE_CONTRACT_STATUS_MISSING: &str = "missing";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EvidenceContractRepositoryStatus {
    pub contract_id: String,
    pub label: String,
    pub target_schema: String,
    pub observed_schema: Option<String>,
    pub status: String,
    pub repository_id: String,
    pub repository_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EvidenceContractFleetCoverage {
    pub contract_id: String,
    pub label: String,
    pub target_schema: String,
    pub status: String,
    pub repository_count: usize,
    pub current_repository_count: usize,
    pub legacy_repository_count: usize,
    pub missing_repository_count: usize,
    #[serde(default)]
    pub observed_schema_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub affected_repository_ids: Vec<String>,
    pub message: String,
    pub next_safe_step: String,
}

pub fn evaluate_repository_contract(
    contract_id: &str,
    label: &str,
    target_schema: &str,
    observed_schema: Option<&str>,
    repository_id: &str,
    repository_name: &str,
) -> EvidenceContractRepositoryStatus {
    let observed_schema = observed_schema
        .map(str::trim)
        .filter(|schema| !schema.is_empty())
        .map(str::to_string);
    let status = match observed_schema.as_deref() {
        Some(schema) if schema == target_schema => EVIDENCE_CONTRACT_STATUS_CURRENT,
        Some(_) => EVIDENCE_CONTRACT_STATUS_AUDIT_REQUIRED,
        None => EVIDENCE_CONTRACT_STATUS_MISSING,
    };
    let message = match observed_schema.as_deref() {
        Some(schema) if schema == target_schema => {
            format!("{label} is assessed against the current contract {target_schema}.")
        }
        Some(schema) => {
            format!("{label} was assessed against {schema}; re-audit against {target_schema}.")
        }
        None => format!("{label} has no recorded contract schema; audit against {target_schema}."),
    };
    EvidenceContractRepositoryStatus {
        contract_id: contract_id.to_string(),
        label: label.to_string(),
        target_schema: target_schema.to_string(),
        observed_schema,
        status: status.to_string(),
        repository_id: repository_id.to_string(),
        repository_name: repository_name.to_string(),
        message,
    }
}

pub fn aggregate_contract_coverage(
    contract_id: &str,
    label: &str,
    target_schema: &str,
    states: &[EvidenceContractRepositoryStatus],
) -> EvidenceContractFleetCoverage {
    let current_repository_count = states
        .iter()
        .filter(|state| state.status == EVIDENCE_CONTRACT_STATUS_CURRENT)
        .count();
    let missing_repository_count = states
        .iter()
        .filter(|state| state.status == EVIDENCE_CONTRACT_STATUS_MISSING)
        .count();
    let legacy_repository_count =
        states.len() - current_repository_count - missing_repository_count;
    let mut observed_schema_counts = BTreeMap::new();
    for state in states {
        let key = state
            .observed_schema
            .clone()
            .unwrap_or_else(|| "missing".to_string());
        *observed_schema_counts.entry(key).or_insert(0) += 1;
    }
    let affected_repository_ids = states
        .iter()
        .filter(|state| state.status != EVIDENCE_CONTRACT_STATUS_CURRENT)
        .map(|state| state.repository_id.clone())
        .collect::<Vec<_>>();
    let status = if affected_repository_ids.is_empty() {
        EVIDENCE_CONTRACT_STATUS_CURRENT
    } else {
        EVIDENCE_CONTRACT_STATUS_AUDIT_REQUIRED
    };
    let message = if status == EVIDENCE_CONTRACT_STATUS_CURRENT {
        format!(
            "All {} repositories are assessed against {target_schema}.",
            states.len()
        )
    } else {
        format!(
            "Full fleet audit required: {current_repository_count}/{} repositories are assessed against {target_schema}; {legacy_repository_count} use legacy evidence and {missing_repository_count} have no schema.",
            states.len()
        )
    };
    EvidenceContractFleetCoverage {
        contract_id: contract_id.to_string(),
        label: label.to_string(),
        target_schema: target_schema.to_string(),
        status: status.to_string(),
        repository_count: states.len(),
        current_repository_count,
        legacy_repository_count,
        missing_repository_count,
        observed_schema_counts,
        affected_repository_ids,
        message,
        next_safe_step: format!(
            "Run the owning producer's fleet audit for {target_schema}, then refresh Pronto."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(schema: Option<&str>, id: &str) -> EvidenceContractRepositoryStatus {
        evaluate_repository_contract(
            "example-contract",
            "Example evidence",
            "example/v3",
            schema,
            id,
            id,
        )
    }

    #[test]
    fn current_schema_is_current() {
        let state = state(Some("example/v3"), "repo/current");
        assert_eq!(state.status, EVIDENCE_CONTRACT_STATUS_CURRENT);
        assert_eq!(state.observed_schema.as_deref(), Some("example/v3"));
    }

    #[test]
    fn legacy_and_unknown_schemas_require_audit_without_becoming_missing() {
        for schema in ["example/v2", "unexpected/vendor-schema"] {
            let state = state(Some(schema), schema);
            assert_eq!(state.status, EVIDENCE_CONTRACT_STATUS_AUDIT_REQUIRED);
            assert_eq!(state.observed_schema.as_deref(), Some(schema));
        }
    }

    #[test]
    fn absent_schema_is_missing() {
        let state = state(None, "repo/missing");
        assert_eq!(state.status, EVIDENCE_CONTRACT_STATUS_MISSING);
    }

    #[test]
    fn mixed_fleet_requires_a_full_audit_and_preserves_counts() {
        let states = vec![
            state(Some("example/v3"), "repo/current"),
            state(Some("example/v2"), "repo/legacy"),
            state(None, "repo/missing"),
        ];
        let fleet = aggregate_contract_coverage(
            "example-contract",
            "Example evidence",
            "example/v3",
            &states,
        );
        assert_eq!(fleet.status, EVIDENCE_CONTRACT_STATUS_AUDIT_REQUIRED);
        assert_eq!(fleet.current_repository_count, 1);
        assert_eq!(fleet.legacy_repository_count, 1);
        assert_eq!(fleet.missing_repository_count, 1);
        assert_eq!(fleet.affected_repository_ids.len(), 2);
    }
}
