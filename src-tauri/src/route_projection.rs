//! Default orientation omits heavyweight detail while preserving the doctor gate.
use serde_json::{json, Value};

pub fn compact(full: Value) -> Value {
    let detail = &full["repository"];
    let repository = &detail["repository"];
    let next = &full["next"];
    let mut output = json!({
        "schema_version": "pronto-agent-route/v2",
        "generated_at": full["generated_at"], "scope": full["scope"],
        "status": full["status"], "ready": full["ready"],
        "doctor": full["doctor"], "next_safe_step": full["next_safe_step"],
        "authorization": full["authorization"],
        "repository": if repository.is_object() { json!({
            "id": repository["id"], "name": repository["name"], "path": repository["path"],
            "compass_status": repository["project_compass"]["status"],
            "compass_projection_status": repository["project_compass"]["projection_status"]
        }) } else { Value::Null },
        "change_maturity": full["change_maturity"],
        "developer_legibility": full["developer_legibility"],
        "change_surface_hotspots": full["change_surface_hotspots"],
        "next_actions": next["actions"].as_array().map(|items| items.iter().take(3).map(|item| json!({
            "summary": item["summary"], "next_safe_step": item["next_safe_step"], "severity": item["severity"],
            "category": item["category"], "attention_id": item["attention_id"], "recommended_projection": item["recommended_projection"]
        })).collect::<Vec<_>>()).unwrap_or_default(),
        "detail_omitted": true,
        "drill_down": ["pronto repo <repository> --json", "pronto quality <repository> --json",
            "pronto compass family <workspace> --json", "pronto compass change-context <workspace> --path <path> --base <revision> --json",
            "pronto route <repository> --full --json"]
    });
    // Never silently clip safety evidence. Oversized scope returns a blocked bounded envelope.
    if serde_json::to_string_pretty(&output).map_or(true, |bytes| bytes.len() + 1 > 32768) {
        output = json!({"schema_version":"pronto-agent-route/v2", "ready":false, "status":"ScopeTooLarge",
            "next_safe_step":"Select one repository; the full doctor evidence exceeds the 32 KiB orientation budget.",
            "drill_down":["pronto doctor <repository> --json", "pronto route <repository> --full --json"],
            "authorization":full["authorization"]});
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_actual_next_action_fields() {
        let output = compact(
            json!({"next":{"actions":[{"summary":"Intent unresolved", "severity":"warning", "category":"project_compass", "attention_id":"compass", "recommended_projection":"repo", "next_safe_step":"Reconcile intent"}]}}),
        );
        assert_eq!(output["next_actions"][0]["summary"], "Intent unresolved");
        assert_eq!(output["next_actions"][0]["severity"], "warning");
        assert_eq!(output["next_actions"][0]["attention_id"], "compass");
    }

    #[test]
    fn omits_heavy_detail_and_keeps_blocked_doctor() {
        let full = json!({"ready":false, "status":"Blocked", "doctor":{"ready":false,"checks":["stale"]},
            "repository":{"repository":{"name":"pilot", "path":"/pilot", "quality":{"findings":vec!["large";10000]}}}});
        let output = compact(full);
        assert_eq!(output["doctor"]["checks"][0], "stale");
        assert_eq!(output["ready"], false);
        assert!(serde_json::to_string_pretty(&output).unwrap().len() < 2000);
        assert!(output["detail_omitted"].as_bool().unwrap());
    }
}
