<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useSessionStore } from '@/stores/session'
import { useSettingsStore } from '@/stores/settings'
import AppIcon from './AppIcon.vue'
import type { BaseItem } from '@/types'

const session = useSessionStore()
const settings = useSettingsStore()
const route = useRoute()
/** 头像取不到时退回首字母，避免显示浏览器的破图图标 */
const avatarFailed = ref(false)

const viewIcon = (v: BaseItem) => {
  switch (v.collectionType) {
    case 'movies':
      return 'film'
    case 'tvshows':
      return 'tv'
    case 'music':
      return 'music'
    default:
      return 'folder'
  }
}

const activeLibrary = computed(() => (route.name === 'library' ? String(route.params.id) : ''))
</script>

<template>
  <nav class="sidebar">
    <div class="brand">
      <span class="mark" />
      <span class="t-footnote">ShenhePlayer</span>
    </div>

    <div class="scroll">
      <div class="group">
        <RouterLink :to="{ name: 'home' }" class="item" active-class="active">
        <AppIcon name="home" :size="18" />
        <span>首页</span>
      </RouterLink>
        <RouterLink :to="{ name: 'search' }" class="item" active-class="active">
          <AppIcon name="search" :size="18" />
          <span>搜索</span>
        </RouterLink>
        <RouterLink :to="{ name: 'servers' }" class="item" active-class="active">
          <AppIcon name="server" :size="18" />
          <span>服务器</span>
        </RouterLink>
      </div>

      <!-- 会话恢复期间先摆几条占位，别让侧边栏看着是空的 -->
      <div v-if="session.restoring && !session.views.length" class="group">
        <div class="group-title t-caption-2">媒体库</div>
        <div v-for="n in 5" :key="n" class="sk-item" />
      </div>

      <div v-else-if="session.views.length" class="group">
        <div class="group-title t-caption-2">媒体库</div>
        <RouterLink
          v-for="view in session.views"
          :key="view.id"
          :to="{ name: 'library', params: { id: view.id } }"
          class="item"
          :class="{ active: activeLibrary === view.id }"
        >
          <AppIcon :name="viewIcon(view)" :size="18" />
          <span class="truncate">{{ view.name }}</span>
        </RouterLink>
      </div>
    </div>

    <div class="bottom">
      <RouterLink :to="{ name: 'settings' }" class="item" active-class="active">
        <AppIcon name="settings" :size="18" />
        <span>设置</span>
        <span
          v-if="settings.pendingUpdate"
          class="dot"
          :title="`有新版本 ${settings.pendingUpdate.latest}`"
        />
      </RouterLink>

      <RouterLink
        v-if="!session.session"
        :to="{ name: 'servers' }"
        class="account offline"
      >
        <div class="avatar dim">
          <AppIcon name="server" :size="15" />
        </div>
        <div class="who">
          <div class="truncate t-footnote">未连接</div>
          <div class="truncate t-caption-2 dim-3">选择一台服务器</div>
        </div>
      </RouterLink>

      <RouterLink v-else :to="{ name: 'servers' }" class="account">
        <img
          v-if="session.session.avatarUrl && !avatarFailed"
          :src="session.session.avatarUrl"
          alt=""
          @error="avatarFailed = true"
        />
        <div v-else class="avatar">
          {{ session.session.userName.slice(0, 1).toUpperCase() }}
        </div>
        <div class="who">
          <div class="truncate t-footnote">{{ session.session.userName }}</div>
          <div class="truncate t-caption-2 dim-3">{{ session.session.serverName }}</div>
        </div>
        <AppIcon name="chevron-right" :size="15" class="switch" />
      </RouterLink>
    </div>
  </nav>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  width: var(--sidebar-w);
  flex: none;
  padding: var(--sp-4) 0.75rem var(--sp-3);
  border-right: 1px solid var(--separator);
  /* 整条不滚：媒体库一多就会把「设置」和账号挤出可视区 */
  overflow: hidden;
}

/* 只有导航区滚动 */
.scroll {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
  overflow-y: auto;
  overflow-x: hidden;
  /* 滚动条贴着边，不要挤压文字 */
  margin-right: -0.35rem;
  padding-right: 0.35rem;
}

/* 设置与账号常驻底部 */
.bottom {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  padding-top: var(--sp-3);
}

.brand {
  flex: none;
  margin-bottom: var(--sp-5);
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0 0.5rem;
  color: var(--label-2);
  font-weight: 600;
  letter-spacing: -0.004em;
}

/* 图标不用渐变——Apple 的做法是单色几何形，克制 */
.mark {
  position: relative;
  width: 17px;
  height: 17px;
  border-radius: 5px;
  background: var(--label);
}

.mark::after {
  content: '';
  position: absolute;
  left: 6px;
  top: 4.5px;
  border-left: 6px solid var(--bg);
  border-top: 4px solid transparent;
  border-bottom: 4px solid transparent;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.group-title {
  padding: 0 0.6rem 0.4rem;
  font-weight: 600;
  color: var(--label-3);
  text-transform: uppercase;
}

/* 选中态用一块圆角填充，不用左侧色条——更接近 Apple 侧边栏 */
.item {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.45rem 0.6rem;
  border-radius: var(--r-sm);
  font-size: 0.9375rem;
  font-weight: 500;
  letter-spacing: -0.002em;
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease),
    transform var(--t-fast) var(--ease);
}

.item span {
  min-width: 0;
}

.item:hover {
  background: var(--fill-1);
  color: var(--label);
}

.item:active {
  transform: scale(0.985);
}

.item.active {
  background: var(--fill-2);
  color: var(--label);
  font-weight: 570;
}

.item.active svg {
  color: var(--accent);
}

/* 媒体库加载中的占位条 */
.sk-item {
  height: 1.55rem;
  margin: 0.45rem 0.6rem;
  border-radius: var(--r-xs);
  background: var(--fill-1);
  position: relative;
  overflow: hidden;
}

.sk-item::after {
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

@media (prefers-reduced-motion: reduce) {
  .sk-item::after {
    animation: none;
  }
}

/* 有新版本时的小红点 */
.dot {
  width: 7px;
  height: 7px;
  margin-left: auto;
  border-radius: var(--r-full);
  background: var(--accent);
}

.account {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.5rem;
  border-radius: var(--r-sm);
  background: var(--fill-1);
  transition: background var(--t-fast) var(--ease);
}

.account:hover {
  background: var(--fill-2);
}

.switch {
  flex: none;
  margin-left: auto;
  color: var(--label-3);
}

.account img,
.avatar {
  width: 28px;
  height: 28px;
  flex: none;
  border-radius: var(--r-full);
  object-fit: cover;
}

.avatar {
  display: grid;
  place-items: center;
  background: var(--fill-3);
  color: var(--label);
  font-size: 0.8125rem;
  font-weight: 600;
}

.who {
  min-width: 0;
  line-height: 1.25;
}
</style>
