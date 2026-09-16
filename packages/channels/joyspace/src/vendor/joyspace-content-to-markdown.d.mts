export function joyspaceContentToMarkdown(input: {
  title?: string;
  content?: unknown[];
  diagrams?: Map<string, unknown> | Record<string, unknown>;
  pageUrl?: string;
}): {
  title: string;
  markdown: string;
  warnings: string[];
};
