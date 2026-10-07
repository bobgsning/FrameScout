import { createApp, watch } from 'vue'
import App from './App.vue'
import { i18n, setLocale } from './i18n'
import { usePreferences } from './composables/usePreferences'

// 全局样式：变量 -> 动画 -> 通用基础样式（顺序不可颠倒）
import './styles/variables.css'
import './styles/animations.css'
import './styles/common.css'

const app = createApp(App)
app.use(i18n)

// 初始化 UI 语言：默认英文，随偏好持久化并即时切换
const { language } = usePreferences()
setLocale(language.value)
watch(language, (v) => setLocale(v))

app.mount('#app')
