/**
 * Score mapping utilities.
 *
 * 后端返回的是原始相似度（文本检索与以图搜图的量纲完全不同），
 * 这里统一映射成 0.1 ~ 99.9 的「人类可理解分数」，便于 UI 展示与阈值判断。
 */

/** 线性区间映射：把 [minR, maxR] 上的 val 映射到 [minH, maxH] */
function mapScore(val: number, minR: number, maxR: number, minH: number, maxH: number): number {
  return minH + ((val - minR) / (maxR - minR)) * (maxH - minH)
}

/**
 * 原始分 -> 人类可读分
 * @param rawScore 后端原始分
 * @param isImage  是否为以图搜图（与文本检索使用两套完全不同的阈值）
 */
export function mapToHumanScore(rawScore: number, isImage: boolean): number {
  let humanScore: number

  if (isImage) {
    if (rawScore >= 0.84) {
      humanScore = mapScore(rawScore, 0.84, 1.0, 90, 99.9)
    } else if (rawScore >= 0.65) {
      humanScore = mapScore(rawScore, 0.65, 0.84, 60, 89.9)
    } else if (rawScore >= 0.45) {
      humanScore = mapScore(rawScore, 0.45, 0.65, 30, 59.9)
    } else {
      humanScore = mapScore(rawScore, 0.0, 0.45, 1, 29.9)
    }
  } else {
    if (rawScore >= 0.10) {
      humanScore = mapScore(rawScore, 0.10, 0.15, 85, 99.9)
    } else if (rawScore >= 0.065) {
      humanScore = mapScore(rawScore, 0.065, 0.10, 60, 84.9)
    } else if (rawScore >= 0.025) {
      humanScore = mapScore(rawScore, 0.025, 0.065, 30, 59.9)
    } else {
      humanScore = mapScore(rawScore, 0.0, 0.025, 1, 29.9)
    }
  }

  if (humanScore > 99.9) humanScore = 99.9
  if (humanScore < 0.1) humanScore = 0.1

  return humanScore
}

/** 带图标的展示文案，例如 "🌟 93.2% Match" */
export function formatScore(rawScore: number, isImage: boolean = false): string {
  // 2.0 是「新入库文件 / 浏览模式」的哨兵值，直接给满分
  if (rawScore >= 2.0) return '🔥 100.0%'

  const humanScore = mapToHumanScore(rawScore, isImage)

  let icon: string
  if (humanScore >= 90) icon = '🌟'
  else if (humanScore >= 60) icon = '✅'
  else if (humanScore >= 30) icon = '⚠️'
  else icon = '❌'

  return `${icon} ${humanScore.toFixed(1)}% Match`
}

/** 纯数值分，用于阈值比较（折叠、标签判定） */
export function getNumericScore(rawScore: number, isImage: boolean = false): number {
  if (rawScore >= 2.0) return 100.0
  return mapToHumanScore(rawScore, isImage)
}

/** 低置信度阈值：低于此值的图片卡片默认折叠 */
export const LOW_SCORE_THRESHOLD = 30
