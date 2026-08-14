import { createRouter, createWebHashHistory } from 'vue-router'
import { useSessionStore } from '@/stores/session'

// 没有「必须登录」这回事：主界面随时可进，没连服务器的页面自己显示空状态。
// 需要连接的页面用 meta.needsServer 标记，未连接时引导到服务器列表。
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: () => import('@/views/HomeView.vue') },
    { path: '/servers', name: 'servers', component: () => import('@/views/ServersView.vue') },
    { path: '/servers/add', name: 'add-server', component: () => import('@/views/LoginView.vue') },
    {
      path: '/library/:id',
      name: 'library',
      component: () => import('@/views/LibraryView.vue'),
      props: true,
      meta: { needsServer: true },
    },
    {
      path: '/item/:id',
      name: 'item',
      component: () => import('@/views/DetailView.vue'),
      props: true,
      meta: { needsServer: true },
    },
    { path: '/search', name: 'search', component: () => import('@/views/SearchView.vue') },
    { path: '/settings', name: 'settings', component: () => import('@/views/SettingsView.vue') },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
  scrollBehavior: () => ({ top: 0 }),
})

router.beforeEach((to) => {
  const session = useSessionStore()
  if (to.meta.needsServer && !session.isAuthed) return { name: 'servers' }
  return true
})

export default router
