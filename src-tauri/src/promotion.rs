use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const AWL_RELATIVE_ROOT: &str = "projects/ai-workflow-leverage";
const JAS_RELATIVE_ROOT: &str = "projects/jakyeamos-agent-skills";
const INBOX_SCHEMA_VERSION: &str = "leverage-promotion-inbox/v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromotionCandidate {
    pub candidate_id: String,
    pub title: String,
    pub asset_kind: String,
    #[serde(default)]
    pub improvement_key: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub quantification: Option<Value>,
    pub portability: String,
    pub status: String,
    pub review_status: String,
    #[serde(default)]
    pub maturity: Option<String>,
    pub package_status: String,
    pub candidate_kind: String,
    pub candidate_source: String,
    pub candidate_artifact: String,
    pub candidate_provenance_hash: String,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub decision_at: Option<String>,
    #[serde(default)]
    pub decision_reason: Option<String>,
    #[serde(default)]
    pub decision_reviewer: Option<String>,
    #[serde(default)]
    pub decision_artifact: Option<String>,
    #[serde(default)]
    pub jas_projection_status: Option<String>,
    #[serde(default)]
    pub jas_projection_visibility: Option<String>,
    #[serde(default)]
    pub jas_admission: Option<Value>,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromotionCounts {
    pub total: usize,
    pub pending: usize,
    pub deferred: usize,
    pub rejected: usize,
    pub accepted: usize,
    pub complete: usize,
    pub drafts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionInbox {
    pub schema_version: String,
    pub visibility: String,
    pub generated_at: String,
    pub source_root: String,
    pub candidates: Vec<PromotionCandidate>,
    pub counts: PromotionCounts,
    #[serde(default)]
    pub coverage: Option<Value>,
    #[serde(default)]
    pub discovery: Option<Value>,
    #[serde(default)]
    pub funnel: Option<Value>,
    pub errors: Vec<Value>,
    pub manual_review_required: bool,
    pub jas_mutation: bool,
    pub status: String,
    pub provenance_hash: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub jas_admission: Option<Value>,
}

fn awl_root() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Pronto could not resolve the home directory.".to_string())?;
    let root = home.join(AWL_RELATIVE_ROOT);
    if !root.join("leverage").is_dir() || !root.join("pyproject.toml").is_file() {
        return Err(format!(
            "The AWL checkout is not available at {}.",
            root.display()
        ));
    }
    Ok(root)
}

fn python_executable(root: &Path) -> PathBuf {
    let candidates = [
        root.join(".venv/bin/python"),
        PathBuf::from("/usr/local/bin/python3"),
        PathBuf::from("/opt/homebrew/bin/python3"),
        PathBuf::from("/usr/bin/python3"),
    ];
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from("python3"))
}

fn run_awl(root: &Path, command_args: &[String]) -> Result<Value, String> {
    let mut args = vec![
        "-m".to_string(),
        "leverage".to_string(),
        "--root".to_string(),
        root.display().to_string(),
        "--json".to_string(),
    ];
    args.extend(command_args.iter().cloned());
    let output = Command::new(python_executable(root))
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("Pronto could not run the AWL promotion command: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json = stdout
        .trim()
        .parse::<Value>()
        .or_else(|_| stderr.trim().parse::<Value>());
    if !output.status.success() {
        if let Ok(value) = &json {
            if value.get("schema_version").and_then(Value::as_str) == Some(INBOX_SCHEMA_VERSION) {
                return Ok(value.clone());
            }
        }
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            format!("The AWL promotion command exited with {}.", output.status)
        } else {
            format!("AWL promotion command failed: {detail}")
        });
    }
    json.map_err(|_| {
        format!(
            "AWL returned a non-JSON promotion response: {}",
            stdout.trim()
        )
    })
}

fn record_jas_admission(
    root: &Path,
    candidate: &PromotionCandidate,
    decision: &str,
    admission: &Value,
) -> Result<(), String> {
    let status = admission
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| "JAS returned no admission status".to_string())?;
    if !matches!(status, "JAS_APPLIED" | "JAS_ALREADY_APPLIED" | "blocked") {
        return Err("JAS returned an unsupported admission status".to_string());
    }
    let mut args = vec![
        "candidate".to_string(),
        "record-admission".to_string(),
        "--candidate-id".to_string(),
        candidate.candidate_id.clone(),
        "--candidate-provenance-hash".to_string(),
        candidate.candidate_provenance_hash.clone(),
        "--decision".to_string(),
        decision.to_string(),
        "--status".to_string(),
        status.to_string(),
        "--reviewer".to_string(),
        "local-owner".to_string(),
    ];
    if admission
        .get("mutated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        args.push("--mutated".to_string());
    }
    if let Some(target) = admission.get("target").and_then(Value::as_str) {
        args.push("--target".to_string());
        args.push(target.to_string());
    }
    if let Some(install_status) = admission.get("install_status").and_then(Value::as_str) {
        args.push("--install-status".to_string());
        args.push(install_status.to_string());
    }
    let result = run_awl(root, &args)?;
    if result.get("status").and_then(Value::as_str) != Some("recorded") {
        return Err("AWL did not record the JAS admission receipt".to_string());
    }
    Ok(())
}

fn unavailable(message: String) -> PromotionInbox {
    PromotionInbox {
        schema_version: INBOX_SCHEMA_VERSION.to_string(),
        visibility: "private".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        source_root: String::new(),
        candidates: Vec::new(),
        counts: PromotionCounts::default(),
        coverage: None,
        discovery: None,
        funnel: None,
        errors: vec![Value::String(message.clone())],
        manual_review_required: true,
        jas_mutation: false,
        status: "unavailable".to_string(),
        provenance_hash: String::new(),
        message: Some(message),
        jas_admission: None,
    }
}

fn parse_inbox(value: Value) -> Result<PromotionInbox, String> {
    serde_json::from_value(value)
        .map_err(|error| format!("AWL promotion inbox is malformed: {error}"))
}

fn validate_decision_candidate(inbox: &PromotionInbox, candidate_id: &str) -> Result<(), String> {
    if inbox.status != "pass" {
        return Err(inbox
            .message
            .clone()
            .unwrap_or_else(|| "AWL promotion review is not available.".to_string()));
    }
    let candidate = inbox
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == candidate_id)
        .ok_or_else(|| {
            format!("Promotion candidate {candidate_id} is not in the current AWL inbox.")
        })?;
    if candidate.decision.is_some() {
        return Err("Promotion decisions can only be recorded once.".to_string());
    }
    if candidate.candidate_kind != "complete" {
        return Err(
            "Promotion decisions require a complete candidate packet; this record remains in the AWL pipeline."
                .to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn get_promotion_inbox() -> PromotionInbox {
    let root = match awl_root() {
        Ok(root) => root,
        Err(message) => return unavailable(message),
    };
    match run_awl(&root, &["candidate".into(), "inbox".into(), "--all".into()])
        .and_then(parse_inbox)
    {
        Ok(inbox) => inbox,
        Err(message) => unavailable(message),
    }
}

#[tauri::command]
pub fn decide_promotion(
    candidate_id: String,
    decision: String,
    reason: Option<String>,
) -> Result<PromotionInbox, String> {
    let root = awl_root()?;
    let current_inbox = run_awl(&root, &["candidate".into(), "inbox".into(), "--all".into()])
        .and_then(parse_inbox)?;
    validate_decision_candidate(&current_inbox, &candidate_id)?;
    let decision_text = decision.clone();
    let mut args = vec![
        "candidate".to_string(),
        "decide".to_string(),
        "--candidate-id".to_string(),
        candidate_id.clone(),
        "--decision".to_string(),
        decision,
        "--reviewer".to_string(),
        "local-owner".to_string(),
    ];
    if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
        args.push("--reason".to_string());
        args.push(reason);
    }
    let value = run_awl(&root, &args)?;
    let inbox_value = value
        .get("inbox")
        .cloned()
        .ok_or_else(|| "AWL did not return the updated promotion inbox.".to_string())?;
    let mut inbox = parse_inbox(inbox_value)?;
    if matches!(decision_text.as_str(), "public" | "private" | "both") {
        let selected_candidate = inbox
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == candidate_id)
            .cloned();
        let mut admission = match selected_candidate.as_ref() {
            None => jas_blocked(
                &candidate_id,
                &decision_text,
                "the accepted candidate is not present in the updated AWL inbox",
            ),
            Some(candidate) if candidate.candidate_kind != "complete" => jas_blocked(
                &candidate_id,
                &decision_text,
                "JAS apply requires a complete candidate packet",
            ),
            Some(candidate) => run_jas_apply(&root, candidate, &decision_text),
        };
        if let Some(candidate) = selected_candidate.as_ref() {
            if record_jas_admission(&root, candidate, &decision_text, &admission).is_err() {
                if let Some(object) = admission.as_object_mut() {
                    object.insert("receipt_status".to_string(), json!("blocked"));
                    object.insert(
                        "receipt_message".to_string(),
                        json!("JAS returned a result, but AWL could not persist its admission receipt."),
                    );
                }
            }
        }
        inbox.jas_mutation = admission
            .get("mutated")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        inbox.jas_admission = Some(admission);
    }
    Ok(inbox)
}

fn jas_root() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Pronto could not resolve the home directory.".to_string())?;
    let root = home.join(JAS_RELATIVE_ROOT);
    if !root.join("scripts/promotion_admission.py").is_file()
        || !root.join("catalog/manifest.json").is_file()
    {
        return Err(format!(
            "The JAS checkout is not available at {}.",
            root.display()
        ));
    }
    Ok(root)
}

fn run_jas(root: &Path, command_args: &[String]) -> Result<Value, String> {
    let mut args = vec!["scripts/promotion_admission.py".to_string()];
    args.extend(command_args.iter().cloned());
    let output = Command::new(python_executable(root))
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("Pronto could not run the JAS admission command: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json = stdout
        .trim()
        .parse::<Value>()
        .or_else(|_| stderr.trim().parse::<Value>());
    if let Ok(value) = json {
        return Ok(value);
    }
    let detail = stderr.trim();
    Err(if detail.is_empty() {
        format!("The JAS admission command exited with {}.", output.status)
    } else {
        format!("JAS admission command failed: {detail}")
    })
}

fn candidate_artifact(root: &Path, candidate: &PromotionCandidate) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("Pronto could not resolve the AWL root: {error}"))?;
    let raw = PathBuf::from(&candidate.candidate_artifact);
    let path = if raw.is_absolute() {
        raw
    } else {
        root.join(raw)
    };
    let resolved = path
        .canonicalize()
        .map_err(|error| format!("the selected AWL candidate artifact is unavailable: {error}"))?;
    if !resolved.starts_with(&root) {
        return Err("the selected AWL candidate artifact is outside the AWL checkout".to_string());
    }
    Ok(resolved)
}

fn run_jas_apply(root: &Path, candidate: &PromotionCandidate, decision: &str) -> Value {
    let jas = match jas_root() {
        Ok(root) => root,
        Err(message) => return jas_blocked(&candidate.candidate_id, decision, &message),
    };
    let artifact = match candidate_artifact(root, candidate) {
        Ok(path) => path,
        Err(message) => return jas_blocked(&candidate.candidate_id, decision, &message),
    };
    let home = match dirs::home_dir() {
        Some(home) => home,
        None => {
            return jas_blocked(
                &candidate.candidate_id,
                decision,
                "Pronto could not resolve the home directory",
            )
        }
    };
    let approval_path = std::env::temp_dir().join(format!(
        "pronto-jas-approval-{}.json",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let approval = json!({
        "schema_version": "jas-promotion-approval/v1",
        "candidate_id": candidate.candidate_id,
        "decision": "approve",
        "reviewer": "local-owner",
        "reviewed_at": Utc::now().to_rfc3339(),
    });
    if let Err(error) = fs::write(
        &approval_path,
        serde_json::to_vec(&approval).unwrap_or_else(|_| b"{}".to_vec()),
    ) {
        return jas_blocked(
            &candidate.candidate_id,
            decision,
            &format!("Pronto could not create the JAS approval receipt: {error}"),
        );
    }
    let overlay_path = home.join(".config/jas/private-overlay.json");
    let private_root = root.join("artifacts/private-packages");
    let args = vec![
        "apply".to_string(),
        "--candidate".to_string(),
        artifact.display().to_string(),
        "--approval".to_string(),
        approval_path.display().to_string(),
        "--mode".to_string(),
        decision.to_string(),
        "--root".to_string(),
        jas.display().to_string(),
        "--overlay-path".to_string(),
        overlay_path.display().to_string(),
        "--private-root".to_string(),
        private_root.display().to_string(),
        "--target-root".to_string(),
        home.display().to_string(),
        "--apply".to_string(),
        "--json".to_string(),
    ];
    let result = run_jas(&jas, &args)
        .unwrap_or_else(|message| jas_blocked(&candidate.candidate_id, decision, &message));
    let _ = fs::remove_file(&approval_path);
    if result.get("status").and_then(Value::as_str) == Some("blocked")
        && result.get("message").is_none()
    {
        let message = result
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("JAS admission was blocked without a reason.");
        jas_blocked(&candidate.candidate_id, decision, message)
    } else {
        result
    }
}

fn jas_blocked(candidate_id: &str, decision: &str, message: &str) -> Value {
    json!({
        "schema_version": "jas-promotion-admission/v1",
        "status": "blocked",
        "candidate_id": candidate_id,
        "decision": decision,
        "mutated": false,
        "message": message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_inbox(candidate_kind: &str, decision: Option<&str>) -> PromotionInbox {
        let mut inbox = unavailable("test unavailable".to_string());
        inbox.status = "pass".to_string();
        inbox.message = None;
        inbox.candidates = vec![PromotionCandidate {
            candidate_id: "candidate-1".to_string(),
            candidate_kind: candidate_kind.to_string(),
            decision: decision.map(str::to_string),
            ..PromotionCandidate::default()
        }];
        inbox
    }

    #[test]
    fn unavailable_inbox_preserves_the_non_mutating_boundary() {
        let inbox = unavailable("test unavailable".to_string());
        assert_eq!(inbox.status, "unavailable");
        assert!(!inbox.jas_mutation);
        assert!(inbox.manual_review_required);
        assert!(inbox.jas_admission.is_none());
    }

    #[test]
    fn decisions_require_complete_undecided_candidates() {
        let draft = test_inbox("draft", None);
        let draft_error = validate_decision_candidate(&draft, "candidate-1").unwrap_err();
        assert!(draft_error.contains("complete candidate packet"));

        let decided = test_inbox("complete", Some("defer"));
        let decided_error = validate_decision_candidate(&decided, "candidate-1").unwrap_err();
        assert!(decided_error.contains("only be recorded once"));

        let complete = test_inbox("complete", None);
        assert!(validate_decision_candidate(&complete, "candidate-1").is_ok());
    }

    #[test]
    fn candidate_projection_preserves_jas_readiness_metadata() {
        let candidate = PromotionCandidate {
            jas_projection_status: Some("ready".to_string()),
            jas_projection_visibility: Some("private".to_string()),
            ..PromotionCandidate::default()
        };
        let encoded = serde_json::to_value(&candidate).expect("candidate should serialize");
        let decoded: PromotionCandidate =
            serde_json::from_value(encoded).expect("candidate should deserialize");
        assert_eq!(decoded.jas_projection_status.as_deref(), Some("ready"));
        assert_eq!(
            decoded.jas_projection_visibility.as_deref(),
            Some("private")
        );
    }
}
