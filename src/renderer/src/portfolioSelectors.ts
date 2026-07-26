import type { RepositorySnapshot } from "./types";

export function countActiveConditions(
  repositories: RepositorySnapshot[],
): number {
  return repositories.reduce(
    (total, repository) =>
      total +
      repository.conditions.filter((condition) => condition.status === "Active")
        .length,
    0,
  );
}

export function countDirtyRepositories(
  repositories: RepositorySnapshot[],
): number {
  return repositories.filter((repository) => repository.workspace.dirty).length;
}

export function countUnsyncedRepositories(
  repositories: RepositorySnapshot[],
): number {
  return repositories.filter(
    (repository) => repository.workspace.sync_state !== "Synced",
  ).length;
}

export function currentDateLabel(): string {
  return new Intl.DateTimeFormat("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).format(new Date());
}
