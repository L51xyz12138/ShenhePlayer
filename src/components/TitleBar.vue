<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import AppIcon from './AppIcon.vue'

const router = useRouter()
const route = useRoute()
const win = getCurrentWindow()

const maximized = ref(false)
const term = ref('')

let unlisten: (() => void) | undefined

onMounted(async () => {
  maximized.value = await win.isMaximized()
  unlisten = await win.onResized(async () => {
    maximized.value = await win.isMaximized()
  })
})

onUnmounted(() => unlisten?.())

function submit() {
  const q = term.value.trim()
  if (!q) return
  router.push({ name: 'search', query: { q } })
}

const canGoBack = () => window.history.length > 1 && route.name !== 'home'
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="left" data-tauri-drag-region>
      <button class="nav" :disabled="!canGoBack()" title="返回" @click="router.back()">
        <AppIcon name="chevron-left" :size="19" />
      </button>
    </div>

    <label class="search">
      <AppIcon name="search" :size="15" />
      <input
        v-model="term"
        type="search"
        placeholder="搜索"
        spellcheck="false"
        @keydown.enter="submit"
      />
    </label>

    <div class="right">
      <button class="win-btn" title="最小化" @click="win.minimize()">
        <AppIcon name="minimize" :size="14" :stroke="1.5" />
      </button>
      <button
        class="win-btn"
        :title="maximized ? '还原' : '最大化'"
        @click="win.toggleMaximize()"
      >
        <AppIcon :name="maximized ? 'restore' : 'maximize'" :size="12" :stroke="1.5" />
      </button>
      <button class="win-btn danger" title="关闭" @click="win.close()">
        <AppIcon name="close" :size="14" :stroke="1.5" />
      </button>
    </div>
  </header>
</template>

<style scoped>
/* 顶栏是一层浮在内容之上的材质，内容从下面滚过去，不用硬分割线 */
.titlebar {
  position: relative;
  z-index: 40;
  display: grid;
  /* 中间一栏吃掉返回按钮和窗口按钮之外的全部宽度 */
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: var(--sp-4);
  height: var(--titlebar-h);
  padding-left: 0.5rem;
  flex: none;
  background: var(--material);
  backdrop-filter: var(--material-blur);
  border-bottom: 1px solid var(--separator);
}

.left,
.right {
  display: flex;
  align-items: center;
}

.right {
  justify-content: flex-end;
}

.nav {
  display: grid;
  place-items: center;
  width: 2rem;
  height: 2rem;
  border-radius: var(--r-full);
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}

.nav:hover:not(:disabled) {
  background: var(--fill-2);
  color: var(--label);
}

.nav:disabled {
  opacity: 0.25;
}

/* ---- 搜索框 ---- */
.search {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.24rem 0.7rem;
  border-radius: var(--r-full);
  background: var(--fill-1);
  color: var(--label-3);
  transition: background var(--t-fast) var(--ease), box-shadow var(--t-fast) var(--ease);
}

.search:focus-within {
  background: var(--fill-2);
  color: var(--label-2);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.search input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  color: var(--label);
  font: inherit;
  font-size: 0.8125rem;
  line-height: 1.7;
}

.search input::placeholder {
  color: var(--label-3);
}

.search input::-webkit-search-cancel-button {
  appearance: none;
}

/* ---- 窗口按钮：贴 Windows 的尺寸习惯 ---- */
.win-btn {
  display: grid;
  place-items: center;
  width: 46px;
  height: var(--titlebar-h);
  color: var(--label-2);
  transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
}

.win-btn:hover {
  background: var(--fill-2);
  color: var(--label);
}

.win-btn.danger:hover {
  background: #e81123;
  color: #fff;
}
</style>
