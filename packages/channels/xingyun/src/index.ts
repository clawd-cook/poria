import type { IChannel, CapabilityMetadata, ChannelContext } from "@poria/core";
import type { JacpCredentials } from "./types.js";
import type { CardAttachment } from "./jacp/cards.js";
import type { DemandDetail, DemandActionResult } from "./jacp/demands.js";
import {
  asCardAttachments,
  asCardSpaceId,
  getCardByCode,
} from "./jacp/cards.js";
import {
  getDemandById,
  communicateDemand,
  acceptDemand,
} from "./jacp/demands.js";
import { createChange, queryBindDeployApps } from "./jacp/easyci.js";
import { asSpaceKey, getSpaceById } from "./jacp/spaces.js";
import {
  createAndPushBranch,
  detectOriginHeadBranch,
  getLocalGitContext,
  repoSearchPathFromGitUrl,
  sameGitUrl,
} from "./git.js";
import { featureBranchName } from "./demandUrl.js";
import {
  FIXTURE_ATTACHMENTS,
  FIXTURE_BIND,
  FIXTURE_DEMAND,
  isXingyunFixtureMode,
} from "./fixture.js";
import { resolvePrdFromAttachments } from "./prd.js";

export interface XingyunChannelInput {
  action:
    | "getDemand"
    | "listCardAttachments"
    | "resolvePrdLink"
    | "bindBranch"
    | "communicate"
    | "accept";
  demandId?: number;
  demandCode?: string;
  gitUrl?: string;
  branch?: string;
  baseBranch?: string;
  createLocal?: boolean;
}

export type XingyunChannelOutput =
  | { action: "getDemand"; demand: DemandDetail }
  | { action: "listCardAttachments"; attachments: CardAttachment[] }
  | { action: "resolvePrdLink"; url: string; attachment: CardAttachment }
  | { action: "bindBranch"; branch: string; changeId: string; baseBranch: string }
  | { action: "communicate"; demandId: number; demandStatusCode?: number }
  | { action: "accept"; demandId: number; demandStatusCode?: number };

const metadata: CapabilityMetadata = {
  id: "channel:xingyun",
  name: "Xingyun",
  description: "Xingyun demand, card, PRD, and branch binding operations",
  version: "0.1.0",
};

function requireCredentials(ctx: ChannelContext): JacpCredentials {
  const creds = ctx.credentials as JacpCredentials | null | undefined;
  if (!creds?.cookie) {
    throw new Error("AuthRequired: 请先运行 poria auth login");
  }
  return creds;
}

async function loadAttachments(
  credentials: JacpCredentials,
  demand: DemandDetail,
): Promise<CardAttachment[]> {
  const code = demand.demandCode?.trim();
  if (!code) return [];
  try {
    return asCardAttachments(await getCardByCode(credentials, code));
  } catch {
    return [];
  }
}

async function resolveRepoForWorkspace(
  credentials: JacpCredentials,
  gitUrl: string,
): Promise<{ code: string; defaultBranchName?: string; gitUrl: string }> {
  // Inline repo resolution to avoid circular dependency on @poria/channel-coding
  const { queryAllRepos } = await import("./jacp/easyci-repo.js");
  const searchPath = repoSearchPathFromGitUrl(gitUrl) ?? "";
  const nameLike = searchPath.split("/").filter(Boolean).pop() ?? "";
  const repos = await queryAllRepos(credentials, nameLike || searchPath, 50);
  const matched = repos.filter(
    (repo: { gitUrl: string }) => sameGitUrl(repo.gitUrl, gitUrl),
  );
  if (matched.length === 0) {
    throw new Error(
      `No EasyCI repository matches workspace gitUrl: ${gitUrl}. ` +
        `Search path was "${searchPath}".`,
    );
  }
  if (matched.length > 1) {
    const lines = matched
      .map((r: { code: string; gitUrl: string }) => `  - ${r.code}\t${r.gitUrl}`)
      .join("\n");
    throw new Error(
      `Multiple EasyCI repositories match workspace gitUrl:\n${lines}`,
    );
  }
  return matched[0]!;
}

class XingyunChannelImpl implements IChannel<XingyunChannelInput, XingyunChannelOutput> {
  readonly metadata = metadata;

  async execute(input: XingyunChannelInput, context: ChannelContext): Promise<XingyunChannelOutput> {
    switch (input.action) {
      case "getDemand":
        return this.getDemand(input.demandId!, context);
      case "listCardAttachments":
        return this.listCardAttachments(input, context);
      case "resolvePrdLink":
        return this.resolvePrdLink(input.demandId!, context);
      case "bindBranch":
        return this.bindBranch(input, context);
      case "communicate":
        return this.communicate(input.demandId!, context);
      case "accept":
        return this.accept(input.demandId!, context);
      default:
        throw new Error(`Unknown action: ${input.action}`);
    }
  }

  private async getDemand(
    demandId: number,
    ctx: ChannelContext,
  ): Promise<XingyunChannelOutput> {
    if (isXingyunFixtureMode()) {
      return { action: "getDemand", demand: { ...FIXTURE_DEMAND, id: demandId } };
    }
    const credentials = requireCredentials(ctx);
    const demand = await getDemandById(credentials, demandId);
    return { action: "getDemand", demand };
  }

  private async listCardAttachments(
    input: XingyunChannelInput,
    ctx: ChannelContext,
  ): Promise<XingyunChannelOutput> {
    if (isXingyunFixtureMode()) {
      return { action: "listCardAttachments", attachments: [...FIXTURE_ATTACHMENTS] };
    }
    const credentials = requireCredentials(ctx);
    let demandCode = input.demandCode?.trim();
    if (!demandCode && input.demandId != null) {
      const demand = await getDemandById(credentials, input.demandId);
      demandCode = demand.demandCode;
    }
    if (!demandCode) {
      throw new Error("listCardAttachments requires demandId or demandCode");
    }
    const attachments = asCardAttachments(
      await getCardByCode(credentials, demandCode),
    );
    return { action: "listCardAttachments", attachments };
  }

  private async resolvePrdLink(
    demandId: number,
    ctx: ChannelContext,
  ): Promise<XingyunChannelOutput> {
    if (isXingyunFixtureMode()) {
      return {
        action: "resolvePrdLink",
        url: FIXTURE_ATTACHMENTS[0]!.url,
        attachment: FIXTURE_ATTACHMENTS[0]!,
      };
    }
    const credentials = requireCredentials(ctx);
    const demand = await getDemandById(credentials, demandId);
    const attachments = await loadAttachments(credentials, demand);
    const result = resolvePrdFromAttachments(attachments);
    return { action: "resolvePrdLink", ...result };
  }

  private async bindBranch(
    input: XingyunChannelInput,
    ctx: ChannelContext,
  ): Promise<XingyunChannelOutput> {
    if (isXingyunFixtureMode()) {
      const demandCode =
        input.branch?.replace(/^feature_/, "") || FIXTURE_DEMAND.demandCode;
      return {
        action: "bindBranch",
        branch: input.branch ?? `feature_${demandCode}`,
        changeId: FIXTURE_BIND.changeId,
        baseBranch: input.baseBranch ?? FIXTURE_BIND.baseBranch,
      };
    }

    const credentials = requireCredentials(ctx);
    const demand = await getDemandById(credentials, input.demandId!);
    const issueCode = demand.demandCode?.trim();
    if (!issueCode) {
      throw new Error("Demand has no demandCode; cannot bind branch");
    }

    const local = await getLocalGitContext(process.cwd());
    const workspaceGitUrl = input.gitUrl?.trim() || local.gitUrl;
    if (!workspaceGitUrl) {
      throw new Error("No git remote in workspace; cannot bind branch");
    }
    if (input.gitUrl && local.gitUrl && !sameGitUrl(input.gitUrl, local.gitUrl)) {
      throw new Error(
        `Workspace gitUrl (${local.gitUrl}) does not match expected (${input.gitUrl})`,
      );
    }

    const repo = await resolveRepoForWorkspace(credentials, workspaceGitUrl);
    const card = await getCardByCode(credentials, issueCode);
    const spaceId = asCardSpaceId(card);
    if (spaceId == null) {
      throw new Error("Card has no spaceId; cannot resolve EasyCI space");
    }
    const space = await getSpaceById(credentials, spaceId);
    const spaceKey = asSpaceKey(space);
    if (!spaceKey) {
      throw new Error("Space has no key; cannot createChange");
    }

    const bindAppsResult = await queryBindDeployApps(
      credentials,
      spaceKey,
      repo.code,
    );
    const devSpaceId = bindAppsResult.devSpaceId?.trim();
    if (!devSpaceId) {
      throw new Error(
        `EasyCI space "${spaceKey}" returned no devSpaceId for repo ${repo.code}`,
      );
    }

    const bindApps = bindAppsResult.records
      .filter((app) => app.systemId && app.appKey)
      .map((app) => ({
        systemId: app.systemId as string,
        appKey: app.appKey,
      }));

    const branch =
      input.branch?.trim() ||
      featureBranchName(demand.demandCode, demand.id);
    const createLocal = input.createLocal !== false;

    const baseBranch =
      input.baseBranch?.trim() ||
      (await detectOriginHeadBranch(local.workspacePath ?? process.cwd())) ||
      repo.defaultBranchName?.trim() ||
      "master";

    if (createLocal) {
      await createAndPushBranch({
        cwd: local.workspacePath,
        expectedGitUrl: workspaceGitUrl,
        defaultBranch: baseBranch,
        newBranch: branch,
      });
    }

    const changeId = await createChange(credentials, {
      devSpaceId,
      name: "",
      code: repo.code,
      issueCode,
      branch,
      branchOperateType: "SELECT",
      bindApps,
    });

    return { action: "bindBranch", branch, changeId, baseBranch };
  }

  private async communicate(
    demandId: number,
    ctx: ChannelContext,
  ): Promise<XingyunChannelOutput> {
    if (isXingyunFixtureMode()) {
      return { action: "communicate", demandId, demandStatusCode: 20 };
    }
    const credentials = requireCredentials(ctx);
    const result: DemandActionResult = await communicateDemand(credentials, demandId);
    return { action: "communicate", demandId, demandStatusCode: result.demandStatusCode };
  }

  private async accept(
    demandId: number,
    ctx: ChannelContext,
  ): Promise<XingyunChannelOutput> {
    if (isXingyunFixtureMode()) {
      return { action: "accept", demandId, demandStatusCode: 30 };
    }
    const credentials = requireCredentials(ctx);
    const result: DemandActionResult = await acceptDemand(credentials, demandId);
    return { action: "accept", demandId, demandStatusCode: result.demandStatusCode };
  }
}

export function createXingyunChannel(): XingyunChannelImpl {
  return new XingyunChannelImpl();
}

// Re-exports
export {
  parseXingyunDemandUrl,
  featureBranchName,
  featureSlug,
} from "./demandUrl.js";
export { resolvePrdFromAttachments, PrdResolveError } from "./prd.js";
export {
  isXingyunFixtureMode,
  FIXTURE_DEMAND,
  FIXTURE_ATTACHMENTS,
} from "./fixture.js";
export { isJoySpacePrdLink, parsePrdLink } from "./jacp/prdAttachmentLink.js";
export { isDemandActive } from "./demand-guard.js";
export type { CardAttachment } from "./jacp/cards.js";
export type { DemandDetail, UserVO, DemandActionResult } from "./jacp/demands.js";
export type { JacpCredentials } from "./types.js";
