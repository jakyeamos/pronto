import type { TelescopeAction } from "../types/telescope";

export interface TelescopeActionMatch {
  action: TelescopeAction;
  score: number;
  relationship: "direct" | "related";
  matchedTerms: string[];
  explanation: string;
}

type ActionTextField = {
  text: string;
  weight: number;
};

const STOP_WORDS = new Set([
  "a",
  "about",
  "an",
  "and",
  "are",
  "can",
  "could",
  "did",
  "do",
  "does",
  "for",
  "how",
  "i",
  "in",
  "is",
  "it",
  "its",
  "just",
  "like",
  "me",
  "of",
  "on",
  "or",
  "our",
  "please",
  "should",
  "show",
  "tell",
  "that",
  "the",
  "this",
  "to",
  "we",
  "what",
  "when",
  "where",
  "which",
  "why",
  "with",
  "work",
  "works",
  "would",
  "you",
  "your",
]);

const RELATED_TERMS: Record<string, readonly string[]> = {
  action: ["operation", "task", "workflow", "behavior"],
  architecture: ["structure", "system", "codebase", "repository", "repo"],
  behavior: ["behavioral", "workflow", "scenario", "assurance"],
  built: ["implementation", "source", "code", "technology"],
  changes: ["change", "diff", "branch", "worktree", "ownership"],
  code: ["source", "implementation", "built"],
  delivery: ["release", "ship", "publish", "deploy"],
  explain: ["understand", "inspect", "trace", "review"],
  evidence: ["proof", "verification", "verified", "source", "receipt"],
  find: ["search", "lookup", "query", "locate", "discover"],
  flow: ["path", "process", "journey", "trace", "handoff"],
  health: ["quality", "healthy", "findings", "checks"],
  inspect: ["examine", "review", "understand", "evidence"],
  implementation: ["built", "source", "code", "technology"],
  lookup: ["search", "find", "query", "locate"],
  map: ["architecture", "structure", "repository", "codebase"],
  quality: ["health", "healthy", "findings", "checks", "evidence"],
  query: ["search", "find", "lookup", "locate"],
  release: ["delivery", "ship", "publish", "deploy"],
  repo: ["repository", "codebase", "project", "workspace"],
  repository: ["repo", "codebase", "project", "workspace"],
  review: ["inspect", "examine", "understand", "evidence"],
  search: ["find", "lookup", "query", "locate", "discover"],
  source: ["code", "implementation", "built", "file"],
  understand: ["explain", "inspect", "trace", "review"],
  workspace: ["repository", "repo", "project", "worktree"],
};

const QUESTION_WORDS =
  /\b(can|could|does|how|is|should|what|where|why|would)\b/i;

export function routeTelescopeActions(
  query: string,
  actions: TelescopeAction[],
  limit = 10,
): TelescopeActionMatch[] {
  const terms = tokenize(query);
  if (terms.length === 0) {
    return actions.slice(0, limit).map((action) => ({
      action,
      score: 0,
      relationship: "direct",
      matchedTerms: [],
      explanation: "Browse the action catalog.",
    }));
  }

  return actions
    .map((action, index) => matchAction(query, terms, action, index))
    .filter((match): match is TelescopeActionMatch => match !== null)
    .sort((left, right) => right.score - left.score)
    .slice(0, limit);
}

function matchAction(
  query: string,
  terms: string[],
  action: TelescopeAction,
  sourceIndex: number,
): TelescopeActionMatch | null {
  const fields = actionFields(action);
  const fieldTokens = fields.map(({ text, weight }) => ({
    tokens: new Set(tokenize(text)),
    weight,
  }));
  const matchedTerms: string[] = [];
  let directMatches = 0;
  let score = 0;

  for (const term of terms) {
    let bestScore = 0;
    let direct = false;
    for (const field of fieldTokens) {
      if (field.tokens.has(term)) {
        bestScore = Math.max(bestScore, field.weight);
        direct = true;
        continue;
      }
      for (const related of RELATED_TERMS[term] ?? []) {
        if (field.tokens.has(related)) {
          bestScore = Math.max(bestScore, field.weight * 0.55);
          break;
        }
      }
    }
    if (bestScore > 0) {
      matchedTerms.push(term);
      score += bestScore;
      if (direct) directMatches += 1;
    }
  }

  const isQuestion = QUESTION_WORDS.test(query);
  if (
    isQuestion &&
    ["explain", "inspect", "trace", "review", "understand"].some((term) =>
      fieldTokens.some((field) => field.tokens.has(term)),
    )
  ) {
    score += 0.75;
  }

  if (score === 0) return null;
  const relationship = directMatches > 0 ? "direct" : "related";
  const matched = matchedTerms.slice(0, 3).join(", ");
  return {
    action,
    score: score + sourceIndex * -0.0001,
    relationship,
    matchedTerms,
    explanation:
      relationship === "direct"
        ? `Direct match on ${matched}.`
        : `Related through ${matched}.`,
  };
}

function actionFields(action: TelescopeAction): ActionTextField[] {
  return [
    { text: action.label, weight: 9 },
    { text: action.verb, weight: 6 },
    { text: action.category, weight: 5 },
    { text: action.what_it_does, weight: 5 },
    { text: action.how_its_built, weight: 3 },
    { text: action.behavior_id ?? "", weight: 2 },
    { text: action.scenario_ids?.join(" ") ?? "", weight: 2 },
    {
      text: action.source_anchors.map((anchor) => anchor.path).join(" "),
      weight: 2,
    },
  ];
}

function tokenize(value: string): string[] {
  return value
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .map(stem)
    .filter((token) => token.length > 1 && !STOP_WORDS.has(token));
}

function stem(token: string): string {
  if (token.length > 5 && token.endsWith("ies"))
    return `${token.slice(0, -3)}y`;
  if (token.length > 5 && token.endsWith("ing")) return token.slice(0, -3);
  if (token.length > 4 && token.endsWith("ed")) return token.slice(0, -2);
  if (token.length > 5 && token.endsWith("es")) return token.slice(0, -2);
  if (token.length > 4 && token.endsWith("s")) return token.slice(0, -1);
  return token;
}
