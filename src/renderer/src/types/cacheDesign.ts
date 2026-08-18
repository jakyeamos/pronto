export interface CacheDesignAssessment {
  schema: string;
  status:
    | "absent"
    | "discoverable"
    | "validated"
    | "maintained"
    | "unknown"
    | "stale"
    | "blocked"
    | "failed"
    | "not_applicable"
    | "missing"
    | string;
  score?: number;
  measurement_complete: boolean;
  totals: {
    logical_bytes: number;
    allocated_bytes: number;
    exclusive_allocated_bytes: number;
    shared_allocated_bytes: number;
    file_count: number;
    shared_file_count: number;
  };
  categories: Record<
    string,
    {
      logical_bytes?: number;
      allocated_bytes: number;
      exclusive_allocated_bytes?: number;
      shared_allocated_bytes?: number;
      file_count: number;
      shared_file_count?: number;
    }
  >;
  risk_flags: string[];
  growth: Record<string, unknown>;
}

declare module "./quality" {
  interface QualityMaturity {
    cache_design?: CacheDesignAssessment;
  }
}
