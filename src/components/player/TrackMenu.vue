<script setup lang="ts">
import AppIcon from '../AppIcon.vue'
import type { TrackInfo } from '@/types'

defineProps<{
  icon: string
  label: string
  tracks: TrackInfo[]
  current: number
  /** 字幕可以关掉，音轨不行 */
  allowOff?: boolean
}>()

/** 菜单是点击打开的，所以开合由父组件掌握，不用 hover 控制 */
const open = defineModel<boolean>('open', { default: false })

const emit = defineEmits<{ select: [number] }>()

const LANGS: Record<string, string> = {
  chi: '中文',
  zho: '中文',
  chs: '简体中文',
  cht: '繁体中文',
  eng: '英语',
  jpn: '日语',
  kor: '韩语',
  fre: '法语',
  fra: '法语',
  ger: '德语',
  deu: '德语',
  spa: '西班牙语',
  rus: '俄语',
  ita: '意大利语',
}

const langName = (lang: string) => (lang ? (LANGS[lang.toLowerCase()] ?? lang) : '')

function trackName(t: TrackInfo): string {
  const parts = [t.title, langName(t.lang), t.codec?.toUpperCase(), t.detail].filter(Boolean)
  return parts.join(' · ') || `轨道 ${t.id}`
}

function choose(id: number) {
  emit('select', id)
  open.value = false
}
</script>

<template>
  <div class="track-menu">
    <button class="glyph" :class="{ on: open }" :title="label" @click="open = !open">
      <AppIcon :name="icon" :size="20" />
    </button>

    <Transition name="pop">
      <div v-if="open" class="menu">
        <span class="bridge" />
        <div class="menu-title t-caption-2">{{ label }}</div>

        <button
          v-if="allowOff"
          class="menu-item"
          :class="{ active: current < 0 }"
          @click="choose(-1)"
        >
          <AppIcon name="check" :size="14" class="tick" />
          <span>关闭</span>
        </button>

        <button
          v-for="t in tracks"
          :key="t.id"
          class="menu-item"
          :class="{ active: t.id === current }"
          @click="choose(t.id)"
        >
          <AppIcon name="check" :size="14" class="tick" />
          <span class="truncate">{{ trackName(t) }}</span>
          <em v-if="t.external">外挂</em>
        </button>

        <div v-if="!tracks.length" class="empty t-footnote">没有可用轨道</div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.track-menu {
  position: relative;
}

/* 和播放窗口里其它控制按钮保持一致。scoped 样式不会从父组件穿透到子组件
   内部，所以这份按钮样式必须写在这里，不能指望 PlayerRoot 的 .glyph。 */
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

.menu {
  position: absolute;
  bottom: calc(100% + 0.6rem);
  right: 0;
  min-width: 13rem;
  max-width: 22rem;
  max-height: 19rem;
  overflow-y: auto;
  padding: 0.3rem;
  border-radius: var(--r-md);
  background: var(--material-thick);
  backdrop-filter: var(--material-blur);
  border: 1px solid var(--separator-strong);
  box-shadow: var(--shadow-lg);
  /* 从触发它的按钮长出来，保持空间关联 */
  transform-origin: bottom right;
}

/* 补上按钮与菜单之间的空隙，鼠标划过去不会落到视频上 */
.bridge {
  position: absolute;
  left: 0;
  right: 0;
  top: 100%;
  height: 0.6rem;
}

.menu-title {
  padding: 0.35rem 0.6rem 0.4rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.4);
  text-transform: uppercase;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  width: 100%;
  padding: 0.42rem 0.6rem;
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
  font-weight: 560;
}

.menu-item span {
  flex: 1;
  min-width: 0;
}

.menu-item em {
  flex: none;
  font-style: normal;
  font-size: 0.6875rem;
  padding: 0.05rem 0.32rem;
  border-radius: var(--r-xs);
  background: rgba(255, 255, 255, 0.14);
  color: rgba(255, 255, 255, 0.6);
}

.tick {
  flex: none;
  opacity: 0;
  color: var(--accent);
}

.menu-item.active .tick {
  opacity: 1;
}

.empty {
  padding: 0.55rem 0.6rem;
  color: rgba(255, 255, 255, 0.4);
}

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
