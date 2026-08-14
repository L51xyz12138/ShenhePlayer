<script setup lang="ts">
import { computed } from 'vue'
import LazyImage from './LazyImage.vue'
import AppIcon from './AppIcon.vue'
import { thumbUrl } from '@/api/images'
import { formatDuration, playedFraction } from '@/utils/format'
import type { BaseItem } from '@/types'

const props = defineProps<{ item: BaseItem }>()
const emit = defineEmits<{ play: [BaseItem] }>()

const progress = computed(() => playedFraction(props.item))
const watched = computed(() => props.item.userData?.played ?? false)
</script>

<template>
  <article class="episode" @click="emit('play', item)">
    <div class="thumb">
      <LazyImage :src="thumbUrl(item, 480)" :alt="item.name" ratio="16 / 9">
        <template #fallback><AppIcon name="tv" :size="20" /></template>
      </LazyImage>

      <div class="play-layer">
        <AppIcon name="play" :size="18" filled />
      </div>

      <div v-if="progress > 0 && progress < 1" class="progress">
        <span :style="{ width: `${progress * 100}%` }" />
      </div>
    </div>

    <div class="body">
      <div class="line">
        <span class="index">{{ item.indexNumber ?? '·' }}</span>
        <h3 class="name truncate">{{ item.name }}</h3>
        <AppIcon v-if="watched" name="check-circle" :size="15" class="done" />
        <span class="duration">{{ formatDuration(item.runTimeTicks) }}</span>
      </div>
      <p v-if="item.overview" class="overview clamp-2">{{ item.overview }}</p>
    </div>
  </article>
</template>

<style scoped>
.episode {
  display: flex;
  gap: 1rem;
  padding: 0.6rem;
  border-radius: var(--r-md);
  border: 1px solid transparent;
  cursor: pointer;
  transition: background var(--t-fast) var(--ease), border-color var(--t-fast) var(--ease),
    transform var(--t-fast) var(--ease);
}

.episode:hover {
  background: var(--fill-1);
  border-color: var(--separator);
}

.episode:active {
  transform: scale(0.995);
  transition-duration: 90ms;
}

.thumb {
  position: relative;
  width: 11rem;
  flex: none;
  border-radius: var(--r-sm);
  overflow: hidden;
}

.play-layer {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.5);
  color: #fff;
  opacity: 0;
  transition: opacity var(--t-fast) var(--ease);
}

.episode:hover .play-layer {
  opacity: 1;
}

.progress {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 3px;
  background: rgba(255, 255, 255, 0.22);
}

.progress span {
  display: block;
  height: 100%;
  background: var(--accent);
}

.body {
  flex: 1;
  min-width: 0;
  padding-top: 0.15rem;
}

.line {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.index {
  flex: none;
  min-width: 1.5rem;
  font-size: 0.8125rem;
  font-weight: 640;
  font-variant-numeric: tabular-nums;
  color: var(--label-3);
}

.name {
  flex: 1;
  min-width: 0;
  font-size: 0.9375rem;
  font-weight: 570;
  margin: 0;
}

.done {
  flex: none;
  color: var(--green);
}

.duration {
  flex: none;
  font-size: 0.75rem;
  color: var(--label-3);
}

.overview {
  margin: 0.3rem 0 0 2.1rem;
  font-size: 0.8125rem;
  line-height: 1.5;
  color: var(--label-3);
}
</style>
