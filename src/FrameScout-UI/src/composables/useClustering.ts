/**
 * useClustering — 视觉相似度聚类视图。
 * 依赖 useSearch（进入聚类前缓存结果，退出后恢复）。
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ClusterGroup } from '@/types/search'
import { useSearch } from './useSearch'

const isClusteringView = ref(false)
const clusterThreshold = ref(0.8)
const clusters = ref<ClusterGroup[]>([])

export function useClustering() {
  const { results, fullResultsCache } = useSearch()

  async function fetchClusters() {
    try {
      const res: ClusterGroup[] = await invoke('cluster_similar_images', {
        threshold: clusterThreshold.value
      })
      clusters.value = res
    } catch (err) {
      console.error('Clustering error:', err)
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
    toggleClustering,
    fetchClusters
  }
}
