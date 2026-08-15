<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import * as api from '@/api'
import PosterCard from '@/components/PosterCard.vue'
import AppIcon from '@/components/AppIcon.vue'
import { usePlayerStore } from '@/stores/player'
import { useSessionStore } from '@/stores/session'
import { typeLabel } from '@/utils/format'
import type { BaseItem } from '@/types'

const route = useRoute()
const router = useRouter()
const player = usePlayerStore()
const session = useSessionStore()

const term = ref(String(route.query.q ?? ''))
const results = ref<BaseItem[]>([])
const loading = ref(false)
const searched = ref(false)
const input = ref<HTMLInputElement>()

let timer: number | undefined

/** 输入停 280ms 才发请求：既跟手又不会把服务器打爆 */
function schedule() {
  if (timer) window.clearTimeout(timer)
  timer = window.setTimeout(run, 280)
}

async function run() {
  if (!session.isAuthed) return
  const q = term.value.trim()
  router.replace({ query: q ? { q } : {} })

  if (!q) {
    results.value = []
    searched.value = false
    return
  }

  loading.value = true
  try {
    results.value = await api.search(q, 60)
    searched.value = true
  } finally {
    loading.value = false
  }
}

watch(term, schedule)

onMounted(() => {
  input.value?.focus()
  if (term.value) void run()
})

// 按类型分组，电影/剧集/单集分开看更清楚
const groups = computed(() => {
  const order = ['Movie', 'Series', 'Season', 'Episode', 'BoxSet', 'Person']
  const map = new Map<string, BaseItem[]>()
  for (const item of results.value) {
    const list = map.get(item.type) ?? []
    list.push(item)
    map.set(item.type, list)
  }
  const rank = (t: string) => {
    const i = order.indexOf(t)
    return i === -1 ? order.length : i
  }
  return [...map.entries()].sort((a, b) => rank(a[0]) - rank(b[0]))
})

async function play(item: BaseItem) {
  try {
    await player.play(item, true)
  } catch {
    /* 播放层展示错误 */
  }
}
</script>

<template>
  <div class="search-view">
    <div class="search-head">
      <label class="big-search">
        <AppIcon name="search" :size="20" />
        <input
          ref="input"
          v-model="term"
          type="search"
          placeholder="搜索影片、剧集、演员…"
          spellcheck="false"
          @keydown.enter="run"
        />
        <button v-if="term" class="clear" title="清空" @click="term = ''">
          <AppIcon name="close" :size="15" />
        </button>
      </label>
      <p v-if="searched" class="t-caption count">找到 {{ results.length }} 个结果</p>
    </div>

    <div v-if="!session.isAuthed" class="state">
      <AppIcon name="server" :size="30" />
      <p class="t-title-3">未连接服务器</p>
      <RouterLink :to="{ name: 'servers' }" class="btn btn-primary">选择服务器</RouterLink>
    </div>

    <div v-else-if="loading && !results.length" class="state">
      <span class="t-caption">搜索中…</span>
    </div>

    <div v-else-if="searched && !results.length" class="state">
      <AppIcon name="search" :size="30" />
      <p class="t-title-3">没有匹配的内容</p>
      <p class="t-caption">换个关键词试试</p>
    </div>

    <div v-else-if="!searched" class="state">
      <AppIcon name="sparkles" :size="30" />
      <p class="t-caption">输入片名、演员或年份开始搜索</p>
    </div>

    <div v-for="[type, list] in groups" :key="type" class="group">
      <h2 class="t-title-3">{{ typeLabel(type) }}</h2>
      <div class="grid">
        <PosterCard
          v-for="(item, i) in list"
          :key="item.id"
          :item="item"
          :shape="type === 'Episode' ? 'thumb' : 'poster'"
          :eager="i < 12"
          @play="play"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-view {
  padding: 1.75rem var(--page-pad) 3rem;
}

.search-head {
  margin-bottom: 1.75rem;
}

.big-search {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  padding: 0.7rem 1.1rem;
  border-radius: var(--r-full);
  background: var(--fill-1);
  border: 1px solid var(--separator);
  color: var(--label-3);
  transition: border-color var(--t-fast) var(--ease), box-shadow var(--t-fast) var(--ease);
}

.big-search:focus-within {
  border-color: var(--accent);
  color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.big-search input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  color: var(--label);
  font: inherit;
  font-size: 1rem;
}

.big-search input::placeholder {
  color: var(--label-3);
}

.big-search input::-webkit-search-cancel-button {
  appearance: none;
}

.clear {
  display: grid;
  place-items: center;
  width: 1.35rem;
  height: 1.35rem;
  border-radius: var(--r-full);
  background: var(--fill-3);
  color: var(--label-2);
}

.clear:hover {
  background: var(--fill-3);
  color: var(--label);
}

.count {
  margin: 0.7rem 0 0 0.3rem;
}

.group {
  margin-bottom: 2.25rem;
}

.group h2 {
  margin-bottom: 1rem;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr));
  gap: 1.4rem 1.1rem;
}

.state {
  display: grid;
  place-items: center;
  gap: 0.5rem;
  padding: 5rem 2rem;
  color: var(--label-3);
  text-align: center;
}

.state p {
  margin: 0;
}

.state .btn {
  margin-top: 0.7rem;
}
</style>
