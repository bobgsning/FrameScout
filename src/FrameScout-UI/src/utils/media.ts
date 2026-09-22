/**
 * Media helpers: asset URL conversion & video detection.
 */
import { convertFileSrc } from '@tauri-apps/api/core'

const VIDEO_EXT_RE = /\.(mp4|mov|avi|webm|mkv|flv)$/

/** 本地绝对路径 -> WebView 可加载的 asset URL */
export function getAssetUrl(path: string): string {
  return convertFileSrc(path)
}

/** 是否为视频文件（决定渲染 <video> 还是 <img>） */
export function isVideo(path: string): boolean {
  if (!path) return false
  return VIDEO_EXT_RE.test(path.toLowerCase())
}

/** 拼接视频帧定位锚点：src + '#t=12.5' */
export function withFrameAnchor(url: string, timestamp: number): string {
  return `${url}#t=${timestamp}`
}
