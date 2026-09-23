/**
 * E2E mock 响应标准 headers 模板。
 *
 * 两套策略：
 * - API_RESPONSE_HEADERS：access-control-allow-origin: *
 *   用于不需要 app 真正消费的 API（如已下线的接口），浏览器 CORS 拦截后 app 走 catch
 * - API_RESPONSE_HEADERS_CREDENTIALED：access-control-allow-origin 从 TEST_BASE_URL 取 origin
 *   用于 app 必须消费的 API（credentials:include 场景，CORS 禁止 wildcard）
 *
 * 如果项目请求层有额外校验头（如自定义网关 code 头），在项目 _helpers 中扩展这些 headers。
 */

function getOriginFromBaseURL(): string {
  const baseURL = process.env.TEST_BASE_URL || "http://localhost:3000";
  try {
    const url = new URL(baseURL);
    return url.origin;
  } catch {
    return baseURL;
  }
}

export const API_RESPONSE_HEADERS: Record<string, string> = {
  "content-type": "text/plain;charset=utf-8",
  "access-control-allow-origin": "*",
  "access-control-allow-credentials": "true",
};

export function getCredentialedHeaders(): Record<string, string> {
  return {
    "content-type": "text/plain;charset=utf-8",
    "access-control-allow-origin": getOriginFromBaseURL(),
    "access-control-allow-credentials": "true",
  };
}

export const API_RESPONSE_HEADERS_CREDENTIALED: Record<string, string> = getCredentialedHeaders();
