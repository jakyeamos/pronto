fn assess_map_readiness(
    narrative: &TelescopeNarrative,
    groups: &[TelescopeGroup],
    nodes: &[TelescopeNode],
    edges: &[TelescopeEdge],
    flows: &[TelescopeFlow],
    actions: &[TelescopeAction],
    actors: &[TelescopeActor],
    payloads: &[TelescopePayload],
    current_narrative_fingerprint: &str,
    extraction_unavailable: bool,
) -> (
    TelescopeMapReadiness,
    Vec<TelescopeKnowledgeGap>,
    Vec<TelescopeKnowledgeGap>,
    Vec<TelescopeKnowledgeTask>,
) {
    let mut requirements = Vec::new();
    let mut gaps = Vec::new();
    let source_anchor = nodes
        .iter()
        .flat_map(|node| node.source_anchors.iter().cloned())
        .take(6)
        .collect::<Vec<_>>();
    let applicability = |key: &str| {
        narrative
            .applicability
            .iter()
            .find(|decision| decision.requirement == key)
    };
    let mut add_requirement = |key: &str,
                               label: &str,
                               satisfied: bool,
                               reason: String,
                               question: &str,
                               unlocks: Vec<String>,
                               candidates: Vec<String>,
                               dependencies: Vec<String>,
                               manifest_fields: Vec<String>,
                               order: usize,
                               blocking: bool| {
        if let Some(decision) = applicability(key) {
            if decision.state == "not_applicable" && !decision.reason.trim().is_empty() {
                requirements.push(TelescopeReadinessRequirement {
                    key: key.to_string(),
                    label: label.to_string(),
                    applicability: "not_applicable".to_string(),
                    status: "not_applicable".to_string(),
                    reason: decision.reason.clone(),
                    evidence: Vec::new(),
                });
                return;
            }
        }
        requirements.push(TelescopeReadinessRequirement {
            key: key.to_string(),
            label: label.to_string(),
            applicability: "applicable".to_string(),
            status: if satisfied { "satisfied" } else { "missing" }.to_string(),
            reason: reason.clone(),
            evidence: if satisfied {
                source_anchor.clone()
            } else {
                Vec::new()
            },
        });
        if !satisfied {
            gaps.push((
                order,
                TelescopeKnowledgeGap {
                    key: format!("telescope-readiness:{key}"),
                    category: key.to_string(),
                    question: question.to_string(),
                    why_source_cannot_answer: reason,
                    unlocks,
                    candidate_answers: candidates,
                    evidence: source_anchor.clone(),
                    allowed_responses: vec![
                        "confirm".to_string(),
                        "choose".to_string(),
                        "edit".to_string(),
                        "not_applicable".to_string(),
                        "unknown".to_string(),
                        "point_to_evidence".to_string(),
                    ],
                    depends_on: dependencies,
                    completion_criteria: manifest_fields
                        .iter()
                        .map(|field| {
                            format!("{field} contains reviewed or explicit draft evidence.")
                        })
                        .collect(),
                    manifest_fields,
                    blocking,
                    freshness: "current-workspace".to_string(),
                    provenance: "telescope-readiness-assessment".to_string(),
                },
            ));
        }
    };

    add_requirement(
        "identity",
        "Purpose, audience, and outcomes",
        !narrative.identity.purpose.trim().is_empty()
            && !narrative.identity.audience.is_empty()
            && !narrative.identity.outcomes.is_empty(),
        "Source can show implementation shape, but it cannot confirm why the repository exists or who success is for.".to_string(),
        "What does someone enter this city to accomplish, and who is it for?",
        vec!["City introduction".to_string(), "Primary outcome".to_string()],
        vec![narrative.identity.purpose.clone()].into_iter().filter(|value| !value.is_empty()).collect(),
        Vec::new(),
        vec!["identity.purpose".to_string(), "identity.audience".to_string(), "identity.outcomes".to_string()],
        1,
        true,
    );
    add_requirement(
        "actors",
        "Important actors and entry points",
        !actors.is_empty()
            && actors
                .iter()
                .all(|actor| !actor.action_ids.is_empty() && !actor.node_ids.is_empty()),
        "Entrypoints are measurable, but each important person or system must be tied to a canonical action and a source-backed facility.".to_string(),
        "Who enters this city, and which gates do they use?",
        vec!["People and crews".to_string(), "Action tour starting points".to_string()],
        actions.iter().take(4).map(|action| action.label.clone()).collect(),
        vec!["telescope-readiness:identity".to_string()],
        vec!["actors".to_string()],
        2,
        true,
    );
    add_requirement(
        "boundaries",
        "Major boundaries and responsibilities",
        !groups.is_empty()
            && groups.iter().all(|group| group.summary.split_whitespace().count() >= 7)
            && nodes.iter().any(|node| node.visual_building_id.is_some()),
        "Directory boundaries are measured; meaningful subsystem responsibility must be reviewed.".to_string(),
        "Pronto found areas doing different work. Which are real districts, and what is each responsible for?",
        vec!["District boundaries".to_string(), "Building responsibilities".to_string()],
        groups.iter().map(|group| group.label.clone()).collect(),
        vec!["telescope-readiness:identity".to_string()],
        vec!["groups".to_string(), "nodes[].explanation.responsibilities".to_string()],
        3,
        true,
    );
    add_requirement(
        "actions",
        "Primary user and system actions",
        actions
            .iter()
            .any(|action| action.provenance == "authored-action-inventory")
            && actions.iter().any(|action| action.behavior_id.is_some()),
        "Static calls do not establish which behaviors matter to users or operators.".to_string(),
        "Which actions are the important journeys through this repository?",
        vec![
            "Action catalog".to_string(),
            "Guided city stories".to_string(),
        ],
        actions
            .iter()
            .take(6)
            .map(|action| action.label.clone())
            .collect(),
        vec![
            "telescope-readiness:actors".to_string(),
            "telescope-readiness:boundaries".to_string(),
        ],
        vec!["actions".to_string()],
        4,
        true,
    );
    add_requirement(
        "movement",
        "Data, control, state, and payload movement",
        !flows.is_empty() && !edges.is_empty() && !payloads.is_empty(),
        "Imports reveal possible handoffs, but payload meaning and state transitions are not source facts by themselves.".to_string(),
        "What moves through the main journey, and where does it change state?",
        vec!["Rails and vehicles".to_string(), "Payload labels".to_string(), "State transitions".to_string()],
        flows.iter().take(4).map(|flow| flow.label.clone()).collect(),
        vec!["telescope-readiness:actions".to_string()],
        vec!["payloads".to_string(), "flows".to_string(), "actions[].explanation.stateChanges".to_string()],
        5,
        true,
    );
    add_requirement(
        "constraints",
        "Important decisions, failures, and constraints",
        !narrative.decisions.is_empty() && !narrative.failures.is_empty(),
        "Failure policy and architectural intent are rarely recoverable from static topology alone.".to_string(),
        "Which decisions and failure states must a newcomer understand before changing this city?",
        vec!["Decision checkpoints".to_string(), "Failure explanations".to_string()],
        narrative.failures.iter().map(|failure| failure.label.clone()).collect(),
        vec!["telescope-readiness:actions".to_string()],
        vec!["decisions".to_string(), "failures".to_string()],
        5,
        true,
    );
    add_requirement(
        "city_metaphor",
        "City roles, landmarks, actors, and flow metaphors",
        !actors.is_empty()
            && !payloads.is_empty()
            && nodes.iter().filter(|node| node.visual_building_id.is_some()).all(|node| !node.city_role.is_empty()),
        "Pronto can suggest forms from code kinds, but a useful metaphor must explain real behavior rather than add atmosphere.".to_string(),
        "Do these facilities, people, and vehicles make the repository easier to explain?",
        vec!["Living-city vocabulary".to_string(), "Recognizable silhouettes".to_string()],
        Vec::new(),
        vec!["telescope-readiness:boundaries".to_string(), "telescope-readiness:movement".to_string()],
        vec!["actors[].metaphor".to_string(), "payloads[].metaphor".to_string(), "nodes[].cityRole".to_string()],
        6,
        true,
    );
    add_requirement(
        "source_evidence",
        "Representative source-backed relationships",
        !nodes.is_empty()
            && nodes
                .iter()
                .filter(|node| node.visual_building_id.is_some())
                .all(|node| !node.source_anchors.is_empty())
            && edges.iter().any(|edge| edge.source_anchor.is_some()),
        "Every claim needs a repository-relative handoff to measured source evidence.".to_string(),
        "Which files or symbols best prove each important facility and route?",
        vec![
            "Claim-level evidence".to_string(),
            "Readable source detail".to_string(),
        ],
        Vec::new(),
        vec!["telescope-readiness:boundaries".to_string()],
        vec!["nodes[].files".to_string(), "edges".to_string()],
        6,
        true,
    );
    let review_current = narrative.status == "reviewed"
        && narrative.review.reviewed_fingerprint.as_deref() == Some(current_narrative_fingerprint);
    add_requirement(
        "review",
        "Explicit review of high-impact claims",
        review_current,
        "A draft city may be useful for review, but it cannot become authoritative without an explicit current-workspace review.".to_string(),
        "Does this candidate city accurately explain the repository's purpose, boundaries, primary journeys, and evidence?",
        vec!["Reviewed map publication".to_string()],
        Vec::new(),
        vec![
            "telescope-readiness:identity".to_string(),
            "telescope-readiness:actors".to_string(),
            "telescope-readiness:boundaries".to_string(),
            "telescope-readiness:actions".to_string(),
            "telescope-readiness:movement".to_string(),
            "telescope-readiness:constraints".to_string(),
            "telescope-readiness:city_metaphor".to_string(),
            "telescope-readiness:source_evidence".to_string(),
        ],
        vec!["status".to_string(), "review".to_string()],
        7,
        true,
    );

    gaps.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.key.cmp(&right.1.key))
    });
    let semantic_blockers = gaps
        .iter()
        .filter(|(_, gap)| gap.blocking && gap.category != "review")
        .map(|(_, gap)| gap.key.clone())
        .collect::<Vec<_>>();
    let blocking_gaps = gaps
        .iter()
        .filter(|(_, gap)| gap.blocking)
        .map(|(_, gap)| gap.clone())
        .collect::<Vec<_>>();
    let enhancement_gaps = gaps
        .iter()
        .filter(|(_, gap)| !gap.blocking)
        .map(|(_, gap)| gap.clone())
        .collect::<Vec<_>>();
    let state = if extraction_unavailable {
        "unavailable"
    } else if narrative.status == "stale" || (narrative.status == "reviewed" && !review_current) {
        "stale"
    } else if narrative.status == "missing" {
        "measured"
    } else if !semantic_blockers.is_empty() {
        "needs_information"
    } else if review_current {
        "reviewed"
    } else {
        "reviewable"
    };
    let reason = match state {
        "unavailable" => "Source extraction or workspace binding failed; no city can be assessed.",
        "measured" => "Measured topology exists, but repository meaning has not been authored.",
        "needs_information" => "Consequential questions still block a trustworthy architectural explanation.",
        "reviewable" => "The candidate city has enough evidence for explicit human review.",
        "reviewed" => "Important architecture and city metaphors were reviewed against this workspace fingerprint.",
        "stale" => "Reviewed meaning is not bound to the current workspace fingerprint.",
        _ => "Telescope readiness is unknown.",
    }
    .to_string();
    let knowledge_tasks = gaps
        .iter()
        .map(|(order, gap)| TelescopeKnowledgeTask {
            id: stable_id("knowledge-task", &gap.key),
            stable_gap_key: gap.key.clone(),
            domain: "telescope_readiness".to_string(),
            status: "open".to_string(),
            title: if gap.category == "review" {
                "Review the candidate architecture city".to_string()
            } else {
                format!(
                    "Complete Telescope knowledge: {}",
                    gap.category.replace('_', " ")
                )
            },
            question: gap.question.clone(),
            summary: gap.why_source_cannot_answer.clone(),
            priority: if gap.blocking { "P1" } else { "P2" }.to_string(),
            dependency_order: *order,
            depends_on: gap.depends_on.clone(),
            unlocks: gap.unlocks.clone(),
            candidate_answers: gap.candidate_answers.clone(),
            allowed_responses: gap.allowed_responses.clone(),
            completion_criteria: gap.completion_criteria.clone(),
            manifest_fields: gap.manifest_fields.clone(),
            evidence: gap.evidence.clone(),
            freshness: gap.freshness.clone(),
            provenance: "telescope-readiness-to-remediation-projection".to_string(),
            read_only: true,
            guarded_handoff: true,
        })
        .collect::<Vec<_>>();
    (
        TelescopeMapReadiness {
            state: state.to_string(),
            reason,
            requirements,
            blocking_gap_keys: blocking_gaps.iter().map(|gap| gap.key.clone()).collect(),
            enhancement_gap_keys: enhancement_gaps.iter().map(|gap| gap.key.clone()).collect(),
            reviewed_fingerprint: narrative.review.reviewed_fingerprint.clone(),
            current_fingerprint: Some(current_narrative_fingerprint.to_string()),
        },
        blocking_gaps,
        enhancement_gaps,
        knowledge_tasks,
    )
}


fn city_role_for_kind(kind: &str) -> &'static str {
    match kind {
        "route" | "entrypoint" => "gate",
        "store" => "archive",
        "interface" => "terminal",
        "integration" => "port",
        "service" => "workplace",
        "worker" => "workshop",
        _ => "facility",
    }
}
