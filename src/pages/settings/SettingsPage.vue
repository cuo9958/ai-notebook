<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from "vue";
import {
  Bot,
  Bug,
  Cloud,
  Eye,
  EyeOff,
  FolderCog,
  ImagePlus,
  Info,
  Pencil,
  Plus,
  Trash2,
} from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import AIProviderModal from "@/components/ai/AIProviderModal.vue";
import ImageHostModal from "@/components/image-host/ImageHostModal.vue";
import {
  deleteAIProvider,
  listAIProviders,
  saveAIProvider,
} from "@/services/ai";
import {
  closeDebugLogWindow,
  getAppVersionLabel,
  openDebugLogWindow,
} from "@/services/debug-window";
import { getDebugSettings, updateDebugSettings } from "@/services/debug";
import {
  deleteImageHost,
  listImageHosts,
  saveImageHost,
} from "@/services/image-host";
import {
  getBackupSettings,
  getNoteSyncSettings,
  getNoteWorkspaceSettings,
  updateBackupSettings,
  updateNoteSyncSettings,
  updateNoteWorkspaceSettings,
} from "@/services/note";
import type { AIProviderConfig, AIProviderInput } from "@/types/ai";
import type { ImageHostConfig, ImageHostInput } from "@/types/image-host";

type SettingsSectionId =
  | "sync"
  | "workspace"
  | "ai"
  | "image-host"
  | "debug"
  | "about";

const loading = ref(false);
const saving = ref(false);
const aiSaving = ref(false);
const imageHostSaving = ref(false);
const picking = ref<"workspace" | "backup" | "mail" | "">("");
const message = ref("");
const error = ref("");
const syncKeyVisible = ref(false);
const debugSaving = ref(false);
const versionLabel = ref("");
const activeSection = ref<SettingsSectionId>("sync");
const settingsContentRef = ref<HTMLElement | null>(null);
const sectionRefs = reactive<Partial<Record<SettingsSectionId, HTMLElement>>>(
  {},
);

const providerEditorVisible = ref(false);
const editingProvider = ref<AIProviderConfig | null>(null);
const providers = ref<AIProviderConfig[]>([]);

const imageHostEditorVisible = ref(false);
const editingImageHost = ref<ImageHostConfig | null>(null);
const imageHosts = ref<ImageHostConfig[]>([]);

const navItems: Array<{
  id: SettingsSectionId;
  label: string;
  icon: typeof Cloud;
}> = [
  { id: "sync", label: "同步管理", icon: Cloud },
  { id: "workspace", label: "目录与备份", icon: FolderCog },
  { id: "ai", label: "AI 供应商", icon: Bot },
  { id: "image-host", label: "图床管理", icon: ImagePlus },
  { id: "debug", label: "调试模式", icon: Bug },
  { id: "about", label: "关于", icon: Info },
];

const form = reactive({
  notesRootPath: "",
  backupRootPath: "",
  mailRootPath: "",
  backupRetentionDays: 7,
  noteSyncServerUrl: "",
  noteSyncApiKey: "",
  noteSyncLastSyncedAt: "",
  debugModeEnabled: false,
});

function setSectionRef(id: SettingsSectionId, element: unknown) {
  if (element instanceof HTMLElement) {
    sectionRefs[id] = element;
  }
}

async function scrollToSection(id: SettingsSectionId) {
  activeSection.value = id;
  await nextTick();

  const container = settingsContentRef.value;
  const section = sectionRefs[id];

  if (!container || !section) {
    return;
  }

  const targetTop = section.offsetTop - container.offsetTop;
  const maxScrollTop = Math.max(
    0,
    container.scrollHeight - container.clientHeight,
  );
  const nextScrollTop = Math.min(Math.max(targetTop, 0), maxScrollTop);

  container.scrollTo({
    top: nextScrollTop,
    behavior: "smooth",
  });
}

async function refreshProviders() {
  providers.value = await listAIProviders();
}

async function refreshImageHosts() {
  imageHosts.value = await listImageHosts();
}

async function loadSettings() {
  loading.value = true;
  error.value = "";

  try {
    const [workspace, backup, sync, debug] = await Promise.all([
      getNoteWorkspaceSettings(),
      getBackupSettings(),
      getNoteSyncSettings(),
      getDebugSettings(),
    ]);

    form.notesRootPath = workspace.notesRootPath;
    form.backupRootPath = backup.backupRootPath;
    form.mailRootPath = workspace.mailRootPath;
    form.backupRetentionDays = backup.backupRetentionDays;
    form.noteSyncServerUrl = sync.serverUrl;
    form.noteSyncApiKey = sync.apiKey;
    form.noteSyncLastSyncedAt = sync.lastSyncedAt ?? "";
    form.debugModeEnabled = debug.enabled;
    versionLabel.value = await getAppVersionLabel().catch(() => "");

    await Promise.all([refreshProviders(), refreshImageHosts()]);
  } catch (err) {
    error.value = err instanceof Error ? err.message : "加载设置失败";
  } finally {
    loading.value = false;
  }
}

async function pickDirectory(type: "workspace" | "backup" | "mail") {
  picking.value = type;
  error.value = "";

  try {
    const titleMap = {
      workspace: "选择笔记工作区",
      backup: "选择备份目录",
      mail: "选择邮件目录",
    };

    const selected = await open({
      directory: true,
      multiple: false,
      title: titleMap[type],
    });

    if (typeof selected === "string" && selected.trim()) {
      if (type === "workspace") {
        form.notesRootPath = selected;
      } else if (type === "backup") {
        form.backupRootPath = selected;
      } else {
        form.mailRootPath = selected;
      }
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : "选择目录失败";
  } finally {
    picking.value = "";
  }
}

async function handleDebugModeToggle(event: Event) {
  const nextValue =
    event.target instanceof HTMLInputElement
      ? event.target.checked
      : form.debugModeEnabled;
  form.debugModeEnabled = nextValue;
  debugSaving.value = true;
  error.value = "";
  message.value = "";

  try {
    const debug = await updateDebugSettings(nextValue);
    form.debugModeEnabled = debug.enabled;

    if (debug.enabled) {
      await openDebugLogWindow();
      message.value = "调试模式已开启";
    } else {
      await closeDebugLogWindow();
      message.value = "调试模式已关闭";
    }
  } catch (err) {
    form.debugModeEnabled = !nextValue;
    error.value = err instanceof Error ? err.message : "保存调试模式失败";
  } finally {
    debugSaving.value = false;
  }
}

async function openDebugWindowFromSettings() {
  try {
    await openDebugLogWindow();
  } catch (err) {
    error.value = err instanceof Error ? err.message : "打开日志窗口失败";
  }
}

async function saveSettings() {
  saving.value = true;
  error.value = "";
  message.value = "";

  try {
    const [workspace, backup, sync] = await Promise.all([
      updateNoteWorkspaceSettings(
        form.notesRootPath.trim(),
        form.mailRootPath.trim(),
      ),
      updateBackupSettings(
        form.backupRootPath.trim(),
        Number(form.backupRetentionDays),
      ),
      updateNoteSyncSettings(
        form.noteSyncServerUrl.trim(),
        form.noteSyncApiKey.trim(),
      ),
    ]);

    form.notesRootPath = workspace.notesRootPath;
    form.backupRootPath = backup.backupRootPath;
    form.mailRootPath = workspace.mailRootPath;
    form.backupRetentionDays = backup.backupRetentionDays;
    form.noteSyncServerUrl = sync.serverUrl;
    form.noteSyncApiKey = sync.apiKey;
    form.noteSyncLastSyncedAt = sync.lastSyncedAt ?? "";
    message.value = "设置已保存";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "保存设置失败";
  } finally {
    saving.value = false;
  }
}

function openCreateProvider() {
  editingProvider.value = null;
  providerEditorVisible.value = true;
}

function openEditProvider(provider: AIProviderConfig) {
  editingProvider.value = provider;
  providerEditorVisible.value = true;
}

async function handleSaveProvider(payload: AIProviderInput) {
  aiSaving.value = true;
  error.value = "";
  message.value = "";

  try {
    providers.value = await saveAIProvider(payload);
    providerEditorVisible.value = false;
    editingProvider.value = null;
    message.value = "AI 供应商已保存";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "保存 AI 供应商失败";
  } finally {
    aiSaving.value = false;
  }
}

async function handleDeleteProvider(provider: AIProviderConfig) {
  const confirmed = window.confirm(`确认删除 AI 供应商“${provider.name}”吗？`);
  if (!confirmed) {
    return;
  }

  providers.value = await deleteAIProvider(provider.id);
  message.value = "AI 供应商已删除";
}

function openCreateImageHost() {
  editingImageHost.value = null;
  imageHostEditorVisible.value = true;
}

function openEditImageHost(host: ImageHostConfig) {
  editingImageHost.value = host;
  imageHostEditorVisible.value = true;
}

async function handleSaveImageHost(payload: ImageHostInput) {
  imageHostSaving.value = true;
  error.value = "";
  message.value = "";

  try {
    imageHosts.value = await saveImageHost(payload);
    imageHostEditorVisible.value = false;
    editingImageHost.value = null;
    message.value = "图床配置已保存";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "保存图床配置失败";
  } finally {
    imageHostSaving.value = false;
  }
}

async function handleDeleteImageHost(host: ImageHostConfig) {
  const confirmed = window.confirm(`确认删除图床“${host.name}”吗？`);
  if (!confirmed) {
    return;
  }

  imageHosts.value = await deleteImageHost(host.id);
  message.value = "图床配置已删除";
}

onMounted(() => {
  void loadSettings();
});
</script>

<template>
  <section class="settings-page">
    <AIProviderModal
      v-model="providerEditorVisible"
      :provider="editingProvider"
      :loading="aiSaving"
      @save="handleSaveProvider"
    />

    <ImageHostModal
      v-model="imageHostEditorVisible"
      :host="editingImageHost"
      :loading="imageHostSaving"
      @save="handleSaveImageHost"
    />

    <aside class="settings-sidebar">
      <h2>设置</h2>
      <button
        v-for="item in navItems"
        :key="item.id"
        type="button"
        class="settings-nav-item"
        :class="{ 'settings-nav-item--active': activeSection === item.id }"
        @click="scrollToSection(item.id)"
      >
        <component :is="item.icon" :size="17" :stroke-width="2" />
        <span>{{ item.label }}</span>
      </button>
    </aside>

    <main class="settings-content">
      <div ref="settingsContentRef" class="settings-scroll">
        <section
          :ref="(el) => setSectionRef('sync', el)"
          class="settings-block"
        >
          <header class="settings-block__header">
            <div>
              <p class="settings-block__eyebrow">SYNC</p>
              <h3>同步管理</h3>
            </div>
          </header>

          <div class="settings-grid">
            <label class="settings-field">
              <span>API 地址</span>
              <input
                v-model.trim="form.noteSyncServerUrl"
                type="text"
                placeholder="http://localhost:3000"
                :disabled="loading || saving"
              />
            </label>

            <label class="settings-field">
              <span>校验 Key</span>
              <div class="settings-picker">
                <input
                  v-model.trim="form.noteSyncApiKey"
                  :type="syncKeyVisible ? 'text' : 'password'"
                  autocomplete="off"
                  placeholder="请输入服务端 X-API-Key"
                  :disabled="loading || saving"
                />
                <button
                  type="button"
                  class="picker-btn"
                  :disabled="loading || saving"
                  @click="syncKeyVisible = !syncKeyVisible"
                >
                  <EyeOff v-if="syncKeyVisible" :size="16" :stroke-width="2" />
                  <Eye v-else :size="16" :stroke-width="2" />
                  <span>{{ syncKeyVisible ? "隐藏" : "显示" }}</span>
                </button>
              </div>
            </label>
          </div>
        </section>

        <section
          :ref="(el) => setSectionRef('workspace', el)"
          class="settings-block"
        >
          <header class="settings-block__header">
            <div>
              <p class="settings-block__eyebrow">DIRECTORY</p>
              <h3>目录与备份</h3>
            </div>
          </header>

          <div class="settings-grid">
            <label class="settings-field settings-field--wide">
              <span>笔记工作区</span>
              <div class="settings-picker">
                <input
                  :value="form.notesRootPath"
                  type="text"
                  readonly
                  placeholder="未选择目录"
                />
                <button
                  type="button"
                  class="picker-btn"
                  :disabled="loading || saving || picking === 'workspace'"
                  @click="pickDirectory('workspace')"
                >
                  {{ picking === "workspace" ? "选择中..." : "编辑" }}
                </button>
              </div>
            </label>

            <label class="settings-field">
              <span>备份目录</span>
              <div class="settings-picker">
                <input
                  :value="form.backupRootPath"
                  type="text"
                  readonly
                  placeholder="未选择目录"
                />
                <button
                  type="button"
                  class="picker-btn"
                  :disabled="loading || saving || picking === 'backup'"
                  @click="pickDirectory('backup')"
                >
                  {{ picking === "backup" ? "选择中..." : "编辑" }}
                </button>
              </div>
            </label>

            <label class="settings-field">
              <span>邮件目录</span>
              <div class="settings-picker">
                <input
                  :value="form.mailRootPath"
                  type="text"
                  readonly
                  placeholder="未选择目录"
                />
                <button
                  type="button"
                  class="picker-btn"
                  :disabled="loading || saving || picking === 'mail'"
                  @click="pickDirectory('mail')"
                >
                  {{ picking === "mail" ? "选择中..." : "编辑" }}
                </button>
              </div>
            </label>

            <label class="settings-field">
              <span>备份保留天数</span>
              <input
                v-model.number="form.backupRetentionDays"
                type="number"
                min="1"
                max="30"
                :disabled="loading || saving"
              />
            </label>
          </div>
        </section>

        <section :ref="(el) => setSectionRef('ai', el)" class="settings-block">
          <header class="settings-block__header">
            <div>
              <p class="settings-block__eyebrow">AI GATEWAY</p>
              <h3>AI 供应商</h3>
            </div>

            <button
              type="button"
              class="primary-btn primary-btn--small"
              @click="openCreateProvider"
            >
              <Plus :size="16" :stroke-width="2.2" />
              <span>添加供应商</span>
            </button>
          </header>

          <div v-if="providers.length" class="provider-list">
            <article
              v-for="provider in providers"
              :key="provider.id"
              class="provider-card"
            >
              <div class="provider-card__main">
                <div class="provider-card__title">
                  <Bot :size="16" :stroke-width="2" />
                  <strong>{{ provider.name }}</strong>
                  <span class="provider-card__tag">{{ provider.vendor }}</span>
                  <span
                    v-if="!provider.enabled"
                    class="provider-card__tag provider-card__tag--muted"
                    >已停用</span
                  >
                </div>

                <p>模型：{{ provider.model }}</p>
                <p>Base URL：{{ provider.baseUrl }}</p>
                <p>超时：{{ provider.timeoutMs }} ms</p>
              </div>

              <div class="provider-card__actions">
                <button
                  type="button"
                  class="icon-action"
                  title="编辑 AI 供应商"
                  @click="openEditProvider(provider)"
                >
                  <Pencil :size="16" :stroke-width="2" />
                </button>
                <button
                  type="button"
                  class="icon-action icon-action--danger"
                  title="删除 AI 供应商"
                  @click="handleDeleteProvider(provider)"
                >
                  <Trash2 :size="16" :stroke-width="2" />
                </button>
              </div>
            </article>
          </div>
          <div v-else class="settings-empty">
            <p>
              还没有配置 AI 供应商。支持通义千问、DeepSeek、智谱、豆包以及
              OpenAI Compatible 接口。
            </p>
          </div>
        </section>

        <section
          :ref="(el) => setSectionRef('image-host', el)"
          class="settings-block"
        >
          <header class="settings-block__header">
            <div>
              <p class="settings-block__eyebrow">IMAGE HOST</p>
              <h3>图床管理</h3>
            </div>

            <button
              type="button"
              class="primary-btn primary-btn--small"
              @click="openCreateImageHost"
            >
              <Plus :size="16" :stroke-width="2.2" />
              <span>添加图床</span>
            </button>
          </header>

          <div v-if="imageHosts.length" class="provider-list">
            <article
              v-for="host in imageHosts"
              :key="host.id"
              class="provider-card"
            >
              <div class="provider-card__main">
                <div class="provider-card__title">
                  <ImagePlus :size="16" :stroke-width="2" />
                  <strong>{{ host.name }}</strong>
                  <span class="provider-card__tag">{{ host.vendor }}</span>
                  <span
                    v-if="!host.enabled"
                    class="provider-card__tag provider-card__tag--muted"
                    >已停用</span
                  >
                </div>

                <p>Bucket：{{ host.bucket }}</p>
                <p>区域：{{ host.region }}</p>
                <p>访问域名：{{ host.cdnUrl || "未设置" }}</p>
              </div>

              <div class="provider-card__actions">
                <button
                  type="button"
                  class="icon-action"
                  title="编辑图床"
                  @click="openEditImageHost(host)"
                >
                  <Pencil :size="16" :stroke-width="2" />
                </button>
                <button
                  type="button"
                  class="icon-action icon-action--danger"
                  title="删除图床"
                  @click="handleDeleteImageHost(host)"
                >
                  <Trash2 :size="16" :stroke-width="2" />
                </button>
              </div>
            </article>
          </div>
          <div v-else class="settings-empty">
            <p>
              还没有配置图床。当前支持七牛云和阿里云
              OSS，上传图片时可直接走统一图床接口。
            </p>
          </div>
        </section>

        <section
          :ref="(el) => setSectionRef('debug', el)"
          class="settings-block"
        >
          <header class="settings-block__header">
            <div>
              <p class="settings-block__eyebrow">DEBUG</p>
              <h3>调试模式</h3>
            </div>
            <button
              v-if="form.debugModeEnabled"
              type="button"
              class="secondary-btn"
              :disabled="debugSaving"
              @click="openDebugWindowFromSettings"
            >
              打开日志窗口
            </button>
          </header>

          <!-- 修改此处：添加 settings-field--row 类使其水平排列 -->
          <div class="settings-grid">
            <label
              class="settings-field settings-field--wide settings-field--row"
            >
              <span class="settings-field__label-text">日志采集</span>

              <label class="mini-toggle">
                <input
                  v-model="form.debugModeEnabled"
                  type="checkbox"
                  :disabled="loading || saving || debugSaving"
                  @change="handleDebugModeToggle($event)"
                />
                <span class="mini-toggle__track">
                  <i class="mini-toggle__thumb" />
                </span>
                <!-- 这里添加状态文字放在开关右侧 -->
                <span class="mini-toggle__status">
                  {{ form.debugModeEnabled ? "ON" : "OFF" }}
                </span>
              </label>
            </label>
          </div>
        </section>

        <section
          :ref="(el) => setSectionRef('about', el)"
          class="settings-block"
        >
          <header class="settings-block__header">
            <div>
              <p class="settings-block__eyebrow">ABOUT</p>
              <h3>关于</h3>
            </div>
          </header>

          <div class="settings-grid">
            <label class="settings-field settings-field--wide">
              <span>版本号</span>
              <strong class="settings-version">{{
                versionLabel || "未知版本"
              }}</strong>
            </label>
          </div>
        </section>
      </div>

      <div class="settings-footer">
        <div class="settings-feedback" aria-live="polite">
          <p v-if="message" class="settings-message settings-message--success">
            {{ message }}
          </p>
          <p v-if="error" class="settings-message settings-message--error">
            {{ error }}
          </p>
        </div>

        <button
          type="button"
          class="secondary-btn"
          :disabled="loading || saving"
          @click="loadSettings"
        >
          {{ loading ? "加载中..." : "刷新" }}
        </button>
        <button
          type="button"
          class="primary-btn"
          :disabled="loading || saving"
          @click="saveSettings"
        >
          {{ saving ? "保存中..." : "保存设置" }}
        </button>
      </div>
    </main>
  </section>
</template>

<style scoped>
/* =========================================
   SETTINGS PAGE - Modern Minimalist Blue Theme
   ========================================= */

/* 布局容器 */
.settings-page {
  display: grid;
  grid-template-columns: 240px 1fr; /* 侧边栏收紧一点 */
  height: 100vh;
  background: #fff;
  overflow: hidden;
}

/* 侧边栏：纯白背景，右侧极细边框 */
.settings-sidebar {
  background: #f8fafc;
  border-right: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  padding: 16px 0 0 0;
}

/* 隐藏原来的大标题，更简洁（如需保留可调整 display） */
.settings-sidebar h2 {
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
  padding: 0 20px 16px;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid #e2e8f0;
}

/* 导航项 */
.settings-nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 10px 20px;
  border: none;
  background: transparent;
  color: #64748b;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: left;
}

.settings-nav-item:hover {
  background: #f1f5f9;
  color: #0f172a;
}

/* 选中态：淡蓝背景 + 蓝色文字 + 左侧蓝色指示条 */
.settings-nav-item--active {
  background: #eff6ff;
  color: #2563eb;
  font-weight: 600;
  border-right: 2px solid #2563eb;
}

/* 主内容区 */
.settings-content {
  background: #fff;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* 滚动区域：增加内边距 */
.settings-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 20px 32px;
}

/* 设置区块：去除厚重卡片设计，采用无框段落布局 */
.settings-block {
  margin-bottom: 32px;
  border: none !important;
  border-radius: 0 !important;
  background: transparent !important;
  padding: 0 !important;
  box-shadow: none !important;
}

/* 区块标题 */
.settings-block__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid #e2e8f0;
}

.settings-block__eyebrow {
  color: #2563eb;
  font-size: 10px;
  letter-spacing: 1px;
  margin: 0 0 4px;
  text-transform: uppercase;
  font-weight: 700;
}

.settings-block h3 {
  color: #0f172a;
  font-size: 18px;
  font-weight: 650;
  margin: 0;
}

/* 栅格表单布局 */
.settings-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px 20px;
}

.settings-field--wide {
  grid-column: span 2;
}
.settings-field--row {
  display: flex !important;
  flex-direction: row !important;
  align-items: center;
  justify-content: space-between;
  flex-wrap: nowrap !important;
  min-height: 40px; /* 固定高度 */
  width: 200px;
}
.settings-field__label-text {
  color: #0f172a;
  font-size: 14px;
  font-weight: 600;
  margin-right: 12px;
}
.mini-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  margin-left: auto; /* 靠右对齐 */
  user-select: none;
}
/* 隐藏原生 checkbox */
.mini-toggle input {
  display: none;
}

/* 轨道：小尺寸的椭圆 */
.mini-toggle__track {
  width: 32px;
  height: 18px;
  background: #cbd5e1;
  border-radius: 20px;
  display: inline-block;
  position: relative;
  transition: background 0.2s ease;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.1);
}
/* 滑块：白色小圆点 */
.mini-toggle__thumb {
  width: 14px;
  height: 14px;
  background: #fff;
  border-radius: 50%;
  position: absolute;
  top: 2px;
  left: 2px;
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}

/* 状态文字 */
.mini-toggle__status {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: #64748b;
}

/* 激活态样式 */
.mini-toggle input:checked + .mini-toggle__track {
  background: #2563eb;
}

.mini-toggle input:checked + .mini-toggle__track .mini-toggle__thumb {
  transform: translateX(14px);
}

.mini-toggle input:checked ~ .mini-toggle__status {
  color: #2563eb;
}
/* 表单项样式 */
.settings-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.settings-field span {
  color: #0f172a;
  font-size: 13px;
  font-weight: 500;
}

.settings-field input,
.settings-field select {
  width: 100%;
  height: 36px;
  padding: 0 12px;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #fff;
  color: #0f172a;
  font-size: 14px;
  transition: all 0.2s ease;
  outline: none;
}

.settings-field input:focus,
.settings-field select:focus {
  border-color: #2563eb;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
}

.settings-field input[readonly] {
  background-color: #f8fafc !important; /* 浅灰背景，增强存在感 */
  color: #475569;
  border: 1px solid #d1d5db !important; /* 强制显示灰色边框，不再透明 */
  border-right-width: 0; /* 与紧贴的按钮衔接 */
}

/* 底部按钮栏 */
.settings-footer {
  background: #fff;
  border-top: 1px solid #e2e8f0;
  padding: 12px 32px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

/* 统一按钮样式 */
.primary-btn,
.secondary-btn,
.picker-btn {
  height: 36px;
  padding: 0 16px;
  border: 1px solid transparent;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.2s ease;
}

.primary-btn {
  background: #2563eb;
  color: #fff;
  box-shadow: 0 2px 4px rgba(37, 99, 235, 0.2);
}

.primary-btn:hover:not(:disabled) {
  background: #1d4ed8;
  transform: translateY(-1px);
}

.secondary-btn {
  background: #fff;
  border-color: #e2e8f0;
  color: #0f172a;
}

.secondary-btn:hover:not(:disabled) {
  background: #f1f5f9;
  color: #2563eb;
  border-color: #2563eb;
}

.picker-btn {
  width: auto;
  padding: 0 12px;
  color: #64748b;
  background: #fff;
  border: 1px solid #e2e8f0;
  font-size: 12px;
  white-space: nowrap;
}

.picker-btn:hover {
  border-color: #2563eb;
  color: #2563eb;
  background: #eff6ff;
}
/* 输入框和按钮的容器布局 */
.settings-picker {
  display: flex !important;
  align-items: center;
  width: 100%;
  height: 36px; /* 统一高度 */
  border: 1px solid #e2e8f0; /* 统一的浅灰色边框 */
  border-radius: 6px; /* 统一圆角 */
  overflow: hidden; /* 裁剪内部溢出的角 */
  background: #fff;
  transition: all 0.2s ease;
}
/* 当鼠标悬停在任何一部分时，边框稍微加深 */
.settings-picker:hover {
  border-color: #cbd5e1;
}
/* 输入框样式重置 */
.settings-picker input {
  flex: 1;
  height: 100%;
  padding: 0 12px;
  border: none;
  border-radius: 0 !important; /* 关键：强制去除圆角 */
  background: transparent !important; /* 去除背景色干扰 */
  outline: none !important;
  font-size: 13px;
  color: #334155;
}
.settings-picker input::placeholder {
  color: #94a3b8;
}

.settings-picker input:focus {
  box-shadow: none;
}
/* 当输入框获得焦点时，外层边框变蓝 */
.settings-picker:has(input:focus),
.settings-picker:focus-within {
  border-color: #2563eb;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
}
/* 按钮样式重置 */
.settings-picker .picker-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: auto;
  height: 100%;
  padding: 0 14px;
  border: none;
  border-radius: 0 !important; /* 强制去除圆角 */
  border-left: 1px solid #e2e8f0; /* 分割线 */
  background: #f8fafc; /* 按钮区域背景微灰，区分开来 */
  color: #64748b;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}
.settings-picker .picker-btn:hover {
  background: #eff6ff;
  color: #2563eb;
}
/* 让开关组件靠左显示，并对齐文字中线 */
.debug-mode-switch {
  justify-content: flex-start !important;
  align-items: center !important;
  margin-top: 4px;
  width: auto;
}

/* 增强未选中/未开启状态下的可见性 */
.settings-toggle__track {
  background-color: #e2e8f0; /* 更明显的灰色 */
  border: 1px solid #cbd5e1; /* 增加边框 */
  opacity: 1; /* 确保不透明 */
}

/* 滑块 (Thumb) 样式优化，确保清晰可见 */
.settings-toggle__thumb {
  width: 18px;
  height: 18px;
  background: #ffffff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3); /* 深色阴影增强立体感 */
  border: 1px solid rgba(255, 255, 255, 0.2);
}
/* 列表/卡片样式重构：扁平化 */
.provider-list {
  display: grid;
  gap: 12px;
}

.provider-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #fff;
  transition: all 0.2s ease;
}

.provider-card:hover {
  border-color: #2563eb;
  background: #eff6ff;
}

.provider-card__title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.provider-card__title strong {
  color: #0f172a;
}

.provider-card__tag {
  background: #f1f5f9;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  color: #64748b;
}

.provider-card__actions {
  display: flex;
  gap: 8px;
}

.icon-action {
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  border: 1px solid transparent;
  background: transparent;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s ease;
}

.icon-action:hover {
  background: #fff;
  border: 1px solid #e2e8f0;
  color: #2563eb;
}

.icon-action--danger:hover {
  color: #ef4444;
  border-color: #ef4444;
  background: #fee2e2;
}

/* 开关控件微调 */
.settings-toggle input:checked + .settings-toggle__track {
  background: #2563eb;
}

/* 响应式适配 */
@media (max-width: 800px) {
  .settings-page {
    grid-template-columns: 1fr !important;
  }
  .settings-sidebar {
    padding: 0;
    height: auto;
    flex-direction: row;
    overflow-x: auto;
    border-right: none;
    border-bottom: 1px solid #e2e8f0;
    align-items: center;
  }
  .settings-sidebar h2 {
    display: none;
  }
  .settings-nav-item {
    padding: 12px;
    border-bottom: 2px solid transparent;
    border-right: none;
  }
  .settings-nav-item--active {
    border-bottom: 2px solid #2563eb;
    background: transparent;
  }
  .settings-grid {
    grid-template-columns: 1fr;
  }
}
</style>
