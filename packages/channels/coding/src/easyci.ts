import type { JacpCredentials, CodingRepo, CodingBranch } from "./types.js";

export const DEFAULT_EASYCI_GQL_URL = "http://easyci.jd.com/gql";

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

const QUERY_BRANCHES = `query queryBranches($gitUrl: String!, $query: BranchQuery) {
  findRepository(gitUrl: $gitUrl) {
    branches(query: $query) {
      branches {
        name
        status
        __typename
      }
      totalCount
      __typename
    }
    __typename
  }
}`;

function asOptionalText(value: unknown): string | undefined {
  if (value == null) return undefined;
  const text = String(value).trim();
  return text ? text : undefined;
}

function asUnknownList(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  return [];
}

function repoLabel(homeUrl?: string, gitUrl?: string): string {
  return homeUrl || gitUrl || "";
}

function asRepo(value: unknown): CodingRepo | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as Record<string, unknown>;
  const gitUrl = asOptionalText(item.gitUrl);
  const code = asOptionalText(item.code);
  if (!gitUrl || !code) return undefined;
  const homeUrl = asOptionalText(item.homeUrl);
  return {
    code,
    defaultBranchName: asOptionalText(item.defaultBranchName),
    gitUrl,
    homeUrl,
    repoLabel: repoLabel(homeUrl, gitUrl),
  };
}

function asBranch(value: unknown): CodingBranch | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as Record<string, unknown>;
  const name = asOptionalText(item.name);
  if (!name) return undefined;
  return { name, status: asOptionalText(item.status) };
}

async function easyciGql<T>(
  credentials: JacpCredentials,
  errorLabel: string,
  operationName: string,
  query: string,
  variables: Record<string, unknown>,
): Promise<T> {
  let response: Response;
  try {
    response = await fetch(DEFAULT_EASYCI_GQL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Cookie: credentials.cookie,
      },
      body: JSON.stringify({ operationName, variables, query }),
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${errorLabel}: ${message}`);
  }

  if (response.status === 401) {
    throw new Error("登录态已失效，请重新登录（poria auth login）");
  }
  if (!response.ok) {
    throw new Error(`${errorLabel}: HTTP ${response.status}`);
  }

  const body = (await response.json()) as {
    data?: T;
    errors?: Array<{ message?: string }>;
  };
  const graphqlError = body.errors
    ?.map((item) => item.message)
    .filter(Boolean)
    .join("; ");
  if (graphqlError && body.data == null) {
    throw new Error(`${errorLabel}: ${graphqlError}`);
  }
  if (body.data == null) {
    throw new Error(`${errorLabel}: empty response`);
  }
  return body.data;
}

export async function queryAllRepos(
  credentials: JacpCredentials,
  nameLike = "",
  size = 20,
): Promise<CodingRepo[]> {
  const pageSize = size > 0 ? size : 20;
  const data = await easyciGql<{
    allRepositories?: { repositories?: unknown };
  }>(credentials, "仓库搜索失败", "queryAllRepos", QUERY_ALL_REPOS, {
    query: { page: 1, nameLike, size: pageSize },
  });
  return asUnknownList(data.allRepositories?.repositories)
    .map(asRepo)
    .filter((item): item is CodingRepo => Boolean(item));
}

export async function queryBranches(
  credentials: JacpCredentials,
  gitUrl: string,
  nameLike = "",
): Promise<CodingBranch[]> {
  const url = gitUrl.trim();
  if (!url) throw new Error("缺少仓库地址");
  const data = await easyciGql<{
    findRepository?: { branches?: { branches?: unknown } } | null;
  }>(credentials, "分支搜索失败", "queryBranches", QUERY_BRANCHES, {
    gitUrl: url,
    query: { page: 1, nameLike, size: 20 },
  });
  return asUnknownList(data.findRepository?.branches?.branches)
    .map(asBranch)
    .filter((item): item is CodingBranch => Boolean(item));
}

/** Offline / CI fixture repos when PORIA_CODING_FIXTURE=1 */
export const FIXTURE_REPOS: CodingRepo[] = [
  {
    code: "fixture-poria-demo",
    gitUrl: "git@coding.jd.com:poria/demo.git",
    homeUrl: "https://coding.jd.com/poria/demo",
    defaultBranchName: "master",
    repoLabel: "poria/demo",
  },
  {
    code: "fixture-poria-cli",
    gitUrl: "git@coding.jd.com:poria/cli.git",
    homeUrl: "https://coding.jd.com/poria/cli",
    defaultBranchName: "main",
    repoLabel: "poria/cli",
  },
];

export const FIXTURE_BRANCHES: CodingBranch[] = [
  { name: "master", status: "active" },
  { name: "feature/demo", status: "active" },
  { name: "develop", status: "active" },
];

export function isCodingFixtureMode(): boolean {
  return process.env.PORIA_CODING_FIXTURE === "1";
}

export async function searchReposFixture(query: {
  nameLike?: string;
  size?: number;
}): Promise<CodingRepo[]> {
  const like = (query.nameLike ?? "").toLowerCase();
  let repos = FIXTURE_REPOS;
  if (like) {
    repos = repos.filter(
      (r) =>
        r.code.toLowerCase().includes(like) ||
        r.gitUrl.toLowerCase().includes(like) ||
        (r.repoLabel ?? "").toLowerCase().includes(like),
    );
  }
  const size = query.size && query.size > 0 ? query.size : 20;
  return repos.slice(0, size);
}

export async function listBranchesFixture(
  _gitUrl: string,
  nameLike?: string,
): Promise<CodingBranch[]> {
  const like = (nameLike ?? "").toLowerCase();
  if (!like) return [...FIXTURE_BRANCHES];
  return FIXTURE_BRANCHES.filter((b) => b.name.toLowerCase().includes(like));
}

export async function createMergeRequestFixture(input: {
  title: string;
  description?: string;
  sourceBranch?: string;
  targetBranch?: string;
  projectId?: string;
}): Promise<{
  url: string;
  iid: number;
  sourceBranch: string;
  targetBranch: string;
}> {
  const sourceBranch = input.sourceBranch?.trim() || "feature_demo";
  const targetBranch = input.targetBranch?.trim() || "master";
  const project = input.projectId?.trim() || "poria/demo";
  return {
    url: `https://coding.jd.com/${project}/merge_requests/1`,
    iid: 1,
    sourceBranch,
    targetBranch,
  };
}
