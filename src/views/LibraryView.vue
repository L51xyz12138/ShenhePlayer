<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as api from '@/api'
import PosterCard from '@/components/PosterCard.vue'
import AppIcon from '@/components/AppIcon.vue'
import { useSessionStore } from '@/stores/session'
import { usePlayerStore } from '@/stores/player'
import { useGridWindow } from '@/composables/useGridWindow'
import type { BaseItem } from '@/types'

const props = defineProps<{ id: string }>()

const session = useSessionStore()
const player = usePlayerStore()

const items = ref<BaseItem[]>([])
const total = ref(0)
const loading = ref(false)
const error = ref('')
const done = ref(false)
const sentinel = ref<HTMLElement>()

const PAGE = 60

// 只渲染可视区域的行。参数要和下面 .grid 的 CSS 对上。
const { gridEl, visibleItems, firstIndex, padTop, padBottom, readGeometry } = useGridWindow(
  items,
  { minCardRem: 9.5, gapXRem: 1.1, gapYRem: 1.4, estimatedRowPx: 300 },
)

const SORTS = [
  { value: 'SortName', label: '名称' },
  { value: 'DateCreated', label: '添加时间' },
  { value: 'PremiereDate', label: '上映时间' },
  { value: 'CommunityRating', label: '评分' },
  { value: 'Random', label: '随机' },
] as const

const FILTERS = [
  { value: '', label: '全部' },
  { value: 'unplayed', label: '未看' },
  { value: 'played', label: '已看' },
  { value: 'favorite', label: '收藏' },
] as const

const sortBy = ref<string>('SortName')
const sortOrder = ref<'Ascending' | 'Descending'>('Ascending')
const filter = ref<string>('')

const view = computed(() => session.views.find((v) => v.id === props.id))
const title = computed(() => view.value?.name ?? '媒体库')

const itemTypes = computed(() => {
  switch (view.value?.collectionType) {
    case 'movies':
      return 'Movie'
    case 'tvshows':
      return 'Series'
    case 'music':
      return 'MusicAlbum'
    default:
      return undefined
  }
})

async function fetchPage(reset = false) {
  if (loading.value) return
  if (reset) {
    items.value = []
    done.value = false
  }
  if (done.value) return

  loading.value = true
  error.value = ''
  try {
    const result = await api.getItems({
      ParentId: props.id,
      IncludeItemTypes: itemTypes.value,
      Recursive: true,
      SortBy: sortBy.value,
      SortOrder: sortOrder.value,
      StartIndex: items.value.length,
      Limit: PAGE,
      IsPlayed: filter.value === 'played' ? true : filter.value === 'unplayed' ? false : undefined,
      Filters: filter.value === 'favorite' ? 'IsFavorite' : undefined,
    })
    items.value.push(...result.items)
    total.value = result.totalRecordCount
    // 追加后网格变高，重新量一次几何
    void nextTick(readGeometry)
    if (result.items.length < PAGE || items.value.length >= result.totalRecordCount) {
      done.value = true
    }
  } catch (e) {
    error.value = String(e)
    done.value = true
  } finally {
    loading.value = false
  }
}

let observer: IntersectionObserver | undefined

onMounted(() => {
  void fetchPage(true)
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) void fetchPage()
    },
    { rootMargin: '600px 0px' },
  )
  if (sentinel.value) observer.observe(sentinel.value)
})

onBeforeUnmount(() => observer?.disconnect())

watch([() => props.id, sortBy, sortOrder, filter], () => fetchPage(true))

function toggleOrder() {
  sortOrder.value = sortOrder.value === 'Ascending' ? 'Descending' : 'Ascending'
}

async function play(item: BaseItem) {
  try {
    await player.play(item, true)
  } catch {
    /* 错误展示在播放层 */
  }
}
</script>

<template>
  <div class="library">
    <header class="head">
      <div>
        <h1 class="t-title-2">{{ title }}</h1>
        <p class="t-caption">{{ total }} 项</p>
      </div>

      <div class="tools">
        <div class="segmented">
          <button
            v-for="f in FILTERS"
            :key="f.value"
            class="seg"
            :class="{ active: filter === f.value }"
            @click="filter = f.value"
          >
            {{ f.label }}
          </button>
        </div>

        <div class="sort">
          <select v-model="sortBy" class="select">
            <option v-for="s in SORTS" :key="s.value" :value="s.value">
              {{ s.label }}
            </option>
          </select>
          <button
            class="btn btn-icon order"
            :title="sortOrder === 'Ascending' ? '升序' : '降序'"
            @click="toggleOrder"
          >
            <AppIcon
              name="chevron-down"
              :size="16"
              :style="{ rotate: sortOrder === 'Ascending' ? '180deg' : '0deg' }"
            />
          </button>
        </div>
      </div>
    </header>

    <!-- 上下留白撑起未渲染的行，滚动条长度才是对的 -->
    <div :style="{ paddingTop: `${padTop}px`, paddingBottom: `${padBottom}px` }">
      <div ref="gridEl" class="grid">
        <PosterCard
          v-for="(item, i) in visibleItems"
          :key="item.id"
          :item="item"
          :eager="firstIndex + i < 18"
          @play="play"
        />

        <!-- 首屏加载时占位，保证网格不塌陷 -->
        <div v-for="n in loading && !items.length ? 18 : 0" :key="`sk-${n}`" class="sk-card" />
      </div>
    </div>

    <div v-if="error" class="state">
      <AppIcon name="info" :size="26" />
      <p class="t-caption">{{ error }}</p>
      <button class="btn" @click="fetchPage(true)">重试</button>
    </div>

    <div v-else-if="!loading && !items.length" class="state">
      <AppIcon name="folder" :size="30" />
      <p class="t-title-3">这里没有内容</p>
      <p class="t-caption">换个筛选条件试试</p>
    </div>

    <div ref="sentinel" class="sentinel">
      <span v-if="loading && items.length" class="t-caption">加载中…</span>
    </div>
  </div>
</template>

<style scoped>
.library {
  padding: 1.75rem var(--page-pad) 3rem;
}

.head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.head p {
  margin: 0.25rem 0 0;
}

.tools {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.segmented {
  display: flex;
  padding: 3px;
  border-radius: var(--r-full);
  background: var(--fill-1);
  border: 1px solid var(--separator);
}

.seg {
  padding: 0.32rem 0.85rem;
  border-radius: var(--r-full);
  font-size: 0.8125rem;
  font-weight: 540;
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}

.seg:hover {
  color: var(--label);
}

.seg.active {
  background: var(--fill-3);
  color: var(--label);
}

.sort {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.select {
  padding: 0.4rem 0.7rem;
  border-radius: var(--r-full);
  background: var(--fill-1);
  border: 1px solid var(--separator);
  color: var(--label);
  font: inherit;
  font-size: 0.8125rem;
  cursor: pointer;
}

.select:focus {
  outline: none;
  border-color: var(--accent);
}

.select option {
  background: var(--bg-1);
}

.order svg {
  transition: rotate var(--t-base) var(--ease);
}

/* 自适应列数：卡片最小 9.5rem，屏幕越宽列越多 */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr));
  gap: 1.4rem 1.1rem;
}

.sk-card {
  aspect-ratio: 2 / 3;
  border-radius: var(--r-md);
  background: rgba(255, 255, 255, 0.045);
}

.state {
  display: grid;
  place-items: center;
  gap: 0.5rem;
  padding: 4rem 2rem;
  color: var(--label-3);
  text-align: center;
}

.state p {
  margin: 0;
}

.sentinel {
  display: grid;
  place-items: center;
  height: 3rem;
  margin-top: 1rem;
}
</style>
