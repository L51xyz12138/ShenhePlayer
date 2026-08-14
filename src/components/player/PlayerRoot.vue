<script setup lang="ts">
// 播放控制层。跑在一个透明的 overlay 窗口里，叠在 player 窗口的视频之上。
//
// 为什么不合成一个窗口：Tauri 的透明窗口带 WS_EX_NOREDIRECTIONBITMAP，
// 没有重定向表面，子窗口不参与合成，视频放在透明 WebView2 底下根本不显示。
// 所以视频铺满不透明的 player 窗口，控制条由这个透明窗口叠上去。
//
// 因此这里所有窗口操作（最小化 / 最大化 / 全屏）都要作用在 player 窗口上，
// 不能用 getCurrentWindow()，那指的是 overlay 自己。
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import * as api from '@/api'
import AppIcon from '../AppIcon.vue'
import SeekBar from './SeekBar.vue'
import TrackMenu from './TrackMenu.vue'
import { usePlayerStore } from '@/stores/player'
import { useSettingsStore } from '@/stores/settings'
import { formatTime } from '@/utils/format'

const player = usePlayerStore()
const settings = useSettingsStore()

const chromeVisible = ref(true)
const fullscreen = ref(false)
const maximized = ref(false)
const volumeOpen = ref(false)
const toast = ref('')

/** 同一时刻只允许一个弹出菜单打开 */
type MenuName = '' | 'speed' | 'audio' | 'sub'
const openMenu = ref<MenuName>('')
const menuOpen = computed(() => openMenu.value !== '')

function toggleMenu(name: Exclude<MenuName, ''>) {
  openMenu.value = openMenu.value === name ? '' : name
  wake()
}

/** 给 TrackMenu 用的 v-model 读写对 */
function menuModel(name: Exclude<MenuName, ''>) {
  return computed({
    get: () => openMenu.value === name,
    set: (v: boolean) => (openMenu.value = v ? name : ''),
  })
}

const audioMenu = menuModel('audio')
const subMenu = menuModel('sub')
const speedMenu = menuModel('speed')

let idleTimer: number | undefined
let toastTimer: number | undefined
let unlistenResize: (() => void) | undefined

const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2]

const snapshot = computed(() => player.snapshot)
const title = computed(() => player.target?.title ?? '')
const subTitle = computed(() => player.target?.subTitle ?? '')

const techLine = computed(() => {
  const s = snapshot.value
  const bits: string[] = []
  if (s.width && s.height) bits.push(`${s.width}×${s.height}`)
  if (s.videoCodec) bits.push(s.videoCodec.toUpperCase())
  if (s.hwdec && s.hwdec !== 'no') bits.push('硬件解码')
  if (player.target) bits.push(player.target.isDirect ? '直接播放' : '服务器转码')
  return bits.join('  ·  ')
})

const speedLabel = computed(() => `${snapshot.value.speed.toFixed(2).replace(/\.?0+$/, '')}×`)

// ---------------------------------------------------------------- 自动隐藏

function wake() {
  chromeVisible.value = true
  if (idleTimer) window.clearTimeout(idleTimer)
  // 暂停时、或有菜单开着时控制条常驻——这两种情况用户都还要继续操作
  if (snapshot.value.paused || menuOpen.value) return
  idleTimer = window.setTimeout(() => {
    chromeVisible.value = false
    volumeOpen.value = false
    openMenu.value = ''
  }, 2800)
}

/** 刚刚那一下点击只是用来关菜单的，别再触发播放/暂停 */
let swallowNextStageClick = false

/** 点菜单以外的地方就关掉它。菜单是点击打开的，不该因为鼠标移开而消失。 */
function onPointerDownCapture(e: PointerEvent) {
  if (!menuOpen.value) return
  const target = e.target as HTMLElement | null
  if (target?.closest('.menu-anchor, .track-menu')) return
  openMenu.value = ''
  // 这一下只用来关菜单，不应该顺带把播放暂停了
  swallowNextStageClick = true
}

function onStageClick() {
  if (swallowNextStageClick) {
    swallowNextStageClick = false
    return
  }
  void togglePause()
}

function showToast(text: string) {
  toast.value = text
  if (toastTimer) window.clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => (toast.value = ''), 1000)
}

// ---------------------------------------------------------------- 操作

async function togglePause() {
  await player.togglePause()
  wake()
}

async function toggleFullscreen() {
  fullscreen.value = !fullscreen.value
  await api.setFullscreen(fullscreen.value)
  wake()
}

async function skip(delta: number) {
  await player.seekRelative(delta)
  showToast(`${delta > 0 ? '快进' : '后退'} ${Math.abs(delta)} 秒`)
  wake()
}

async function changeVolume(delta: number) {
  await player.setVolume(snapshot.value.volume + delta)
  showToast(`音量 ${Math.round(snapshot.value.volume)}%`)
  wake()
}

async function skipIntro() {
  await player.seek(player.position + settings.player.skipIntroSeconds)
  showToast('已跳过片头')
}

/** 关闭播放窗口 = 停止播放并回写进度 */
async function close() {
  if (fullscreen.value) {
    fullscreen.value = false
    await api.setFullscreen(false)
  }
  await player.stop()
}

// ---------------------------------------------------------------- 键盘

function onKey(e: KeyboardEvent) {
  const handlers: Record<string, () => void> = {
    ' ': () => void togglePause(),
    k: () => void togglePause(),
    ArrowRight: () => void skip(e.shiftKey ? 60 : 10),
    ArrowLeft: () => void skip(e.shiftKey ? -60 : -10),
    ArrowUp: () => void changeVolume(5),
    ArrowDown: () => void changeVolume(-5),
    f: () => void toggleFullscreen(),
    m: () => void player.toggleMuted(),
    j: () => void skip(-10),
    l: () => void skip(10),
    Escape: () => {
      // Esc 逐层退出：先关菜单，再退全屏，最后才是关闭播放器
      if (menuOpen.value) openMenu.value = ''
      else if (fullscreen.value) void toggleFullscreen()
      else void close()
    },
  }
  const fn = handlers[e.key]
  if (!fn) return
  e.preventDefault()
  fn()
}

// ---------------------------------------------------------------- 生命周期

onMounted(async () => {
  await settings.load()
  await player.bind()
  await syncWindowState()
  // overlay 由 Rust 侧跟着 player 窗口一起改尺寸，这里顺带刷新一下按钮状态
  unlistenResize = await getCurrentWindow().onResized(syncWindowState)
  window.addEventListener('keydown', onKey)
  window.addEventListener('pointerdown', onPointerDownCapture, true)
  wake()
})

async function syncWindowState() {
  fullscreen.value = await api.isFullscreen()
  maximized.value = await api.playerIsMaximized()
}

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('pointerdown', onPointerDownCapture, true)
  unlistenResize?.()
  if (idleTimer) window.clearTimeout(idleTimer)
  if (toastTimer) window.clearTimeout(toastTimer)
})
</script>

<template>
  <div
    class="player"
    :class="{ idle: !chromeVisible }"
    @pointermove="wake"
    @wheel.prevent="changeVolume($event.deltaY < 0 ? 5 : -5)"
  >
    <!-- 画面区：单击播放/暂停，双击全屏 -->
    <div class="stage" @click="onStageClick" @dblclick="toggleFullscreen" />

    <Transition name="fade">
      <div v-if="snapshot.buffering" class="center-hint">
        <span class="spinner" />
      </div>
    </Transition>

    <Transition name="pop">
      <div v-if="snapshot.paused && !snapshot.buffering" class="center-hint">
        <div class="paused-glyph">
          <AppIcon name="pause" :size="30" :stroke="2.2" />
        </div>
      </div>
    </Transition>

    <Transition name="fade">
      <div v-if="toast" class="toast t-subhead">{{ toast }}</div>
    </Transition>

    <!-- ---------------- 顶栏 ---------------- -->
    <header class="top" @pointerdown.self="api.playerStartDrag()">
      <button class="glyph" title="返回媒体库（Esc）" @click="close">
        <AppIcon name="chevron-left" :size="21" />
      </button>

      <div class="titles" @pointerdown="api.playerStartDrag()">
        <div class="truncate t-headline">{{ title }}</div>
        <div v-if="subTitle" class="truncate t-caption dim">{{ subTitle }}</div>
      </div>

      <div class="tech t-caption dim-3">{{ techLine }}</div>

      <div class="win-buttons">
        <button class="win-btn" title="最小化" @click="api.playerMinimize()">
          <AppIcon name="minimize" :size="14" :stroke="1.6" />
        </button>
        <button
          class="win-btn"
          :title="maximized ? '还原' : '最大化'"
          @click="api.playerToggleMaximize()"
        >
          <AppIcon :name="maximized ? 'restore' : 'maximize'" :size="12" :stroke="1.6" />
        </button>
        <button class="win-btn danger" title="关闭" @click="close">
          <AppIcon name="close" :size="14" :stroke="1.6" />
        </button>
      </div>
    </header>

    <!-- ---------------- 控制条 ---------------- -->
    <footer class="bottom">
      <div class="panel">
        <SeekBar
          :position="player.position"
          :duration="player.duration"
          :buffered="player.buffered"
          @scrub-start="player.beginScrub"
          @scrub-move="player.updateScrub"
          @scrub-end="player.endScrub"
        />

        <div class="controls">
          <div class="time num t-footnote">
            {{ formatTime(player.position) }}
            <span class="dim-3">/ {{ formatTime(player.duration) }}</span>
          </div>

          <div class="transport">
            <button class="glyph" title="后退 10 秒（←）" @click="skip(-10)">
              <AppIcon name="rewind" :size="21" />
            </button>
            <button
              class="glyph play"
              :title="snapshot.paused ? '播放（空格）' : '暂停（空格）'"
              @click="togglePause"
            >
              <AppIcon
                :name="snapshot.paused ? 'play' : 'pause'"
                :size="24"
                :filled="snapshot.paused"
              />
            </button>
            <button class="glyph" title="快进 10 秒（→）" @click="skip(10)">
              <AppIcon name="forward" :size="21" />
            </button>
            <button
              v-if="player.nextItem"
              class="glyph"
              title="下一集"
              @click="player.playNext()"
            >
              <AppIcon name="next" :size="20" />
            </button>
          </div>

          <div class="tools">
            <button class="glyph text" title="跳过片头" @click="skipIntro">跳过片头</button>

            <div class="menu-anchor">
              <button
                class="glyph text num"
                :class="{ on: speedMenu }"
                title="播放速度"
                @click="toggleMenu('speed')"
              >
                {{ speedLabel }}
              </button>
              <Transition name="pop">
                <div v-if="speedMenu" class="menu">
                  <span class="bridge" />
                  <button
                    v-for="s in SPEEDS"
                    :key="s"
                    class="menu-item num"
                    :class="{ active: Math.abs(snapshot.speed - s) < 0.01 }"
                    @click="player.setSpeed(s); openMenu = ''"
                  >
                    <AppIcon name="check" :size="14" class="tick" />
                    <span>{{ s }}×</span>
                  </button>
                </div>
              </Transition>
            </div>

            <TrackMenu
              v-model:open="audioMenu"
              icon="audio"
              label="音轨"
              :tracks="player.audioTracks"
              :current="snapshot.audioTrack"
              @select="player.setTrack('audio', $event)"
            />
            <TrackMenu
              v-model:open="subMenu"
              icon="subtitles"
              label="字幕"
              :tracks="player.subTracks"
              :current="snapshot.subTrack"
              allow-off
              @select="player.setTrack('sub', $event)"
            />

            <div
              class="volume"
              @pointerenter="volumeOpen = true"
              @pointerleave="volumeOpen = false"
            >
              <button class="glyph" title="静音（M）" @click="player.toggleMuted()">
                <AppIcon :name="snapshot.muted ? 'volume-mute' : 'volume'" :size="20" />
              </button>
              <div class="volume-slider" :class="{ open: volumeOpen }">
                <input
                  type="range"
                  min="0"
                  max="130"
                  :value="snapshot.muted ? 0 : snapshot.volume"
                  @input="player.setVolume(+($event.target as HTMLInputElement).value)"
                />
              </div>
            </div>

            <button
              class="glyph"
              :title="fullscreen ? '退出全屏（F）' : '全屏（F）'"
              @click="toggleFullscreen"
            >
              <AppIcon :name="fullscreen ? 'fullscreen-exit' : 'fullscreen'" :size="20" />
            </button>
          </div>
        </div>
      </div>
    </footer>
  </div>
</template>

<style>
/* 整个窗口必须透明，视频才能从底下的子窗口透出来 */
html,
body,
#app {
  background: transparent !important;
}
</style>

<style scoped>
.player {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  color: #fff;
  overflow: hidden;
}

.player.idle {
  cursor: none;
}

.stage {
  position: absolute;
  inset: 0;
}

/* ---------------- 顶栏 ---------------- */

.top {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: 0.55rem 0.7rem 2.5rem;
  background: linear-gradient(to bottom, rgba(0, 0, 0, 0.72), transparent);
  transition: opacity var(--t-base) var(--ease), transform var(--t-base) var(--ease);
}

.titles {
  flex: 1;
  min-width: 0;
  padding-left: 0.15rem;
  text-shadow: 0 1px 10px rgba(0, 0, 0, 0.55);
}

.tech {
  flex: none;
  padding-right: 0.5rem;
  white-space: nowrap;
}

.win-buttons {
  display: flex;
  flex: none;
  gap: 2px;
}

.win-btn {
  display: grid;
  place-items: center;
  width: 2.1rem;
  height: 1.9rem;
  border-radius: var(--r-xs);
  color: rgba(255, 255, 255, 0.72);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}

.win-btn:hover {
  background: rgba(255, 255, 255, 0.16);
  color: #fff;
}

.win-btn.danger:hover {
  background: #e81123;
}

/* ---------------- 底部控制条 ----------------
 * 做成一块浮起来的毛玻璃面板，而不是通栏——面板有明确的边界，
 * 视觉上是「浮在画面之上的一层材质」，符合 Apple 的层次语言。
 */

.bottom {
  position: relative;
  z-index: 2;
  margin-top: auto;
  padding: 3rem 1rem 1rem;
  background: linear-gradient(to top, rgba(0, 0, 0, 0.6), transparent);
  transition: opacity var(--t-base) var(--ease), transform var(--t-base) var(--ease);
}

.panel {
  max-width: 1180px;
  margin: 0 auto;
  padding: 0.5rem 1.1rem 0.7rem;
  border-radius: var(--r-lg);
  background: var(--material);
  backdrop-filter: var(--material-blur);
  /* 顶边高光：像真实材质接住了光 */
  border-top: 1px solid rgba(255, 255, 255, 0.14);
  box-shadow: var(--shadow-lg);
}

.player.idle .top {
  opacity: 0;
  transform: translateY(-10px);
  pointer-events: none;
}

.player.idle .bottom {
  opacity: 0;
  transform: translateY(14px);
  pointer-events: none;
}

.controls {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: var(--sp-3);
  margin-top: 0.1rem;
}

.time {
  white-space: nowrap;
  letter-spacing: 0;
}

.transport {
  display: flex;
  align-items: center;
  gap: 0.2rem;
  justify-self: center;
}

.tools {
  display: flex;
  align-items: center;
  gap: 0.1rem;
  justify-self: end;
}

.glyph {
  display: grid;
  place-items: center;
  width: 2.4rem;
  height: 2.4rem;
  border-radius: var(--r-full);
  color: rgba(255, 255, 255, 0.9);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease),
    transform var(--t-fast) var(--ease);
}

.glyph:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

.glyph:active {
  transform: scale(0.9);
  transition-duration: 80ms;
}

.glyph.on {
  background: rgba(255, 255, 255, 0.18);
}

.glyph.play {
  width: 2.9rem;
  height: 2.9rem;
}

.glyph.text {
  width: auto;
  padding: 0 0.7rem;
  font-size: 0.8125rem;
  font-weight: 560;
}

/* ---------------- 音量 ---------------- */

.volume {
  display: flex;
  align-items: center;
}

.volume-slider {
  width: 0;
  overflow: hidden;
  opacity: 0;
  transition: width var(--t-base) var(--ease-out), opacity var(--t-fast) var(--ease);
}

.volume-slider.open {
  width: 5.5rem;
  opacity: 1;
}

.volume-slider input {
  width: 5.1rem;
  margin: 0 0.2rem;
  accent-color: #fff;
}

/* ---------------- 弹出菜单 ---------------- */

.menu-anchor {
  position: relative;
}

.menu {
  position: absolute;
  bottom: calc(100% + 0.6rem);
  right: 0;
  min-width: 7.5rem;
  padding: 0.3rem;
  border-radius: var(--r-md);
  background: var(--material-thick);
  backdrop-filter: var(--material-blur);
  border: 1px solid var(--separator-strong);
  box-shadow: var(--shadow-lg);
  /* 从触发它的按钮长出来 */
  transform-origin: bottom right;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  width: 100%;
  padding: 0.4rem 0.6rem;
  border-radius: var(--r-sm);
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.78);
  text-align: left;
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}

.menu-item:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.menu-item.active {
  color: #fff;
  font-weight: 570;
}

.tick {
  flex: none;
  opacity: 0;
  color: var(--accent);
}

.menu-item.active .tick {
  opacity: 1;
}

/* ---------------- 中央提示 ---------------- */

.center-hint,
.toast {
  position: absolute;
  left: 50%;
  z-index: 3;
  pointer-events: none;
}

.center-hint {
  top: 50%;
  translate: -50% -50%;
}

.spinner {
  display: block;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: #fff;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    rotate: 360deg;
  }
}

.paused-glyph {
  display: grid;
  place-items: center;
  width: 5rem;
  height: 5rem;
  border-radius: var(--r-full);
  background: rgba(0, 0, 0, 0.42);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.toast {
  bottom: 22%;
  translate: -50% 0;
  padding: 0.5rem 1.1rem;
  border-radius: var(--r-full);
  background: var(--material-thick);
  backdrop-filter: var(--material-blur);
  border: 1px solid var(--separator-strong);
  font-weight: 550;
  white-space: nowrap;
}

/* 材质出现时模糊和缩放一起动，像一块玻璃落下来 */
.pop-enter-active,
.pop-leave-active {
  transition: opacity var(--t-fast) var(--ease), transform var(--t-base) var(--ease-out);
}

.pop-enter-from,
.pop-leave-to {
  opacity: 0;
  transform: translateY(6px) scale(0.94);
}
</style>
