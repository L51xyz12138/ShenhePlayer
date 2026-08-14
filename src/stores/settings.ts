import { defineStore } from 'pinia'
import { onScopeDispose, ref, watchEffect } from 'vue'
import * as api from '@/api'
import type { AppInfo, PlayerSettings, Settings, Theme, UiSettings } from '@/types'

const DEFAULT_PLAYER: PlayerSettings = {
  mode: 'internal',
  mpvPath: '',
  externalPath: '',
  externalKind: 'mpv',
  externalArgs: '',
  quality: 'balanced',
  hwdec: 'auto-safe',
  gpuNext: true,
  volume: 100,
  subFontSize: 46,
  skipIntroSeconds: 85,
  autoNext: true,
  fullscreenOnPlay: false,
  maxBitrate: 0,
}

const DEFAULT_UI: UiSettings = {
  theme: 'dark',
  accent: '#0a84ff',
  gridSize: 0,
  showBackdrop: true,
  reduceMotion: false,
}

export const useSettingsStore = defineStore('settings', () => {
  const player = ref<PlayerSettings>({ ...DEFAULT_PLAYER })
  const ui = ref<UiSettings>({ ...DEFAULT_UI })
  const info = ref<AppInfo | null>(null)
  const loaded = ref(false)

  // 跟随系统时要实时响应 Windows 的浅色/深色切换
  const systemDark = ref(true)
  const media = window.matchMedia?.('(prefers-color-scheme: dark)')
  if (media) {
    systemDark.value = media.matches
    const onChange = (e: MediaQueryListEvent) => (systemDark.value = e.matches)
    media.addEventListener('change', onChange)
    onScopeDispose(() => media.removeEventListener('change', onChange))
  }

  async function load() {
    const s: Settings = await api.getSettings()
    player.value = { ...DEFAULT_PLAYER, ...s.player }
    ui.value = { ...DEFAULT_UI, ...s.ui }
    info.value = await api.appInfo()
    loaded.value = true
  }

  async function savePlayer(patch: Partial<PlayerSettings>) {
    player.value = { ...player.value, ...patch }
    await api.updatePlayerSettings(player.value)
  }

  async function saveUi(patch: Partial<UiSettings>) {
    ui.value = { ...ui.value, ...patch }
    await api.updateUiSettings(ui.value)
  }

  function resolveTheme(theme: Theme): 'light' | 'dark' {
    if (theme === 'system') return systemDark.value ? 'dark' : 'light'
    return theme
  }

  // 主题、强调色、低性能模式直接落到 CSS 变量上
  watchEffect(() => {
    const root = document.documentElement
    root.dataset.theme = resolveTheme(ui.value.theme)
    root.dataset.perf = ui.value.reduceMotion ? 'low' : 'normal'
    root.style.setProperty('--accent', ui.value.accent)
    root.style.setProperty('--accent-soft', hexToRgba(ui.value.accent, 0.16))
    root.style.setProperty('--accent-glow', hexToRgba(ui.value.accent, 0.38))
    root.style.setProperty('--accent-hover', shift(ui.value.accent, 0.16))
  })

  return { player, ui, info, loaded, systemDark, load, savePlayer, saveUi, resolveTheme }
})

function parseHex(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return [10, 132, 255]
  const n = parseInt(m[1], 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

function hexToRgba(hex: string, alpha: number): string {
  const [r, g, b] = parseHex(hex)
  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

/** hover 态：暗色下提亮，亮色下压深，两边都能拉开对比 */
function shift(hex: string, amount: number): string {
  const [r, g, b] = parseHex(hex)
  const light = document.documentElement.dataset.theme === 'light'
  const f = (c: number) => Math.round(light ? c * (1 - amount) : c + (255 - c) * amount)
  return `rgb(${f(r)}, ${f(g)}, ${f(b)})`
}
