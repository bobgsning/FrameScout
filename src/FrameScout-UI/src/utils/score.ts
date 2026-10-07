/**
 * Score mapping utilities.
 *
 * 后端返回的是原始相似度（文本检索与以图搜图的量纲完全不同），
 * 这里统一映射成 0.1 ~ 99.9 的「人类可理解分数」，便于 UI 展示与阈值判断。
 */
import type { ScoreKind, SearchResult } from '@/types/search'
import { t } from '@/i18n'

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

/** BGE-M3 dense cosine（0~1）→ 人类可读分。文本语义的 cosine 量纲远高于
 *  SigLIP 视觉 cosine，必须用独立映射，否则会把所有命中都挤到 99.9%。 */
export function mapBgeScore(sim: number): number {
  let humanScore: number
  if (sim >= 0.60) {
    humanScore = mapScore(sim, 0.60, 1.0, 85, 99.9)
  } else if (sim >= 0.40) {
    humanScore = mapScore(sim, 0.40, 0.60, 60, 84.9)
  } else if (sim >= 0.25) {
    humanScore = mapScore(sim, 0.25, 0.40, 30, 59.9)
  } else {
    humanScore = mapScore(sim, 0.0, 0.25, 1, 29.9)
  }
  if (humanScore > 99.9) humanScore = 99.9
  if (humanScore < 0.1) humanScore = 0.1
  return humanScore
}

/** 带图标的展示文案，例如 "🌟 93.2% Match" */
export function formatScore(rawScore: number, isImage: boolean = false, kind?: ScoreKind): string {
  // 2.0 是「新入库文件 / 浏览模式」的哨兵值，直接给满分
  if (rawScore >= 2.0) return t('score.perfect')

  const humanScore = kind === 'bge' ? mapBgeScore(rawScore) : mapToHumanScore(rawScore, isImage)

  let icon: string
  if (humanScore >= 90) icon = '🌟'
  else if (humanScore >= 60) icon = '✅'
  else if (humanScore >= 30) icon = '⚠️'
  else icon = '❌'

  return t('score.match', { icon, score: humanScore.toFixed(1) })
}

/** 纯数值分，用于阈值比较（折叠、标签判定） */
export function getNumericScore(rawScore: number, isImage: boolean = false, kind?: ScoreKind): number {
  if (rawScore >= 2.0) return 100.0
  return kind === 'bge' ? mapBgeScore(rawScore) : mapToHumanScore(rawScore, isImage)
}

/** 低置信度阈值：低于此值的图片卡片默认折叠 */
export const LOW_SCORE_THRESHOLD = 30

/** 纯字面命中（无视觉）时，按命中通道数映射人类可读分（方案 C 前端自决）。 */
function resolveLiteralScore(ocrHit?: boolean, noteHit?: boolean, filenameHit?: boolean): number {
  const count = (ocrHit ? 1 : 0) + (noteHit ? 1 : 0) + (filenameHit ? 1 : 0)
  if (count >= 3) return 95
  if (count === 2) return 85
  return 65
}

/**
 * 文字搜索结果的人类可读分（方案 C「后端不做决定」）。
 * 后端返回原始视觉余弦（visual_similarity）+ 各字面通道命中布尔，此处完全自决：
 *  - 视觉命中（visual_similarity > 0）：走 mapToHumanScore 的原始余弦映射
 *    （0.025~0.15 阈值，对 SigLIP 文本余弦准确）。
 *  - 纯字面命中：按命中通道数给分（1≈65 / 2≈85 / 3≈95）。
 *  - 浏览哨兵（score >= 2.0 且无命中）：100。
 */
export function getSearchNumericScore(item: SearchResult): number {
  const visual = item.visual_similarity ?? 0
  if (visual > 0.001) {
    return mapToHumanScore(visual, false)
  }
  if (item.ocr_hit || item.note_hit || item.filename_hit) {
    return resolveLiteralScore(item.ocr_hit, item.note_hit, item.filename_hit)
  }
  if ((item.score ?? 0) >= 2.0) return 100
  return 0
}

/** 带图标的文字搜索分数文案，如 "🌟 93.2% Match"。 */
export function formatSearchScore(item: SearchResult): string {
  const human = getSearchNumericScore(item)
  let icon: string
  if (human >= 90) icon = '🌟'
  else if (human >= 60) icon = '✅'
  else if (human >= 30) icon = '⚠️'
  else icon = '❌'
  return t('score.match', { icon, score: human.toFixed(1) })
}
