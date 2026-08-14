<script setup lang="ts">
// 图片进视口才加载 + 淡入。海报墙动辄几百张图，直接全量加载会把
// 低端机的内存和解码线程打满。
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    src: string
    alt?: string
    /** 2:3 海报 / 16:9 横图 */
    ratio?: string
    rounded?: string
    eager?: boolean
  }>(),
  { alt: '', ratio: '2 / 3', rounded: 'var(--r-md)', eager: false },
)

const el = ref<HTMLElement>()
const loaded = ref(false)
const failed = ref(false)
const visible = ref(props.eager)

let observer: IntersectionObserver | undefined

onMounted(() => {
  if (visible.value || !el.value) return
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        visible.value = true
        observer?.disconnect()
      }
    },
    // 提前一屏开始加载，滚动时不会看到空位
    { rootMargin: '400px 0px' },
  )
  observer.observe(el.value)
})

onBeforeUnmount(() => observer?.disconnect())

watch(
  () => props.src,
  () => {
    loaded.value = false
    failed.value = false
  },
)
</script>

<template>
  <div
    ref="el"
    class="lazy"
    :style="{ aspectRatio: ratio, borderRadius: rounded }"
    :class="{ loaded }"
  >
    <img
      v-if="visible && src && !failed"
      :src="src"
      :alt="alt"
      loading="lazy"
      decoding="async"
      @load="loaded = true"
      @error="failed = true"
    />
    <div v-if="!loaded" class="placeholder">
      <slot name="fallback" />
    </div>
  </div>
</template>

<style scoped>
.lazy {
  position: relative;
  overflow: hidden;
  background: var(--bg-1);
  isolation: isolate;
}

img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity var(--t-slow) var(--ease);
}

.loaded img {
  opacity: 1;
}

.placeholder {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  color: var(--label-3);
  background: linear-gradient(135deg, var(--bg-2), var(--bg-1));
}

/* 骨架微光：只在加载中出现，加载完立刻停掉，不留常驻动画 */
.placeholder::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    100deg,
    transparent 30%,
    rgba(255, 255, 255, 0.045) 50%,
    transparent 70%
  );
  background-size: 220% 100%;
  animation: shimmer 1.5s linear infinite;
}

@keyframes shimmer {
  from {
    background-position: 180% 0;
  }
  to {
    background-position: -80% 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .placeholder::after {
    animation: none;
  }
}
</style>
