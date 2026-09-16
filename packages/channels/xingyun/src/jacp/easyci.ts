import {
  asOptionalNumber,
  asOptionalText,
  fetchWithRetry,
  type JacpCredentials,
} from "./client.js";

export const DEFAULT_EASYCI_GQL_URL = "http://easyci.jd.com/gql";

export type EasyciDeployApp = {
  id: string;
  name: string;
  appKey: string;
  systemId?: string;
  systemKey?: string;
  gitUrl?: string;
  deploySystemName?: string;
  alreadyBound: boolean;
};

export type BindDeployAppsResult = {
  devSpaceId?: string;
  records: EasyciDeployApp[];
};

export type CreateChangeInput = {
  devSpaceId: string;
  name?: string;
  code: string;
  issueCode: string;
  branch: string;
  branchOperateType: "SELECT" | "CREATE";
  bindApps: Array<{ systemId: string; appKey: string }>;
};

const QUERY_BIND_DEPLOY_APPS = `query queryBindDeployApps($key: String!, $repoCode: String!, $withBindApps: Boolean = true) {
  devSpace(key: $key) {
    id
    alreadyBindDeployApps(query: {code: $repoCode}) {
      name
      appKey
      systemId
      gitUrl
      deploySystem {
        id
        name
        env
        systemKey
        __typename
      }
      __typename
    }
    bindApps(query: {code: $repoCode, page: 1, size: 200}) @include(if: $withBindApps) {
      totalCount
      bindApps {
        app {
          name
          appKey
          systemId
          gitUrl
          deploySystem {
            id
            name
            env
            systemKey
            __typename
          }
          __typename
        }
        __typename
      }
      __typename
    }
    __typename
  }
}`;

const MUTATION_CREATE_CHANGE = `mutation createChange($input: CreateChangeInput!) {
  createChange(input: $input) {
    id
    __typename
  }
}`;

function asUnknownList(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function deployAppId(appKey: string, systemId?: string): string {
  return `${appKey}::${systemId ?? ""}`;
}

function asDeployApp(
  value: unknown,
  alreadyBound: boolean,
): EasyciDeployApp | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as Record<string, unknown>;
  const appKey = asOptionalText(item.appKey);
  if (!appKey) return undefined;
  const deploySystem =
    item.deploySystem && typeof item.deploySystem === "object"
      ? (item.deploySystem as Record<string, unknown>)
      : undefined;
  const systemId =
    asOptionalText(item.systemId) || asOptionalText(deploySystem?.id);
  return {
    id: deployAppId(appKey, systemId),
    name: asOptionalText(item.name) || appKey,
    appKey,
    systemId,
    systemKey: asOptionalText(deploySystem?.systemKey),
    gitUrl: asOptionalText(item.gitUrl),
    deploySystemName: asOptionalText(deploySystem?.name),
    alreadyBound,
  };
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
    response = await fetchWithRetry(DEFAULT_EASYCI_GQL_URL, {
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

export async function queryBindDeployApps(
  credentials: JacpCredentials,
  spaceKey: string,
  repoCode: string,
): Promise<BindDeployAppsResult> {
  const key = spaceKey.trim();
  const code = repoCode.trim();
  if (!key) throw new Error("缺少空间 key");
  if (!code) throw new Error("缺少仓库编码");
  const data = await easyciGql<{
    devSpace?: {
      id?: unknown;
      alreadyBindDeployApps?: unknown;
      bindApps?: { bindApps?: unknown };
    } | null;
  }>(
    credentials,
    "绑定应用查询失败",
    "queryBindDeployApps",
    QUERY_BIND_DEPLOY_APPS,
    { withBindApps: true, key, repoCode: code },
  );
  const merged = new Map<string, EasyciDeployApp>();
  for (const app of asUnknownList(data.devSpace?.alreadyBindDeployApps)
    .map((item) => asDeployApp(item, true))
    .filter((item): item is EasyciDeployApp => Boolean(item))) {
    merged.set(app.id, app);
  }
  for (const row of asUnknownList(data.devSpace?.bindApps?.bindApps)) {
    const nested =
      row && typeof row === "object"
        ? (row as { app?: unknown }).app
        : undefined;
    const app = asDeployApp(nested, false);
    if (!app) continue;
    if (!merged.has(app.id)) merged.set(app.id, app);
  }
  return {
    devSpaceId:
      asOptionalText(data.devSpace?.id) ||
      String(asOptionalNumber(data.devSpace?.id) ?? ""),
    records: [...merged.values()],
  };
}

export async function createChange(
  credentials: JacpCredentials,
  input: CreateChangeInput,
): Promise<string> {
  const data = await easyciGql<{
    createChange?: { id?: unknown } | null;
  }>(credentials, "关联分支失败", "createChange", MUTATION_CREATE_CHANGE, {
    input,
  });
  const id =
    asOptionalText(data.createChange?.id) ||
    String(asOptionalNumber(data.createChange?.id) ?? "");
  if (!id) throw new Error("关联分支失败：未返回变更 ID");
  return id;
}
