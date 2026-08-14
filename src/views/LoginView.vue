<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import * as api from '@/api'
import { useSessionStore } from '@/stores/session'
import AppIcon from '@/components/AppIcon.vue'
import type { SystemInfo } from '@/types'

const router = useRouter()
const route = useRoute()
const session = useSessionStore()

const url = ref('')
const username = ref('')
const password = ref('')
const allowInvalidCerts = ref(false)

const probing = ref(false)
const submitting = ref(false)
const error = ref('')
const serverInfo = ref<SystemInfo | null>(null)
/** 从服务器列表点「登录」进来时，预填那台服务器的信息 */
const editingId = ref('')

onMounted(async () => {
  const id = route.query.id
  if (typeof id !== 'string') return

  const known = (await api.savedServers()).find((s) => s.id === id)
  if (!known) return

  editingId.value = known.id
  url.value = known.url
  username.value = known.username
  allowInvalidCerts.value = known.allowInvalidCerts
  void probe()
})

/** 地址填完先探一下，能提前发现写错，不用等到提交 */
async function probe() {
  const value = url.value.trim()
  if (!value) return
  probing.value = true
  error.value = ''
  serverInfo.value = null
  try {
    serverInfo.value = await api.connectServer(value, allowInvalidCerts.value)
  } catch (e) {
    error.value = String(e)
  } finally {
    probing.value = false
  }
}

async function submit() {
  if (submitting.value) return
  error.value = ''

  if (!url.value.trim()) {
    error.value = '请填写服务器地址'
    return
  }
  if (!username.value.trim()) {
    error.value = '请填写用户名'
    return
  }

  submitting.value = true
  try {
    await session.signIn(
      url.value.trim(),
      username.value.trim(),
      password.value,
      allowInvalidCerts.value,
    )
    await session.loadViews()
    await router.replace({ name: 'home' })
  } catch (e) {
    error.value = String(e)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="login">
    <form class="panel" @submit.prevent="submit">
      <div class="brand">
        <span class="mark" />
        <h1 class="t-title-1">{{ editingId ? '重新登录' : '添加服务器' }}</h1>
        <p class="t-subhead dim">填写 Emby 服务器地址与账户</p>
      </div>

      <div class="fields">
        <label class="field">
          <span class="t-footnote dim">服务器地址</span>
          <input
            v-model="url"
            class="input"
            type="text"
            placeholder="http://192.168.1.10:8096"
            spellcheck="false"
            autocomplete="off"
            @blur="probe"
          />
        </label>

        <label class="field">
          <span class="t-footnote dim">用户名</span>
          <input
            v-model="username"
            class="input"
            type="text"
            spellcheck="false"
            autocomplete="username"
          />
        </label>

        <label class="field">
          <span class="t-footnote dim">密码</span>
          <input
            v-model="password"
            class="input"
            type="password"
            autocomplete="current-password"
            placeholder="没有密码可留空"
          />
        </label>
      </div>

      <label class="toggle">
        <input v-model="allowInvalidCerts" type="checkbox" />
        <span class="t-footnote">允许自签名 HTTPS 证书</span>
      </label>

      <Transition name="fade" mode="out-in">
        <p v-if="probing" class="status t-caption dim-3">正在连接…</p>
        <p v-else-if="error" class="status t-caption bad">{{ error }}</p>
        <p v-else-if="serverInfo" class="status t-caption ok">
          <AppIcon name="check-circle" :size="13" />
          {{ serverInfo.serverName }} · Emby {{ serverInfo.version }}
        </p>
      </Transition>

      <button class="btn btn-primary btn-lg submit" type="submit" :disabled="submitting">
        {{ submitting ? '登录中…' : '登录' }}
      </button>

      <RouterLink :to="{ name: 'servers' }" class="t-caption dim-3 to-settings">
        返回服务器列表
      </RouterLink>
    </form>
  </div>
</template>

<style scoped>
.login {
  display: grid;
  place-items: center;
  min-height: 100%;
  padding: var(--sp-6) var(--sp-4);
}

/* 一块干净的卡片就够了，不需要光晕和渐变来撑场面 */
.panel {
  width: min(24rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
}

.brand {
  text-align: center;
}

.mark {
  position: relative;
  display: inline-block;
  width: 52px;
  height: 52px;
  margin-bottom: var(--sp-4);
  border-radius: 15px;
  background: var(--label);
}

.mark::after {
  content: '';
  position: absolute;
  left: 19px;
  top: 15px;
  border-left: 18px solid var(--bg);
  border-top: 11px solid transparent;
  border-bottom: 11px solid transparent;
}

.brand p {
  margin: var(--sp-2) 0 0;
}

/* ---- 表单 ---- */
.fields {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--label-2);
}

.toggle input {
  width: 15px;
  height: 15px;
  accent-color: var(--accent);
}

.status {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.3rem;
  min-height: 1.2rem;
  margin: 0;
  text-align: center;
  line-height: 1.4;
}

.status.ok {
  color: var(--green);
}

.status.bad {
  color: var(--red);
}

.submit {
  width: 100%;
}

.to-settings {
  text-align: center;
  transition: color var(--t-fast) var(--ease);
}

.to-settings:hover {
  color: var(--label-2);
}
</style>
