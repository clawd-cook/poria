function normalizeWhitespace(value) {
  return String(value || "")
      .replace(/[\u200B\uFEFF]/g, "")
      .replace(/\r/g, "")
      .replace(/[ \t]+\n/g, "\n");
}

function normalizeText(value) {
  return normalizeWhitespace(value).replace(/\s+/g, " ").trim();
}

function escapeTableCell(value) {
  return value.replace(/\|/g, "\\|");
}

function decorateMarkedText(text, decorator) {
  const value = String(text || "");
  const leading = value.match(/^\s*/u)?.[0] || "";
  const trailing = value.match(/\s*$/u)?.[0] || "";
  const core = value.slice(leading.length, value.length - trailing.length);

  if (!core) {
    return value;
  }

  return `${leading}${decorator(core)}${trailing}`;
}

function hasInlineHighlight(node) {
  if (!node || typeof node !== "object") {
    return false;
  }
  if (node.highlight === true) {
    return true;
  }
  const bgColor = typeof node.bgColor === "string" ? node.bgColor.trim() : "";
  if (bgColor && bgColor.toLowerCase() !== "transparent") {
    return true;
  }
  const backgroundColor =
      typeof node.backgroundColor === "string" ? node.backgroundColor.trim() : "";
  if (backgroundColor && backgroundColor.toLowerCase() !== "transparent") {
    return true;
  }
  return false;
}

function applyMarks(text, node) {
  let output = text;
  if (!output) {
    return "";
  }
  if (node.code) {
    output = decorateMarkedText(output, (value) => `\`${value}\``);
  }
  if (node.bold) {
    output = decorateMarkedText(output, (value) => `**${value}**`);
  }
  if (node.italic) {
    output = decorateMarkedText(output, (value) => `*${value}*`);
  }
  if (node.strikethrough || node.strike) {
    output = decorateMarkedText(output, (value) => `~~${value}~~`);
  }
  if (node.underline) {
    output = decorateMarkedText(output, (value) => `<u>${value}</u>`);
  }
  if (hasInlineHighlight(node)) {
    output = decorateMarkedText(output, (value) => `<mark>${value}</mark>`);
  }
  return output;
}

function needsInlineSpace(left, right) {
  const leftTrimmed = String(left || "").replace(/\s+$/u, "");
  const rightTrimmed = String(right || "").replace(/^\s+/u, "");

  if (!leftTrimmed || !rightTrimmed) {
    return false;
  }

  if (!/^[@[]/.test(rightTrimmed)) {
    return false;
  }

  if (/[：:，。、；！？,.!?([{（【「『]$/u.test(leftTrimmed)) {
    return false;
  }

  return /[@)\]\w\u4e00-\u9fff）】」』]$/u.test(leftTrimmed);
}

function joinInlineFragments(parts) {
  let output = "";

  for (const part of parts) {
    if (!part) {
      continue;
    }

    if (needsInlineSpace(output, part)) {
      output += " ";
    }

    output += part;
  }

  return output;
}

function inlineNodeToMarkdown(node) {
  if (!node) {
    return "";
  }

  if (Array.isArray(node)) {
    return joinInlineFragments(node.map((item) => inlineNodeToMarkdown(item)));
  }

  if (typeof node === "string") {
    return normalizeWhitespace(node);
  }

  if (typeof node.text === "string") {
    return applyMarks(normalizeWhitespace(node.text), node);
  }

  if (node.type === "mention") {
    return node?.value?.name ? `@${node.value.name}` : "";
  }

  if (node.type === "docfile") {
    const docTitle = node?.value?.title;
    const docId = node?.value?.id;
    if (docTitle && docId) return `[${docTitle}](https://joyspace.jd.com/pages/${docId})`;
    return docTitle ? `[doc] ${docTitle}` : "";
  }

  if (node.type === "icon-link") {
    const title = normalizeText(node.title || node?.value?.title || "");
    const url = normalizeText(node.url || node?.value?.url || "");
    if (title && url) {
      return `[${title}](${url})`;
    }
    return title || url;
  }

  if (node.type === "link") {
    const label = normalizeText(inlineNodeToMarkdown(node.children));
    const href =
        normalizeText(node.url || node.href || node.link || node?.value?.url) || label;
    if (!href) {
      return label;
    }
    return label ? `[${label}](${href})` : href;
  }

  if (Array.isArray(node.children)) {
    return joinInlineFragments(node.children.map((item) => inlineNodeToMarkdown(item)));
  }

  return "";
}

function inlineChildrenToMarkdown(children) {
  return normalizeWhitespace(inlineNodeToMarkdown(children)).trim();
}

function blockPlainText(block) {
  if (!block) {
    return "";
  }

  if (Array.isArray(block)) {
    return normalizeText(joinInlineFragments(block.map((item) => blockPlainText(item))));
  }

  if (typeof block.text === "string") {
    return normalizeText(block.text);
  }

  if (block.type === "mention") {
    return block?.value?.name ? `@${block.value.name}` : "";
  }

  if (block.type === "docfile") {
    const docTitle = block?.value?.title;
    const docId = block?.value?.id;
    if (docTitle && docId) return `[${docTitle}](https://joyspace.jd.com/pages/${docId})`;
    return docTitle ? `[doc] ${docTitle}` : "";
  }

  if (block.type === "icon-link") {
    const title = normalizeText(block.title || block?.value?.title || "");
    const url = normalizeText(block.url || block?.value?.url || "");
    if (title && url) {
      return `[${title}](${url})`;
    }
    return title || url;
  }

  if (Array.isArray(block.children)) {
    return normalizeText(joinInlineFragments(block.children.map((item) => blockPlainText(item))));
  }

  return "";
}

function pruneOrderedCounters(counters, indent) {
  for (const key of [...counters.keys()]) {
    if (key > indent) {
      counters.delete(key);
    }
  }
}

function renderBlocks(blocks, state) {
  const pieces = [];
  for (const block of blocks || []) {
    const rendered = renderBlock(block, state);
    if (rendered) {
      pieces.push(rendered);
    }
    if (block?.type !== "list" || block?.header) {
      state.orderedCounters.clear();
    }
  }
  return pieces.join("");
}

function lookupDiagramInfo(state, diagramId) {
  if (!state?.diagrams || !diagramId) {
    return null;
  }
  if (typeof state.diagrams.get === "function") {
    return state.diagrams.get(diagramId) || null;
  }
  if (typeof state.diagrams === "object") {
    return state.diagrams[diagramId] || null;
  }
  return null;
}

function normalizeDiagramSvg(svg) {
  const text = String(svg || "").trim();
  if (!text) {
    return "";
  }
  return text;
}

function renderDiagramCaption(info, diagramId, state) {
  const titleText = normalizeText(info?.title || "");
  const pageUrl = info?.pageUrl || state?.pageUrl || "";
  const linkUrl = info?.linkUrl || "";
  const headline = `**JoySpace 绘图${titleText ? `：${titleText}` : ""}**`;

  const metaLines = [];
  if (pageUrl) {
    metaLines.push(`- 原页面：${pageUrl}`);
  }
  if (linkUrl) {
    metaLines.push(`- [drawio 源文件（签名链接，短期有效）](${linkUrl})`);
  }
  if (diagramId) {
    metaLines.push(`- diagramId: \`${diagramId}\``);
  }

  if (metaLines.length === 0) {
    return `${headline}\n\n`;
  }
  return `${headline}\n\n${metaLines.join("\n")}\n\n`;
}

function renderDiagramLinkLines(info, diagramId, state, prefix = "> ") {
  const titleText = normalizeText(info?.title || "");
  const lines = [];
  lines.push(`${prefix}**JoySpace 绘图${titleText ? `：${titleText}` : ""}**`);
  lines.push(prefix.trimEnd());
  const pageUrl = info?.pageUrl || state?.pageUrl;
  if (pageUrl) {
    lines.push(`${prefix}- 原页面：${pageUrl}`);
  }
  if (info?.linkUrl) {
    lines.push(`${prefix}- drawio 源文件（签名链接，短期有效）：${info.linkUrl}`);
  }
  if (diagramId) {
    lines.push(`${prefix}- diagramId: ${diagramId}`);
  }
  return lines;
}

function renderDiagramBlock(block, state) {
  const diagramId = normalizeText(block.diagramId || block.id || "");
  const info = lookupDiagramInfo(state, diagramId);

  if (info?.svg) {
    const caption = renderDiagramCaption(info, diagramId, state);
    state.warnings.add("diagram-svg");
    return `${caption}${normalizeDiagramSvg(info.svg)}\n\n`;
  }

  if (info?.mermaid) {
    const caption = renderDiagramCaption(info, diagramId, state);
    state.warnings.add("diagram-mermaid");
    return `${caption}\`\`\`mermaid\n${info.mermaid.trim()}\n\`\`\`\n\n`;
  }

  if (!info) {
    state.warnings.add("diagram");
    return `> JoySpace diagram not exported (diagramId: ${diagramId || "unknown"})\n\n`;
  }

  state.warnings.add("diagram-link");
  const lines = renderDiagramLinkLines(info, diagramId, state, "> ");
  return `${lines.join("\n")}\n\n`;
}

function renderAttachmentBlock(block) {
  const fileName = normalizeText(block.value?.fileName || "");
  const fileId = normalizeText(block.value?.fileId || "");
  if (fileId) {
    return `- [${fileName}](https://joyspace.jd.com/api/files/${fileId})\n`;
  }
  const relativeUrl = normalizeText(block.value?.url || "");
  if (relativeUrl) {
    return `- [${fileName}](https://joyspace.jd.com${relativeUrl})\n`;
  }
  return fileName ? `- ${fileName}\n` : "";
}
function renderDiagramInlineFallback(block, state) {
  const diagramId = normalizeText(block.diagramId || block.id || "");
  const info = lookupDiagramInfo(state, diagramId);
  if (!info) {
    state.warnings.add("diagram");
    return `JoySpace diagram not exported (diagramId: ${diagramId || "unknown"})`;
  }
  state.warnings.add("diagram-link");
  const titleText = normalizeText(info.title || "");
  const label = titleText
      ? `JoySpace 绘图：${titleText}`
      : `JoySpace 绘图（diagramId: ${diagramId}）`;
  const href = info.pageUrl || state.pageUrl || "";
  return href ? `[${label}](${href})` : label;
}

function renderListBlock(block, state) {
  if (block.header) {
    const headingText = blockPlainText(block.children);
    state.orderedCounters.clear();
    if (!headingText) {
      return "";
    }
    const level = Math.max(2, Math.min(Number(block.header) + 1, 6));
    return `${"#".repeat(level)} ${headingText}\n\n`;
  }

  const indent = Math.max(0, Number(block.indent || 0));
  pruneOrderedCounters(state.orderedCounters, indent);

  let prefix = "- ";
  if (block.value === "checkbox") {
    prefix = block.checked ? "- [x] " : "- [ ] ";
    state.orderedCounters.delete(indent);
  } else if (block.value === "ordered") {
    const nextIndex = (state.orderedCounters.get(indent) || 0) + 1;
    state.orderedCounters.set(indent, nextIndex);
    prefix = `${nextIndex}. `;
  } else {
    state.orderedCounters.delete(indent);
  }

  const content = inlineChildrenToMarkdown(block.children);
  if (!content) {
    return "";
  }
  return `${"  ".repeat(indent)}${prefix}${content}\n`;
}

function tableCellToMarkdown(cell, parentState) {
  const state = {
    warnings: parentState?.warnings || new Set(),
    orderedCounters: new Map(),
    diagrams: parentState?.diagrams || null,
    pageUrl: parentState?.pageUrl || "",
  };
  const lines = (cell?.children || [])
      .map((child) => renderTableCellBlock(child, state))
      .map((line) => String(line || "").replace(/\r/g, "").replace(/\n+/g, "<br>"))
      .map((line) => line.replace(/[ \t]+$/u, ""))
      .filter((line) => line.trim().length > 0);
  return escapeTableCell(lines.join("<br>"));
}

function renderTableCellBlock(block, state) {
  if (!block || typeof block !== "object") {
    return "";
  }

  switch (block.type) {
    case "p":
      return inlineChildrenToMarkdown(block.children);
    case "img": {
      const src = normalizeText(block.url || block.src);
      return src ? `![image](${src})` : "";
    }
    case "list": {
      const content = inlineChildrenToMarkdown(block.children);
      if (!content) {
        return "";
      }
      if (block.header) {
        state.orderedCounters.clear();
        return content;
      }
      const indent = Math.max(0, Number(block.indent || 0));
      const indentPrefix = "  ".repeat(indent);
      if (block.value === "checkbox") {
        pruneOrderedCounters(state.orderedCounters, indent);
        state.orderedCounters.delete(indent);
        return `${indentPrefix}${block.checked ? "[x]" : "[ ]"} ${content}`;
      }
      if (block.value === "ordered") {
        pruneOrderedCounters(state.orderedCounters, indent);
        const nextIndex = (state.orderedCounters.get(indent) || 0) + 1;
        state.orderedCounters.set(indent, nextIndex);
        return `${indentPrefix}${nextIndex}. ${content}`;
      }
      pruneOrderedCounters(state.orderedCounters, indent);
      state.orderedCounters.delete(indent);
      return `${indentPrefix}- ${content}`;
    }
    case "text-draw":
      return (block.children || [])
          .map((line) => inlineChildrenToMarkdown(line.children))
          .filter(Boolean)
          .join("<br>");
    case "highlight-block":
    case "foldable-block":
      return (block.children || [])
          .map((child) => renderTableCellBlock(child, state))
          .filter(Boolean)
          .join("<br>");
    case "divider":
      return "---";
    case "diagram":
      return renderDiagramInlineFallback(block, state);
    default: {
      // JoySpace table cells often use bare text leaves: { text: "..." } without type/p wrapper.
      if (typeof block.text === "string") {
        return inlineChildrenToMarkdown([block]);
      }
      if (Array.isArray(block.children) && block.children.some((child) => child?.type)) {
        return block.children
            .map((child) => renderTableCellBlock(child, state))
            .filter(Boolean)
            .join("<br>");
      }
      return inlineChildrenToMarkdown(block.children);
    }
  }
}

function tableToMarkdown(block, state) {
  const rows = (block.children || []).filter((item) => item?.type === "table-row");
  if (!rows.length) {
    return "";
  }

  const renderedRows = rows
      .map((row) =>
          (row.children || [])
              .filter((cell) => cell?.type === "table-cell")
              .map((cell) => tableCellToMarkdown(cell, state)),
      )
      .filter((row) => row.length > 0);

  if (!renderedRows.length) {
    return "";
  }

  let output = "";
  renderedRows.forEach((row, index) => {
    output += `| ${row.join(" | ")} |\n`;
    if (index === 0) {
      output += `| ${row.map(() => "---").join(" | ")} |\n`;
    }
  });
  return `${output}\n`;
}

function codeBlockToMarkdown(block) {
  const language = normalizeText(block.lang || block.language || "text").toLowerCase();
  const code = (block.children || [])
      .map((line) => inlineChildrenToMarkdown(line.children))
      .join("\n")
      .replace(/\n+$/, "");
  return `\`\`\`${language}\n${code}\n\`\`\`\n\n`;
}

function quoteMarkdown(markdown) {
  return markdown
      .trim()
      .split("\n")
      .map((line) => (line ? `> ${line}` : ">"))
      .join("\n");
}

function fixInlineSpacing(line) {
  return line.replace(/(@[^\s@|<]+)(?=@)/gu, "$1 ");
}

function shouldWrapMarkdownLine(line, inCodeFence) {
  const trimmed = line.trim();

  if (!trimmed || inCodeFence) {
    return false;
  }

  if (
      /^#{1,6}\s/u.test(trimmed) ||
      /^\|/.test(trimmed) ||
      /^```/.test(trimmed) ||
      /^---$/u.test(trimmed) ||
      /^!\[/.test(trimmed) ||
      /^</.test(trimmed)
  ) {
    return false;
  }

  if (/^>?\s*-\s/.test(trimmed) && /(https?:\/\/\S+|\]\()/.test(trimmed)) {
    return false;
  }

  return trimmed.length > 100;
}

function tokenizeForWrap(content) {
  const tokens = [];
  let remaining = String(content || "");

  const matchers = [
    /^!\[[^\]]*\]\([^)]+\)/u,
    /^\[[^\]]+\]\([^)]+\)/u,
    /^`[^`]+`/u,
    /^<\/?[^>]+>/u,
    /^https?:\/\/\S+/u,
    /^\s+/u,
    /^[A-Za-z0-9_./#:%?=&+-]+/u,
    /^[^\s，。；：！？、,.!?;:<>\[\]()`*_~]+/u,
  ];

  while (remaining) {
    let matched = "";
    for (const matcher of matchers) {
      const result = remaining.match(matcher)?.[0];
      if (result) {
        matched = result;
        break;
      }
    }

    if (!matched) {
      matched = remaining[0];
    }

    tokens.push(matched);
    remaining = remaining.slice(matched.length);
  }

  return tokens;
}

function isClosingPunctuation(token) {
  return /^[，。；：！？、,.!?;:）】」』]+$/u.test(token);
}

function parseWrapPrefix(line) {
  const leadingIndent = line.match(/^\s*/u)?.[0] || "";
  let rest = line.slice(leadingIndent.length);
  let blockquotePrefix = "";

  while (rest.startsWith("> ")) {
    blockquotePrefix += "> ";
    rest = rest.slice(2);
  }

  const listMarker =
      rest.match(/^(?:- \[[ xX]\] |- |\d+\. )/u)?.[0] ||
      rest.match(/^([*-+] )/u)?.[0] ||
      "";

  if (listMarker) {
    rest = rest.slice(listMarker.length);
  }

  return {
    firstPrefix: `${leadingIndent}${blockquotePrefix}${listMarker}`,
    continuationPrefix: `${leadingIndent}${blockquotePrefix}${" ".repeat(listMarker.length)}`,
    content: rest.trim(),
  };
}

function wrapMarkdownLine(line, width = 100) {
  const { firstPrefix, continuationPrefix, content } = parseWrapPrefix(line);
  if (!content) {
    return line;
  }

  const tokens = tokenizeForWrap(content);
  const lines = [];
  let current = "";
  let currentLength = 0;
  let currentPrefix = firstPrefix;
  let availableWidth = Math.max(20, width - currentPrefix.length);

  const pushCurrent = () => {
    if (!current.trim()) {
      return;
    }
    lines.push(`${currentPrefix}${current.trimEnd()}`);
    current = "";
    currentLength = 0;
    currentPrefix = continuationPrefix;
    availableWidth = Math.max(20, width - currentPrefix.length);
  };

  const appendToken = (token) => {
    current += token;
    currentLength += token.length;
  };

  for (const token of tokens) {
    if (/^\s+$/u.test(token)) {
      if (current && !current.endsWith(" ")) {
        appendToken(" ");
      }
      continue;
    }

    let pendingToken = token;
    while (pendingToken) {
      const tokenLength = pendingToken.length;

      if (
          !current &&
          tokenLength > availableWidth &&
          !/^!\[[^\]]*\]\([^)]+\)$/u.test(pendingToken) &&
          !/^\[[^\]]+\]\([^)]+\)$/u.test(pendingToken)
      ) {
        appendToken(pendingToken.slice(0, availableWidth));
        pendingToken = pendingToken.slice(availableWidth);
        pushCurrent();
        continue;
      }

      if (
          current &&
          currentLength + tokenLength > availableWidth &&
          !isClosingPunctuation(pendingToken)
      ) {
        pushCurrent();
        continue;
      }

      appendToken(pendingToken);
      pendingToken = "";
    }
  }

  pushCurrent();

  return lines.length > 0 ? lines.join("\n") : line;
}

function formatMarkdown(markdown) {
  const result = [];
  let inCodeFence = false;
  let inSvgBlock = false;

  for (const rawLine of String(markdown || "").split("\n")) {
    let line = inSvgBlock ? rawLine : fixInlineSpacing(rawLine.replace(/[ \t]+$/u, ""));

    if (!inSvgBlock && /^```/.test(line.trim())) {
      inCodeFence = !inCodeFence;
      result.push(line);
      continue;
    }

    if (!inCodeFence) {
      if (!inSvgBlock && /<svg[\s>]/i.test(line)) {
        inSvgBlock = true;
      }
    }

    if (inSvgBlock) {
      result.push(line);
      if (/<\/svg>/i.test(line)) {
        inSvgBlock = false;
      }
      continue;
    }

    if (shouldWrapMarkdownLine(line, inCodeFence)) {
      line = wrapMarkdownLine(line);
    }

    result.push(line);
  }

  return result.join("\n").replace(/\n{3,}/g, "\n\n").trimEnd();
}

function renderBlock(block, state) {
  if (!block || typeof block !== "object") {
    return "";
  }

  switch (block.type) {
    case "p": {
      const content = inlineChildrenToMarkdown(block.children);
      return content ? `${content}\n\n` : "";
    }
    case "list":
      return renderListBlock(block, state);
    case "table":
      return tableToMarkdown(block, state);
    case "highlight-block": {
      const nested = renderBlocks(block.children, {
        warnings: state.warnings,
        orderedCounters: new Map(),
        diagrams: state.diagrams,
        attachments: state.attachments,
        pageUrl: state.pageUrl,
      });
      if (!nested.trim()) {
        return "";
      }
      return `${quoteMarkdown(nested)}\n\n`;
    }
    case "foldable-block": {
      const summary = normalizeText(block.name || "Foldable Block");
      const nested = renderBlocks(block.children, {
        warnings: state.warnings,
        orderedCounters: new Map(),
        diagrams: state.diagrams,
        pageUrl: state.pageUrl,
      }).trim();
      if (!nested) {
        return `\n<details><summary>${summary}</summary>\n\n</details>\n\n`;
      }
      return `\n<details><summary>${summary}</summary>\n\n${nested}\n\n</details>\n\n`;
    }
    case "text-draw":
      return codeBlockToMarkdown(block);
    case "img": {
      const src = normalizeText(block.url || block.src);
      return src ? `![image](${src})\n\n` : "";
    }
    case "attachment":
      return renderAttachmentBlock(block);
    case "divider":
      return "---\n\n";
    case "diagram":
      return renderDiagramBlock(block, state);
    default: {
      if (Array.isArray(block.children) && block.children.some((item) => item?.type)) {
        state.warnings.add(block.type || "unknown");
        return renderBlocks(block.children, {
          warnings: state.warnings,
          orderedCounters: new Map(),
          diagrams: state.diagrams,
          pageUrl: state.pageUrl,
        });
      }
      const content = inlineChildrenToMarkdown(block.children);
      if (!content) {
        return "";
      }
      state.warnings.add(block.type || "unknown");
      return `${content}\n\n`;
    }
  }
}

function joyspaceContentToMarkdown({ title, content, diagrams, pageUrl }) {
  const resolvedTitle = normalizeText(title) || "Doc";
  const warnings = new Set();
  const blocks = Array.isArray(content) ? [...content] : [];

  while (blocks.length > 0) {
    if (normalizeText(blockPlainText(blocks[0])) === resolvedTitle) {
      blocks.shift();
      continue;
    }
    break;
  }

  let markdown = `# ${resolvedTitle}\n\n${renderBlocks(blocks, {
    warnings,
    orderedCounters: new Map(),
    diagrams: diagrams || null,
    pageUrl: normalizeText(pageUrl || ""),
  })}`;

  markdown = markdown
      .replace(/\n{3,}/g, "\n\n")
      .replace(/[ \t]+\n/g, "\n")
      .trim();
  markdown = formatMarkdown(markdown);
  markdown += "\n";

  return {
    title: resolvedTitle,
    markdown,
    warnings: [...warnings],
  };
}

export { joyspaceContentToMarkdown };