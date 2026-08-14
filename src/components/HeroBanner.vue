<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import AppIcon from './AppIcon.vue'
import { backdropUrl, logoUrl } from '@/api/images'
import {
  displaySubtitle,
  displayTitle,
  formatDuration,
  formatRating,
  isResumable,
} from '@/utils/format'
import type { BaseItem } from '@/types'

const props = defineProps<{ items: BaseItem[] }>()
const emit = defineEmits<{ play: [BaseItem] }>()

const router = useRouter()
const index = ref(0)
const current = computed(() => props.items[index.value])
const backdrops = computed(() => props.items.map((i) => backdropUrl(i, 1920)))

let timer: number | undefined

function start() {
  stop()
  if (props.items.length <= 1) return
  timer = window.setInterval(() => {
    index.value = (index.value + 1) % props.items.length
  }, 9000)
}

function stop() {
  if (timer) window.clearInterval(timer)
  timer = undefined
}

function goto(i: number) {
  index.value = i
  start()
}

onMounted(start)
onBeforeUnmount(stop)
watch(() => props.items, start)

const action = computed(() =>
  current.value && isResumable(current.value) ? '继续观看' : '播放',
)
</script>

<template>
  <section v-if="current" class="hero" @mouseenter="stop" @mouseleave="start">
    <div class="art">
      <div
        v-for="(url, i) in backdrops"
        :key="i"
        class="shot"
        :class="{ on: i === index }"
        :style="url ? { backgroundImage: `url(${url})` } : undefined"
      />
    </div>
    <div class="scrim" />

    <Transition name="hero" mode="out-in">
      <div :key="current.id" class="info">
        <img
          v-if="logoUrl(current)"
          :src="logoUrl(current)"
          :alt="displayTitle(current)"
          class="logo"
        />
        <h1 v-else class="t-large-title">{{ displayTitle(current) }}</h1>

        <div class="facts t-footnote dim">
          <span v-if="current.productionYear">{{ current.productionYear }}</span>
          <span v-if="formatRating(current.communityRating)" class="rating">
            <AppIcon name="star" :size="11" filled />
            {{ formatRating(current.communityRating) }}
          </span>
          <span v-if="formatDuration(current.runTimeTicks)">
            {{ formatDuration(current.runTimeTicks) }}
          </span>
          <span v-if="current.officialRating" class="chip">{{ current.officialRating }}</span>
          <span v-if="current.type === 'Episode'">{{ displaySubtitle(current) }}</span>
        </div>

        <p v-if="current.overview" class="overview t-subhead dim clamp-2">
          {{ current.overview }}
        </p>

        <div class="actions">
          <button class="btn btn-white btn-lg" @click="emit('play', current)">
            <AppIcon name="play" :size="16" filled />
            {{ action }}
          </button>
          <button
            class="btn btn-lg glass"
            @click="router.push({ name: 'item', params: { id: current.id } })"
          >
            详情
          </button>
        </div>
      </div>
    </Transition>

    <div v-if="items.length > 1" class="dots">
      <button
        v-for="(item, i) in items"
        :key="item.id"
        class="dot"
        :class="{ on: i === index }"
        :aria-label="`第 ${i + 1} 项`"
        @click="goto(i)"
      />
    </div>
  </section>
</template>

<style scoped>
.hero {
  position: relative;
  min-height: 27rem;
  display: flex;
  align-items: flex-end;
  padding: 5rem var(--page-pad) var(--sp-6);
  overflow: hidden;
}

.art,
.shot,
.scrim {
  position: absolute;
  inset: 0;
}

.shot {
  background-size: cover;
  background-position: center 18%;
  opacity: 0;
  /* 大面积图片切换要慢，避免亮度突变 */
  transition: opacity 900ms var(--ease);
}

.shot.on {
  opacity: 1;
}

/* 大图是照片表面，上面的文字永远是白的，所以遮罩必须一直是深色。
   暗色主题下底部淡入页面背景做无缝衔接；亮色主题下不能淡入白色，
   否则白字直接消失，改成保持深色、用一条硬边收住。 */
.scrim {
  background:
    linear-gradient(to top, var(--bg) 1%, rgba(0, 0, 0, 0.5) 40%, transparent 88%),
    linear-gradient(to right, rgba(0, 0, 0, 0.85) 0%, rgba(0, 0, 0, 0.2) 58%, transparent 100%);
}

:root[data-theme='light'] .scrim {
  background:
    linear-gradient(to top, rgba(0, 0, 0, 0.82) 0%, rgba(0, 0, 0, 0.42) 45%, rgba(0, 0, 0, 0.1) 100%),
    linear-gradient(to right, rgba(0, 0, 0, 0.7) 0%, rgba(0, 0, 0, 0.2) 60%, transparent 100%);
}

/* 大图上的文字永远是浅色系：.dim / .dim-3 会跟着主题翻转成深灰，
   叠在照片上就看不清了，这里在组件内固定住。 */
.hero {
  color: #fff;
}

.hero :deep(.dim),
.hero .dim {
  color: rgba(235, 235, 245, 0.72);
}

.hero :deep(.dim-3),
.hero .dim-3 {
  color: rgba(235, 235, 245, 0.5);
}

.info {
  position: relative;
  max-width: 38rem;
}

.logo {
  max-width: 20rem;
  max-height: 6.5rem;
  object-fit: contain;
  object-position: left bottom;
  margin-bottom: 0.85rem;
  filter: drop-shadow(0 3px 14px rgba(0, 0, 0, 0.6));
}

.facts {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.8rem;
  margin: 0.7rem 0;
}

.rating {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  color: var(--yellow);
}

.chip {
  padding: 0.05rem 0.4rem;
  border: 1px solid var(--separator-strong);
  border-radius: var(--r-xs);
  font-size: 0.6875rem;
}

.overview {
  margin: 0 0 1.35rem;
  max-width: 34rem;
}

.actions {
  display: flex;
  gap: 0.65rem;
}

/* 视频/图片上的次级按钮用材质，而不是实色，才不会压住画面 */
.glass {
  background: rgba(255, 255, 255, 0.16);
  backdrop-filter: blur(20px);
  color: #fff;
}

.glass:hover {
  background: rgba(255, 255, 255, 0.24);
}

.dots {
  position: absolute;
  right: var(--page-pad);
  bottom: var(--sp-6);
  display: flex;
  gap: 0.35rem;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: var(--r-full);
  background: rgba(255, 255, 255, 0.3);
  transition: width var(--t-base) var(--ease-out), background var(--t-base) var(--ease);
}

.dot.on {
  width: 18px;
  background: #fff;
}

.dot:hover {
  background: rgba(255, 255, 255, 0.6);
}

/* 文案进出走同一条路径 */
.hero-enter-active,
.hero-leave-active {
  transition: opacity var(--t-base) var(--ease), transform var(--t-base) var(--ease);
}

.hero-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.hero-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
