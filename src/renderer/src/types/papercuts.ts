export type PapercutSource = "manual" | "design-friction";
export type PapercutPriority = "P0" | "P1" | "P2" | "P3";
export type PapercutStatus = "open" | "in_progress" | "deferred" | "resolved";

export interface Papercut {
  id: string;
  title: string;
  detail: string;
  family: "design-audit" | string;
  surface: string;
  source: PapercutSource | string;
  evidence_refs: string[];
  impact: string;
  priority: PapercutPriority | string;
  status: PapercutStatus | string;
  next_action: string;
  created_at: string;
  updated_at: string;
  resolved_at?: string | null;
}

export interface PapercutCounts {
  total: number;
  open: number;
  in_progress: number;
  deferred: number;
  resolved: number;
}

export interface PapercutBacklog {
  schema_version: "pronto-papercuts/v1" | string;
  family: "design-audit" | string;
  generated_at: string;
  papercuts: Papercut[];
  counts: PapercutCounts;
}

export interface CreatePapercutInput {
  title: string;
  detail: string;
  surface: string;
  source: PapercutSource;
  priority: PapercutPriority;
  evidenceRefs: string[];
  impact: string;
  nextAction: string;
}
