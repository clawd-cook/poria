import zlib from "node:zlib";

function decodeDrawioMxGraphXml(drawioXml) {
  const text = String(drawioXml || "");
  if (!text) {
    return "";
  }

  if (/^\s*<mxGraphModel\b/.test(text)) {
    return text;
  }

  const diagramMatch = text.match(/<diagram\b[^>]*>([\s\S]*?)<\/diagram>/);
  if (!diagramMatch) {
    return "";
  }

  const payload = diagramMatch[1].trim();
  if (!payload) {
    return "";
  }

  if (/^\s*<mxGraphModel\b/.test(payload)) {
    return payload;
  }

  let inflated;
  try {
    const buffer = Buffer.from(payload, "base64");
    inflated = zlib.inflateRawSync(buffer).toString("binary");
  } catch {
    return "";
  }

  let decoded = inflated;
  try {
    decoded = decodeURIComponent(inflated);
  } catch {
    decoded = inflated;
  }

  return /<mxGraphModel\b/.test(decoded) ? decoded : "";
}

function parseMxCells(mxGraphXml) {
  const cells = [];
  const cellRe = /<mxCell\b([^>]*?)(\/>|>([\s\S]*?)<\/mxCell>)/g;
  const attrRe = /(\w[\w-]*)\s*=\s*"([^"]*)"/g;
  let match;
  while ((match = cellRe.exec(mxGraphXml))) {
    const attrsText = match[1];
    const attrs = {};
    let attrMatch;
    while ((attrMatch = attrRe.exec(attrsText))) {
      attrs[attrMatch[1]] = attrMatch[2];
    }
    cells.push(attrs);
  }
  return cells;
}

function decodeHtmlEntities(input) {
  return String(input || "")
    .replace(/&#(\d+);/g, (_, code) => {
      const value = Number(code);
      return Number.isFinite(value) ? String.fromCodePoint(value) : "";
    })
    .replace(/&#x([0-9a-fA-F]+);/g, (_, code) =>
      String.fromCodePoint(parseInt(code, 16)),
    )
    .replace(/&nbsp;/g, " ")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&amp;/g, "&");
}

function cleanMermaidLabel(raw) {
  if (raw == null) {
    return "";
  }
  let text = decodeHtmlEntities(raw);
  text = text
    .replace(/<br\s*\/?>/gi, " ")
    .replace(/<\/?[^>]+>/g, "")
    .replace(/[\r\n]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  text = text.replace(/"/g, "'");
  return text;
}

function inferShape(style) {
  const s = String(style || "").toLowerCase();
  if (/\brhombus\b/.test(s)) {
    return { open: "{", close: "}" };
  }
  if (/\bellipse\b/.test(s)) {
    return { open: "((", close: "))" };
  }
  if (/\bparallelogram\b/.test(s)) {
    return { open: "[/", close: "/]" };
  }
  if (/\bhexagon\b/.test(s)) {
    return { open: "{{", close: "}}" };
  }
  if (/\bcylinder\b/.test(s)) {
    return { open: "[(", close: ")]" };
  }
  if (/shape=mxgraph\.flowchart\.process\b|\bshape=process\b/.test(s)) {
    return { open: "[[", close: "]]" };
  }
  if (/\brounded=1\b/.test(s)) {
    return { open: "(", close: ")" };
  }
  return { open: "[", close: "]" };
}

function isEdgeLabelCell(cell) {
  return /\bedgeLabel\b/i.test(String(cell.style || ""));
}

function isDecorativeCell(cell) {
  const style = String(cell.style || "").toLowerCase();
  if (!style) {
    return false;
  }
  return (
    /\bcurlybracket\b/.test(style) ||
    /\bshape=bracket\b/.test(style) ||
    /\bshape=line\b/.test(style)
  );
}

function mxGraphToMermaid(mxGraphXml, { maxNodes = 200 } = {}) {
  if (!/<mxGraphModel\b/.test(String(mxGraphXml || ""))) {
    return "";
  }

  const cells = parseMxCells(mxGraphXml);
  if (cells.length === 0) {
    return "";
  }

  const edgeCells = cells.filter((cell) => cell.edge === "1");
  const edgeIdSet = new Set(edgeCells.map((cell) => cell.id));

  const edgeLabelByEdge = new Map();
  for (const cell of cells) {
    if (
      cell.vertex === "1" &&
      isEdgeLabelCell(cell) &&
      cell.parent &&
      edgeIdSet.has(cell.parent)
    ) {
      const label = cleanMermaidLabel(cell.value);
      if (!label) {
        continue;
      }
      const existing = edgeLabelByEdge.get(cell.parent);
      edgeLabelByEdge.set(
        cell.parent,
        existing ? `${existing} ${label}` : label,
      );
    }
  }

  const vertexCells = cells.filter((cell) => {
    if (cell.vertex !== "1") {
      return false;
    }
    if (isEdgeLabelCell(cell)) {
      return false;
    }
    if (isDecorativeCell(cell) && !cleanMermaidLabel(cell.value)) {
      return false;
    }
    return true;
  });

  if (vertexCells.length === 0 || vertexCells.length > maxNodes) {
    return "";
  }

  const idMap = new Map();
  vertexCells.forEach((vertex, index) => {
    idMap.set(vertex.id, `n${index + 1}`);
  });

  const lines = ["flowchart TD"];
  for (const vertex of vertexCells) {
    const mid = idMap.get(vertex.id);
    const label = cleanMermaidLabel(vertex.value) || mid;
    const shape = inferShape(vertex.style);
    lines.push(`    ${mid}${shape.open}"${label}"${shape.close}`);
  }

  let edgeCount = 0;
  for (const edge of edgeCells) {
    const source = idMap.get(edge.source);
    const target = idMap.get(edge.target);
    if (!source || !target) {
      continue;
    }
    edgeCount += 1;
    const label =
      cleanMermaidLabel(edge.value) || edgeLabelByEdge.get(edge.id) || "";
    if (label) {
      lines.push(`    ${source} -->|"${label}"| ${target}`);
    } else {
      lines.push(`    ${source} --> ${target}`);
    }
  }

  if (edgeCount === 0 && vertexCells.length < 2) {
    return "";
  }

  return lines.join("\n");
}

function drawioXmlToMermaid(drawioXml, options) {
  const mxGraph = decodeDrawioMxGraphXml(drawioXml);
  if (!mxGraph) {
    return "";
  }
  return mxGraphToMermaid(mxGraph, options);
}

export {
  cleanMermaidLabel,
  decodeDrawioMxGraphXml,
  drawioXmlToMermaid,
  inferShape,
  mxGraphToMermaid,
  parseMxCells,
};
