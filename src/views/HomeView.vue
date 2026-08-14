<script setup lang="ts">
import { ref, watch } from 'vue'
import * as api from '@/api'
import HeroBanner from '@/components/HeroBanner.vue'
import MediaRow from '@/components/MediaRow.vue'
import AppIcon from '@/components/AppIcon.vue'
import { usePlayerStore } from '@/stores/player'
import { useSessionStore } from '@/stores/session'
import type { BaseItem, HomeData } from '@/types'

const player = usePlayerStore()
const session = useSessionStore()
const data = ref<HomeData | null>(null)
const loading = ref(true)
const error = ref('')

async function load() {
  if (!session.isAuthed) {
    loading.value = false
    return
  }
  loading.value = true
  error.value = ''
  try {
    data.value = await api.getHome()
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

// 首页比 App 的会话恢复先挂载，所以不能只在 onMounted 里加载一次：
// 等 session 就绪（或切换服务器）后要重新拉数据
watch(() => session.isAuthed, (ok) => { if (ok) void load() }, { immediate: true })

async function play(item: BaseItem) {
  try {
    await player.play(item, true)
  } catch {
    // 错误已经进 player.error，播放页会展示
  }
}

// 「继续观看」用横版缩略图，能看到剧情画面；其余用竖版海报
const shapeFor = (kind: string) => (kind === 'resume' || kind === 'nextup' ? 'thumb' : 'poster')
</script>

<template>
  <div class="home">
    <!-- 会话恢复和首页数据都算加载中，避免先闪一下「未连接」再闪骨架 -->
    <div v-if="session.restoring || loading" class="skeleton">
      <div class="sk-hero">
        <div class="sk-hero-text">
          <div class="sk-line title" />
          <div class="sk-line meta" />
          <div class="sk-line desc" />
          <div class="sk-line desc short" />
          <div class="sk-buttons">
            <div class="sk-btn" />
            <div class="sk-btn narrow" />
          </div>
        </div>
      </div>

      <div v-for="n in 2" :key="n" class="sk-row">
        <div class="sk-line heading" />
        <div class="sk-cards">
          <div v-for="c in 8" :key="c" class="sk-card" :class="{ thumb: n === 1 }" />
        </div>
      </div>
    </div>

    <!-- 没连服务器也能进来，这里给一条明确的去路 -->
    <div v-else-if="!session.isAuthed" class="state">
      <AppIcon name="server" :size="30" />
      <p class="t-title-3">未连接服务器</p>
      <p class="t-footnote dim">连接一台 Emby 服务器后，这里会显示你的媒体库</p>
      <RouterLink :to="{ name: 'servers' }" class="btn btn-primary">选择服务器</RouterLink>
    </div>

    <div v-else-if="error" class="state">
      <AppIcon name="info" :size="30" />
      <p class="t-title-3">加载失败</p>
      <p class="t-caption">{{ error }}</p>
      <button class="btn" @click="load">
        <AppIcon name="refresh" :size="16" />
        重试
      </button>
    </div>

    <template v-else-if="data">
      <HeroBanner v-if="data.hero.length" :items="data.hero" @play="play" />

      <div v-else class="welcome">
        <h1 class="t-large-title">欢迎回来</h1>
        <p class="t-caption">从左侧选一个媒体库开始</p>
      </div>

      <div class="rows">
        <MediaRow
          v-for="section in data.sections"
          :key="section.id"
          :title="section.title"
          :items="section.items"
          :shape="shapeFor(section.kind)"
          :to="
            section.parentId ? { name: 'library', params: { id: section.parentId } } : undefined
          "
          @play="play"
        />
      </div>

      <div v-if="!data.sections.length" class="state">
        <AppIcon name="sparkles" :size="30" />
        <p class="t-title-3">媒体库还是空的</p>
        <p class="t-caption">在 Emby 服务器里添加媒体后回来刷新</p>
        <button class="btn" @click="load">
          <AppIcon name="refresh" :size="16" />
          刷新
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.home {
  padding-bottom: 3rem;
}

.rows {
  display: flex;
  flex-direction: column;
  gap: 1.9rem;
  padding-top: 1.9rem;
}

.welcome {
  padding: 3.5rem var(--page-pad) 1rem;
}

.welcome p {
  margin-top: 0.5rem;
}

.state {
  display: grid;
  place-items: center;
  gap: 0.55rem;
  padding: 5rem 2rem;
  color: var(--label-3);
  text-align: center;
}

.state p {
  margin: 0;
}

.state .btn {
  margin-top: 0.9rem;
}

/* ---- 骨架屏 ----
 * 结构照着真实布局摆，加载完不跳版；微光只在加载时跑，出内容立刻停。
 */
.skeleton {
  --sk: var(--fill-1);
}

.sk-line,
.sk-card,
.sk-btn {
  background: var(--sk);
  border-radius: var(--r-sm);
  position: relative;
  overflow: hidden;
}

.sk-line::after,
.sk-card::after,
.sk-btn::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    100deg,
    transparent 30%,
    rgba(255, 255, 255, 0.06) 50%,
    transparent 70%
  );
  background-size: 220% 100%;
  animation: sk-shimmer 1.4s linear infinite;
}

@keyframes sk-shimmer {
  from {
    background-position: 180% 0;
  }
  to {
    background-position: -80% 0;
  }
}

.sk-hero {
  display: flex;
  align-items: flex-end;
  min-height: 27rem;
  padding: 5rem var(--page-pad) var(--sp-6);
  background: linear-gradient(180deg, var(--fill-1), transparent);
}

.sk-hero-text {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  width: min(34rem, 60%);
}

.sk-line.title {
  height: 2.4rem;
  width: 55%;
  border-radius: var(--r-md);
}

.sk-line.meta {
  height: 0.9rem;
  width: 40%;
}

.sk-line.desc {
  height: 0.8rem;
  width: 100%;
}

.sk-line.desc.short {
  width: 72%;
}

.sk-buttons {
  display: flex;
  gap: 0.65rem;
  margin-top: 0.5rem;
}

.sk-btn {
  width: 8rem;
  height: 2.6rem;
  border-radius: var(--r-full);
}

.sk-btn.narrow {
  width: 5.5rem;
}

.sk-row {
  padding: 1.9rem var(--page-pad) 0;
}

.sk-line.heading {
  width: 9rem;
  height: 1.3rem;
  margin-bottom: 1.1rem;
}

.sk-cards {
  display: flex;
  gap: 1.1rem;
  overflow: hidden;
}

.sk-card {
  width: 10.5rem;
  flex: none;
  aspect-ratio: 2 / 3;
  border-radius: var(--r-md);
}

.sk-card.thumb {
  width: 18rem;
  aspect-ratio: 16 / 9;
}

@media (prefers-reduced-motion: reduce) {
  .sk-line::after,
  .sk-card::after,
  .sk-btn::after {
    animation: none;
  }
}
</style>
