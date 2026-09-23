import sharp from 'sharp';
// eslint-disable-next-line @typescript-eslint/no-var-requires
const pixelmatch = require('pixelmatch');
import { PNG } from 'pngjs';
import * as fs from 'fs';
import * as path from 'path';

export interface DiffRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  /** 该连通块实际的偏差像素数 */
  pixelCount: number;
  /**
   * 填充率 = pixelCount / (width * height)，区分「块状偏差」与「文字噪声」。
   * 元素缺失/错位是实心块（填充率高）；字体抗锯齿是稀疏轮廓（填充率低）。
   * 面积单独判不出二者——24px 标题的轮廓 bbox 面积与小图标相当。
   */
  fillRatio: number;
}

export interface DiffResult {
  similarity: number;
  diffPixelCount: number;
  totalPixels: number;
  diffImagePath: string;
  regions: DiffRegion[];
  error?: string;
}

interface NormalizedPair {
  designBuf: Buffer;
  actualBuf: Buffer;
  width: number;
  height: number;
}

async function normalize(
  designPath: string,
  actualPath: string,
  targetWidth: number,
): Promise<NormalizedPair> {
  const designBuf = await sharp(designPath)
    .resize(targetWidth, null, { fit: 'inside' })
    .png()
    .toBuffer();

  const actualBuf = await sharp(actualPath)
    .resize(targetWidth, null, { fit: 'inside' })
    .png()
    .toBuffer();

  const dMeta = await sharp(designBuf).metadata();
  const aMeta = await sharp(actualBuf).metadata();
  const height = Math.min(dMeta.height!, aMeta.height!);

  const heightDiff = Math.abs(dMeta.height! - aMeta.height!);
  const heightDiffRatio = heightDiff / Math.max(dMeta.height!, aMeta.height!);
  if (heightDiffRatio > 0.05) {
    console.warn(
      `[WARN] Height mismatch: design=${dMeta.height}px actual=${aMeta.height}px` +
        ` (diff ${(heightDiffRatio * 100).toFixed(1)}%) — bottom ${heightDiff}px cropped and excluded from comparison`,
    );
  }

  const finalDesign = await sharp(designBuf)
    .extract({ left: 0, top: 0, width: targetWidth, height })
    .png()
    .toBuffer();

  const finalActual = await sharp(actualBuf)
    .extract({ left: 0, top: 0, width: targetWidth, height })
    .png()
    .toBuffer();

  return { designBuf: finalDesign, actualBuf: finalActual, width: targetWidth, height };
}

/**
 * 判定 diff 图中某像素是否为「真实偏差像素」。
 *
 * pixelmatch 的输出图里三类像素长相不同：
 * - 偏差像素：饱和红 `(255,0,0)`（diffColor 默认值）
 * - 抗锯齿像素：黄 `(255,255,0)`（aaColor 默认值），**不计入** diffPixelCount
 * - 未变化像素：原图转灰度后按 alpha=0.1 混到白底，得到 r=g=b 的浅灰
 *
 * 故不能按「亮度 > 阈值」判偏差——白底页面的未变化像素灰度值接近 255，会让
 * 整张图被判成一个偏差块（flood fill 直接吞掉整个画布）。这里按颜色特征判：
 * 红通道高 + 绿蓝通道低。r=g=b 的灰度像素与黄色 AA 像素都被排除，
 * 与 `diffPixelCount` 的计数口径保持一致。
 */
function isDiffPixel(data: Buffer | Uint8Array, idx: number): boolean {
  // matches pixelmatch default diffColor (255,0,0); r>150 avoids pure grays and yellow AA pixels
  return data[idx] > 150 && data[idx + 1] < 100 && data[idx + 2] < 100;
}

function extractRegions(diffPng: PNG): DiffRegion[] {
  const { width, height, data } = diffPng;
  const visited = new Uint8Array(width * height);
  const regions: DiffRegion[] = [];

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const idx = (y * width + x) * 4;
      if (!isDiffPixel(data, idx) || visited[y * width + x]) continue;

      let minX = x, maxX = x, minY = y, maxY = y;
      let pixelCount = 0;
      const queue: [number, number][] = [[x, y]];
      visited[y * width + x] = 1;

      // 用游标而非 shift()——shift() 是 O(n)，大图上单个连通块可达数十万像素
      let head = 0;
      while (head < queue.length) {
        const [cx, cy] = queue[head++];
        pixelCount++;
        minX = Math.min(minX, cx);
        maxX = Math.max(maxX, cx);
        minY = Math.min(minY, cy);
        maxY = Math.max(maxY, cy);

        for (const [nx, ny] of [[cx-1,cy],[cx+1,cy],[cx,cy-1],[cx,cy+1]]) {
          if (nx < 0 || nx >= width || ny < 0 || ny >= height) continue;
          if (visited[ny * width + nx]) continue;
          const nIdx = (ny * width + nx) * 4;
          if (isDiffPixel(data, nIdx)) {
            visited[ny * width + nx] = 1;
            queue.push([nx, ny]);
          }
        }
      }

      const regionWidth = maxX - minX + 1;
      const regionHeight = maxY - minY + 1;
      // 过滤掉宽或高 ≤3px 的单像素噪点块；比 regionMinArea 更早的第一道粗筛，不替代 pickSignificantRegions
      if (regionWidth > 3 && regionHeight > 3) {
        regions.push({
          x: minX,
          y: minY,
          width: regionWidth,
          height: regionHeight,
          pixelCount,
          fillRatio: pixelCount / (regionWidth * regionHeight),
        });
      }
    }
  }

  return regions;
}

export async function compareImages(
  designPath: string,
  actualPath: string,
  diffOutputPath: string,
  options: { threshold?: number; targetWidth?: number } = {},
): Promise<DiffResult> {
  const { threshold = 0.1, targetWidth = 375 } = options;

  if (!fs.existsSync(designPath)) {
    return { similarity: 0, diffPixelCount: 0, totalPixels: 0, diffImagePath: '', regions: [], error: `Design image not found: ${designPath}` };
  }
  if (!fs.existsSync(actualPath)) {
    return { similarity: 0, diffPixelCount: 0, totalPixels: 0, diffImagePath: '', regions: [], error: `Actual image not found: ${actualPath}` };
  }

  const { designBuf, actualBuf, width, height } = await normalize(designPath, actualPath, targetWidth);

  const designPng = PNG.sync.read(designBuf);
  const actualPng = PNG.sync.read(actualBuf);
  const diffPng = new PNG({ width, height });

  const diffPixelCount = pixelmatch(
    designPng.data,
    actualPng.data,
    diffPng.data,
    width,
    height,
    { threshold },
  );

  const totalPixels = width * height;
  const similarity = 1 - diffPixelCount / totalPixels;

  fs.mkdirSync(path.dirname(diffOutputPath), { recursive: true });
  fs.writeFileSync(diffOutputPath, PNG.sync.write(diffPng));

  const regions = extractRegions(diffPng);

  return { similarity, diffPixelCount, totalPixels, diffImagePath: diffOutputPath, regions };
}
