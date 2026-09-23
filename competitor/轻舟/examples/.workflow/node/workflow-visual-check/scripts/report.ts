import * as fs from 'fs';
import * as path from 'path';
import { DiffResult, DiffRegion } from './diff';

export interface ComponentReport {
  name: string;
  similarity: number;
  passed: boolean;
  designImage: string;
  actualImage: string;
  diffImage: string;
  regions: DiffRegion[];
  /**
   * 被判为「显著偏差块」的 regions（面积达标 且 填充率达标）。
   *
   * 相似度是全局比例，小元素偏差会被大面积正确像素稀释——20px 元素错位 10px
   * 在 375x812 页面里占比约 0.06%，相似度纹丝不动。故单独暴露显著偏差块，
   * 让「整体像但局部错」也能被机器拦下，不全靠 AI 目视。
   *
   * 判定要两个条件而非只看面积：字体抗锯齿的轮廓 bbox 面积与小图标相当，
   * 只按面积判则「能拦住小图标的阈值」必然同时被文字噪声触发。填充率把二者分开
   * ——元素缺失/错位是实心块，抗锯齿是稀疏轮廓。
   */
  significantRegions: DiffRegion[];
  significantRegionCount: number;
  selectorUsed: string;
  error: string | null;
}

export interface VisualRegressionReport {
  passed: boolean;
  timestamp: string;
  config: {
    threshold: { component: number; page: number };
    /** 判定为「显著偏差块」的最小像素面积 */
    regionMinArea: number;
    /** 判定为「显著偏差块」的最小填充率 */
    regionMinFillRatio: number;
    viewport: string;
    pageUrl: string;
  };
  page: {
    similarity: number;
    passed: boolean;
    designImage: string;
    actualImage: string;
    diffImage: string;
    significantRegionCount: number;
    error: string | null;
  };
  components: ComponentReport[];
}

function pickSignificantRegions(
  regions: DiffRegion[],
  minArea: number,
  minFillRatio: number,
): DiffRegion[] {
  return regions.filter(
    (r) => r.width * r.height >= minArea && r.fillRatio >= minFillRatio,
  );
}

export function generateReport(
  pageResult: DiffResult & { designImage: string; actualImage: string },
  componentResults: Array<{
    name: string;
    selector: string;
    result: DiffResult;
    designImage: string;
    actualImage: string;
  }>,
  config: {
    threshold: { component: number; page: number };
    regionMinArea: number;
    regionMinFillRatio: number;
    viewport: string;
    pageUrl: string;
  },
  outputPath: string,
): VisualRegressionReport {
  const pageRegions = pickSignificantRegions(
    pageResult.regions,
    config.regionMinArea,
    config.regionMinFillRatio,
  );
  const pagePass =
    !pageResult.error &&
    pageResult.similarity >= config.threshold.page &&
    pageRegions.length === 0;

  const components: ComponentReport[] = componentResults.map((c) => {
    const sig = pickSignificantRegions(
      c.result.regions,
      config.regionMinArea,
      config.regionMinFillRatio,
    );
    return {
      name: c.name,
      similarity: c.result.similarity,
      // 双闸：相似度达标 且 无显著偏差块。任一不满足即失败
      passed:
        !c.result.error && c.result.similarity >= config.threshold.component && sig.length === 0,
      designImage: c.designImage,
      actualImage: c.actualImage,
      diffImage: c.result.diffImagePath,
      regions: c.result.regions,
      significantRegions: sig,
      significantRegionCount: sig.length,
      selectorUsed: c.selector,
      error: c.result.error || null,
    };
  });

  const allComponentsPass = components.every((c) => c.passed);

  const report: VisualRegressionReport = {
    passed: pagePass && allComponentsPass,
    timestamp: new Date().toISOString(),
    config,
    page: {
      similarity: pageResult.similarity,
      passed: pagePass,
      designImage: pageResult.designImage,
      actualImage: pageResult.actualImage,
      diffImage: pageResult.diffImagePath,
      significantRegionCount: pageRegions.length,
      error: pageResult.error || null,
    },
    components,
  };

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, JSON.stringify(report, null, 2));

  return report;
}
