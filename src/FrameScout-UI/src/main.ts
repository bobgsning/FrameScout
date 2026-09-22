import { createApp } from 'vue'
import App from './App.vue'

// 全局样式：变量 -> 动画 -> 通用基础样式（顺序不可颠倒）
import './styles/variables.css'
import './styles/animations.css'
import './styles/common.css'

createApp(App).mount('#app')
