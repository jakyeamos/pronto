export const MAC_CONTROL_TARGET_IDS = {
  portfolio: "pronto.navigation.portfolio",
  remediation: "pronto.navigation.remediation",
  refresh: "pronto.remediation.refresh",
  settings: "pronto.settings",
} as const;

export const MAC_CONTROL_NAVIGATION_TARGET_IDS: Record<string, string> = {
  portfolio: MAC_CONTROL_TARGET_IDS.portfolio,
  remediation: MAC_CONTROL_TARGET_IDS.remediation,
  settings: MAC_CONTROL_TARGET_IDS.settings,
};
