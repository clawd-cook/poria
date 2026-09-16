import fs from "node:fs";
import path from "node:path";
import type { JacpCredentials, Logger } from "./types.js";
import {
  createJoySpaceApiContext,
  downloadDiagramXml,
  extractPageIdFromUrl,
  fetchDiagramDetail,
  fetchPageBasic,
  fetchPageContent,
} from "./vendor/joyspace-api-client.mjs";
import { joyspaceContentToMarkdown } from "./vendor/joyspace-content-to-markdown.mjs";
import { drawioXmlToMermaid } from "./vendor/joyspace-drawio-to-mermaid.mjs";

export function isJoySpaceFixtureMode(): boolean {
  return process.env.PORIA_JOYSPACE_FIXTURE === "1";
}

export type ExportJoySpaceInput = {
  url: string;
  outputDir?: string;
  outputName?: string;
  tenantCode?: string;
  credentials?: JacpCredentials;
  logger?: Logger;
};

export type ExportJoySpaceResult = {
  outputPath: string;
  title: string;
  cookieSource?: string;
};

function sanitizeFilename(value: string): string {
  const cleaned = String(value || "Doc")
    .replace(/[/\\:*?"<>|]/g, "_")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || "Doc";
}

function ensureMarkdownSuffix(filename: string): string {
  return filename.toLowerCase().endsWith(".md") ? filename : `${filename}.md`;
}

function resolveOutputPath(outputDir: string, filename: string): string {
  const initialPath = path.join(outputDir, ensureMarkdownSuffix(filename));
  if (!fs.existsSync(initialPath)) {
    return initialPath;
  }

  const parsed = path.parse(initialPath);
  let index = 1;
  while (true) {
    const candidate = path.join(
      parsed.dir,
      `${parsed.name}-${index}${parsed.ext}`,
    );
    if (!fs.existsSync(candidate)) {
      return candidate;
    }
    index += 1;
  }
}

function collectDiagramIds(content: unknown[]): string[] {
  const ids = new Set<string>();
  const visit = (nodes: unknown): void => {
    if (!Array.isArray(nodes)) return;
    for (const node of nodes) {
      if (!node || typeof node !== "object") continue;
      const record = node as Record<string, unknown>;
      if (record.type === "diagram") {
        const id = String(record.diagramId || record.id || "").trim();
        if (id) ids.add(id);
      }
      if (Array.isArray(record.children)) {
        visit(record.children);
      }
    }
  };
  visit(content);
  return [...ids];
}

async function renderSvgFromXml(xml: string, logger?: Logger): Promise<string> {
  const exportUrl = process.env.JOYSPACE_DRAWIO_EXPORT_URL;
  if (!exportUrl || !xml) return "";
  try {
    const response = await fetch(exportUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/xml",
        Accept: "image/svg+xml",
      },
      body: xml,
    });
    if (!response.ok) {
      logger?.warn?.(
        `[diagram] SVG render failed HTTP ${response.status} ${response.statusText}`,
      );
      return "";
    }
    const svg = await response.text();
    return svg.includes("<svg") ? svg : "";
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    logger?.warn?.(`[diagram] SVG render error: ${message}`);
    return "";
  }
}

type DiagramInfo = {
  title: string;
  linkUrl: string;
  pageUrl: string;
  svg: string;
  mermaid: string;
  error?: string;
};

type ApiContext = Awaited<ReturnType<typeof createJoySpaceApiContext>>;

async function fetchDiagramEntry(input: {
  diagramId: string;
  pageId: string;
  apiContext: ApiContext;
  pageUrl: string;
  logger?: Logger;
}): Promise<[string, DiagramInfo]> {
  const { diagramId, pageId, apiContext, pageUrl, logger } = input;
  try {
    const detail = await fetchDiagramDetail({
      diagramId,
      pageId,
      cookieHeader: apiContext.cookieHeader,
      teamHeaderId: apiContext.teamHeaderId,
    });

    const linkUrl = String(
      detail?.linkUrl || detail?.link_url || detail?.url || "",
    );
    const title = String(
      detail?.title || detail?.name || detail?.fileName || "",
    ).trim();

    let xml = "";
    if (linkUrl) {
      try {
        xml = await downloadDiagramXml(linkUrl);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        logger?.warn?.(`[diagram ${diagramId}] XML download failed: ${message}`);
      }
    }

    const svg = xml ? await renderSvgFromXml(xml, logger) : "";
    let mermaid = "";
    if (xml && !svg) {
      try {
        mermaid = drawioXmlToMermaid(xml) || "";
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        logger?.warn?.(
          `[diagram ${diagramId}] mermaid conversion failed: ${message}`,
        );
      }
    }

    return [
      diagramId,
      { title, linkUrl, pageUrl, svg, mermaid },
    ];
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    logger?.warn?.(`[diagram ${diagramId}] detail fetch failed: ${message}`);
    return [
      diagramId,
      {
        title: "",
        linkUrl: "",
        pageUrl,
        svg: "",
        mermaid: "",
        error: message,
      },
    ];
  }
}

async function fetchDiagramsMap(input: {
  content: unknown[];
  pageId: string;
  apiContext: ApiContext;
  pageUrl: string;
  logger?: Logger;
  concurrency?: number;
}): Promise<Map<string, DiagramInfo>> {
  const {
    content,
    pageId,
    apiContext,
    pageUrl,
    logger,
    concurrency = 4,
  } = input;
  const ids = collectDiagramIds(content);
  const diagrams = new Map<string, DiagramInfo>();
  if (ids.length === 0) return diagrams;

  let cursor = 0;
  const runWorker = async (): Promise<void> => {
    while (cursor < ids.length) {
      const index = cursor;
      cursor += 1;
      const diagramId = ids[index]!;
      const [, info] = await fetchDiagramEntry({
        diagramId,
        pageId,
        apiContext,
        pageUrl,
        logger,
      });
      diagrams.set(diagramId, info);
    }
  };

  await Promise.all(
    Array.from({ length: Math.min(concurrency, ids.length) }, () =>
      runWorker(),
    ),
  );
  return diagrams;
}

function writeFixtureMarkdown(input: ExportJoySpaceInput): ExportJoySpaceResult {
  const outputDir = path.resolve(input.outputDir || process.cwd());
  fs.mkdirSync(outputDir, { recursive: true });
  const title = "JoySpace Fixture Page";
  const requestedName = input.outputName
    ? sanitizeFilename(input.outputName)
    : sanitizeFilename(title);
  const outputPath = resolveOutputPath(outputDir, requestedName);
  const markdown = [
    `# ${title}`,
    "",
    `Source: ${input.url}`,
    "",
    "This is offline fixture markdown from `PORIA_JOYSPACE_FIXTURE=1`.",
    "",
  ].join("\n");
  fs.writeFileSync(outputPath, markdown, "utf8");
  return { outputPath, title, cookieSource: "fixture" };
}

export async function exportJoySpaceMarkdown(
  input: ExportJoySpaceInput,
): Promise<ExportJoySpaceResult> {
  if (isJoySpaceFixtureMode()) {
    return writeFixtureMarkdown(input);
  }

  const url = input.url?.trim();
  if (!url) {
    throw new Error("JoySpace URL is required");
  }

  const pageId = extractPageIdFromUrl(url);
  const outputDir = path.resolve(input.outputDir || process.cwd());
  fs.mkdirSync(outputDir, { recursive: true });

  const credentialOverrides = input.credentials?.cookie
    ? {
        cookieHeader: input.credentials.cookie,
        cookieSource: input.credentials.username
          ? `poria-auth:${input.credentials.username}`
          : "poria-auth",
      }
    : {};

  const apiContext = await createJoySpaceApiContext({
    tenantCode: input.tenantCode,
    ...credentialOverrides,
  });

  const basic = await fetchPageBasic({
    pageId,
    cookieHeader: apiContext.cookieHeader,
    teamHeaderId: apiContext.teamHeaderId,
  });

  let effectivePageId = pageId;
  let titleSource: Record<string, unknown> = basic;
  const pageType = basic?.page_type ?? basic?.type;
  let usedOriginId: string | null = null;

  if (pageType === 5 && basic?.origin_id) {
    effectivePageId = String(basic.origin_id);
    usedOriginId = effectivePageId;
    input.logger?.info?.(
      `[info] page_type=${pageType} (reference page), falling back to origin_id=${effectivePageId}`,
    );
    try {
      const originBasic = await fetchPageBasic({
        pageId: effectivePageId,
        cookieHeader: apiContext.cookieHeader,
        teamHeaderId: apiContext.teamHeaderId,
      });
      titleSource = originBasic;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      input.logger?.warn?.(`[warn] Failed to fetch origin basic: ${message}`);
    }
  } else if (pageType && !["13", 13, 5, "5"].includes(String(pageType))) {
    input.logger?.warn?.(
      `[warn] page_type=${pageType} may not be fully supported`,
    );
  }

  const contentPayload = await fetchPageContent({
    pageId: effectivePageId,
    cookieHeader: apiContext.cookieHeader,
    teamHeaderId: apiContext.teamHeaderId,
  });

  const content = contentPayload?.content;
  if (!Array.isArray(content) || content.length === 0) {
    throw new Error(
      `JoySpace API returned no valid data.content. page_type=${pageType ?? "unknown"}, pageId=${pageId}${
        usedOriginId ? ` (used origin ${usedOriginId})` : ""
      }`,
    );
  }

  const diagrams = await fetchDiagramsMap({
    content,
    pageId: effectivePageId,
    apiContext,
    pageUrl: url,
    logger: input.logger,
  });

  const conversion = joyspaceContentToMarkdown({
    title: String(
      titleSource?.title ||
        titleSource?.full_name ||
        basic?.title ||
        basic?.full_name ||
        "Doc",
    ),
    content,
    diagrams,
    pageUrl: url,
  });

  if (!conversion.markdown.trim()) {
    throw new Error("Markdown conversion produced empty output");
  }

  const requestedName = input.outputName
    ? sanitizeFilename(input.outputName)
    : sanitizeFilename(conversion.title);
  const outputPath = resolveOutputPath(outputDir, requestedName);
  const markdown = conversion.markdown.endsWith("\n")
    ? conversion.markdown
    : `${conversion.markdown}\n`;

  fs.writeFileSync(outputPath, markdown, "utf8");

  return {
    outputPath,
    title: conversion.title,
    cookieSource: apiContext.cookieSource || undefined,
  };
}
