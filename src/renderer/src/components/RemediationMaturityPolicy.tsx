import type { ReactElement } from "react";
import type { RemediationRun } from "../types";

type MaturityPolicy = NonNullable<
  RemediationRun["plans"][number]["goal"]["maturity_policy"]
>;

export function remediationMaturityPolicySummary(
  policy: MaturityPolicy,
): string {
  return `maturity ${policy.minimum_closure_score.toFixed(1)}/4 minimum · ${policy.ideal_score.toFixed(1)}/4 ideal`;
}

export function RemediationMaturityPolicyMeta({
  policy,
}: {
  policy: MaturityPolicy;
}): ReactElement {
  return (
    <>
      <span>Maturity {policy.minimum_closure_score.toFixed(1)}/4 minimum</span>
      <span>{policy.ideal_score.toFixed(1)}/4 evidence-backed ideal</span>
    </>
  );
}

export function RemediationMaturityPolicyCriteria({
  policy,
}: {
  policy: MaturityPolicy;
}): ReactElement {
  return (
    <>
      <li>{policy.improvement_rule}</li>
      <li>{policy.integrity_rule}</li>
    </>
  );
}
