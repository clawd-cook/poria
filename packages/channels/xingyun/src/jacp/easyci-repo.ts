/**
 * EasyCI repo query — inlined from channel-coding to avoid circular dependency.
 * Used by xingyun's resolveRepoForWorkspace.
 */
import { fetchWithRetry, type JacpCredentials } from "./client.js";

const DEFAULT_EASYCI_GQL_URL = "http://easyci.jd.com/gql";

const QUERY_ALL_REPOS = `query queryAllRepos($query: RepositoryQuery) {
  allRepositories(query: $query) {
    repositories {
      code
      defaultBranchName
      gitUrl
      homeUrl
      __typename
    }
    totalCount
    __typename
  }
}`;

function asOptionalText(value: unknown): string | undefined {
  if (value == null) return undefined;
  const text = String(value).trim();
  return text ? text : undefined;
}

function asUnknownList(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

interface EasyciRepo {
  code: string;
  defaultBranchName?: string;
  gitUrl: string;
  homeUrl?: string;
}

function asRepo(value: unknown): EasyciRepo | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as Record<string, unknown>;
  const gitUrl = asOptionalText(item.gitUrl);
  const code = asOptionalText(item.code);
  if (!gitUrl || !code) return undefined;
  return {
    code,
    defaultBranchName: asOptionalText(item.defaultBranchName),
    gitUrl,
    homeUrl: asOptionalText(item.homeUrl),
  };
}

export async function queryAllRepos(
  credentials: JacpCredentials,
  nameLike = "",
  size = 20,
): Promise<EasyciRepo[]> {
  const pageSize = size > 0 ? size : 20;

  let response: Response;
  try {
    response = await fetchWithRetry(DEFAULT_EASYCI_GQL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Cookie: credentials.cookie,
      },
      body: JSON.stringify({
        operationName: "queryAllRepos",
        variables: { query: { page: 1, nameLike, size: pageSize } },
        query: QUERY_ALL_REPOS,
      }),
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`仓库搜索失败: ${message}`);
  }

  if (response.status === 401) {
    throw new Error("登录态已失效，请重新登录（poria auth login）");
  }
  if (!response.ok) {
    throw new Error(`仓库搜索失败: HTTP ${response.status}`);
  }

  const body = (await response.json()) as {
    data?: { allRepositories?: { repositories?: unknown } };
    errors?: Array<{ message?: string }>;
  };

  if (body.errors?.length && body.data == null) {
    throw new Error(
      `仓库搜索失败: ${body.errors.map((e) => e.message).join("; ")}`,
    );
  }
  if (body.data == null) {
    throw new Error("仓库搜索失败: empty response");
  }

  return asUnknownList(body.data.allRepositories?.repositories)
    .map(asRepo)
    .filter((item): item is EasyciRepo => Boolean(item));
}
