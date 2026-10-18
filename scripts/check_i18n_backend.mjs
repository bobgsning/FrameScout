// 校验各语言包的 `backend.*` 码表是否与 en 一致：
//   1) 键集合是否齐全（缺失会回退到英文）；
//   2) 是否有多余键（拼写错误）；
//   3) 插值占位符是否一致（{0}/{1}/{detail} 少一个就会直接显示占位符）。
// 退出码 0 = 全部一致。
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const dir = join(here, '..', 'src', 'FrameScout-UI', 'src', 'i18n', 'locales')
const files = ['en', 'zh-CN', 'zh-TW', 'ja', 'ru', 'ko', 'de', 'fr', 'es']

const data = {}
for (const f of files) {
  const raw = readFileSync(join(dir, f + '.json'), 'utf8')
  data[f] = JSON.parse(raw)

  // JSON.parse 对重复键会静默保留最后一个，必须靠原文检查。
  // 顶层键必然是 2 空格缩进，据此判断是否有重名块（重复会让新码表整体失效）。
  const dupes = new Set()
  const seen = new Set()
  for (const m of raw.matchAll(/^ {2}"([^"]+)": \{/gm)) {
    if (seen.has(m[1])) dupes.add(m[1])
    seen.add(m[1])
  }
  if (dupes.size) {
    console.log(`FATAL ${f}: duplicate root keys -> ${[...dupes].join(', ')}`)
    process.exit(1)
  }
}

const placeholders = (s) => (String(s).match(/\{\w+\}/g) || []).sort().join(',')

const base = data.en.backend
const baseKeys = Object.keys(base)
console.log(`en: ${baseKeys.length} backend keys\n`)

let bad = 0
for (const f of files) {
  const b = data[f].backend || {}
  const missing = baseKeys.filter((k) => !(k in b))
  const extra = Object.keys(b).filter((k) => !baseKeys.includes(k))
  const phBad = baseKeys
    .filter((k) => k in b && placeholders(base[k]) !== placeholders(b[k]))
    .map((k) => `${k} [${placeholders(base[k])} vs ${placeholders(b[k])}]`)

  if (missing.length || extra.length || phBad.length) {
    bad++
    console.log(`--- ${f}: missing=${missing.length} extra=${extra.length} ph=${phBad.length}`)
    if (missing.length) console.log('  MISSING: ' + missing.join(', '))
    if (extra.length) console.log('  EXTRA:   ' + extra.join(', '))
    if (phBad.length) console.log('  PH:      ' + phBad.join(' | '))
  } else {
    console.log(`OK ${f} (${Object.keys(b).length} keys)`)
  }
}

process.exit(bad ? 1 : 0)
