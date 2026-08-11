export function papercutLabel(value: string): string {
  return value.replaceAll("_", " ");
}

export function papercutStatusTone(status: string): string {
  if (status === "resolved" || status === "accepted") return "mint";
  if (status === "in_progress") return "blue";
  if (status === "deferred") return "amber";
  if (status === "rejected") return "coral";
  return "violet";
}
