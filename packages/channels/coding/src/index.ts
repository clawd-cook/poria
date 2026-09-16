import type { IChannel, CapabilityMetadata, ChannelContext } from "@poria/core";
import type {
  JacpCredentials,
  CodingRepo,
  CodingBranch,
  MrStatus,
  MrInfo,
  FindMrQuery,
  CreateMrInput,
  CreateMrResult,
} from "./types.js";
import {
  isCodingFixtureMode,
  queryAllRepos,
  queryBranches,
  searchReposFixture,
  listBranchesFixture,
  createMergeRequestFixture,
} from "./easyci.js";
import {
  createMergeRequestLive,
  detectCurrentBranch,
  detectDefaultTargetBranch,
  getMrStatusLive,
  getMrStatusFixture,
  findMrLive,
  findMrFixture,
  projectIdFromGitUrl,
} from "./mergeRequest.js";
import { getOriginGitUrl } from "./gitUrl.js";

export interface CodingChannelInput {
  action: "searchRepos" | "listBranches" | "createMergeRequest" | "getMrStatus" | "findMr";
  searchRepos?: { nameLike?: string; size?: number };
  listBranches?: { gitUrl: string; nameLike?: string };
  createMergeRequest?: CreateMrInput;
  getMrStatus?: { projectPath: string; iid: number };
  findMr?: FindMrQuery;
}

export type CodingChannelOutput =
  | { action: "searchRepos"; repos: CodingRepo[] }
  | { action: "listBranches"; branches: CodingBranch[] }
  | { action: "createMergeRequest"; result: CreateMrResult }
  | { action: "getMrStatus"; status: MrStatus }
  | { action: "findMr"; mr: MrInfo | null };

const metadata: CapabilityMetadata = {
  id: "channel:coding",
  name: "Coding",
  description: "EasyCI repository, branch, and MR operations",
  version: "0.1.0",
};

function requireCredentials(ctx: ChannelContext): JacpCredentials {
  const creds = ctx.credentials as JacpCredentials | null | undefined;
  if (!creds?.cookie) {
    throw new Error("AuthRequired: 请先运行 poria auth login");
  }
  return creds;
}

class CodingChannelImpl implements IChannel<CodingChannelInput, CodingChannelOutput> {
  readonly metadata = metadata;

  async execute(input: CodingChannelInput, context: ChannelContext): Promise<CodingChannelOutput> {
    switch (input.action) {
      case "searchRepos":
        return this.searchRepos(input.searchRepos ?? {}, context);
      case "listBranches":
        return this.listBranches(input.listBranches!, context);
      case "createMergeRequest":
        return this.createMergeRequest(input.createMergeRequest!, context);
      case "getMrStatus":
        return this.getMrStatus(input.getMrStatus!, context);
      case "findMr":
        return this.findMr(input.findMr!, context);
      default:
        throw new Error(`Unknown action: ${input.action}`);
    }
  }

  private async searchRepos(
    query: { nameLike?: string; size?: number },
    ctx: ChannelContext,
  ): Promise<CodingChannelOutput> {
    if (isCodingFixtureMode()) {
      return { action: "searchRepos", repos: await searchReposFixture(query) };
    }
    const credentials = requireCredentials(ctx);
    const repos = await queryAllRepos(credentials, query.nameLike ?? "", query.size ?? 20);
    return { action: "searchRepos", repos };
  }

  private async listBranches(
    input: { gitUrl: string; nameLike?: string },
    ctx: ChannelContext,
  ): Promise<CodingChannelOutput> {
    if (isCodingFixtureMode()) {
      return { action: "listBranches", branches: await listBranchesFixture(input.gitUrl, input.nameLike) };
    }
    const credentials = requireCredentials(ctx);
    const branches = await queryBranches(credentials, input.gitUrl, input.nameLike ?? "");
    return { action: "listBranches", branches };
  }

  private async createMergeRequest(
    input: CreateMrInput,
    ctx: ChannelContext,
  ): Promise<CodingChannelOutput> {
    if (isCodingFixtureMode()) {
      return { action: "createMergeRequest", result: await createMergeRequestFixture(input) };
    }
    const credentials = requireCredentials(ctx);
    const cwd = process.cwd();
    const sourceBranch =
      input.sourceBranch?.trim() || (await detectCurrentBranch(cwd)) || "";
    if (!sourceBranch) {
      throw new Error("Missing source branch. Pass sourceBranch or run inside a git checkout.");
    }
    const targetBranch =
      input.targetBranch?.trim() || (await detectDefaultTargetBranch(cwd)) || "master";

    let projectId = input.projectId?.trim();
    if (!projectId) {
      const gitUrl = await getOriginGitUrl(cwd);
      projectId = gitUrl ? projectIdFromGitUrl(gitUrl) : undefined;
    }
    if (!projectId) {
      throw new Error("Missing Coding project id. Pass projectId or ensure origin remote exists.");
    }

    const result = await createMergeRequestLive(credentials, {
      projectId,
      title: input.title,
      description: input.description,
      sourceBranch,
      targetBranch,
    });
    return { action: "createMergeRequest", result };
  }

  private async getMrStatus(
    input: { projectPath: string; iid: number },
    ctx: ChannelContext,
  ): Promise<CodingChannelOutput> {
    if (isCodingFixtureMode()) {
      return { action: "getMrStatus", status: getMrStatusFixture(input.projectPath, input.iid) };
    }
    const credentials = requireCredentials(ctx);
    const status = await getMrStatusLive(credentials, input.projectPath, input.iid);
    return { action: "getMrStatus", status };
  }

  private async findMr(
    query: FindMrQuery,
    ctx: ChannelContext,
  ): Promise<CodingChannelOutput> {
    if (isCodingFixtureMode()) {
      return { action: "findMr", mr: findMrFixture(query) };
    }
    const credentials = requireCredentials(ctx);
    const mr = await findMrLive(credentials, query);
    return { action: "findMr", mr };
  }
}

export function createCodingChannel(): CodingChannelImpl {
  return new CodingChannelImpl();
}

// Re-exports for direct usage by other packages
export {
  normalizeGitUrl,
  repoNameFromGitUrl,
  repoSearchPathFromGitUrl,
  sameGitUrl,
  getOriginGitUrl,
} from "./gitUrl.js";
export { queryAllRepos, queryBranches, isCodingFixtureMode } from "./easyci.js";
export {
  projectIdFromGitUrl,
  getMrStatusLive,
  getMrStatusFixture,
  findMrLive,
  findMrFixture,
  createMergeRequestLive,
  detectCurrentBranch,
  detectDefaultTargetBranch,
} from "./mergeRequest.js";
export type {
  JacpCredentials,
  CodingRepo,
  CodingBranch,
  MrStatus,
  MrInfo,
  FindMrQuery,
  CreateMrInput,
  CreateMrResult,
} from "./types.js";
