import { describe, it, expect } from "vitest";
import { topologicalSort, CircularDependencyError } from "../multi-repo.js";
import type { RepoConfig } from "../../types/repo.js";

function repo(name: string, dependsOn?: string[]): RepoConfig {
  return {
    name,
    gitUrl: `git@example.com:org/${name}.git`,
    branch: `feature/${name}`,
    baseBranch: "main",
    gitlabProjectPath: `org/${name}`,
    dependsOn,
  };
}

describe("topologicalSort", () => {
  it("returns all repos when none have dependencies", () => {
    const repos = [repo("a"), repo("b"), repo("c")];
    const sorted = topologicalSort(repos);
    expect(sorted).toHaveLength(3);
    expect(new Set(sorted.map(r => r.name))).toEqual(new Set(["a", "b", "c"]));
  });

  it("sorts a linear chain A → B → C", () => {
    const repos = [repo("c", ["b"]), repo("a"), repo("b", ["a"])];
    const sorted = topologicalSort(repos);
    const names = sorted.map(r => r.name);
    expect(names.indexOf("a")).toBeLessThan(names.indexOf("b"));
    expect(names.indexOf("b")).toBeLessThan(names.indexOf("c"));
  });

  it("sorts a diamond: A → B, A → C, B → D, C → D", () => {
    const repos = [
      repo("d", ["b", "c"]),
      repo("b", ["a"]),
      repo("c", ["a"]),
      repo("a"),
    ];
    const sorted = topologicalSort(repos);
    const names = sorted.map(r => r.name);
    expect(names.indexOf("a")).toBeLessThan(names.indexOf("b"));
    expect(names.indexOf("a")).toBeLessThan(names.indexOf("c"));
    expect(names.indexOf("b")).toBeLessThan(names.indexOf("d"));
    expect(names.indexOf("c")).toBeLessThan(names.indexOf("d"));
  });

  it("throws CircularDependencyError on a cycle", () => {
    const repos = [repo("a", ["b"]), repo("b", ["a"])];
    expect(() => topologicalSort(repos)).toThrow(CircularDependencyError);
  });

  it("ignores unknown dependencies gracefully", () => {
    const repos = [repo("a", ["nonexistent"]), repo("b")];
    const sorted = topologicalSort(repos);
    expect(sorted).toHaveLength(2);
  });
});
