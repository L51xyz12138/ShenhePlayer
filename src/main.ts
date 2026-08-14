import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './styles/base.css'

/**
 * 禁掉 WebView2 的默认右键菜单：桌面应用里出现「返回 / 刷新」很怪，
 * 点下去还会把单页应用导航坏掉。输入框上保留，否则右键粘贴用不了。
 */
function suppressBrowserContextMenu() {
  window.addEventListener('contextmenu', (e) => {
    const el = e.target as HTMLElement | null
    if (el?.closest('input, textarea, [contenteditable="true"]')) return
    e.preventDefault()
  })
}

async function boot() {
  const label = getCurrentWindow().label
  suppressBrowserContextMenu()

  // player 窗口的 WebView2 完全被视频子窗口盖住，不需要渲染任何东西。
  // 挂个空壳，省掉一次 Vue 应用的初始化开销。
  if (label === 'player') {
    document.body.style.background = '#000'
    return
  }

  const pinia = createPinia()

  // overlay 是叠在视频之上的透明窗口，只挂载播放控制界面
  if (label === 'overlay') {
    // 控制条叠在视频上，无论应用主题是什么都必须走深色
    document.documentElement.dataset.surface = 'player'
    const PlayerRoot = (await import('@/components/player/PlayerRoot.vue')).default
    createApp(PlayerRoot).use(pinia).mount('#app')
    return
  }

  const App = (await import('@/App.vue')).default
  const router = (await import('@/router')).default
  createApp(App).use(pinia).use(router).mount('#app')
}

void boot()
