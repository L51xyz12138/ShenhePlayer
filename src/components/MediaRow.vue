<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import PosterCard from './PosterCard.vue'
import AppIcon from './AppIcon.vue'
import type { BaseItem } from '@/types'

const props = withDefaults(
  defineProps<{
    title: string
    items: BaseItem[]
    shape?: 'poster' | 'thumb'
    to?: { name: string; params?: Record<string, string> }
  }>(),
  { shape: 'poster' },
)

const emit = defineEmits<{ play: [BaseItem] }>()

const track = ref<HTMLElement>()
const atStart = ref(true)
const atEnd = ref(false)

function updateEdges() {
  const el = track.value
  if (!el) return
  atStart.value = el.scrollLeft <= 2
  atEnd.value = el.scrollLeft + el.clientWidth >= el.scrollWidth - 2
}

/** 一次翻一屏，留一点重叠让人知道是同一条 */
function page(dir: 1 | -1) {
  const el = track.value
  if (!el) return
  el.scrollBy({ left: dir * el.clientWidth * 0.85, behavior: 'smooth' })
}

onMounted(updateEdges)

const cardWidth = computed(() => (props.shape === 'thumb' ? '18rem' : '10.5rem'))
</script>

<template>
  <section v-if="items.length" class="media-row">
    <header class="head">
      <component :is="to ? 'RouterLink' : 'h2'" :to="to" class="t-title-3 heading">
        {{ title }}
        <AppIcon v-if="to" name="chevron-right" :size="16" class="chevron" />
      </component>

      <div class="arrows">
        <button class="arrow" :disabled="atStart" title="向左" @click="page(-1)">
          <AppIcon name="chevron-left" :size="16" />
        </button>
        <button class="arrow" :disabled="atEnd" title="向右" @click="page(1)">
          <AppIcon name="chevron-right" :size="16" />
        </button>
      </div>
    </header>

    <div
      ref="track"
      class="track"
      :style="{ '--w': cardWidth }"
      @scroll.passive="updateEdges"
    >
      <PosterCard
        v-for="(item, i) in items"
        :key="item.id"
        :item="item"
        :shape="shape"
        :eager="i < 7"
        class="slot"
        @play="emit('play', $event)"
      />
    </div>
  </section>
</template>

<style scoped>
.media-row {
  /* 滚出视口的行不参与渲染，长首页在低端机上也不掉帧 */
  content-visibility: auto;
  contain-intrinsic-size: auto 340px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  padding: 0 var(--page-pad) 0.75rem;
}

.heading {
  display: inline-flex;
  align-items: center;
  gap: 0.1rem;
  color: var(--label);
}

a.heading:hover {
  color: var(--label-2);
}

.chevron {
  color: var(--label-3);
  opacity: 0;
  translate: -4px 0;
  transition: opacity var(--t-fast) var(--ease), translate var(--t-fast) var(--ease);
}

a.heading:hover .chevron {
  opacity: 1;
  translate: 0 0;
}

.arrows {
  display: flex;
  gap: 0.3rem;
  opacity: 0;
  transition: opacity var(--t-base) var(--ease);
}

.media-row:hover .arrows {
  opacity: 1;
}

.arrow {
  display: grid;
  place-items: center;
  width: 1.9rem;
  height: 1.9rem;
  border-radius: var(--r-full);
  background: var(--fill-1);
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), transform var(--t-fast) var(--ease);
}

.arrow:hover:not(:disabled) {
  background: var(--fill-3);
  color: var(--label);
}

.arrow:active:not(:disabled) {
  transform: scale(0.9);
}

.arrow:disabled {
  opacity: 0.25;
}

.track {
  display: flex;
  gap: 1.1rem;
  /* 上下留白给卡片 hover 放大的余量，否则会被裁掉 */
  padding: 0.5rem var(--page-pad) 0.75rem;
  overflow-x: auto;
  overflow-y: hidden;
  scroll-snap-type: x proximity;
  scrollbar-width: none;
}

.track::-webkit-scrollbar {
  display: none;
}

.slot {
  width: var(--w);
  flex: none;
  scroll-snap-align: start;
}
</style>
