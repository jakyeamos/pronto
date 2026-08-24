use crate::core::RepositorySnapshot;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ASSESSMENT_SCHEMA: &str = "quality-runner-behavior-assurance/v2";
pub const SUMMARY_SCHEMA: &str = "quality-runner-behavior-assurance-summary/v2";
pub const AUDIT_SCHEMA: &str = "pronto-behavior-assurance-audit/v2";
pub const CONTRACT_SCHEMA: &str = "pronto-behavior-assurance/v2";
pub const CONTRACT_PATH: &str = ".pronto/behavior-assurance.json";

fn default_behavior_assurance_state() -> String {
    "unknown".to_string()
}

include!("behavior_assurance/part-01.rs");
include!("behavior_assurance/part-02.rs");
include!("behavior_assurance/part-03.rs");

#[cfg(test)]
mod tests {
    use super::*;
    include!("behavior_assurance/tests.rs");
}
