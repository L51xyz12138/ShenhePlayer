<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { listen } from '@tauri-apps/api/event'
import * as api from '@/api'
import AppIcon from '@/components/AppIcon.vue'
import { useSettingsStore } from '@/stores/settings'
import { useSessionStore } from '@/stores/session'
import type { DetectedPlayer, DownloadProgress, QualityPreset, Theme, UpdateInfo } from '@/types'

const settings = useSettingsStore()
const session = useSessionStore()
const router = useRouter()

const players = ref<DetectedPlayer[]>([])
const saving = ref(false)
const testError = ref('')

const update = ref<UpdateInfo | null>(null)
const checking = ref(false)
const updateError = ref('')

async function checkUpdate() {
  if (checking.value) return
  checking.value = true
  updateError.value = ''
  try {
    update.value = await api.checkUpdate()
  } catch (e) {
    updateError.value = String(e)
  } finally {
    checking.value = false
  }
}

// ---- 应用内更新 ----
const downloading = ref(false)
const progress = ref<DownloadProgress | null>(null)
const installerPath = ref('')
let unlistenProgress: (() => void) | undefined

onMounted(async () => {
  unlistenProgress = await listen<DownloadProgress>('update:progress', (e) => {
    progress.value = e.payload
  })
})

onBeforeUnmount(() => unlistenProgress?.())

const percent = computed(() => {
  const p = progress.value
  if (!p || !p.total) return 0
  return Math.min(100, Math.round((p.downloaded / p.total) * 100))
})

const sizeText = computed(() => {
  const p = progress.value
  if (!p) return ''
  const mb = (n: number) => (n / 1024 / 1024).toFixed(1)
  return p.total ? `${mb(p.downloaded)} / ${mb(p.total)} MB` : `${mb(p.downloaded)} MB`
})

async function download() {
  if (downloading.value) return
  downloading.value = true
  updateError.value = ''
  progress.value = null
  try {
    installerPath.value = await api.downloadUpdate()
  } catch (e) {
    updateError.value = String(e)
  } finally {
    downloading.value = false
  }
}

/** 安装程序一起来本进程就退出，否则覆盖不了正在运行的文件 */
async function install() {
  try {
    await api.installUpdate(installerPath.value)
  } catch (e) {
    updateError.value = String(e)
  }
}

const updateText = computed(() => {
  if (updateError.value) return updateError.value
  const u = update.value
  if (!u) return ''
  if (u.available) return `发现新版本 ${u.latest}`
  if (u.noRelease) return '还没有发布版本'
  return '已是最新版本'
})

const QUALITY: { value: QualityPreset; label: string; desc: string }[] = [
  { value: 'performance', label: '性能优先', desc: '双线性缩放，核显 / 老机器最省' },
  { value: 'balanced', label: '均衡', desc: 'spline36 缩放，默认推荐' },
  { value: 'quality', label: '画质优先', desc: 'EWA 缩放 + 去色带，需要独显' },
]

const HWDEC = [
  { value: 'auto-safe', label: '自动（推荐）' },
  { value: 'd3d11va', label: 'D3D11VA' },
  { value: 'd3d11va-copy', label: 'D3D11VA (copy)' },
  { value: 'nvdec', label: 'NVDEC（N 卡）' },
  { value: 'no', label: '关闭硬解' },
]

// Apple 深色模式的系统色
const ACCENTS = ['#0a84ff', '#bf5af2', '#ff375f', '#ff9f0a', '#32d74b', '#64d2ff']

const THEMES: { value: Theme; label: string }[] = [
  { value: 'light', label: '亮色' },
  { value: 'dark', label: '暗色' },
  { value: 'system', label: '跟随系统' },
]

onMounted(async () => {
  if (!settings.loaded) await settings.load()
  players.value = await api.listExternalPlayers()
})

const mpvStatus = computed(() => {
  const info = settings.info
  if (!info) return { ok: false, text: '检测中…' }
  return info.mpvAvailable
    ? { ok: true, text: info.mpvPath ?? '' }
    : { ok: false, text: '未找到 mpv.exe，内置播放器不可用' }
})

async function save<T extends 'player' | 'ui'>(kind: T, patch: object) {
  saving.value = true
  try {
    if (kind === 'player') await settings.savePlayer(patch)
    else await settings.saveUi(patch)
  } finally {
    saving.value = false
  }
}

async function pickExe(target: 'mpv' | 'external') {
  const picked = await open({
    multiple: false,
    filters: [{ name: '可执行文件', extensions: ['exe'] }],
  })
  if (typeof picked !== 'string') return

  if (target === 'mpv') {
    await save('player', { mpvPath: picked })
    settings.info = await api.appInfo()
  } else {
    await save('player', { externalPath: picked, externalKind: guessKind(picked) })
  }
}

function guessKind(path: string): string {
  const name = path.toLowerCase()
  if (name.includes('mpv')) return 'mpv'
  if (name.includes('potplayer')) return 'potplayer'
  if (name.includes('vlc')) return 'vlc'
  if (name.includes('mpc-be')) return 'mpc-be'
  if (name.includes('mpc')) return 'mpc-hc'
  return 'custom'
}

async function useDetected(p: DetectedPlayer) {
  await save('player', { externalPath: p.path, externalKind: p.kind })
}

/** 用 mpv 的合成测试源验证：进程能起、硬解可用、画面能嵌进窗口 */
async function testPlayer() {
  testError.value = ''
  try {
    await api.playTestPattern()
  } catch (e) {
    testError.value = String(e)
  }
}

async function signOut() {
  await session.signOut()
  await router.replace({ name: 'servers' })
}
</script>

<template>
  <div class="settings">
    <header class="head">
      <h1 class="t-title-2">设置</h1>
      <span v-if="saving" class="t-caption">保存中…</span>
    </header>

    <!-- ---------------- 播放方式 ---------------- -->
    <section class="card group">
      <h2 class="group-title">
        <AppIcon name="play" :size="16" />
        播放方式
      </h2>

      <div class="mode-cards">
        <button
          class="mode"
          :class="{ active: settings.player.mode === 'internal' }"
          @click="save('player', { mode: 'internal' })"
        >
          <AppIcon name="film" :size="20" />
          <b>内置播放器</b>
          <span>嵌入 mpv，硬件解码，画面在窗口内播放</span>
        </button>
        <button
          class="mode"
          :class="{ active: settings.player.mode === 'external' }"
          @click="save('player', { mode: 'external' })"
        >
          <AppIcon name="external" :size="20" />
          <b>外置播放器</b>
          <span>交给 PotPlayer / mpv / VLC 等已装的播放器</span>
        </button>
      </div>
    </section>

    <!-- ---------------- 内置播放器 ---------------- -->
    <section v-if="settings.player.mode === 'internal'" class="card group">
      <h2 class="group-title">
        <AppIcon name="settings" :size="16" />
        内置播放器
      </h2>

      <div class="field">
        <div class="field-label">
          <span>mpv 程序</span>
          <em :class="{ bad: !mpvStatus.ok }">{{ mpvStatus.text }}</em>
        </div>
        <div class="row-input">
          <input
            :value="settings.player.mpvPath"
            class="input mono"
            placeholder="留空则自动查找"
            readonly
          />
          <button class="btn" @click="pickExe('mpv')">浏览…</button>
          <button class="btn" title="播放一段合成画面，检查播放器是否正常" @click="testPlayer">
            <AppIcon name="play" :size="15" />
            测试
          </button>
        </div>
        <p v-if="testError" class="test-error">{{ testError }}</p>
      </div>

      <div class="field">
        <div class="field-label"><span>画质档位</span></div>
        <div class="quality">
          <button
            v-for="q in QUALITY"
            :key="q.value"
            class="quality-card"
            :class="{ active: settings.player.quality === q.value }"
            @click="save('player', { quality: q.value })"
          >
            <b>{{ q.label }}</b>
            <span>{{ q.desc }}</span>
          </button>
        </div>
      </div>

      <div class="field two">
        <label>
          <div class="field-label"><span>硬件解码</span></div>
          <select
            class="select"
            :value="settings.player.hwdec"
            @change="save('player', { hwdec: ($event.target as HTMLSelectElement).value })"
          >
            <option v-for="h in HWDEC" :key="h.value" :value="h.value">{{ h.label }}</option>
          </select>
        </label>

        <label>
          <div class="field-label"><span>字幕字号</span></div>
          <div class="slider-row">
            <input
              type="range"
              min="24"
              max="80"
              :value="settings.player.subFontSize"
              @change="save('player', { subFontSize: +($event.target as HTMLInputElement).value })"
            />
            <span class="mono val">{{ settings.player.subFontSize }}</span>
          </div>
        </label>
      </div>

      <label class="switch-row">
        <input
          type="checkbox"
          :checked="settings.player.gpuNext"
          @change="save('player', { gpuNext: ($event.target as HTMLInputElement).checked })"
        />
        <div>
          <b>使用 gpu-next 渲染器</b>
          <span>画质更好；个别老显卡驱动可能不稳定，异常时关掉</span>
        </div>
      </label>

      <label class="switch-row">
        <input
          type="checkbox"
          :checked="settings.player.autoNext"
          @change="save('player', { autoNext: ($event.target as HTMLInputElement).checked })"
        />
        <div>
          <b>自动播放下一集</b>
          <span>当前这集放完后接着播同一季的下一集</span>
        </div>
      </label>

      <label class="switch-row">
        <input
          type="checkbox"
          :checked="settings.player.fullscreenOnPlay"
          @change="
            save('player', { fullscreenOnPlay: ($event.target as HTMLInputElement).checked })
          "
        />
        <div>
          <b>播放时自动全屏</b>
          <span>开始播放就切到全屏</span>
        </div>
      </label>
    </section>

    <!-- ---------------- 外置播放器 ---------------- -->
    <section v-else class="card group">
      <h2 class="group-title">
        <AppIcon name="external" :size="16" />
        外置播放器
      </h2>

      <div v-if="players.length" class="detected">
        <button
          v-for="p in players"
          :key="p.path"
          class="detected-item"
          :class="{ active: settings.player.externalPath === p.path }"
          @click="useDetected(p)"
        >
          <AppIcon name="play" :size="15" />
          <div class="truncate">
            <b>{{ p.name }}</b>
            <em class="mono">{{ p.path }}</em>
          </div>
        </button>
      </div>
      <p v-else class="t-caption">没有自动检测到播放器，请手动指定。</p>

      <div class="field">
        <div class="field-label"><span>播放器程序</span></div>
        <div class="row-input">
          <input :value="settings.player.externalPath" class="input mono" readonly placeholder="未选择" />
          <button class="btn" @click="pickExe('external')">浏览…</button>
        </div>
      </div>

      <div class="field">
        <div class="field-label">
          <span>自定义启动参数</span>
          <em>可用占位符：{url} {title} {start} {start_hms} {start_ms}</em>
        </div>
        <input
          :value="settings.player.externalArgs"
          class="input mono"
          placeholder="留空使用为该播放器预设的参数"
          @change="save('player', { externalArgs: ($event.target as HTMLInputElement).value })"
        />
      </div>
    </section>

    <!-- ---------------- 界面 ---------------- -->
    <section class="card group">
      <h2 class="group-title">
        <AppIcon name="sparkles" :size="16" />
        界面
      </h2>

      <div class="field">
        <div class="field-label"><span>外观</span></div>
        <div class="segmented">
          <button
            v-for="t in THEMES"
            :key="t.value"
            class="seg"
            :class="{ active: settings.ui.theme === t.value }"
            @click="save('ui', { theme: t.value })"
          >
            {{ t.label }}
          </button>
        </div>
      </div>

      <div class="field">
        <div class="field-label"><span>主题色</span></div>
        <div class="accents">
          <button
            v-for="c in ACCENTS"
            :key="c"
            class="accent-dot"
            :class="{ active: settings.ui.accent === c }"
            :style="{ background: c }"
            @click="save('ui', { accent: c })"
          />
        </div>
      </div>

      <label class="switch-row">
        <input
          type="checkbox"
          :checked="settings.ui.reduceMotion"
          @change="save('ui', { reduceMotion: ($event.target as HTMLInputElement).checked })"
        />
        <div>
          <b>低性能模式</b>
          <span>关闭毛玻璃与过渡动画，老机器更流畅</span>
        </div>
      </label>
    </section>

    <!-- ---------------- 账号 ---------------- -->
    <section class="card group">
      <h2 class="group-title">
        <AppIcon name="user" :size="16" />
        账号
      </h2>

      <div v-if="session.session" class="account-row">
        <div>
          <b>{{ session.session.userName }}</b>
          <span class="t-caption">{{ session.session.serverName }} · {{ session.session.serverUrl }}</span>
        </div>
        <button class="btn danger" @click="signOut">
          <AppIcon name="logout" :size="16" />
          退出登录
        </button>
      </div>

    </section>

    <!-- ---------------- 关于 ---------------- -->
    <section class="card group">
      <h2 class="group-title">
        <AppIcon name="info" :size="16" />
        关于
      </h2>

      <div class="about-row">
        <div>
          <b class="t-footnote">ShenhePlayer {{ settings.info?.version ?? '' }}</b>
          <span v-if="updateText" class="t-caption" :class="{ hot: update?.available, bad: updateError }">
            {{ updateText }}
          </span>
          <span v-else class="t-caption dim-3">点右边检查是否有新版本</span>
        </div>

        <div class="about-actions">
          <button
            v-if="update?.available && installerPath"
            class="btn btn-primary"
            @click="install"
          >
            <AppIcon name="check" :size="16" />
            立即安装
          </button>
          <button
            v-else-if="update?.available"
            class="btn btn-primary"
            :disabled="downloading"
            @click="download"
          >
            <AppIcon name="download" :size="16" />
            {{ downloading ? '下载中…' : '下载并更新' }}
          </button>

          <button
            v-if="update?.available"
            class="btn"
            title="在浏览器里打开发布页"
            @click="api.openReleasePage(update!.url)"
          >
            <AppIcon name="external" :size="16" />
          </button>

          <button class="btn" :disabled="checking || downloading" @click="checkUpdate">
            <AppIcon name="refresh" :size="16" />
            {{ checking ? '检查中…' : '检查更新' }}
          </button>
        </div>
      </div>

      <div v-if="downloading || installerPath" class="progress">
        <div class="track">
          <span :style="{ transform: `scaleX(${percent / 100})` }" />
        </div>
        <span class="t-caption dim num">
          {{ installerPath ? '下载完成，点「立即安装」后程序会自动更新并重新打开' : `${percent}% · ${sizeText}` }}
        </span>
      </div>

      <p v-if="update?.available && update.notes" class="notes t-caption">
        {{ update.notes.slice(0, 400) }}
      </p>

      <p v-if="settings.info" class="t-caption version">
        配置文件 {{ settings.info.configPath }}
      </p>
    </section>
  </div>
</template>

<style scoped>
.settings {
  max-width: 56rem;
  padding: 1.75rem var(--page-pad) 3.5rem;
}

.head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 1.5rem;
}

.group {
  padding: 1.35rem 1.5rem;
  margin-bottom: 1.1rem;
}

.group-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9375rem;
  font-weight: 640;
  margin-bottom: 1.15rem;
  color: var(--label);
}

.group-title svg {
  color: var(--accent);
}

.field {
  margin-bottom: 1.15rem;
}

.field.two {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.25rem;
}

.field-label {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 0.45rem;
  font-size: 0.8125rem;
  font-weight: 560;
}

.field-label em {
  font-style: normal;
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--label-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-label em.bad {
  color: var(--orange);
}

.row-input {
  display: flex;
  gap: 0.5rem;
}

.row-input .input {
  font-size: 0.8125rem;
}

.test-error {
  margin: 0.5rem 0 0;
  font-size: 0.75rem;
  color: var(--red);
  line-height: 1.45;
}

/* ---- 播放方式卡片 ---- */
.mode-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

.mode {
  display: grid;
  gap: 0.3rem;
  padding: 1rem;
  border-radius: var(--r-md);
  background: var(--fill-1);
  border: 1px solid var(--separator);
  text-align: left;
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), border-color var(--t-fast) var(--ease),
    transform var(--t-fast) var(--ease);
}

.mode:hover {
  background: var(--fill-2);
}

.mode:active {
  transform: scale(0.99);
}

.mode.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--label);
}

.mode svg {
  color: var(--accent);
}

.mode b {
  font-size: 0.875rem;
  font-weight: 620;
}

.mode span {
  font-size: 0.75rem;
  color: var(--label-3);
  line-height: 1.45;
}

/* ---- 画质档位 ---- */
.quality {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0.6rem;
}

.quality-card {
  display: grid;
  gap: 0.2rem;
  padding: 0.7rem 0.8rem;
  border-radius: var(--r-sm);
  background: var(--fill-1);
  border: 1px solid var(--separator);
  text-align: left;
  transition: background var(--t-fast) var(--ease), border-color var(--t-fast) var(--ease);
}

.quality-card:hover {
  background: var(--fill-2);
}

.quality-card.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.quality-card b {
  font-size: 0.8125rem;
  font-weight: 620;
}

.quality-card span {
  font-size: 0.6875rem;
  color: var(--label-3);
  line-height: 1.4;
}

.select {
  width: 100%;
  padding: 0.55rem 0.75rem;
  border-radius: var(--r-sm);
  background: rgba(0, 0, 0, 0.28);
  border: 1px solid var(--separator);
  color: var(--label);
  font: inherit;
  font-size: 0.8125rem;
  cursor: pointer;
}

.select:focus {
  outline: none;
  border-color: var(--accent);
}

.select option {
  background: var(--bg-1);
}

.slider-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.slider-row input[type='range'] {
  flex: 1;
  accent-color: var(--accent);
}

.val {
  min-width: 2rem;
  font-size: 0.8125rem;
  color: var(--label-2);
  text-align: right;
}

/* ---- 开关行 ---- */
.switch-row {
  display: flex;
  align-items: flex-start;
  gap: 0.7rem;
  padding: 0.7rem 0;
  border-top: 1px solid var(--separator);
  cursor: pointer;
}

.switch-row input {
  margin-top: 0.15rem;
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
  flex: none;
}

.switch-row b {
  display: block;
  font-size: 0.8125rem;
  font-weight: 570;
}

.switch-row span {
  font-size: 0.75rem;
  color: var(--label-3);
}

/* ---- 检测到的播放器 ---- */
.detected {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin-bottom: 1.15rem;
}

.detected-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.7rem;
  border-radius: var(--r-sm);
  background: var(--fill-1);
  border: 1px solid transparent;
  color: var(--label-2);
  text-align: left;
}

.detected-item:hover {
  background: var(--fill-2);
}

.detected-item.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--label);
}

.detected-item b {
  display: block;
  font-size: 0.8125rem;
  font-weight: 580;
}

.detected-item em {
  font-style: normal;
  font-size: 0.6875rem;
  color: var(--label-3);
}

/* ---- 外观 ---- */
.segmented {
  display: inline-flex;
  padding: 3px;
  border-radius: var(--r-full);
  background: var(--fill-1);
}

.seg {
  padding: 0.35rem 1rem;
  border-radius: var(--r-full);
  font-size: 0.8125rem;
  font-weight: 540;
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}

.seg:hover {
  color: var(--label);
}

.seg.active {
  background: var(--fill-3);
  color: var(--label);
}

/* ---- 主题色 ---- */
.accents {
  display: flex;
  gap: 0.55rem;
}

.accent-dot {
  width: 1.75rem;
  height: 1.75rem;
  border-radius: var(--r-full);
  border: 2px solid transparent;
  outline-offset: 3px;
  transition: transform var(--t-fast) var(--ease-bounce), box-shadow var(--t-fast) var(--ease);
}

.accent-dot:hover {
  transform: scale(1.12);
}

.accent-dot.active {
  box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px currentColor;
  transform: scale(1.08);
}

/* ---- 账号 ---- */
.account-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.account-row b {
  display: block;
  font-size: 0.9375rem;
}

.account-row .t-caption {
  display: block;
}

.btn.danger {
  color: var(--red);
  background: rgba(255, 69, 58, 0.16);
}

.btn.danger:hover {
  background: rgba(255, 69, 58, 0.26);
}

.about-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
}

.about-row b {
  display: block;
  font-weight: 600;
}

.about-row span {
  display: block;
  color: var(--label-2);
}

.about-row .hot {
  color: var(--accent);
  font-weight: 560;
}

.about-row .bad {
  color: var(--red);
}

.about-actions {
  display: flex;
  gap: 0.4rem;
  flex: none;
}

.about-actions .btn {
  padding: 0.42rem 0.9rem;
  font-size: 0.875rem;
}

.progress {
  margin-top: 0.9rem;
}

.progress .track {
  height: 5px;
  border-radius: var(--r-full);
  background: var(--fill-2);
  overflow: hidden;
}

.progress .track span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
  transform-origin: left center;
  transition: transform 200ms var(--ease);
}

.progress .t-caption {
  display: block;
  margin-top: 0.4rem;
}

.notes {
  margin: 0.9rem 0 0;
  padding: 0.7rem 0.85rem;
  border-radius: var(--r-sm);
  background: var(--fill-1);
  color: var(--label-2);
  white-space: pre-wrap;
  line-height: 1.55;
  max-height: 12rem;
  overflow-y: auto;
}

.version {
  margin: 1.1rem 0 0;
  padding-top: 0.9rem;
  border-top: 1px solid var(--separator);
  font-size: 0.6875rem;
  word-break: break-all;
}
</style>
