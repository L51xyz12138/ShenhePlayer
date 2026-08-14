import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as api from '@/api'
import { setImageContext } from '@/api/images'
import type { BaseItem, SessionInfo } from '@/types'

export const useSessionStore = defineStore('session', () => {
  const session = ref<SessionInfo | null>(null)
  const views = ref<BaseItem[]>([])
  const restoring = ref(true)
  const error = ref('')

  const isAuthed = computed(() => session.value !== null)

  function apply(info: SessionInfo | null) {
    session.value = info
    if (info) {
      setImageContext(info.serverUrl, info.token)
    } else {
      setImageContext('', '')
      views.value = []
    }
  }

  /** 启动时用保存的 token 静默登录 */
  async function restore(): Promise<boolean> {
    restoring.value = true
    try {
      apply(await api.restoreSession())
      return isAuthed.value
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      restoring.value = false
    }
  }

  async function signIn(url: string, username: string, password: string, allowInvalidCerts: boolean) {
    error.value = ''
    apply(await api.login(url, username, password, allowInvalidCerts))
  }

  async function signOut() {
    await api.logout()
    apply(null)
  }

  async function loadViews() {
    if (!isAuthed.value) return
    views.value = await api.getViews()
  }

  return { session, views, restoring, error, isAuthed, restore, signIn, signOut, loadViews, apply }
})
