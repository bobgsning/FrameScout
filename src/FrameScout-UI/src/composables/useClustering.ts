/**
 * useClustering — 视觉相似度聚类视图。
 * 依赖 useSearch（进入聚类前缓存结果，退出后恢复）。
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ClusterGroup, ClusterResult } from '@/types/search'
import { useSearch } from './useSearch'
import { useToast } from './useToast'
import { t } from '@/i18n'

const isClusteringView = ref(false)
const clusterThreshold = ref(0.8)
const clusters = ref<ClusterGroup[]>([])
/** 债单 B8：因「单帧簇 ≤50」限制而被丢弃的独特帧数（告知而非静默消失） */
const truncatedSingleFrames = ref(0)

export function useClustering() {
  const { results, fullResultsCache } = useSearch()

  async function fetchClusters() {
    try {
      const res: ClusterResult = await invoke('cluster_similar_images', {
        threshold: clusterThreshold.value
      })
      clusters.value = res.groups
      truncatedSingleFrames.value = res.truncated_single_frames
    } catch (err) {
      // P1-10：聚类失败此前只 console.error，界面纹丝不动；现给可见反馈
      const { push } = useToast()
      push(t('cluster.clusterFailed', { err }), 'error')
    }
  }

  async function toggleClustering() {
    if (!isClusteringView.value) {
      // 进入：缓存当前结果并拉取聚类
      fullResultsCache.value = [...results.value]
      isClusteringView.value = true
      await fetchClusters()
    } else {
      // 退出：还原进入前的列表
      results.value = fullResultsCache.value
      isClusteringView.value = false
    }
  }

  return {
    isClusteringView,
    clusterThreshold,
    clusters,
    truncatedSingleFrames,
    toggleClustering,
    fetchClusters
  }
}
