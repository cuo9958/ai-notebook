<script setup lang="ts">
import { computed, watch } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { Mail, NotebookText, PenLine, Settings } from '@lucide/vue'
import { logFrontendAction } from '@/services/debug'

const route = useRoute()

const navItems = computed(() => [
  { en: 'NOTE', zh: '笔记', to: '/notes', icon: NotebookText },
  { en: 'WRITE', zh: '写作', to: '/writing', icon: PenLine },
  { en: 'MAIL', zh: '邮件', to: '/mail', icon: Mail },
])

const settingNavItem = computed(() => ({ en: 'SETTING', zh: '设置', to: '/settings', icon: Settings }))

watch(
  () => route.fullPath,
  (value) => {
    logFrontendAction('route.change', value)
  },
  { immediate: true },
)
</script>

<template>
  <div class="app-shell">
    <aside class="app-shell__rail">
      <nav class="app-shell__nav" aria-label="主导航">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="app-shell__nav-link"
          active-class="app-shell__nav-link--active"
          :title="`${item.en} ${item.zh}`"
        >
          <component :is="item.icon" :size="17" :stroke-width="2" />
          <span class="app-shell__nav-text">
            <span class="app-shell__nav-en">{{ item.en }}</span>
            <span class="app-shell__nav-zh">{{ item.zh }}</span>
          </span>
        </RouterLink>
      </nav>

      <RouterLink
        :to="settingNavItem.to"
        class="app-shell__nav-link app-shell__nav-link--bottom"
        active-class="app-shell__nav-link--active"
        :title="`${settingNavItem.en} ${settingNavItem.zh}`"
      >
        <component :is="settingNavItem.icon" :size="17" :stroke-width="2" />
        <span class="app-shell__nav-text">
          <span class="app-shell__nav-en">{{ settingNavItem.en }}</span>
          <span class="app-shell__nav-zh">{{ settingNavItem.zh }}</span>
        </span>
      </RouterLink>
    </aside>

    <main class="app-shell__body">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100vh;
  min-height: 100vh;
  background: var(--paper);
  overflow: hidden;
}

.app-shell__rail {
  display: flex;
  width: 58px;
  flex: 0 0 58px;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  border-right: 1px solid rgba(15, 23, 42, 0.08);
  background: rgba(255, 255, 255, 0.96);
  padding: 14px 8px;
}

.app-shell__nav {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  width: 100%;
}

.app-shell__nav-link {
  position: relative;
  display: flex;
  width: 32px;
  min-height: 32px;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: 10px;
  color: #667085;
}

.app-shell__nav-link:hover {
  background: #f4f7ff;
  color: var(--accent);
}

.app-shell__nav-link--bottom {
  flex: 0 0 auto;
}

.app-shell__nav-text {
  position: absolute;
  left: calc(100% + 10px);
  z-index: 20;
  display: none;
  min-width: 92px;
  flex-direction: column;
  gap: 2px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: #fff;
  padding: 8px 10px;
  box-shadow: 0 14px 30px rgba(15, 23, 42, 0.12);
}

.app-shell__nav-link:hover .app-shell__nav-text {
  display: flex;
}

.app-shell__nav-en {
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 800;
}

.app-shell__nav-zh {
  color: #344054;
  font-size: 12px;
  font-weight: 700;
}

.app-shell__nav-link--active {
  border-color: transparent;
  background: rgba(37, 99, 235, 0.075);
  color: var(--accent);
}

.app-shell__body {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
</style>
