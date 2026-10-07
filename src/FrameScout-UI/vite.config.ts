import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'
import { readFileSync } from 'node:fs'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// 从 package.json 读取版本号，注入到前端代码中供设置页等处使用
const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf-8'))

export default defineConfig({
  plugins: [vue()],

  // Tauri 必需：相对路径 base。默认 '/' 会让构建产物里的 /assets/*.js 成为绝对路径，
  // 在 tauri://localhost 协议下加载失败 → release 黑屏（dev 走 devUrl 所以正常）。
  base: './',

  // 注入应用版本号（从 package.json 读取），供设置页等处使用
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version)
  },

  // 路径别名
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },

  // Vite options tailored for Tauri development
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  // 构建配置
  build: {
    // 关键：不能用 'esnext'！那会让产物保留最新 ES 语法、不做降级，
    // Windows 的 WebView2 内核版本可能不兼容 → JS 解析失败 → release 黑屏。
    // Tauri 官方推荐 Windows 用 chrome105（macOS/Linux 用 safari13）。
    target: process.env.TAURI_ENV_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  }
})