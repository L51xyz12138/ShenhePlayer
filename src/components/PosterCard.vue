<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import LazyImage from './LazyImage.vue'
import AppIcon from './AppIcon.vue'
import { posterUrl, thumbUrl } from '@/api/images'
import { displaySubtitle, displayTitle, playedFraction } from '@/utils/format'
import type { BaseItem } from '@/types'

const props = withDefaults(
  defineProps<{
    item: BaseItem
    /** poster = 2:3 竖版，thumb = 16:9 横版 */
    shape?: 'poster' | 'thumb'
    eager?: boolean
  }>(),
  { shape: 'poster', eager: false },
)

const emit = defineEmits<{ play: [BaseItem] }>()
const router = useRouter()

const isThumb = computed(() => props.shape === 'thumb')
const src = computed(() =>
  isThumb.value ? thumbUrl(props.item, 500) : posterUrl(props.item, 480),
)
const progress = computed(() => playedFraction(props.item))
const watched = computed(() => props.item.userData?.played ?? false)
const unplayed = computed(() => props.item.userData?.unplayedItemCount ?? 0)

function open() {
  router.push({ name: 'item', params: { id: props.item.id } })
}
</script>

<template>
  <article class="poster-card" :class="shape" @click="open">
    <div class="art">
      <LazyImage
        :src="src"
        :alt="item.name"
        :ratio="isThumb ? '16 / 9' : '2 / 3'"
        :eager="eager"
        rounded="var(--r-md)"
      >
        <template #fallback>
          <AppIcon :name="isThumb ? 'tv' : 'film'" :size="24" />
        </template>
      </LazyImage>

      <div class="veil">
        <button class="play" title="播放" @click.stop="emit('play', item)">
          <AppIcon name="play" :size="19" filled />
        </button>
      </div>

      <span v-if="watched" class="tag done" title="已看完">
        <AppIcon name="check" :size="11" :stroke="2.8" />
      </span>
      <span v-else-if="unplayed > 0" class="tag num">{{ unplayed }}</span>

      <div v-if="progress > 0 && progress < 1" class="bar">
        <span :style="{ transform: `scaleX(${progress})` }" />
      </div>
    </div>

    <div class="meta">
      <div class="truncate t-footnote name">{{ displayTitle(item) }}</div>
      <div class="truncate t-caption dim-3">{{ displaySubtitle(item) }}</div>
    </div>
  </article>
</template>

<style scoped>
.poster-card {
  min-width: 0;
}

.art {
  position: relative;
  border-radius: var(--r-md);
  overflow: hidden;
  /* 只有 transform 参与动画，走合成层，滚动海报墙不掉帧 */
  transition: transform var(--t-base) var(--ease-out),
    box-shadow var(--t-base) var(--ease-out);
  will-change: transform;
}

.poster-card:hover .art {
  transform: scale(1.04);
  box-shadow: var(--shadow-md);
}

/* 按下立刻给反馈，不等 click */
.poster-card:active .art {
  transform: scale(1.005);
  transition-duration: 80ms;
}

.veil {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.36);
  opacity: 0;
  transition: opacity var(--t-base) var(--ease);
}

.poster-card:hover .veil {
  opacity: 1;
}

.play {
  display: grid;
  place-items: center;
  width: 2.75rem;
  height: 2.75rem;
  padding-left: 2px;
  border-radius: var(--r-full);
  background: rgba(255, 255, 255, 0.94);
  color: #000;
  backdrop-filter: blur(10px);
  transform: scale(0.8);
  transition: transform var(--t-base) var(--ease-out), background var(--t-fast) var(--ease);
}

.poster-card:hover .play {
  transform: scale(1);
}

.play:hover {
  background: #fff;
}

.play:active {
  transform: scale(0.9);
  transition-duration: 80ms;
}

.tag {
  position: absolute;
  top: 0.4rem;
  right: 0.4rem;
  display: grid;
  place-items: center;
  min-width: 1.15rem;
  height: 1.15rem;
  padding: 0 0.3rem;
  border-radius: var(--r-full);
  font-size: 0.6875rem;
  font-weight: 640;
  color: #fff;
  background: var(--accent);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
}

.tag.done {
  background: var(--green);
  color: #000;
}

.bar {
  position: absolute;
  left: 0.4rem;
  right: 0.4rem;
  bottom: 0.4rem;
  height: 3px;
  border-radius: var(--r-full);
  background: rgba(255, 255, 255, 0.28);
  overflow: hidden;
}

.bar span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: #fff;
  transform-origin: left center;
}

.meta {
  padding: 0.5rem 0.1rem 0;
}

.name {
  font-weight: 550;
  letter-spacing: -0.002em;
}
</style>
