interface ShowcaseDimension {
  status: "assessed" | "unknown" | "blocked" | "not_applicable";
  score?: number | null;
  evidence: string;
}

export interface ShowcaseProjectSnapshot {
  repository_name: string;
  display_name: string;
  repository_id?: string | null;
  repository_path?: string | null;
  registration_status: "registered" | "unregistered";
  public_eligibility:
    | "public_showcase"
    | "private_client"
    | "not_applicable"
    | "blocked"
    | "unknown";
  disposition_source: string;
  work_disposition:
    | "largely_product_ready"
    | "targeted_gap_closure"
    | "material_build_or_restoration"
    | "conditional_gate"
    | "private_client"
    | "not_applicable"
    | "blocked"
    | "unknown";
  work_disposition_summary: string;
  next_step_category:
    "product" | "demo_integration" | "evidence" | "content" | "packaging";
  product_readiness: ShowcaseDimension;
  demo_materials: ShowcaseDimension;
  career_signal: ShowcaseDimension;
  showcase_score?: number | null;
  priority_score?: number | null;
  lane:
    | "publish_ready"
    | "create_materials"
    | "product_first"
    | "private_client"
    | "blocked"
    | "unknown"
    | "not_applicable";
  publishable: boolean;
  blockers: string[];
  missing_materials: string[];
  next_step: string;
}

export interface ShowcasePortfolioSnapshot {
  schema_version: "pronto-showcase/v2";
  status: "Ready" | "Missing" | "Invalid";
  contract_path: string;
  reviewed_at?: string | null;
  quality_bar_source?: string | null;
  goal: {
    target_publishable_demo_count: number;
    publishable_demo_count: number;
    remaining_demo_count: number;
    status: string;
  };
  scoring?: {
    product_weight: number;
    materials_weight: number;
    priority_career_weight: number;
    priority_product_weight: number;
    priority_materials_gap_weight: number;
    publishable_product_minimum: number;
    publishable_materials_minimum: number;
  } | null;
  public_queue: string[];
  private_client_count: number;
  projects: ShowcaseProjectSnapshot[];
  error?: string | null;
}
