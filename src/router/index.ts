import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: '/notes',
    },
    {
      path: '/notes',
      name: 'notes',
      component: () => import('@/pages/notes/NotesPage.vue'),
    },
    {
      path: '/writing',
      name: 'writing',
      component: () => import('@/pages/writing/WritingPage.vue'),
    },
    {
      path: '/mail',
      name: 'mail',
      component: () => import('@/pages/mail/MailPage.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/pages/settings/SettingsPage.vue'),
    },
    {
      path: '/debug',
      name: 'debug',
      component: () => import('@/pages/debug/DebugLogsPage.vue'),
    },
  ],
})

export default router
