<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import * as api from '@/api'
import TitleBar from '@/components/TitleBar.vue'
import SideBar from '@/components/SideBar.vue'
import PlaybackStatus from '@/components/PlaybackStatus.vue'
import { useSessionStore } from '@/stores/session'
import { useSettingsStore } from '@/stores/settings'
import { usePlayerStore } from '@/stores/player'

const session = useSessionStore()
const settings = useSettingsStore()
const player = usePlayerStore()
const router = useRouter()
const ready = ref(false)

onMounted(async () => {
  try {
    await settings.load()
    await player.bind()

    const ok = await session.restore()
    if (ok) {
      await session.loadViews()
    } else if (!(await api.savedServers()).length) {
      // 一台服务器都没存过：直接引导去添加，别丢一个空首页给用户
      await router.replace({ name: 'add-server' })
    }
  } catch (e) {
    // 初始化失败也要把界面亮出来，否则用户只看到一个不存在的窗口
    console.error('初始化失败', e)
    session.error = String(e)
  } finally {
    ready.value = true
    // 内容渲染完再显示窗口，避免启动白屏
    await getCurrentWindow().show()
  }
})
</script>

<template>
  <div class="shell">
    <TitleBar />

    <div class="body">
      <SideBar />

      <main class="content">
        <RouterView v-slot="{ Component }">
          <Transition name="page" mode="out-in">
            <component :is="Component" />
          </Transition>
        </RouterView>
      </main>
    </div>

    <PlaybackStatus />

    <Transition name="fade">
      <div v-if="!ready" class="boot">
        <span class="mark" />
        <span class="boot-text">正在连接媒体库…</span>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg);
}

.body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.content {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  overflow-x: hidden;
  /* 顶部与标题栏交界处用渐变过渡，而不是硬分割线 */
  scrollbar-gutter: stable;
}

.boot {
  position: absolute;
  inset: 0;
  z-index: 60;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 1.1rem;
  background: var(--bg);
}

.mark {
  width: 46px;
  height: 46px;
  border-radius: 13px;
  background: var(--label);
  animation: pulse 1.6s var(--ease) infinite;
}

.boot-text {
  font-size: 0.8125rem;
  color: var(--label-3);
  letter-spacing: 0.02em;
}

@keyframes pulse {
  0%,
  100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(0.9);
    opacity: 0.65;
  }
}
</style>
