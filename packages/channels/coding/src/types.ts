/**
 * JacpCredentials type — migrated from @dj-lib/poria-plugin-sdk.
 * Represents the SSO cookie-based credentials for JACP API access.
 */
export interface JacpCredentials {
  cookie: string;
  username: string;
}

/** Coding repository info from EasyCI GQL API. */
export interface CodingRepo {
  code: string;
  defaultBranchName?: string;
  gitUrl: string;
  homeUrl?: string;
  repoLabel?: string;
}

/** Branch info from EasyCI GQL API. */
export interface CodingBranch {
  name: string;
  status?: string;
}

/** MR status from GitLab API. */
export type MrStatus = "opened" | "closed" | "merged" | "locked";

/** MR info returned from findMr. */
export interface MrInfo {
  url: string;
  iid: number;
  sourceBranch: string;
  targetBranch: string;
  state: MrStatus;
}

/** Query parameters for finding an existing MR. */
export interface FindMrQuery {
  projectPath: string;
  sourceBranch: string;
  targetBranch: string;
  state?: MrStatus;
}

/** MR creation input. */
export interface CreateMrInput {
  title: string;
  description?: string;
  sourceBranch?: string;
  targetBranch?: string;
  projectId?: string;
}

/** MR creation result. */
export interface CreateMrResult {
  url: string;
  iid?: number;
  sourceBranch: string;
  targetBranch: string;
}
