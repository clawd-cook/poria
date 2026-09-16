import type { RepoConfig } from "../types/repo.js";

export class CircularDependencyError extends Error {
  constructor(public readonly cycle: string[]) {
    super(`Circular dependency detected: ${cycle.join(" → ")}`);
    this.name = "CircularDependencyError";
  }
}

export function topologicalSort(repos: RepoConfig[]): RepoConfig[] {
  const nameToRepo = new Map<string, RepoConfig>();
  const inDegree = new Map<string, number>();
  const adjacency = new Map<string, string[]>();

  for (const repo of repos) {
    nameToRepo.set(repo.name, repo);
    inDegree.set(repo.name, 0);
    adjacency.set(repo.name, []);
  }

  for (const repo of repos) {
    if (!repo.dependsOn) continue;
    for (const dep of repo.dependsOn) {
      if (!nameToRepo.has(dep)) continue;
      adjacency.get(dep)!.push(repo.name);
      inDegree.set(repo.name, (inDegree.get(repo.name) ?? 0) + 1);
    }
  }

  const queue: string[] = [];
  for (const [name, degree] of inDegree) {
    if (degree === 0) queue.push(name);
  }

  const sorted: RepoConfig[] = [];
  while (queue.length > 0) {
    const name = queue.shift()!;
    sorted.push(nameToRepo.get(name)!);
    for (const neighbor of adjacency.get(name) ?? []) {
      const newDegree = (inDegree.get(neighbor) ?? 1) - 1;
      inDegree.set(neighbor, newDegree);
      if (newDegree === 0) queue.push(neighbor);
    }
  }

  if (sorted.length !== repos.length) {
    const remaining = repos.filter(r => !sorted.includes(r)).map(r => r.name);
    throw new CircularDependencyError(remaining);
  }

  return sorted;
}
