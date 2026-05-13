<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { fetch as httpFetch } from "@tauri-apps/plugin-http";
import { convertFileSrc } from "@tauri-apps/api/core";
import { marked } from "marked";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import markdown from "highlight.js/lib/languages/markdown";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import {
  Bot,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  FolderTree,
  Link2,
  Pencil,
  Plus,
  Save,
  Send,
  Sparkles,
  Type,
} from "@lucide/vue";
import NoteAlertDialog from "@/components/notes/NoteAlertDialog.vue";
import TextContextMenu from "@/components/common/TextContextMenu.vue";
import VditorEditor from "@/components/common/VditorEditor.vue";
import type { VditorToolbarAction } from "@/components/common/VditorEditor.vue";
import WritingMaterialModal from "@/components/writing/WritingMaterialModal.vue";
import WritingOutlineDialog from "@/components/writing/WritingOutlineDialog.vue";
import WritingPublishDialog from "@/components/writing/WritingPublishDialog.vue";
import { listImageHosts, uploadImageWithHost } from "@/services/image-host";
import { invokeAIChatStream, listAIProviders } from "@/services/ai";
import { listNotes } from "@/services/note";
import {
  addWritingMaterial,
  createWritingProject,
  deleteWritingMaterial,
  deleteWritingProject,
  getWritingProject,
  listWritingProjects,
  publishWritingProject,
  saveWritingImage,
  saveWritingProject,
  updateWritingMaterial,
} from "@/services/writing";
import type { NoteTreeNode } from "@/types/note";
import type { AIProviderConfig } from "@/types/ai";
import type {
  WritingMaterialInput,
  WritingProjectDetail,
  WritingProjectSummary,
  WritingMaterial,
  WritingSection,
} from "@/types/writing";

const LAST_PUBLISH_DIRECTORY_KEY = "ai-markdown.writing.last-publish-directory";
const batchUploadIcon =
  '<svg viewBox="0 0 24 24"><path d="M12 3v10m0-10 4 4m-4-4-4 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><path d="M4 14v4a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M7 10h10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>';

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("markdown", markdown);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("html", xml);

const markdownRenderer = new marked.Renderer();
markdownRenderer.code = ({ text, lang }) => {
  const language = lang && hljs.getLanguage(lang) ? lang : "plaintext";
  const highlighted =
    language === "plaintext"
      ? escapeHtml(text)
      : hljs.highlight(text, { language }).value;

  return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
};
markdownRenderer.image = ({ href, title, text }) => {
  const source = resolveMarkdownImageSource(href ?? "");
  const safeAlt = escapeHtml(text ?? "");
  const safeSource = escapeHtml(source);
  const titleAttr = title ? ` title="${escapeHtml(title)}"` : "";
  return `<img src="${safeSource}" alt="${safeAlt}"${titleAttr}>`;
};

marked.setOptions({
  breaks: true,
  gfm: true,
  renderer: markdownRenderer,
});

const loadingProjects = ref(false);
const loadingProject = ref(false);
const saving = ref(false);
const publishing = ref(false);
const materialSaving = ref(false);
const dirty = ref(false);
const message = ref("");
const error = ref("");

const projects = ref<WritingProjectSummary[]>([]);
const projectDraft = ref<WritingProjectDetail | null>(null);
const activeProjectId = ref("");
const noteDirectoryOptions = ref<Array<{ label: string; value: string }>>([]);
const projectRailCollapsed = ref(false);

const materialModalVisible = ref(false);
const editingMaterial = ref<WritingMaterial | null>(null);
const outlineDialogVisible = ref(false);
const editingOutlineId = ref("");
const activeOutlineId = ref("");
const aiProviders = ref<AIProviderConfig[]>([]);
const aiProviderId = ref("");
const publishDialogVisible = ref(false);
const publishMenuVisible = ref(false);
const pendingPublishAfterSelect = ref(false);
const sectionEditorVisible = ref(false);
const editingSectionId = ref("");
const sectionEditContent = ref("");
const draftPanelBodyRef = ref<HTMLElement | null>(null);
const sectionImageUploadingCount = ref(0);
const generatingOutlineIds = reactive<Record<string, boolean>>({});
const materialContextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  materialId: "",
});
const projectContextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  projectId: "",
});
const outlineContextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  outlineId: "",
});
const sectionEditorActions: VditorToolbarAction[] = [
  {
    name: "batch-upload-section-images",
    tip: "批量上传本地图片",
    icon: batchUploadIcon,
    click: uploadAllSectionLocalImages,
  },
];

const sectionElements = new Map<string, HTMLElement>();
let projectAutoSaveTimer: number | null = null;

const dialogState = reactive({
  visible: false,
  mode: "prompt" as "prompt" | "confirm",
  title: "",
  message: "",
  placeholder: "",
  confirmText: "确认",
  cancelText: "取消",
  initialValue: "",
  danger: false,
  action: "" as
    | "create-project"
    | "edit-project"
    | "delete-project"
    | "delete-outline"
    | "",
  targetId: "",
});

const currentProject = computed(() => projectDraft.value);
const hasProject = computed(() => Boolean(currentProject.value));
const publishDirectoryPath = computed(
  () =>
    currentProject.value?.publishDirectoryPath ||
    localStorage.getItem(LAST_PUBLISH_DIRECTORY_KEY) ||
    "",
);
const publishDirectoryLabel = computed(() => {
  const match = noteDirectoryOptions.value.find(
    (item) => item.value === publishDirectoryPath.value,
  );
  if (match) {
    return match.label;
  }

  if (!publishDirectoryPath.value) {
    return "未选择目录";
  }

  return (
    publishDirectoryPath.value.split(/[\\/]/).filter(Boolean).at(-1) ||
    publishDirectoryPath.value
  );
});
const generatedSectionCount = computed(
  () =>
    currentProject.value?.sections.filter((section) => section.content.trim())
      .length ?? 0,
);
const editingOutline = computed(
  () =>
    currentProject.value?.outlines.find(
      (item) => item.id === editingOutlineId.value,
    ) ?? null,
);
const previousOutline = computed(() => {
  if (!currentProject.value || !editingOutlineId.value) {
    return null;
  }

  const index = currentProject.value.outlines.findIndex(
    (item) => item.id === editingOutlineId.value,
  );
  if (index <= 0) {
    return null;
  }

  return currentProject.value.outlines[index - 1] ?? null;
});

function isKnownNoteDirectory(path: string) {
  return noteDirectoryOptions.value.some((item) => item.value === path);
}

function cloneProject(project: WritingProjectDetail) {
  return JSON.parse(JSON.stringify(project)) as WritingProjectDetail;
}

function buildSectionFromOutline(
  outline: { id: string; content: string },
  index: number,
  existing?: WritingSection | null,
): WritingSection {
  return {
    id: outline.id,
    title: getOutlineSummary(outline.content, index),
    content: existing?.content ?? "",
    selected: true,
  };
}

function syncProjectSections(project: WritingProjectDetail) {
  const existingById = new Map(project.sections.map((item) => [item.id, item]));
  const existingByIndex = [...project.sections];

  project.sections = project.outlines.map((outline, index) => {
    const existing =
      existingById.get(outline.id) ?? existingByIndex[index] ?? null;
    return buildSectionFromOutline(outline, index, existing);
  });
}

function resetFeedback() {
  message.value = "";
  error.value = "";
}

function markDirty() {
  dirty.value = true;
  message.value = "";
}

function clearProjectAutosaveTimer() {
  if (projectAutoSaveTimer !== null) {
    window.clearTimeout(projectAutoSaveTimer);
    projectAutoSaveTimer = null;
  }
}

function scheduleProjectAutosave(delay = 700) {
  if (!currentProject.value) {
    return;
  }

  clearProjectAutosaveTimer();
  projectAutoSaveTimer = window.setTimeout(() => {
    projectAutoSaveTimer = null;
    if (
      !currentProject.value ||
      !dirty.value ||
      saving.value ||
      publishing.value
    ) {
      return;
    }

    void saveCurrentProject(false);
  }, delay);
}

function assignProject(project: WritingProjectDetail) {
  projectDraft.value = cloneProject(project);
  if (projectDraft.value) {
    syncProjectSections(projectDraft.value);
  }
  activeProjectId.value = project.id;
  dirty.value = false;
  publishMenuVisible.value = false;
  materialContextMenu.visible = false;
  projectContextMenu.visible = false;
  outlineContextMenu.visible = false;
  outlineDialogVisible.value = false;
  editingOutlineId.value = "";
  activeOutlineId.value = projectDraft.value?.outlines[0]?.id ?? "";
}

function getProjectSummaryLabel(project: WritingProjectSummary) {
  return project.title.trim() || "未命名工程";
}

function getProjectInitial(project: WritingProjectSummary) {
  return getProjectSummaryLabel(project).trim().charAt(0).toUpperCase() || "写";
}

function getOutlineSummary(content: string, index?: number) {
  const compact = content.replace(/\s+/g, " ").trim();
  if (compact) {
    return compact.slice(0, 36);
  }

  if (typeof index === "number") {
    return "大纲 " + (index + 1);
  }

  return "未填写内容";
}

function getSectionForOutline(outlineId: string) {
  return (
    currentProject.value?.sections.find((item) => item.id === outlineId) ?? null
  );
}

function escapeHtml(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function getFileExtension(file: File) {
  const nameExtension = file.name.split(".").pop()?.toLowerCase();
  if (nameExtension && nameExtension !== file.name.toLowerCase()) {
    return nameExtension;
  }

  const typeExtension = file.type.split("/").pop()?.toLowerCase();
  if (typeExtension === "jpeg") {
    return "jpg";
  }

  if (typeExtension === "svg+xml") {
    return "svg";
  }

  return typeExtension || "png";
}

function buildClipboardImageName(file: File) {
  const extension = getFileExtension(file);
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  return `clipboard-${timestamp}.${extension}`;
}

function normalizeWindowsPath(value: string) {
  return value.replace(/\//g, "\\");
}

function isLocalImagePath(value: string) {
  return /^[a-zA-Z]:[\\/]/.test(value) || value.startsWith("\\\\");
}

function fileUriToLocalPath(value: string) {
  const withoutPrefix = decodeURIComponent(value.replace(/^file:\/\//, ""));

  if (/^\/[a-zA-Z]:\//.test(withoutPrefix)) {
    return withoutPrefix.slice(1);
  }

  return withoutPrefix;
}

function resolveMarkdownImageSource(value: string) {
  const trimmed = value.trim().replace(/^<|>$/g, "");

  if (!trimmed) {
    return trimmed;
  }

  if (
    trimmed.startsWith("http://") ||
    trimmed.startsWith("https://") ||
    trimmed.startsWith("data:")
  ) {
    return trimmed;
  }

  if (trimmed.startsWith("file://")) {
    return convertFileSrc(normalizeWindowsPath(fileUriToLocalPath(trimmed)));
  }

  if (isLocalImagePath(trimmed)) {
    return convertFileSrc(normalizeWindowsPath(trimmed));
  }

  return trimmed;
}

function isLocalMarkdownImageSource(value: string) {
  const trimmed = value.trim().replace(/^<|>$/g, "");

  if (!trimmed) {
    return false;
  }

  if (
    trimmed.startsWith("http://") ||
    trimmed.startsWith("https://") ||
    trimmed.startsWith("data:")
  ) {
    return false;
  }

  return trimmed.startsWith("file://") || isLocalImagePath(trimmed);
}

function renderSectionMarkdown(content: string) {
  const source = content.trim();
  if (!source) {
    return "";
  }

  return String(marked.parse(source));
}

function replaceSectionEditorImageUrl(sourceUrl: string, targetUrl: string) {
  if (!sourceUrl || !targetUrl || sourceUrl === targetUrl) {
    return;
  }

  const wrappedSource = `<${sourceUrl}>`;
  const withWrappedReplacement = sectionEditContent.value.replace(
    wrappedSource,
    targetUrl,
  );
  sectionEditContent.value = withWrappedReplacement.replace(
    sourceUrl,
    targetUrl,
  );
}

async function fileFromWritingImageSource(source: string) {
  const resolvedSource = resolveMarkdownImageSource(source);
  const response = await fetch(resolvedSource);

  if (!response.ok) {
    throw new Error(`读取图片失败：${response.status}`);
  }

  const blob = await response.blob();
  const normalizedSource = source.trim().replace(/^<|>$/g, "");
  const sourceName =
    normalizedSource.split("/").pop()?.split("?")[0] || "image";
  const extensionFromName = sourceName.includes(".")
    ? sourceName
    : `${sourceName}.png`;
  return new File([blob], extensionFromName, {
    type: blob.type || "image/png",
  });
}

function extractLocalMarkdownImageSources(content: string) {
  const matches = content.matchAll(/!\[[^\]]*]\(([^)\r\n]+)\)/g);
  const sources: string[] = [];
  const seen = new Set<string>();

  for (const match of matches) {
    const source = match[1]?.trim();
    if (!source || seen.has(source) || !isLocalMarkdownImageSource(source)) {
      continue;
    }

    seen.add(source);
    sources.push(source);
  }

  return sources;
}

async function uploadAllSectionLocalImages() {
  const localSources = extractLocalMarkdownImageSources(
    sectionEditContent.value,
  );
  if (!localSources.length) {
    error.value = "当前段落中没有可上传的本地图片";
    return;
  }

  sectionImageUploadingCount.value += localSources.length;
  error.value = "";

  try {
    for (const source of localSources) {
      try {
        const file = await fileFromWritingImageSource(source);
        const uploaded = await uploadWritingClipboardImage(file);

        if (!uploaded?.url) {
          throw new Error("请先在设置中启用一个图床");
        }

        replaceSectionEditorImageUrl(source, uploaded.url);
      } finally {
        sectionImageUploadingCount.value = Math.max(
          0,
          sectionImageUploadingCount.value - 1,
        );
      }
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : "批量上传图片失败";
  }
}

async function uploadWritingClipboardImage(file: File) {
  const hosts = await listImageHosts();
  const host = hosts.find((item) => item.enabled);

  if (!host) {
    return null;
  }

  const uploadFile = file.name
    ? file
    : new File([file], buildClipboardImageName(file), {
        type: file.type || "image/png",
      });

  return uploadImageWithHost(host.id, uploadFile);
}

async function saveWritingClipboardImageLocally(file: File) {
  if (!currentProject.value) {
    throw new Error("当前没有打开的写作工程");
  }

  const fileName = file.name || buildClipboardImageName(file);
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
  return saveWritingImage(currentProject.value.id, fileName, bytes);
}

async function handleSectionVditorUpload(files: File[]) {
  if (!currentProject.value || !sectionEditorVisible.value) {
    return null;
  }

  const imageFiles = files.filter((file) => file.type.startsWith("image/"));
  if (!imageFiles.length) {
    return null;
  }

  error.value = "";
  sectionImageUploadingCount.value += imageFiles.length;
  const markdownImages: string[] = [];

  for (const imageFile of imageFiles) {
    try {
      const alt = imageFile.name
        ? imageFile.name.replace(/\.[^.]+$/, "")
        : "image";
      const localImage = await saveWritingClipboardImageLocally(imageFile);
      let imageUrl = `<${localImage.markdownPath}>`;

      try {
        const uploaded = await uploadWritingClipboardImage(imageFile);
        if (uploaded?.url) {
          imageUrl = uploaded.url;
        }
      } catch (err) {
        error.value =
          err instanceof Error ? err.message : "上传失败，已保留本地引用";
      }

      markdownImages.push(`![${alt}](${imageUrl})`);
    } catch (err) {
      error.value = err instanceof Error ? err.message : "粘贴图片失败";
    }
  }

  sectionImageUploadingCount.value = Math.max(
    0,
    sectionImageUploadingCount.value - imageFiles.length,
  );
  return markdownImages.length ? `\n${markdownImages.join("\n")}\n` : null;
}

function resolveOutlineReferencedMaterials(outlineContent: string) {
  const materials = currentProject.value?.materials ?? [];
  return materials.filter((material) => {
    const title = material.title.trim();
    return title && outlineContent.includes(`@${title}`);
  });
}

function ensureSectionForOutline(outlineId: string) {
  if (!currentProject.value) {
    return null;
  }

  syncProjectSections(currentProject.value);
  return getSectionForOutline(outlineId);
}

async function refreshAIProviders() {
  aiProviders.value = (await listAIProviders()).filter((item) => item.enabled);
  if (!aiProviders.value.some((item) => item.id === aiProviderId.value)) {
    aiProviderId.value = aiProviders.value[0]?.id ?? "";
  }
}

function setSectionElement(id: string, element: unknown) {
  if (element instanceof HTMLElement) {
    sectionElements.set(id, element);
    return;
  }

  sectionElements.delete(id);
}

function scrollToSection(sectionId: string) {
  const container = draftPanelBodyRef.value;
  const element = sectionElements.get(sectionId);
  if (!container || !element) {
    return;
  }

  const containerRect = container.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();
  const targetTop =
    container.scrollTop + elementRect.top - containerRect.top - 12;
  const maxScrollTop = Math.max(
    0,
    container.scrollHeight - container.clientHeight,
  );
  const nextScrollTop = Math.min(Math.max(targetTop, 0), maxScrollTop);

  container.scrollTo({ top: nextScrollTop, behavior: "smooth" });
}

function selectOutline(outlineId: string) {
  activeOutlineId.value = outlineId;
  scrollToSection(outlineId);
}

function generateAndSelectOutline(outlineId: string) {
  selectOutline(outlineId);
  void generateSectionFromOutline(outlineId);
}

function openCreateProjectDialog() {
  dialogState.visible = true;
  dialogState.mode = "prompt";
  dialogState.title = "创建写作工程";
  dialogState.message = "给这个写作工程起一个名字，后面随时可以修改。";
  dialogState.placeholder = "例如：产品发布稿、采访提纲、故事草稿";
  dialogState.confirmText = "创建";
  dialogState.cancelText = "取消";
  dialogState.initialValue = "";
  dialogState.danger = false;
  dialogState.action = "create-project";
  dialogState.targetId = "";
}

function openDeleteProjectDialog(project: WritingProjectSummary) {
  dialogState.visible = true;
  dialogState.mode = "confirm";
  dialogState.title = "删除写作工程";
  dialogState.message = `确认删除“${project.title}”吗？工程、素材和正文都会一起移除。`;
  dialogState.placeholder = "";
  dialogState.confirmText = "删除";
  dialogState.cancelText = "取消";
  dialogState.initialValue = "";
  dialogState.danger = true;
  dialogState.action = "delete-project";
  dialogState.targetId = project.id;
}

function openEditProjectDialog(project: WritingProjectSummary) {
  dialogState.visible = true;
  dialogState.mode = "prompt";
  dialogState.title = "编辑创作标题";
  dialogState.message = "这里只修改当前创作工程的标题，不会影响正文内容。";
  dialogState.placeholder = "输入新的创作标题";
  dialogState.confirmText = "保存";
  dialogState.cancelText = "取消";
  dialogState.initialValue = project.title;
  dialogState.danger = false;
  dialogState.action = "edit-project";
  dialogState.targetId = project.id;
}

function openProjectContextMenu(event: MouseEvent, projectId: string) {
  event.preventDefault();
  projectContextMenu.visible = true;
  projectContextMenu.x = event.clientX;
  projectContextMenu.y = event.clientY;
  projectContextMenu.projectId = projectId;
}

async function handleDialogConfirm(value: string) {
  if (dialogState.action === "create-project") {
    await handleCreateProject(value);
    return;
  }

  if (dialogState.action === "edit-project") {
    await handleEditProject(dialogState.targetId, value);
    return;
  }

  if (dialogState.action === "delete-project") {
    await handleDeleteProject(dialogState.targetId);
    return;
  }

  if (dialogState.action === "delete-outline") {
    removeOutline(dialogState.targetId);
  }
}

async function refreshNoteDirectories() {
  const workspace = await listNotes();
  const options: Array<{ label: string; value: string }> = [
    { label: "根目录", value: workspace.rootPath },
  ];

  const walk = (nodes: NoteTreeNode[], parents: string[] = []) => {
    for (const node of nodes) {
      if (node.nodeType !== "directory") {
        continue;
      }

      const nextParents = [...parents, node.name];
      options.push({
        label: nextParents.join(" / "),
        value: node.path,
      });

      if (node.children?.length) {
        walk(node.children, nextParents);
      }
    }
  };

  walk(workspace.tree);
  noteDirectoryOptions.value = options;
}

async function loadProjects(preferredProjectId?: string) {
  loadingProjects.value = true;

  try {
    projects.value = await listWritingProjects();
    if (!projects.value.length) {
      projectRailCollapsed.value = false;
    }

    const nextProjectId =
      preferredProjectId ||
      activeProjectId.value ||
      projects.value[0]?.id ||
      "";

    if (!nextProjectId) {
      projectDraft.value = null;
      activeProjectId.value = "";
      return;
    }

    await loadProject(nextProjectId);
  } finally {
    loadingProjects.value = false;
  }
}

async function loadProject(projectId: string) {
  if (!projectId) {
    return;
  }

  loadingProject.value = true;
  resetFeedback();

  try {
    const project = await getWritingProject(projectId);
    assignProject(project);
  } catch (err) {
    error.value = err instanceof Error ? err.message : "加载写作工程失败";
  } finally {
    loadingProject.value = false;
  }
}

async function handleCreateProject(title: string) {
  resetFeedback();
  loadingProject.value = true;

  try {
    const project = await createWritingProject({
      title: title.trim() || undefined,
    });
    projectRailCollapsed.value = false;
    await loadProjects(project.id);
    message.value = "写作工程已创建";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "创建写作工程失败";
  } finally {
    loadingProject.value = false;
  }
}

async function handleDeleteProject(projectId: string) {
  if (!projectId) {
    return;
  }

  resetFeedback();
  loadingProjects.value = true;

  try {
    projects.value = await deleteWritingProject(projectId);
    if (activeProjectId.value === projectId) {
      projectDraft.value = null;
      activeProjectId.value = "";
    }

    if (projects.value.length) {
      projectRailCollapsed.value = false;
      await loadProject(projects.value[0].id);
    } else {
      projectRailCollapsed.value = false;
    }

    message.value = "写作工程已删除";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "删除写作工程失败";
  } finally {
    loadingProjects.value = false;
  }
}

async function handleEditProject(projectId: string, title: string) {
  const nextTitle = title.trim();
  const target = projects.value.find((item) => item.id === projectId);
  if (!target || !nextTitle) {
    return;
  }

  resetFeedback();
  loadingProject.value = true;

  try {
    const detail =
      activeProjectId.value === projectId
        ? currentProject.value
        : await getWritingProject(projectId);

    if (!detail) {
      throw new Error("未找到要编辑的创作工程");
    }

    const saved = await saveWritingProject({
      id: detail.id,
      title: nextTitle,
      publishDirectoryPath: detail.publishDirectoryPath || undefined,
      outlines: detail.outlines.map((item) => ({
        id: item.id,
        title: item.title,
        content: item.content,
      })),
      sections: detail.sections.map((item) => ({
        id: item.id,
        title: item.title,
        content: item.content,
        selected: true,
      })),
    });

    projects.value = await listWritingProjects();
    if (activeProjectId.value === projectId) {
      assignProject(saved);
    }
    message.value = "创作标题已更新";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "更新创作标题失败";
  } finally {
    loadingProject.value = false;
  }
}

function handleProjectMenuSelect(key: string) {
  const project = projects.value.find(
    (item) => item.id === projectContextMenu.projectId,
  );
  if (!project) {
    return;
  }

  if (key === "edit") {
    openEditProjectDialog(project);
    projectContextMenu.visible = false;
    return;
  }

  if (key === "delete") {
    openDeleteProjectDialog(project);
    projectContextMenu.visible = false;
  }
}

function toggleProjectRail() {
  if (!projects.value.length) {
    projectRailCollapsed.value = false;
    return;
  }

  projectRailCollapsed.value = !projectRailCollapsed.value;
}

async function switchProject(projectId: string) {
  await loadProject(projectId);
}

function addOutline() {
  editingOutlineId.value = "";
  outlineDialogVisible.value = true;
  outlineContextMenu.visible = false;
}

function openOutlineContextMenu(event: MouseEvent, outlineId: string) {
  event.preventDefault();
  outlineContextMenu.visible = true;
  outlineContextMenu.x = event.clientX;
  outlineContextMenu.y = event.clientY;
  outlineContextMenu.outlineId = outlineId;
}

function openEditOutlineDialog(outlineId: string) {
  const outline = currentProject.value?.outlines.find(
    (item) => item.id === outlineId,
  );
  if (!outline) {
    return;
  }

  selectOutline(outlineId);
  editingOutlineId.value = outlineId;
  outlineDialogVisible.value = true;
  outlineContextMenu.visible = false;
}

function openDeleteOutlineDialog(outlineId: string) {
  const outline = currentProject.value?.outlines.find(
    (item) => item.id === outlineId,
  );
  if (!outline) {
    return;
  }

  dialogState.visible = true;
  dialogState.mode = "confirm";
  dialogState.title = "删除大纲";
  dialogState.message = `确认删除“${getOutlineSummary(outline.content)}”吗？`;
  dialogState.placeholder = "";
  dialogState.confirmText = "删除";
  dialogState.cancelText = "取消";
  dialogState.initialValue = "";
  dialogState.danger = true;
  dialogState.action = "delete-outline";
  dialogState.targetId = outlineId;
  outlineContextMenu.visible = false;
}

function handleOutlineMenuSelect(key: string) {
  const outlineId = outlineContextMenu.outlineId;
  if (!outlineId) {
    return;
  }

  if (key === "edit") {
    openEditOutlineDialog(outlineId);
    return;
  }

  if (key === "delete") {
    openDeleteOutlineDialog(outlineId);
  }
}

function handleSaveOutline(payload: { content: string }) {
  const project = currentProject.value;
  const outlineId = editingOutlineId.value;
  if (!project) {
    return;
  }

  const nextContent = payload.content.trim();
  if (!outlineId) {
    const nextOutline = {
      id: crypto.randomUUID(),
      title: getOutlineSummary(nextContent),
      content: nextContent,
    };

    project.outlines.push(nextOutline);
    syncProjectSections(project);
    activeOutlineId.value = nextOutline.id;
    markDirty();
    scheduleProjectAutosave();
    outlineDialogVisible.value = false;
    message.value = "大纲已添加";
    requestAnimationFrame(() => scrollToSection(nextOutline.id));
    return;
  }

  const outline = project.outlines.find((item) => item.id === outlineId);
  if (!outline) {
    return;
  }

  outline.content = nextContent;
  outline.title = getOutlineSummary(nextContent);
  syncProjectSections(project);
  activeOutlineId.value = outline.id;
  markDirty();
  scheduleProjectAutosave();
  outlineDialogVisible.value = false;
  editingOutlineId.value = "";
  message.value = "大纲已更新";
}

function handleOutlineChange(payload: { content: string }) {
  const project = currentProject.value;
  const outlineId = editingOutlineId.value;
  const outline = project?.outlines.find((item) => item.id === outlineId);
  if (!project || !outline) {
    return;
  }

  outline.content = payload.content;
  outline.title = getOutlineSummary(payload.content);
  syncProjectSections(project);
  dirty.value = true;
  scheduleProjectAutosave();
}

function openCreateMaterialModal() {
  editingMaterial.value = null;
  materialModalVisible.value = true;
}

function openEditMaterialModal(material: WritingMaterial) {
  editingMaterial.value = { ...material };
  materialModalVisible.value = true;
  materialContextMenu.visible = false;
}

function openMaterialContextMenu(event: MouseEvent, materialId: string) {
  event.preventDefault();
  materialContextMenu.visible = true;
  materialContextMenu.x = event.clientX;
  materialContextMenu.y = event.clientY;
  materialContextMenu.materialId = materialId;
}

async function handleMaterialMenuSelect(key: string) {
  const material = currentProject.value?.materials.find(
    (item) => item.id === materialContextMenu.materialId,
  );
  if (!material) {
    return;
  }

  if (key === "edit") {
    openEditMaterialModal(material);
    return;
  }

  if (key === "delete") {
    await handleDeleteMaterial(material.id);
  }
}

function removeOutline(outlineId: string) {
  const project = currentProject.value;
  if (!project) {
    return;
  }

  project.outlines = project.outlines.filter((item) => item.id !== outlineId);
  delete generatingOutlineIds[outlineId];
  syncProjectSections(project);
  if (activeOutlineId.value === outlineId) {
    activeOutlineId.value = project.outlines[0]?.id ?? "";
  }
  markDirty();
  scheduleProjectAutosave();
}

function openSectionEditor(section: WritingSection) {
  editingSectionId.value = section.id;
  sectionEditContent.value = section.content;
  sectionEditorVisible.value = true;
}

function closeSectionEditor() {
  sectionEditorVisible.value = false;
  editingSectionId.value = "";
  sectionEditContent.value = "";
}

async function saveSectionEditor() {
  const project = currentProject.value;
  const sectionId = editingSectionId.value;
  const section = project?.sections.find((item) => item.id === sectionId);
  if (!project || !section) {
    closeSectionEditor();
    return;
  }

  const nextContent = sectionEditContent.value.trim();
  project.sections = project.sections.map((item) =>
    item.id === sectionId
      ? {
          ...item,
          content: nextContent,
          selected: true,
        }
      : item,
  );

  markDirty();
  closeSectionEditor();
  await saveCurrentProject(false);
}

function buildOutlineGenerationPrompt(outlineId: string) {
  if (!currentProject.value) {
    return "";
  }

  const outline = currentProject.value.outlines.find(
    (item) => item.id === outlineId,
  );
  if (!outline) {
    return "";
  }

  const referencedMaterials = resolveOutlineReferencedMaterials(
    outline.content,
  );
  const referencesText = referencedMaterials
    .map((material, index) => {
      const source = material.sourceUrl ? `\n来源：${material.sourceUrl}` : "";
      return `${index + 1}. ${material.title}${source}\n${material.content.trim()}`;
    })
    .join("\n\n");

  return [
    `根据以下大纲提示完成针对本段文案的优化扩写。可以使用多个段落分别阐述，但是不要补充上下文内容。大纲：${outline.content.trim()}`,
    referencesText ? `\n引用素材内容：\n${referencesText}` : "",
  ]
    .filter(Boolean)
    .join("\n");
}

async function generateSectionFromOutline(outlineId: string) {
  if (!currentProject.value) {
    return;
  }

  await refreshAIProviders();
  if (!aiProviderId.value) {
    error.value = "请先在设置中启用一个 AI 供应商";
    return;
  }

  const outline = currentProject.value.outlines.find(
    (item) => item.id === outlineId,
  );
  if (!outline?.content.trim()) {
    error.value = "请先补充这条大纲内容，再生成正文";
    return;
  }

  const section = ensureSectionForOutline(outlineId);
  if (!section) {
    error.value = "未找到对应的正文段落";
    return;
  }

  generatingOutlineIds[outlineId] = true;
  resetFeedback();
  section.content = "";
  section.title = getOutlineSummary(outline.content);
  dirty.value = true;

  try {
    await invokeAIChatStream(
      {
        providerId: aiProviderId.value,
        messages: [
          {
            role: "system",
            content:
              "你是一名中文写作助手。请根据用户提供的大纲和上下文，只输出当前这一段最终可用的正文，不要输出标题、编号、解释、备注或代码块。",
          },
          {
            role: "user",
            content: buildOutlineGenerationPrompt(outlineId),
          },
        ],
        temperature: 0.7,
      },
      {
        onDelta(_, fullText) {
          section.content = fullText;
          dirty.value = true;
        },
        onDone(fullText) {
          section.content = fullText.trim();
          scheduleProjectAutosave(250);
        },
      },
    );

    requestAnimationFrame(() => scrollToSection(section.id));
    message.value = "该大纲对应文案已生成";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "生成正文失败";
  } finally {
    generatingOutlineIds[outlineId] = false;
  }
}

async function saveCurrentProject(showSuccess = true) {
  const project = currentProject.value;
  if (!project) {
    return null;
  }

  clearProjectAutosaveTimer();
  saving.value = true;
  resetFeedback();

  try {
    syncProjectSections(project);
    const saved = await saveWritingProject({
      id: project.id,
      title: project.title,
      publishDirectoryPath: project.publishDirectoryPath || undefined,
      outlines: project.outlines.map((item) => ({
        id: item.id,
        title: item.title || getOutlineSummary(item.content),
        content: item.content,
      })),
      sections: project.sections.map((item) => ({
        id: item.id,
        title: item.title || getOutlineSummary(item.content),
        content: item.content,
        selected: true,
      })),
    });

    if (showSuccess) {
      assignProject(saved);
    } else {
      syncProjectSections(project);
      dirty.value = false;
    }

    projects.value = await listWritingProjects();

    if (showSuccess) {
      message.value = "写作内容已保存";
    }

    return saved;
  } catch (err) {
    error.value = err instanceof Error ? err.message : "保存写作工程失败";
    return null;
  } finally {
    saving.value = false;
  }
}

function extractReadableText(document: Document) {
  const content = (document.body?.innerText || document.body?.textContent || "")
    .replace(/\s+/g, " ")
    .trim();

  return content.slice(0, 12000);
}

async function resolveLinkMaterial(
  payload: WritingMaterialInput,
): Promise<WritingMaterialInput> {
  const sourceUrl = payload.sourceUrl?.trim() || "";
  if (!sourceUrl) {
    throw new Error("请先填写链接地址");
  }

  try {
    const response = await httpFetch(sourceUrl, {
      method: "GET",
      headers: {
        Accept: "text/html,application/xhtml+xml",
      },
      connectTimeout: 15,
      maxRedirections: 5,
    });
    const html = await response.text();
    const document = new DOMParser().parseFromString(html, "text/html");
    const pageTitle = document.title.trim();
    const content = extractReadableText(document);

    return {
      kind: "link",
      title: payload.title.trim() || pageTitle || sourceUrl,
      content: content || sourceUrl,
      sourceUrl,
    };
  } catch {
    const fallbackTitle = payload.title.trim() || sourceUrl;
    return {
      kind: "link",
      title: fallbackTitle,
      content: sourceUrl,
      sourceUrl,
    };
  }
}

async function handleSaveMaterial(payload: WritingMaterialInput) {
  if (!currentProject.value) {
    return;
  }

  materialSaving.value = true;
  resetFeedback();
  const isEditing = Boolean(editingMaterial.value);

  try {
    const finalPayload =
      payload.kind === "link" ? await resolveLinkMaterial(payload) : payload;

    const project = editingMaterial.value
      ? await updateWritingMaterial(currentProject.value.id, {
          ...editingMaterial.value,
          kind: finalPayload.kind,
          title: finalPayload.title,
          content: finalPayload.content,
          sourceUrl: finalPayload.sourceUrl ?? null,
        })
      : await addWritingMaterial(currentProject.value.id, finalPayload);
    assignProject(project);
    projects.value = await listWritingProjects();
    materialModalVisible.value = false;
    editingMaterial.value = null;
    message.value = isEditing ? "素材已更新" : "素材已加入当前工程";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "保存素材失败";
  } finally {
    materialSaving.value = false;
  }
}

async function handleDeleteMaterial(materialId: string) {
  if (!currentProject.value) {
    return;
  }

  resetFeedback();

  try {
    const project = await deleteWritingMaterial(
      currentProject.value.id,
      materialId,
    );
    assignProject(project);
    projects.value = await listWritingProjects();
    message.value = "素材已删除";
  } catch (err) {
    error.value = err instanceof Error ? err.message : "删除素材失败";
  }
}

function choosePublishDirectory(path: string) {
  if (!currentProject.value) {
    return;
  }

  currentProject.value.publishDirectoryPath = path;
  localStorage.setItem(LAST_PUBLISH_DIRECTORY_KEY, path);
  markDirty();
  scheduleProjectAutosave();

  if (pendingPublishAfterSelect.value) {
    pendingPublishAfterSelect.value = false;
    void handlePublishProject();
  }
}

async function ensurePublishDirectory() {
  await refreshNoteDirectories();
  const current = publishDirectoryPath.value;
  if (current && isKnownNoteDirectory(current)) {
    return current;
  }

  pendingPublishAfterSelect.value = true;
  publishDialogVisible.value = true;
  return "";
}

async function handlePublishProject() {
  const project = currentProject.value;
  if (!project) {
    return;
  }

  resetFeedback();

  syncProjectSections(project);

  if (!project.sections.some((section) => section.content.trim())) {
    error.value = "请至少生成一段正文后再发布";
    return;
  }

  let directoryPath = publishDirectoryPath.value;
  if (!directoryPath) {
    directoryPath = await ensurePublishDirectory();
    if (!directoryPath) {
      return;
    }
  }

  if (!project.publishDirectoryPath) {
    project.publishDirectoryPath = directoryPath;
    markDirty();
  }

  if (dirty.value) {
    const saved = await saveCurrentProject(false);
    if (!saved) {
      return;
    }
  }

  publishing.value = true;

  try {
    const result = await publishWritingProject(project.id, directoryPath);
    localStorage.setItem(LAST_PUBLISH_DIRECTORY_KEY, directoryPath);
    await loadProjects(project.id);
    message.value = "已发布到笔记目录：" + result.noteTitle;
  } catch (err) {
    error.value = err instanceof Error ? err.message : "发布到笔记失败";
  } finally {
    publishing.value = false;
  }
}

async function handleTogglePublishMenu() {
  if (publishMenuVisible.value) {
    publishMenuVisible.value = false;
    return;
  }

  try {
    await refreshNoteDirectories();
    publishMenuVisible.value = true;
  } catch (err) {
    error.value = err instanceof Error ? err.message : "读取笔记目录失败";
  }
}

function handleWindowPointerDown(event: PointerEvent) {
  const target = event.target as HTMLElement | null;
  if (!target?.closest(".action-wrapper")) {
    publishMenuVisible.value = false;
  }
}

function handleProjectTitleInput() {
  markDirty();
  scheduleProjectAutosave();
}

onMounted(async () => {
  window.addEventListener("pointerdown", handleWindowPointerDown);

  try {
    await Promise.all([
      loadProjects(),
      refreshNoteDirectories(),
      refreshAIProviders(),
    ]);
  } catch (err) {
    error.value = err instanceof Error ? err.message : "初始化写作页失败";
  }
});

onBeforeUnmount(() => {
  clearProjectAutosaveTimer();
  window.removeEventListener("pointerdown", handleWindowPointerDown);
});
</script>

<template>
  <section
    class="writing-page"
    :class="{
      'writing-page--collapsed': projectRailCollapsed && projects.length,
    }"
  >
    <!-- 对话框组件 -->
    <WritingMaterialModal
      v-model="materialModalVisible"
      :material="editingMaterial"
      :loading="materialSaving"
      @save="handleSaveMaterial"
    />
    <WritingOutlineDialog
      v-model="outlineDialogVisible"
      :outline="editingOutline"
      :previous-outline="previousOutline"
      :materials="currentProject?.materials ?? []"
      @change="handleOutlineChange"
      @save="handleSaveOutline"
    />
    <WritingPublishDialog
      v-model="publishDialogVisible"
      :options="noteDirectoryOptions"
      :selected-path="publishDirectoryPath"
      @select="choosePublishDirectory"
    />
    <NoteAlertDialog
      v-model="dialogState.visible"
      :mode="dialogState.mode"
      :title="dialogState.title"
      :message="dialogState.message"
      :placeholder="dialogState.placeholder"
      :confirm-text="dialogState.confirmText"
      :cancel-text="dialogState.cancelText"
      :initial-value="dialogState.initialValue"
      :danger="dialogState.danger"
      @confirm="handleDialogConfirm"
    />

    <!-- 右键菜单 -->
    <TextContextMenu
      v-model="materialContextMenu.visible"
      :x="materialContextMenu.x"
      :y="materialContextMenu.y"
      :items="[
        { key: 'edit', label: '编辑' },
        { key: 'delete', label: '删除', danger: true },
      ]"
      @select="handleMaterialMenuSelect"
    />
    <TextContextMenu
      v-model="projectContextMenu.visible"
      :x="projectContextMenu.x"
      :y="projectContextMenu.y"
      :items="[
        { key: 'edit', label: '编辑' },
        { key: 'delete', label: '删除', danger: true },
      ]"
      @select="handleProjectMenuSelect"
    />
    <TextContextMenu
      v-model="outlineContextMenu.visible"
      :x="outlineContextMenu.x"
      :y="outlineContextMenu.y"
      :items="[
        { key: 'edit', label: '编辑' },
        { key: 'delete', label: '删除', danger: true },
      ]"
      @select="handleOutlineMenuSelect"
    />

    <!-- 第一栏：项目列表 (Project Rail) -->
    <aside
      class="project-rail"
      :class="{
        'project-rail--collapsed': projectRailCollapsed && projects.length,
      }"
    >
      <div class="project-rail__header">
        
        <div class="project-rail__tools">
          <!-- 统一样式为 icon-btn -->
          <button
            v-if="!projectRailCollapsed || !projects.length"
            type="button"
            class="icon-btn icon-btn--accent"
            title="新建写作工程"
            @click="openCreateProjectDialog"
          >
            <Plus :size="16" />
          </button>
          <button
            v-if="projects.length"
            type="button"
            class="icon-btn"
            :title="projectRailCollapsed ? '展开' : '收起'"
            @click="toggleProjectRail"
          >
            <ChevronRight v-if="projectRailCollapsed" :size="16" />
            <ChevronLeft v-else :size="16" />
          </button>
        </div>
      </div>

      <div class="project-rail__body">
        <article
          v-for="project in projects"
          :key="project.id"
          class="project-item"
          :class="{ 'project-item--active': activeProjectId === project.id }"
          @contextmenu="openProjectContextMenu($event, project.id)"
        >
          <button
            type="button"
            class="project-item__trigger"
            :title="getProjectSummaryLabel(project)"
            @click="switchProject(project.id)"
          >
            <div
              v-if="projectRailCollapsed && projects.length"
              class="project-item__icon"
            >
              <span v-if="projectRailCollapsed && projects.length">{{
                getProjectInitial(project)
              }}</span>
            </div>
            <div
              v-if="!projectRailCollapsed || !projects.length"
              class="project-item__main"
            >
              <strong>{{ getProjectSummaryLabel(project) }}</strong>
              <span>{{ project.updatedAt }}</span>
            </div>
          </button>
        </article>
        <div v-if="!projects.length && !loadingProjects" class="rail-empty">
          <p>写作工程等待创建</p>
          <button
            type="button"
            class="rail-empty__action"
            @click="openCreateProjectDialog"
          >
            立即开始
          </button>
        </div>
      </div>
    </aside>

    <!-- 第二栏：大纲与素材 (Sidebar) -->
    <div class="writing-sidebar">
      <!-- 大纲面板 -->
      <section class="outline-panel">
        <div class="panel-header">
          <div class="panel-title">
            <FolderTree :size="14" />
            <span>大纲结构</span>
          </div>
          <button
            type="button"
            class="icon-btn icon-btn--accent"
            title="新增大纲"
            @click="addOutline"
          >
            <Plus :size="14" />
          </button>
        </div>
        <div class="outline-panel__body">
          <div
            v-for="(outline, index) in currentProject?.outlines"
            :key="outline.id"
            class="outline-item"
            :class="{ 'outline-item--active': activeOutlineId === outline.id }"
            @contextmenu="openOutlineContextMenu($event, outline.id)"
          >
            <button
              type="button"
              class="outline-item__label"
              :title="'双击编辑 ' + getOutlineSummary(outline.content, index)"
              @click="selectOutline(outline.id)"
              @dblclick="openEditOutlineDialog(outline.id)"
            >
              <span class="outline-index">{{ index + 1 }}.</span>
              <span class="outline-text">{{
                getOutlineSummary(outline.content, index)
              }}</span>
            </button>
          </div>
          <div v-if="!currentProject?.outlines?.length" class="empty-hint">
            暂无大纲，请点击右上角添加
          </div>
        </div>
      </section>

      <!-- 素材列表 -->
      <section class="materials-strip">
        <div class="panel-header">
          <div class="panel-title">
            <Link2 :size="14" />
            <span>参考资料</span>
            <small>{{ currentProject?.materials?.length ?? 0 }} 条</small>
          </div>
          <button
            type="button"
            class="icon-btn icon-btn--accent"
            title="新增素材"
            @click="openCreateMaterialModal"
          >
            <Plus :size="14" />
          </button>
        </div>
        <div class="materials-list">
          <article
            v-for="material in currentProject?.materials"
            :key="material.id"
            class="material-card"
            @contextmenu="openMaterialContextMenu($event, material.id)"
            @click="openEditMaterialModal(material)"
          >
            <div class="material-main">
              <span class="material-kind-icon">
                <Type v-if="material.kind === 'text'" :size="12" />
                <Link2 v-else :size="12" />
              </span>
              <span class="material-title">{{ material.title }}</span>
            </div>
          </article>
          <div v-if="!currentProject?.materials?.length" class="empty-hint">
            暂无参考资料
          </div>
        </div>
      </section>
    </div>

    <!-- 第三栏：正文编辑 (Draft Panel) -->
    <template v-if="hasProject && currentProject">
      <section class="draft-panel">
        <header class="draft-header">
          <input
            v-model="currentProject.title"
            class="draft-title-input"
            type="text"
            placeholder="输入创作标题"
            @input="handleProjectTitleInput"
          />

          <div class="draft-actions">
            <!-- 状态计数 -->
            <span class="action-count"
              >{{ generatedSectionCount }} /
              {{ currentProject.outlines.length }}</span
            >

            <!-- 保存按钮 -->
            <button
              type="button"
              class="action-btn"
              :disabled="saving || publishing"
              @click="saveCurrentProject()"
            >
              <Save :size="14" /> 保存
            </button>

            <!-- 发布组 (无缝融合) -->
            <div class="action-wrapper">
              <button
                type="button"
                class="action-btn action-btn--primary"
                :disabled="saving || publishing"
                @click="handlePublishProject"
              >
                <Send :size="14" /> {{ publishing ? "发布中..." : "发布" }}
              </button>
              <!-- 下拉箭头 -->
              <button
                type="button"
                class="action-btn action-btn--toggle"
                title="选择目录"
                @click.stop="handleTogglePublishMenu"
              >
                <ChevronDown :size="14" />
              </button>

              <!-- 下拉菜单 -->
              <div v-if="publishMenuVisible" class="dropdown-menu" @click.stop>
                <div class="dropdown-item dropdown-header">
                  <span>当前目录</span>
                  <strong>{{ publishDirectoryLabel }}</strong>
                </div>
                <button
                  v-for="option in noteDirectoryOptions"
                  :key="option.value"
                  type="button"
                  class="dropdown-item"
                  :class="{
                    'dropdown-item--active':
                      publishDirectoryPath === option.value,
                  }"
                  @click="choosePublishDirectory(option.value)"
                >
                  {{ option.label }}
                </button>
              </div>
            </div>
          </div>
        </header>

        <div ref="draftPanelBodyRef" class="draft-body">
          <article
            v-for="(section, index) in currentProject.sections"
            :key="section.id"
            :ref="(el) => setSectionElement(section.id, el)"
            class="section-card"
            :class="{ 'section-card--active': activeOutlineId === section.id }"
          >
            <div class="section-header">
              <div class="section-title-wrap">
                <span class="section-index">{{ index + 1 }}</span>
                <strong class="section-title">{{
                  section.title || "段落 " + (index + 1)
                }}</strong>
              </div>
              <div class="section-actions">
                <button
                  type="button"
                  class="section-btn section-btn--ai"
                  :disabled="generatingOutlineIds[section.id]"
                  @click="generateAndSelectOutline(section.id)"
                >
                  <Sparkles
                    v-if="!generatingOutlineIds[section.id]"
                    :size="14"
                  />
                  <Bot v-else :size="14" class="spin" />
                </button>
                <button
                  type="button"
                  class="section-btn section-btn--edit"
                  title="编辑"
                  @click="openSectionEditor(section)"
                >
                  <Pencil :size="14" />
                </button>
              </div>
            </div>

            <div
              v-if="!sectionEditorVisible || editingSectionId !== section.id"
              class="section-content"
              @click="openSectionEditor(section)"
            >
              <div
                v-if="section.content"
                class="markdown-body"
                v-html="renderSectionMarkdown(section.content)"
              />
              <p v-else class="section-placeholder">
                点击此处输入文案，或点击 AI 图标生成内容
              </p>
            </div>

            <div v-else class="section-editor">
              <VditorEditor
                v-model="sectionEditContent"
                mode="ir"
                height="400px"
                placeholder="在此输入..."
                cache-id="section-editor"
                :upload-handler="handleSectionVditorUpload"
                :custom-actions="sectionEditorActions"
              />
              <footer class="editor-footer">
                <button
                  type="button"
                  class="btn btn--outline"
                  @click="closeSectionEditor"
                >
                  取消
                </button>
                <button
                  type="button"
                  class="btn btn--primary"
                  @click="saveSectionEditor"
                >
                  保存
                </button>
              </footer>
            </div>
          </article>
        </div>
      </section>
    </template>

    <template v-else>
      <section class="draft-panel">
        <div class="draft-empty">
          <Sparkles :size="48" :stroke-width="1.5" />
          <h3>准备好开始创作了吗？</h3>
          <p>请选择左侧项目或创建一个新工程。</p>
        </div>
      </section>
    </template>
  </section>
</template>

<style scoped>
/* =========================================
   WRITING PAGE UI - Robust 3-Column Robust Layout
   ========================================= */

.writing-page {
  display: grid;
  /* 关键修复：明确宽度 */
  grid-template-columns: 220px 300px 1fr;
  height: 100vh;
  background: #ffffff;
  color: #0f172a;
  font-family:
    ui-sans-serif,
    system-ui,
    -apple-system,
    BlinkMacSystemFont,
    "Segoe UI",
    Roboto,
    "Helvetica Neue",
    Arial,
    sans-serif;
  overflow: hidden;
}

.writing-page--collapsed {
  grid-template-columns: 56px 300px 1fr;
}

/* 第一栏：项目列表 (Project Rail) */
.project-rail {
  background: #f8fafc;
  border-right: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.project-rail__header {
  height: 56px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #e2e8f0;
  background: #ffffff;
  flex-shrink: 0;
}

.writing-page--collapsed .project-rail__header {
  height: 56px;
  padding: 0;
  justify-content: center;
  align-items: center;
}
.project-rail__tools {
  display: flex;
  gap: 8px;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.writing-page--collapsed .project-rail__tools {
  width: auto;
}

.project-rail__body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

.writing-page--collapsed .project-rail__body {
  padding: 10px 8px;
}

.project-item {
  margin-bottom: 6px;
  border-radius: 8px;
}

.project-item__trigger {
  display: flex;
  align-items: center;
  gap: 0;
  width: 100%;
  padding: 9px 10px;
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  text-align: left;
}

.writing-page--collapsed .project-item__trigger {
  justify-content: center;
  gap: 0;
  padding: 6px 0;
}
.project-item__trigger:hover {
  background: #f1f5f9;
}
.project-item--active .project-item__trigger {
  background: #ffffff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}

.project-item__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: #f1f5f9;
  color: #64748b;
  flex-shrink: 0;
  font-size: 13px;
  font-weight: 700;
}

.writing-page--collapsed .project-item__icon {
  width: 34px;
  height: 34px;
  border-radius: 8px;
}
.project-item--active .project-item__icon {
  background: #eff6ff;
  color: #2563eb;
}

.project-item__main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.project-item__main strong {
  font-size: 13px;
  color: #334155;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.project-item--active .project-item__main strong {
  color: #2563eb;
}
.project-item__main span {
  font-size: 11px;
  color: #94a3b8;
}

.rail-empty {
  text-align: center;
  padding: 20px 10px;
  color: #64748b;
  font-size: 13px;
}
.rail-empty__action {
  margin-top: 10px;
  background: #2563eb;
  color: white;
  border: none;
  padding: 6px;
  border-radius: 6px;
  cursor: pointer;
  width: 100%;
  font-weight: 500;
}

/* 第二栏：大纲与素材 (Sidebar) */
.writing-sidebar {
  background: #ffffff;
  border-right: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}
.panel-header {
  height: 48px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #f1f5f9;
  background: #ffffff;
  flex-shrink: 0;
}
.panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: #334155;
}
.panel-title small {
  font-weight: 400;
  color: #94a3b8;
  margin-left: 4px;
}

/* 大纲面板 */
.outline-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-bottom: 1px solid #e2e8f0;
  min-height: 0;
}
.outline-panel__body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.outline-item {
  margin-bottom: 4px;
  border-radius: 4px;
}
.outline-item__label {
  display: flex;
  gap: 8px;
  padding: 8px;
  width: 100%;
  background: transparent;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  text-align: left;
  font-size: 12px;
  color: #475569;
  align-items: flex-start;
}
.outline-item__label:hover {
  background: #f8fafc;
}
.outline-item--active .outline-item__label {
  background: #eff6ff;
  color: #2563eb;
}
.outline-index {
  color: #94a3b8;
  font-family: monospace;
  margin-right: 2px;
}
.empty-hint {
  text-align: center;
  padding: 12px;
  color: #cbd5e1;
  font-size: 12px;
}

/* 素材面板 */
.materials-strip {
  height: 40%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 160px;
}
.materials-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.material-card {
  display: flex;
  align-items: center;
  padding: 8px;
  margin-bottom: 6px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 4px;
  cursor: pointer;
}
.material-card:hover {
  border-color: #2563eb;
  background: #fcfdff;
}
.material-main {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  overflow: hidden;
}
.material-title {
  font-weight: 500;
  color: #334155;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 全局按钮样式定义 */
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
  padding: 0;
  font: inherit;
}
.icon-btn:hover {
  background: #f1f5f9;
  color: #0f172a;
}
.icon-btn--accent {
  background: #2563eb;
  color: white;
}
.icon-btn--accent:hover {
  background: #1d4ed8;
  color: white;
}

/* 第三栏：正文 (Draft Panel) */
.draft-panel {
  display: flex;
  flex-direction: column;
  background: #f8fafc;
  overflow: hidden;
  min-width: 0;
}

.draft-header {
  height: 60px;
  padding: 0 24px;
  display: flex;
  align-items: center;
  gap: 16px;
  border-bottom: 1px solid #e2e8f0;
  background: #ffffff;
  flex-shrink: 0;
}

.draft-title-input {
  flex: 1;
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
  background: transparent;
  border: 1px solid transparent;
  padding: 6px 10px;
  border-radius: 6px;
}
.draft-title-input:focus {
  background: #ffffff;
  border-color: #cbd5e1;
  outline: none;
}

.draft-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-left: auto;
}

.action-count {
  color: #94a3b8;
  font-size: 12px;
  font-weight: 500;
  font-family: monospace;
}

.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #ffffff;
  color: #475569;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  height: 34px;
}
.action-btn:hover {
  background: #f8fafc;
  border-color: #cbd5e1;
  color: #0f172a;
}

/* 发布区域与按钮融合修复 */
.action-wrapper {
  position: relative;
  display: flex;
  align-items: stretch;
  margin-left: 8px;
}

.action-btn--primary {
  background: #2563eb;
  color: white;
  border: 1px solid #2563eb;
  /* 修复圆角和右侧边框以融合 */
  border-right: none;
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
  height: 34px;
  padding-right: 10px;
}
.action-btn--primary:hover {
  background: #1d4ed8;
}

.action-btn--toggle {
  padding: 6px;
  margin: 0;
  border: 1px solid #2563eb;
  border-left: none;
  border-radius: 0 6px 6px 0;
  background: #2563eb;
  color: white;
  height: 34px;
}
.action-btn--toggle:hover {
  background: #1e40af;
  color: white;
}

/* 下拉菜单 */
.dropdown-menu {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.15);
  z-index: 2000;
  min-width: 240px;
  padding: 4px;
}
.dropdown-header {
  background: #f8fafc;
  padding: 8px 12px;
  border-radius: 4px;
  display: flex;
  justify-content: space-between;
  margin-bottom: 4px;
  color: #64748b;
  font-size: 12px;
  font-weight: bold;
}
.dropdown-item {
  width: 100%;
  text-align: left;
  padding: 8px 12px;
  border: none;
  background: transparent;
  font-size: 13px;
  color: #334155;
  cursor: pointer;
  border-radius: 4px;
}
.dropdown-item:hover,
.dropdown-item--active {
  background: #eff6ff;
  color: #2563eb;
}

/* 草稿内容区 */
.draft-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}
.section-card {
  margin-bottom: 20px;
  padding: 20px;
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  transition: all 0.2s;
  position: relative;
}
.section-card:hover {
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.04);
}
.section-card--active {
  border-color: #2563eb;
  box-shadow: 0 0 0 1px #2563eb;
}

.section-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
}
.section-title-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
}
.section-index {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  background: #f1f5f9;
  color: #64748b;
  font-size: 11px;
  font-weight: 600;
  border-radius: 4px;
}
.section-card--active .section-index {
  background: #2563eb;
  color: white;
}
.section-title {
  font-size: 15px;
  color: #1e293b;
}

.section-actions {
  display: flex;
  gap: 6px;
}
.section-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #fff;
  color: #64748b;
  cursor: pointer;
}
.section-btn:hover {
  background: #f8fafc;
  border-color: #cbd5e1;
}
.section-btn--ai {
  color: #2563eb;
}
.spin {
  animation: spin 1s linear infinite;
}

.section-content {
  cursor: pointer;
  min-height: 30px;
  line-height: 1.6;
  color: #475569;
  border: 1px dashed transparent;
  border-radius: 4px;
  padding: 4px;
}
.section-content:hover {
  background: #fafafa;
  border-color: #e2e8f0;
}
.section-placeholder {
  color: #cbd5e1;
  font-size: 13px;
}

.editor-footer {
  padding: 8px;
  background: #f8fafc;
  border-top: 1px solid #e2e8f0;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  border-radius: 0 0 10px 10px;
}
.btn {
  padding: 6px 14px;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #fff;
  cursor: pointer;
  font-size: 12px;
  color: #475569;
}
.btn--primary {
  background: #2563eb;
  color: white;
  border-color: #2563eb;
}
.btn--primary:hover {
  background: #1d4ed8;
}

.draft-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #94a3b8;
  gap: 16px;
}
.draft-empty h3 {
  margin: 0;
  color: #334155;
}

@keyframes spin {
  100% {
    transform: rotate(360deg);
  }
}

::selection {
  background: #e2e8f0;
  color: #0f172a;
}
</style>
