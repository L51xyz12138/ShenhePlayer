import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import * as api from '@/api'
import type { BaseItem, PlaybackTarget, PlayerSnapshot } from '@/types'
import { useSettingsStore } from './settings'

const EMPTY: PlayerSnapshot = {
  active: false,
  paused: false,
  idle: false,
  buffering: false,
  seeking: false,
  position: 0,
  duration: 0,
  cacheTime: 0,
  volume: 100,
  muted: false,
  speed: 1,
  width: 0,
  height: 0,
  fps: 0,
  hwdec: '',
  videoCodec: '',
  audioCodec: '',
  tracks: [],
  audioTrack: -1,
  subTrack: -1,
  fileLoaded: false,
}

export const usePlayerStore = defineStore('player', () => {
  const target = ref<PlaybackTarget | null>(null)
  const snapshot = ref<PlayerSnapshot>({ ...EMPTY })
  const playing = ref(false)
  const starting = ref(false)
  const error = ref('')
  /** 外置播放器正在放的条目。外部进程结束我们收不到通知，需要用户回报。 */
  const externalItem = ref<BaseItem | null>(null)
  /** 用户拖动进度条时，UI 用这个值，不被后端推送覆盖 */
  const scrubPosition = ref<number | null>(null)
  /** 播放队列里的下一集 */
  const nextItem = ref<BaseItem | null>(null)

  const position = computed(() => scrubPosition.value ?? snapshot.value.position)
  const duration = computed(() => snapshot.value.duration || target.value?.duration || 0)
  const progress = computed(() =>
    duration.value > 0 ? Math.min(position.value / duration.value, 1) : 0,
  )
  const buffered = computed(() =>
    duration.value > 0 ? Math.min(snapshot.value.cacheTime / duration.value, 1) : 0,
  )

  const audioTracks = computed(() => snapshot.value.tracks.filter((t) => t.type === 'audio'))
  const subTracks = computed(() => snapshot.value.tracks.filter((t) => t.type === 'sub'))

  // ------------------------------------------------------------ 播放入口

  /** 从媒体条目开始播放。resume=true 时接着上次位置。 */
  async function play(item: BaseItem, resume = true) {
    const settings = useSettingsStore()
    starting.value = true
    error.value = ''
    try {
      const t = await api.preparePlayback(item.id, resume)
      target.value = t

      if (settings.player.mode === 'external') {
        await api.startExternal()
        playing.value = false
        externalItem.value = item
      } else {
        await api.startInternal()
        playing.value = true
        externalItem.value = null
      }

      void prefetchNext(item)
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      starting.value = false
    }
  }

  async function stop() {
    await api.stopPlayback()
    playing.value = false
    target.value = null
    nextItem.value = null
    snapshot.value = { ...EMPTY }
    scrubPosition.value = null
    if (await api.isFullscreen()) {
      await api.setFullscreen(false)
    }
  }

  /** 提前查出下一集，播完能无缝续上 */
  async function prefetchNext(item: BaseItem) {
    nextItem.value = null
    if (item.type !== 'Episode' || !item.seriesId) return
    try {
      const episodes = await api.getEpisodes(item.seriesId, item.seasonId ?? undefined)
      const idx = episodes.findIndex((e) => e.id === item.id)
      if (idx >= 0 && idx + 1 < episodes.length) {
        nextItem.value = episodes[idx + 1]
      }
    } catch {
      // 拿不到下一集不影响当前播放
    }
  }

  async function playNext() {
    const next = nextItem.value
    if (!next) return
    await play(next, false)
  }

  /** 外置播放结束后把进度写回 Emby。position 未知时按「看完」处理。 */
  async function finishExternal(finished: boolean) {
    const item = externalItem.value
    const position = finished ? 0 : (target.value?.startPosition ?? 0)
    externalItem.value = null
    target.value = null
    if (!item) return
    await api.reportExternalProgress(position, finished)
  }

  // ------------------------------------------------------------ 控制

  const togglePause = () => api.playerSetPause(!snapshot.value.paused)
  const setPause = (paused: boolean) => api.playerSetPause(paused)
  const seek = (pos: number) => api.playerSeek(pos)
  const seekRelative = (delta: number) => api.playerSeekRelative(delta)
  const setSpeed = (speed: number) => api.playerSetSpeed(speed)
  const setTrack = (kind: 'audio' | 'sub', id: number) => api.playerSetTrack(kind, id)
  const toggleMuted = () => api.playerSetMuted(!snapshot.value.muted)

  async function setVolume(v: number) {
    const clamped = Math.max(0, Math.min(150, v))
    snapshot.value.volume = clamped
    await api.playerSetVolume(clamped)
  }

  /** 拖动进度条：先本地跟手，松开才真正 seek */
  function beginScrub(pos: number) {
    scrubPosition.value = pos
  }

  function updateScrub(pos: number) {
    scrubPosition.value = pos
  }

  async function endScrub() {
    const pos = scrubPosition.value
    scrubPosition.value = null
    if (pos != null) await seek(pos)
  }

  // ------------------------------------------------------------ 后端事件

  let bound = false

  /** 浏览窗口和播放窗口都要调用：Tauri 的 emit 是广播给所有窗口的 */
  async function bind() {
    if (bound) return
    bound = true

    const isPlayerWindow = getCurrentWindow().label === 'player'

    await listen<PlayerSnapshot>('player:state', (e) => {
      snapshot.value = e.payload
      if (e.payload.active) playing.value = true
    })

    await listen('player:loaded', async () => {
      playing.value = true
      // 播放窗口是独立的 JS 环境，要自己去后端取当前播放的元信息
      target.value = await api.currentTarget()
    })

    await listen<string>('player:endfile', async (e) => {
      // 只让浏览窗口决定「播完之后干什么」，避免两个窗口重复触发
      if (isPlayerWindow) return
      if (e.payload !== 'eof') return

      const settings = useSettingsStore()
      if (settings.player.autoNext && nextItem.value) {
        await playNext()
      } else {
        await stop()
      }
    })

    await listen('player:closed', () => {
      playing.value = false
      target.value = null
      snapshot.value = { ...EMPTY }
    })

    if (isPlayerWindow) {
      target.value = await api.currentTarget()
      snapshot.value = await api.playerSnapshot()
      playing.value = snapshot.value.active
    }
  }

  return {
    target,
    snapshot,
    playing,
    starting,
    error,
    nextItem,
    externalItem,
    finishExternal,
    position,
    duration,
    progress,
    buffered,
    audioTracks,
    subTracks,
    play,
    playNext,
    stop,
    togglePause,
    setPause,
    seek,
    seekRelative,
    setSpeed,
    setTrack,
    setVolume,
    toggleMuted,
    beginScrub,
    updateScrub,
    endScrub,
    bind,
  }
})
