<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import * as api from '@/api'
import LazyImage from '@/components/LazyImage.vue'
import MediaRow from '@/components/MediaRow.vue'
import AppIcon from '@/components/AppIcon.vue'
import EpisodeItem from '@/components/EpisodeItem.vue'
import PosterCard from '@/components/PosterCard.vue'
import { backdropUrl, logoUrl, personUrl, posterUrl } from '@/api/images'
import {
  formatBitrate,
  formatBytes,
  formatDate,
  formatDuration,
  formatRating,
  formatTime,
  isResumable,
  playedFraction,
  ticksToSeconds,
  typeLabel,
} from '@/utils/format'
import { usePlayerStore } from '@/stores/player'
import type { BaseItem } from '@/types'

const props = defineProps<{ id: string }>()
const player = usePlayerStore()

const item = ref<BaseItem | null>(null)
const seasons = ref<BaseItem[]>([])
const episodes = ref<BaseItem[]>([])
const similar = ref<BaseItem[]>([])
const activeSeason = ref('')
const loading = ref(true)
const error = ref('')
const busy = ref(false)

const isPerson = computed(() => item.value?.type === 'Person')
/** 人物的参演作品 / 合集里的条目 */
const related = ref<BaseItem[]>([])
const backdrop = computed(() => (item.value ? backdropUrl(item.value, 1920) : ''))
const logo = computed(() => (item.value ? logoUrl(item.value) : ''))
const resumable = computed(() => (item.value ? isResumable(item.value) : false))
const resumeAt = computed(() =>
  item.value ? ticksToSeconds(item.value.userData?.playbackPositionTicks) : 0,
)

/** 详情页的媒体信息条：分辨率 / 编码 / 音轨 / 体积 */
const mediaFacts = computed(() => {
  const list = item.value?.mediaSources ?? []
  const source = list.find((s) => s.id === activeSource.value) ?? list[0]
  if (!source) return []
  const video = source.mediaStreams?.find((s) => s.type === 'Video')
  const audio = source.mediaStreams?.find((s) => s.type === 'Audio')
  const facts: string[] = []

  if (video?.height) {
    const h = video.height
    facts.push(h >= 2000 ? '4K' : h >= 1000 ? '1080P' : h >= 700 ? '720P' : `${h}P`)
  }
  if (video?.codec) facts.push(video.codec.toUpperCase())
  if (video?.videoRange && video.videoRange !== 'SDR') facts.push(video.videoRange)
  if (audio?.codec) facts.push(audio.codec.toUpperCase())
  if (audio?.channels === 6) facts.push('5.1')
  else if (audio?.channels === 8) facts.push('7.1')
  if (source.size) facts.push(formatBytes(source.size))
  if (source.bitrate) facts.push(formatBitrate(source.bitrate))
  return facts
})

/** 同一部片子的多个版本（4K / 1080P / 不同压制组） */
const sources = computed(() => item.value?.mediaSources ?? [])
const activeSource = ref('')

function sourceLabel(src: (typeof sources.value)[number], i: number): string {
  const video = src.mediaStreams?.find((s) => s.type === 'Video')
  const bits: string[] = []
  if (video?.height) {
    const h = video.height
    bits.push(h >= 2000 ? '4K' : h >= 1000 ? '1080P' : h >= 700 ? '720P' : `${h}P`)
  }
  if (video?.codec) bits.push(video.codec.toUpperCase())
  if (src.size) bits.push(formatBytes(src.size))
  const tail = bits.join(' · ')
  const name = src.name?.trim()
  if (name && tail) return `${name} · ${tail}`
  return name || tail || `版本 ${i + 1}`
}

const cast = computed(() =>
  (item.value?.people ?? []).filter((p) => p.type === 'Actor').slice(0, 18),
)

async function load() {
  loading.value = true
  error.value = ''
  seasons.value = []
  episodes.value = []
  similar.value = []
  try {
    const data = await api.getItem(props.id)
    item.value = data
    activeSource.value = data.mediaSources?.[0]?.id ?? ''

    if (data.type === 'Series') {
      seasons.value = await api.getSeasons(data.id)
      activeSeason.value = seasons.value[0]?.id ?? ''
      if (activeSeason.value) {
        episodes.value = await api.getEpisodes(data.id, activeSeason.value)
      }
    } else if (data.type === 'Season' && data.seriesId) {
      episodes.value = await api.getEpisodes(data.seriesId, data.id)
    } else if (data.type === 'Person') {
      // 人物页展示参演作品
      const res = await api.getItems({
        PersonIds: data.id,
        Recursive: true,
        IncludeItemTypes: 'Movie,Series',
        SortBy: 'PremiereDate,SortName',
        SortOrder: 'Descending',
        Limit: 60,
      })
      related.value = res.items
    } else if (data.type === 'BoxSet') {
      const res = await api.getItems({
        ParentId: data.id,
        SortBy: 'PremiereDate,SortName',
        SortOrder: 'Ascending',
        Limit: 200,
      })
      related.value = res.items
    }

    // 人物和合集没有「相似内容」这个概念
    similar.value =
      data.type === 'Person' || data.type === 'BoxSet'
        ? []
        : await api.getSimilar(data.id, 12).catch(() => [])
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

watch(() => props.id, load, { immediate: true })

watch(activeSeason, async (seasonId) => {
  if (!seasonId || !item.value?.id) return
  episodes.value = await api.getEpisodes(item.value.id, seasonId)
})

async function play(target?: BaseItem, resume = true) {
  const it = target ?? item.value
  if (!it) return

  // 剧集直接播第一集未看的
  if (it.type === 'Series') {
    const next = episodes.value.find((e) => !e.userData?.played) ?? episodes.value[0]
    if (next) return player.play(next, true)
  }
  // 只有当前条目自己有多版本时才需要指定；剧集里的某一集另算
  const msid = it.id === item.value?.id ? activeSource.value || undefined : undefined
  await player.play(it, resume, msid)
}

async function toggleFavorite() {
  if (!item.value || busy.value) return
  busy.value = true
  const next = !item.value.userData?.isFavorite
  try {
    await api.setFavorite(item.value.id, next)
    if (item.value.userData) item.value.userData.isFavorite = next
  } finally {
    busy.value = false
  }
}

async function togglePlayed() {
  if (!item.value || busy.value) return
  busy.value = true
  const next = !item.value.userData?.played
  try {
    await api.setPlayed(item.value.id, next)
    if (item.value.userData) {
      item.value.userData.played = next
      if (next) item.value.userData.playbackPositionTicks = 0
    }
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="detail">
    <div v-if="loading" class="state"><span class="t-caption">加载中…</span></div>

    <div v-else-if="error" class="state">
      <AppIcon name="info" :size="28" />
      <p class="t-caption">{{ error }}</p>
      <button class="btn" @click="load">重试</button>
    </div>

    <template v-else-if="item">
      <!-- ---- 人物：没有背景图，用素净的头部，跟随主题配色 ---- -->
      <header v-if="isPerson" class="person-hero">
        <LazyImage
          :src="posterUrl(item, 600)"
          :alt="item.name"
          class="person-portrait"
          eager
          rounded="var(--r-lg)"
        >
          <template #fallback><AppIcon name="user" :size="34" /></template>
        </LazyImage>

        <div class="person-info">
          <h1 class="t-large-title">{{ item.name }}</h1>
          <div class="person-meta t-footnote dim">
            <span class="pill type">{{ typeLabel(item.type) }}</span>
            <span v-if="item.productionYear">{{ item.productionYear }} 年生</span>
            <span>{{ related.length }} 部作品</span>
          </div>
          <p v-if="item.overview" class="person-overview t-subhead dim clamp-3">
            {{ item.overview }}
          </p>
        </div>
      </header>

      <!-- ---- 顶部大图 ---- -->
      <div v-else class="hero">
        <div
          v-if="backdrop"
          class="backdrop"
          :style="{ backgroundImage: `url(${backdrop})` }"
        />
        <div class="scrim" />

        <div class="hero-body">
          <LazyImage
            :src="posterUrl(item, 600)"
            :alt="item.name"
            class="hero-poster"
            eager
            rounded="var(--r-md)"
          >
            <template #fallback><AppIcon name="film" :size="30" /></template>
          </LazyImage>

          <div class="info">
            <img v-if="logo" :src="logo" :alt="item.name" class="logo" />
            <h1 v-else class="t-large-title">{{ item.name }}</h1>

            <p v-if="item.originalTitle && item.originalTitle !== item.name" class="original">
              {{ item.originalTitle }}
            </p>

            <div class="facts">
              <span class="pill type">{{ typeLabel(item.type) }}</span>
              <span v-if="item.productionYear">{{ item.productionYear }}</span>
              <span v-if="formatDuration(item.runTimeTicks)">
                {{ formatDuration(item.runTimeTicks) }}
              </span>
              <span v-if="formatRating(item.communityRating)" class="star">
                <AppIcon name="star" :size="12" filled />
                {{ formatRating(item.communityRating) }}
              </span>
              <span v-if="item.officialRating" class="pill">{{ item.officialRating }}</span>
              <span v-if="item.childCount">{{ item.childCount }} 季</span>
            </div>

            <div v-if="sources.length > 1" class="versions">
              <span class="t-caption dim">版本</span>
              <button
                v-for="(src, i) in sources"
                :key="src.id"
                class="version"
                :class="{ active: activeSource === src.id }"
                @click="activeSource = src.id"
              >
                {{ sourceLabel(src, i) }}
              </button>
            </div>

            <div v-if="mediaFacts.length" class="tech">
              <span v-for="f in mediaFacts" :key="f" class="tech-pill">{{ f }}</span>
            </div>

            <p v-if="item.taglines?.length" class="tagline">{{ item.taglines[0] }}</p>
            <p v-if="item.overview" class="overview t-subhead clamp-3">{{ item.overview }}</p>

            <div v-if="item.genres.length" class="genres">
              <span v-for="g in item.genres.slice(0, 6)" :key="g" class="genre">{{ g }}</span>
            </div>

            <div class="actions">
              <button class="btn btn-white lg" @click="play()">
                <AppIcon name="play" :size="17" filled />
                {{ resumable ? `继续 ${formatTime(resumeAt)}` : '播放' }}
              </button>
              <button v-if="resumable" class="btn lg" @click="play(undefined, false)">
                <AppIcon name="rewind" :size="17" />
                从头播放
              </button>
              <button
                class="btn btn-icon lg-icon"
                :class="{ on: item.userData?.isFavorite }"
                title="收藏"
                @click="toggleFavorite"
              >
                <AppIcon name="heart" :size="17" :filled="item.userData?.isFavorite" />
              </button>
              <button
                class="btn btn-icon lg-icon"
                :class="{ on: item.userData?.played }"
                title="标记为已看"
                @click="togglePlayed"
              >
                <AppIcon name="check" :size="17" />
              </button>
            </div>

            <div v-if="playedFraction(item) > 0 && playedFraction(item) < 1" class="resume">
              <span :style="{ width: `${playedFraction(item) * 100}%` }" />
            </div>
          </div>
        </div>
      </div>

      <!-- ---- 剧集列表 ---- -->
      <section v-if="episodes.length || seasons.length" class="block">
        <header class="block-head">
          <h2 class="t-title-3">剧集</h2>
          <select v-if="seasons.length > 1" v-model="activeSeason" class="select">
            <option v-for="s in seasons" :key="s.id" :value="s.id">{{ s.name }}</option>
          </select>
        </header>

        <div class="episodes">
          <EpisodeItem
            v-for="ep in episodes"
            :key="ep.id"
            :item="ep"
            @play="play($event, true)"
          />
        </div>
      </section>

      <!-- ---- 参演作品 / 合集内容 ---- -->
      <section v-if="related.length" class="block">
        <h2 class="t-title-3 block-head">{{ isPerson ? '参演作品' : '合集内容' }}</h2>
        <div class="related">
          <PosterCard
            v-for="(it, i) in related"
            :key="it.id"
            :item="it"
            :eager="i < 12"
            @play="play($event, true)"
          />
        </div>
      </section>

      <!-- ---- 演职人员 ---- -->
      <section v-if="cast.length" class="block">
        <h2 class="title-md block-head">演职人员</h2>
        <div class="cast">
          <component
            :is="p.id ? 'RouterLink' : 'div'"
            v-for="p in cast"
            :key="`${p.id}-${p.name}`"
            :to="p.id ? { name: 'item', params: { id: p.id } } : undefined"
            class="person"
            :class="{ linked: !!p.id }"
          >
            <LazyImage
              :src="personUrl(p.id ?? '', p.primaryImageTag, 240)"
              :alt="p.name"
              ratio="1 / 1"
              rounded="var(--r-full)"
              class="avatar"
            >
              <template #fallback><AppIcon name="user" :size="20" /></template>
            </LazyImage>
            <div class="person-name truncate">{{ p.name }}</div>
            <div class="person-role truncate">{{ p.role }}</div>
          </component>
        </div>
      </section>

      <!-- ---- 相关推荐 ---- -->
      <MediaRow
        v-if="similar.length"
        title="相似内容"
        :items="similar"
        class="block-row"
        @play="play($event, true)"
      />

      <!-- ---- 详细信息 ---- -->
      <section class="block meta-block">
        <h2 class="title-md block-head">详细信息</h2>
        <dl class="meta-grid">
          <template v-if="item.premiereDate">
            <dt>首播</dt>
            <dd>{{ formatDate(item.premiereDate) }}</dd>
          </template>
          <template v-if="item.studios.length">
            <dt>制作</dt>
            <dd>{{ item.studios.map((s) => s.name).join('、') }}</dd>
          </template>
          <template v-if="item.status">
            <dt>状态</dt>
            <dd>{{ item.status === 'Continuing' ? '连载中' : '已完结' }}</dd>
          </template>
          <template v-if="item.container">
            <dt>容器</dt>
            <dd class="mono">{{ item.container.toUpperCase() }}</dd>
          </template>
          <template v-if="item.path">
            <dt>路径</dt>
            <dd class="mono path">{{ item.path }}</dd>
          </template>
        </dl>
      </section>
    </template>
  </div>
</template>

<style scoped>
.detail {
  padding-bottom: 3.5rem;
}

.state {
  display: grid;
  place-items: center;
  gap: 0.6rem;
  padding: 6rem 2rem;
  color: var(--label-3);
}

/* ---- 顶部 ---- */
.hero {
  position: relative;
  padding: 3.5rem var(--page-pad) 2.25rem;
  isolation: isolate;
}

.backdrop {
  position: absolute;
  inset: 0;
  background-size: cover;
  background-position: center 18%;
}

.scrim {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(to top, var(--bg) 3%, rgba(8, 10, 16, 0.72) 45%, rgba(8, 10, 16, 0.4) 100%),
    linear-gradient(to right, rgba(8, 10, 16, 0.9) 0%, rgba(8, 10, 16, 0.35) 70%, transparent 100%);
}

/* 顶部大图上的文字固定浅色，不跟随主题 */
.hero {
  color: #fff;
}

.hero .original,
.hero .facts,
.hero .tagline,
.hero .overview {
  color: rgba(235, 235, 245, 0.72);
}

.hero .tech-pill,
.hero .genre {
  color: rgba(235, 235, 245, 0.8);
  background: rgba(255, 255, 255, 0.14);
  border-color: rgba(255, 255, 255, 0.16);
}

.hero-body {
  position: relative;
  display: flex;
  gap: 2rem;
  align-items: flex-end;
}

.hero-poster {
  width: 13.5rem;
  flex: none;
  box-shadow: var(--shadow-lg);
  border: 1px solid var(--separator-strong);
}

.info {
  flex: 1;
  min-width: 0;
  padding-bottom: 0.25rem;
}

.logo {
  max-width: 24rem;
  max-height: 7.5rem;
  object-fit: contain;
  object-position: left bottom;
  margin-bottom: 0.75rem;
  filter: drop-shadow(0 4px 16px rgba(0, 0, 0, 0.6));
}

.original {
  margin: 0.2rem 0 0;
  font-size: 0.875rem;
  color: var(--label-3);
}

.facts {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.85rem;
  margin: 0.75rem 0;
  font-size: 0.8125rem;
  font-weight: 540;
  color: var(--label-2);
}

.star {
  display: inline-flex;
  align-items: center;
  gap: 0.22rem;
  color: #ffc94d;
}

.pill {
  padding: 0.1rem 0.5rem;
  border: 1px solid var(--separator-strong);
  border-radius: var(--r-xs);
  font-size: 0.6875rem;
}

.pill.type {
  border-color: transparent;
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 620;
}

.versions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem;
  margin-bottom: 0.7rem;
}

.version {
  padding: 0.2rem 0.6rem;
  border-radius: var(--r-full);
  background: rgba(255, 255, 255, 0.14);
  border: 1px solid transparent;
  font-size: 0.75rem;
  font-weight: 540;
  color: rgba(235, 235, 245, 0.85);
  transition: background var(--t-fast) var(--ease), border-color var(--t-fast) var(--ease);
}

.version:hover {
  background: rgba(255, 255, 255, 0.22);
}

.version.active {
  background: var(--accent);
  border-color: transparent;
  color: #fff;
}

.tech {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 0.85rem;
}

.tech-pill {
  padding: 0.12rem 0.45rem;
  border-radius: var(--r-xs);
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid var(--separator);
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--label-2);
}

.tagline {
  margin: 0 0 0.5rem;
  font-size: 0.9375rem;
  font-style: italic;
  color: var(--label-2);
}

.overview {
  max-width: 46rem;
  margin: 0 0 1rem;
  color: var(--label-2);
}

.genres {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin-bottom: 1.25rem;
}

.genre {
  padding: 0.2rem 0.65rem;
  border-radius: var(--r-full);
  background: var(--fill-2);
  border: 1px solid var(--separator);
  font-size: 0.75rem;
  color: var(--label-2);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}

.lg {
  padding: 0.68rem 1.4rem;
  font-size: 0.9375rem;
}

.lg-icon {
  width: 2.75rem;
  height: 2.75rem;
  color: var(--label-2);
}

.lg-icon.on {
  background: var(--accent-soft);
  border-color: var(--accent);
  color: var(--accent);
}

.actions .btn:not(.btn-white) {
  background: rgba(255, 255, 255, 0.16);
  backdrop-filter: blur(20px);
}

.resume {
  margin-top: 1.1rem;
  width: 18rem;
  height: 3px;
  border-radius: var(--r-full);
  background: rgba(255, 255, 255, 0.2);
  overflow: hidden;
}

.resume span {
  display: block;
  height: 100%;
  background: var(--accent);
}

/* ---- 人物头部 ---- */
.person-hero {
  display: flex;
  align-items: flex-end;
  gap: 2rem;
  padding: 2.5rem var(--page-pad) 0;
}

.person-portrait {
  width: 11rem;
  flex: none;
  box-shadow: var(--shadow-md);
}

.person-info {
  flex: 1;
  min-width: 0;
  padding-bottom: 0.4rem;
}

.person-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.8rem;
  margin: 0.7rem 0;
}

.person-overview {
  max-width: 46rem;
  margin: 0;
}

/* ---- 内容区块 ---- */
.block {
  padding: 2.25rem var(--page-pad) 0;
}

.block-row {
  padding-top: 2.25rem;
}

.block-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1rem;
}

.select {
  padding: 0.4rem 0.75rem;
  border-radius: var(--r-full);
  background: var(--fill-1);
  border: 1px solid var(--separator);
  color: var(--label);
  font: inherit;
  font-size: 0.8125rem;
  cursor: pointer;
}

.select option {
  background: var(--bg-1);
}

.episodes {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.related {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(9.5rem, 1fr));
  gap: 1.4rem 1.1rem;
}

.cast {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
  gap: 1.1rem;
}

.person {
  display: block;
  text-align: center;
  min-width: 0;
}

.person.linked {
  transition: transform var(--t-fast) var(--ease);
}

.person.linked:hover {
  transform: translateY(-2px);
}

.person.linked:hover .person-name {
  color: var(--accent);
}

.avatar {
  width: 4.5rem;
  margin: 0 auto 0.5rem;
  border: 1px solid var(--separator);
}

.person-name {
  font-size: 0.8125rem;
  font-weight: 550;
}

.person-role {
  font-size: 0.75rem;
  color: var(--label-3);
}

.meta-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.55rem 1.5rem;
  margin: 0;
  font-size: 0.8125rem;
}

.meta-grid dt {
  color: var(--label-3);
}

.meta-grid dd {
  margin: 0;
  color: var(--label-2);
}

.path {
  word-break: break-all;
  user-select: text;
}
</style>
