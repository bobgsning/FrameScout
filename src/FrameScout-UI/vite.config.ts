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
    target: 'esnext',
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  }
})