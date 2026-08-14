<script setup lang="ts">
// 悬浮状态条：报播放错误，以及外置播放时回收观看进度。
// 外部播放器是独立进程，退出时不会通知我们，所以只能让用户点一下。
import AppIcon from './AppIcon.vue'
import { usePlayerStore } from '@/stores/player'
import { displayTitle } from '@/utils/format'

const player = usePlayerStore()

async function finish(watched: boolean) {
  await player.finishExternal(watched)
}
</script>

<template>
  <div class="status-host">
    <Transition name="slide">
      <div v-if="player.error" class="bar error">
        <AppIcon name="info" :size="18" />
        <div class="text">
          <b>播放失败</b>
          <span>{{ player.error }}</span>
        </div>
        <button class="btn btn-ghost btn-icon" title="关闭" @click="player.error = ''">
          <AppIcon name="close" :size="15" />
        </button>
      </div>
    </Transition>

    <Transition name="slide">
      <div v-if="player.externalItem" class="bar external">
        <span class="pulse" />
        <div class="text">
          <b class="truncate">{{ displayTitle(player.externalItem) }}</b>
          <span>正在用外置播放器播放</span>
        </div>
        <button class="btn" @click="finish(true)">
          <AppIcon name="check" :size="15" />
          已看完
        </button>
        <button class="btn btn-ghost" @click="finish(false)">停止</button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.status-host {
  position: fixed;
  right: 1.25rem;
  bottom: 1.25rem;
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  pointer-events: none;
}

.bar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  max-width: 30rem;
  padding: 0.7rem 0.85rem;
  border-radius: var(--r-md);
  background: var(--material);
  backdrop-filter: var(--material-blur);
  border: 1px solid var(--separator-strong);
  box-shadow: var(--shadow-lg);
  pointer-events: auto;
}

.bar.error {
  border-color: rgba(255, 69, 58, 0.45);
}

.text {
  flex: 1;
  min-width: 0;
  line-height: 1.35;
}

.text b {
  display: block;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--label);
}

.text span {
  font-size: 0.75rem;
  color: var(--label-3);
}

.bar.error .text span {
  color: var(--red);
  word-break: break-word;
}

.bar .btn {
  flex: none;
  padding: 0.4rem 0.8rem;
  font-size: 0.8125rem;
}

/* 正在播放的呼吸点 */
.pulse {
  width: 8px;
  height: 8px;
  flex: none;
  border-radius: var(--r-full);
  background: var(--green);
  box-shadow: 0 0 0 0 rgba(50, 215, 75, 0.6);
  animation: breathe 2s var(--ease) infinite;
}

@keyframes breathe {
  70% {
    box-shadow: 0 0 0 7px rgba(50, 215, 75, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(50, 215, 75, 0);
  }
}

/* 从下方滑入，退出走同一条路径 */
.slide-enter-active,
.slide-leave-active {
  transition: opacity var(--t-base) var(--ease), transform var(--t-base) var(--ease-out);
}

.slide-enter-from,
.slide-leave-to {
  opacity: 0;
  transform: translateY(14px);
}
</style>
