<script setup lang="ts">
// 进度条：拖动时 1:1 跟手，松手才真正 seek。
// 用 Pointer Events + setPointerCapture，指针移出条外依然跟随。
import { computed, ref } from 'vue'
import { formatTime } from '@/utils/format'

const props = defineProps<{
  position: number
  duration: number
  buffered: number
}>()

const emit = defineEmits<{
  scrubStart: [number]
  scrubMove: [number]
  scrubEnd: []
  seek: [number]
}>()

const bar = ref<HTMLElement>()
const dragging = ref(false)
const hovering = ref(false)
const hoverRatio = ref(0)

const playedRatio = computed(() =>
  props.duration > 0 ? Math.min(props.position / props.duration, 1) : 0,
)

function ratioFromEvent(e: PointerEvent | MouseEvent): number {
  const el = bar.value
  if (!el) return 0
  const rect = el.getBoundingClientRect()
  return Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
}

function onPointerDown(e: PointerEvent) {
  if (e.button !== 0) return
  const el = e.currentTarget as HTMLElement
  el.setPointerCapture(e.pointerId)
  dragging.value = true
  emit('scrubStart', ratioFromEvent(e) * props.duration)
}

function onPointerMove(e: PointerEvent) {
  hoverRatio.value = ratioFromEvent(e)
  if (!dragging.value) return
  emit('scrubMove', hoverRatio.value * props.duration)
}

function onPointerUp(e: PointerEvent) {
  if (!dragging.value) return
  const el = e.currentTarget as HTMLElement
  el.releasePointerCapture(e.pointerId)
  dragging.value = false
  emit('scrubEnd')
}

const hoverTime = computed(() => formatTime(hoverRatio.value * props.duration))
</script>

<template>
  <div
    ref="bar"
    class="seek"
    :class="{ active: dragging || hovering }"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
    @pointerenter="hovering = true"
    @pointerleave="hovering = false"
  >
    <div class="rail">
      <div class="buffered" :style="{ transform: `scaleX(${buffered})` }" />
      <div class="played" :style="{ transform: `scaleX(${playedRatio})` }" />
      <div class="knob" :style="{ left: `${playedRatio * 100}%` }" />
    </div>

    <div
      v-if="hovering || dragging"
      class="tip mono"
      :style="{ left: `${hoverRatio * 100}%` }"
    >
      {{ hoverTime }}
    </div>
  </div>
</template>

<style scoped>
.seek {
  position: relative;
  /* 上下留出热区，鼠标不用精确对准 2px 的细条 */
  padding: 10px 0;
  cursor: pointer;
  touch-action: none;
}

.rail {
  position: relative;
  height: 5px;
  border-radius: var(--r-full);
  background: rgba(255, 255, 255, 0.2);
  transition: height var(--t-base) var(--ease-out);
}

/* 悬停/拖动时轨道变粗，是「可以抓住它」的提示 */
.seek.active .rail {
  height: 8px;
}

.buffered,
.played {
  position: absolute;
  inset: 0;
  transform-origin: left center;
  border-radius: inherit;
  /* 用 scaleX 而不是 width：只走合成层，拖动时不触发重排 */
  will-change: transform;
}

.buffered {
  background: rgba(255, 255, 255, 0.24);
}

/* 已播放部分用白色而不是强调色：视频之上白色永远清晰，
   强调色遇到同色系画面就糊了 */
.played {
  background: #fff;
}

.knob {
  position: absolute;
  top: 50%;
  width: 14px;
  height: 14px;
  margin-left: -7px;
  border-radius: var(--r-full);
  background: #fff;
  box-shadow: 0 1px 6px rgba(0, 0, 0, 0.5);
  translate: 0 -50%;
  scale: 0;
  transition: scale var(--t-base) var(--ease-out);
}

.seek.active .knob {
  scale: 1;
}

.tip {
  position: absolute;
  bottom: calc(100% + 2px);
  translate: -50% 0;
  padding: 0.22rem 0.5rem;
  border-radius: var(--r-sm);
  background: var(--material-thick);
  backdrop-filter: var(--material-blur);
  border: 1px solid var(--separator-strong);
  font-size: 0.75rem;
  font-variant-numeric: tabular-nums;
  color: #fff;
  pointer-events: none;
  white-space: nowrap;
}
</style>
