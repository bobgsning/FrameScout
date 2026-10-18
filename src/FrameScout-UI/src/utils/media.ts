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

function dirname(p: string): string {
  const i = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'))
  return i < 0 ? '' : p.slice(0, i)
}

function basename(p: string): string {
  const i = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'))
  return i < 0 ? p : p.slice(i + 1)
}

/**
 * 计算切片段的输出完整路径：导出目录优先，未设置则回退源视频同目录。
 * 文件名形如 `<原文件名>_clip_<12_5s><源扩展名>`（时间戳的 `.` 换成 `_`，避免非法字符）。
 *
 * 保留源容器后缀（而非固定 .mp4）：切片段走 `-c copy` 同容器流拷贝，MKV/WebM 里的
 * VP9/AV1/Opus 等非 MP4 友好编码若硬塞 .mp4 会失败；保留后缀则永不失败、秒级完成。
 * 源无扩展名时才回退 `.mp4`。
 */
export function clipOutputPath(itemPath: string, timestamp: number, exportDir: string): string {
  const base = basename(itemPath) || 'clip'
  const dot = base.lastIndexOf('.')
  const stem = dot > 0 ? base.slice(0, dot) : base
  const ext = dot > 0 ? base.slice(dot) : '.mp4'
  const stamp = timestamp.toFixed(1).replace('.', '_')
  const name = `${stem}_clip_${stamp}s${ext}`
  const dir = (exportDir && exportDir.trim()) || dirname(itemPath)
  return dir ? `${dir}\\${name}` : name
}
