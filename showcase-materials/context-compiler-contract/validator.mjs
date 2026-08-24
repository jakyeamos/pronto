// Deterministic validator snapshot for the checked-in CC-1 fixture.
// The fixture records the source repository and commit; CI must not depend on
// a developer-local checkout of that repository.

export const CONTEXT_COMPILER_RESULT_SCHEMA = "context-compiler-result-v0.1";
export const CONTEXT_ROUTING_MANIFEST_SCHEMA = "context-routing-manifest-v0.1";

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function hasText(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function validateSelectedContextFile(file, index) {
  const issues = [];
  if (!isObject(file)) {
    return [`selected_context_files[${index}] must be an object`];
  }
  for (const key of ["id", "path", "title", "tier", "priority", "reason"]) {
    if (!hasText(file[key])) {
      issues.push(`selected_context_files[${index}].${key} is required`);
    }
  }
  for (const key of ["relevance_score", "final_score", "token_cost_estimate"]) {
    if (typeof file[key] !== "number") {
      issues.push(`selected_context_files[${index}].${key} must be a number`);
    }
  }
  return issues;
}

export function validateContextRoutingManifest(manifest) {
  const issues = [];
  if (!isObject(manifest)) {
    return { passed: false, issues: ["context_routing_manifest must be an object"] };
  }
  if (!hasText(manifest.phase)) {
    issues.push("context_routing_manifest.phase is required");
  }
  if (!Array.isArray(manifest.context_sources_loaded)) {
    issues.push("context_routing_manifest.context_sources_loaded must be a list");
  }
  if (!Array.isArray(manifest.context_sources_skipped)) {
    issues.push("context_routing_manifest.context_sources_skipped must be a list");
  }
  const loadedSources = Array.isArray(manifest.context_sources_loaded)
    ? manifest.context_sources_loaded
    : [];
  const skippedSources = Array.isArray(manifest.context_sources_skipped)
    ? manifest.context_sources_skipped
    : [];
  for (const [index, source] of loadedSources.entries()) {
    if (!hasText(source.id) || !hasText(source.path) || !hasText(source.reason)) {
      issues.push(`context_sources_loaded[${index}] must include id, path, and reason`);
    }
  }
  for (const [index, source] of skippedSources.entries()) {
    if (!hasText(source.id) || !hasText(source.path) || !hasText(source.reason)) {
      issues.push(`context_sources_skipped[${index}] must include id, path, and reason`);
    }
  }
  if (typeof manifest.estimated_context_tokens !== "number") {
    issues.push("context_routing_manifest.estimated_context_tokens must be a number");
  }
  return { passed: issues.length === 0, issues };
}

export function validateCompiledContextResult(result) {
  const issues = [];
  if (!isObject(result)) {
    return {
      schema: CONTEXT_COMPILER_RESULT_SCHEMA,
      passed: false,
      issues: ["result must be an object"],
    };
  }
  if (!hasText(result.task_summary)) {
    issues.push("task_summary is required");
  }
  if (!isObject(result.task_classification)) {
    issues.push("task_classification must be an object");
  }
  if (!Array.isArray(result.selected_context_files) || result.selected_context_files.length === 0) {
    issues.push("selected_context_files must be a non-empty list");
  } else {
    for (const [index, file] of result.selected_context_files.entries()) {
      issues.push(...validateSelectedContextFile(file, index));
    }
  }
  if (!Array.isArray(result.retrieval_trace)) {
    issues.push("retrieval_trace must be a list");
  } else if (!result.retrieval_trace.every((item) => hasText(item.source) && hasText(item.reason))) {
    issues.push("retrieval_trace entries must include source and reason");
  }
  if (!isObject(result.packet_contract)) {
    issues.push("packet_contract must be an object");
  } else {
    if (result.packet_contract.route_compatible !== true) {
      issues.push("packet_contract.route_compatible must be true");
    }
    if (!hasText(result.packet_contract.selection_policy)) {
      issues.push("packet_contract.selection_policy is required");
    }
  }
  const manifestValidation = validateContextRoutingManifest(result.context_routing_manifest);
  if (!manifestValidation.passed) {
    issues.push(...manifestValidation.issues);
  }
  if (!hasText(result.context_receipt)) {
    issues.push("context_receipt is required");
  }
  if (!hasText(result.briefing_markdown)) {
    issues.push("briefing_markdown is required");
  }
  return {
    schema: CONTEXT_COMPILER_RESULT_SCHEMA,
    passed: issues.length === 0,
    issues,
  };
}
