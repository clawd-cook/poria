export interface RepoConfig {
  name: string;
  gitUrl: string;
  branch: string;
  baseBranch: string;
  gitlabProjectPath: string;
  dependsOn?: string[];
  buildCmd?: string;
}
