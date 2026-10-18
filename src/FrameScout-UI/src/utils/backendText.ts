/**
 * 后端「错误码 / 状态码」→ 本地化文案。
 *
 * 实现放在 `i18n/index.ts` 的 `localizeCode`（它需要直接访问 i18n 实例，
 * 且 `t()` 要靠它来自动解析模板里的 `{err}`）。这里只做转发，避免两份实现漂移。
 *
 * 契约见 `localizeCode` 的文档，与 Rust 侧 `src-tauri/src/error.rs` 对齐。
 */
import { localizeCode } from '@/i18n'

/**
 * 把后端回传的码（或任意字符串）转成本地化文案。
 * 非码内容（真实路径、普通句子、空串）原样返回，故可安全用于
 * `scan-progress.file_path` 这类「有时是路径、有时是状态文案」的字段。
 */
export function localizeBackend(raw: unknown): string {
  return localizeCode(raw)
}
