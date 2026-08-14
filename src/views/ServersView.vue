<script setup lang="ts">
// 服务器管理。可以同时保存多个 Emby 服务器，随时切换；
// 没连接任何服务器也能进主界面，不再拿登录页挡在门口。
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as api from '@/api'
import AppIcon from '@/components/AppIcon.vue'
import { useSessionStore } from '@/stores/session'
import type { ServerSummary } from '@/types'

const router = useRouter()
const session = useSessionStore()

const servers = ref<ServerSummary[]>([])
const loading = ref(true)
const busyId = ref('')
const error = ref('')

async function refresh() {
  servers.value = await api.savedServers()
  loading.value = false
}

onMounted(refresh)

/** 切到另一台服务器。凭据失效就带着信息跳到登录表单重新输密码。 */
async function connect(s: ServerSummary) {
  if (busyId.value) return
  error.value = ''
  busyId.value = s.id

  try {
    if (!s.hasToken) {
      await router.push({ name: 'add-server', query: { id: s.id } })
      return
    }

    const info = await api.switchServer(s.id)
    if (!info) {
      error.value = `${s.name} 的登录已失效，请重新输入密码`
      await router.push({ name: 'add-server', query: { id: s.id } })
      return
    }

    session.apply(info)
    await session.loadViews()
    await router.push({ name: 'home' })
  } catch (e) {
    error.value = String(e)
  } finally {
    busyId.value = ''
  }
}

/** 断开当前连接但保留凭据，下次点一下就能回来 */
async function disconnect() {
  await api.disconnect()
  session.apply(null)
  await refresh()
}

async function forget(s: ServerSummary) {
  if (s.isActive) {
    await api.disconnect()
    session.apply(null)
  }
  await api.forgetServer(s.id)
  await refresh()
}
</script>

<template>
  <div class="servers">
    <header class="head">
      <div>
        <h1 class="t-title-2">服务器</h1>
        <p class="t-footnote dim">可以保存多个 Emby 服务器，随时切换</p>
      </div>
      <button class="btn btn-primary" @click="router.push({ name: 'add-server' })">
        <AppIcon name="server" :size="16" />
        添加服务器
      </button>
    </header>

    <p v-if="error" class="error t-footnote">{{ error }}</p>

    <div v-if="loading" class="state t-footnote dim-3">加载中…</div>

    <div v-else-if="!servers.length" class="state">
      <AppIcon name="server" :size="30" />
      <p class="t-title-3">还没有添加服务器</p>
      <p class="t-footnote dim">添加一台 Emby 服务器就能开始浏览媒体库</p>
      <button class="btn btn-primary" @click="router.push({ name: 'add-server' })">
        添加服务器
      </button>
    </div>

    <ul v-else class="list">
      <li v-for="s in servers" :key="s.id" class="item" :class="{ active: s.isActive }">
        <div class="badge" :class="{ on: s.isActive }">
          <AppIcon name="server" :size="18" />
        </div>

        <div class="info">
          <div class="line">
            <span class="t-headline truncate">{{ s.name || s.url }}</span>
            <span v-if="s.isActive" class="chip on">已连接</span>
            <span v-else-if="!s.hasToken" class="chip">需要登录</span>
          </div>
          <div class="t-caption dim-3 truncate">{{ s.username }} · {{ s.url }}</div>
        </div>

        <div class="actions">
          <button
            v-if="!s.isActive"
            class="btn"
            :disabled="busyId === s.id"
            @click="connect(s)"
          >
            {{ busyId === s.id ? '连接中…' : s.hasToken ? '连接' : '登录' }}
          </button>
          <button v-else class="btn" @click="disconnect">断开</button>

          <button class="btn btn-plain danger" title="移除" @click="forget(s)">
            <AppIcon name="close" :size="16" />
          </button>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.servers {
  max-width: 46rem;
  padding: 1.75rem var(--page-pad) 3rem;
}

.head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: var(--sp-4);
  margin-bottom: var(--sp-5);
}

.head p {
  margin: 0.3rem 0 0;
}

.error {
  margin: 0 0 var(--sp-4);
  padding: 0.6rem 0.8rem;
  border-radius: var(--r-sm);
  background: var(--accent-soft);
  color: var(--red);
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.item {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: 0.85rem 1rem;
  border-radius: var(--r-md);
  background: var(--bg-1);
  border: 1px solid var(--separator);
  transition: border-color var(--t-fast) var(--ease), background var(--t-fast) var(--ease);
}

.item:hover {
  background: var(--fill-1);
}

.item.active {
  border-color: var(--accent);
}

.badge {
  display: grid;
  place-items: center;
  width: 2.4rem;
  height: 2.4rem;
  flex: none;
  border-radius: var(--r-sm);
  background: var(--fill-2);
  color: var(--label-2);
}

.badge.on {
  background: var(--accent-soft);
  color: var(--accent);
}

.info {
  flex: 1;
  min-width: 0;
}

.line {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.chip {
  flex: none;
  padding: 0.08rem 0.42rem;
  border-radius: var(--r-xs);
  border: 1px solid var(--separator-strong);
  font-size: 0.6875rem;
  font-weight: 560;
  color: var(--label-2);
}

.chip.on {
  border-color: transparent;
  background: var(--accent-soft);
  color: var(--accent);
}

.actions {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex: none;
}

.actions .btn {
  padding: 0.42rem 0.9rem;
  font-size: 0.875rem;
}

.danger {
  width: 2.1rem;
  height: 2.1rem;
  padding: 0;
  color: var(--label-3);
}

.danger:hover {
  background: var(--red);
  color: #fff;
}

.state {
  display: grid;
  place-items: center;
  gap: 0.5rem;
  padding: 4.5rem 2rem;
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
