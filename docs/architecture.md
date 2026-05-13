# 架构设计

## 1. 项目定位

本项目是一款基于 `Tauri + Vue3` 的桌面端综合管理工具，面向需要同时处理笔记与邮件的个人用户或轻办公场景用户。设计重点如下：

- 本地优先：笔记、备份、账号配置尽量本地管理
- 多端一致：Windows、macOS、Linux 统一体验
- 低资源占用：依托 Tauri 控制包体积与内存使用
- 可扩展：AI 模型、邮箱服务、导出能力后续持续扩展

## 2. 分层架构

```text
+--------------------------------------------------+
| UI Layer                                         |
| Vue Pages / Components / Layouts                 |
+--------------------------------------------------+
| Domain Layer                                     |
| Note / Backup / AI / Mail / Settings             |
+--------------------------------------------------+
| Service Layer                                    |
| Tauri Commands / HTTP Client / Mail Adapters     |
+--------------------------------------------------+
| Storage Layer                                    |
| Markdown Files / Backup Files / Local Config     |
+--------------------------------------------------+
```

### 2.1 UI 层

负责页面展示、交互反馈与状态联动：

- 笔记工作台
- Markdown 编辑器与预览区
- 邮件列表与详情页
- 设置中心
- AI 侧边栏 / 历史记录面板

### 2.2 领域逻辑层

负责业务规则与流程编排：

- `note-domain`：目录树、笔记元信息、搜索排序
- `backup-domain`：自动备份、过期清理、恢复逻辑
- `ai-domain`：模型选择、上下文提取、历史记录
- `mail-domain`：邮箱账号、收件同步、发送与附件处理
- `settings-domain`：全局偏好、路径、主题、语言等

### 2.3 服务层

负责与外部依赖交互：

- Tauri Command：本地文件、系统路径、权限、定时任务
- HTTP Client：统一调用大模型 API
- Mail Adapter：统一封装 IMAP / SMTP 差异

### 2.4 存储层

负责数据落地：

- 笔记正文：`.md`
- 备注与 AI 历史：推荐 `JSON` sidecar 文件或嵌入 frontmatter
- 备份文件：独立目录，按时间戳命名
- 用户配置：本地配置文件或 Tauri Store

## 3. 模块拆分

## 3.1 笔记模块

### 功能职责

- 目录树管理
- 笔记 CRUD
- Markdown 编辑与预览
- 搜索、排序、自动保存

### 建议数据模型

```ts
interface NoteMeta {
  id: string
  title: string
  filePath: string
  directoryId: string
  createdAt: string
  updatedAt: string
  tags?: string[]
}

interface NoteDocument {
  meta: NoteMeta
  content: string
}
```

## 3.2 段落备注与 AI 模块

### 功能职责

- 段落定位
- 备注绑定与展示控制
- AI 指令执行
- 历史记录回溯

### 建议数据模型

```ts
interface ParagraphComment {
  id: string
  noteId: string
  paragraphKey: string
  content: string
  createdAt: string
  updatedAt: string
}

interface AiHistoryItem {
  id: string
  noteId: string
  paragraphKey: string
  provider: string
  model: string
  promptType: 'polish' | 'expand' | 'shorten' | 'custom'
  beforeText: string
  afterText: string
  createdAt: string
}
```

### 段落标识建议

为避免用户编辑后段落错位，建议使用以下策略组合：

- 基于段落文本 hash 生成 `paragraphKey`
- 保存段落顺序索引
- 恢复绑定时优先按 hash，其次按相邻上下文模糊匹配

## 3.3 备份模块

### 功能职责

- 保存时自动备份
- 手动备份与清理
- 过期策略执行
- 内容恢复

### 备份命名建议

```text
{noteName}__{YYYYMMDD_HHmmss}.md
```

### 备份流程建议

1. 用户保存笔记
2. 写入主文件
3. 复制当前内容到备份目录
4. 写入备份日志
5. 触发过期扫描任务

## 3.4 邮件模块

### 功能职责

- 多账号配置
- 定时拉取邮件
- 邮件列表聚合
- 发送、回复、转发、删除
- 附件下载与打开

### 建议数据模型

```ts
interface MailAccount {
  id: string
  name: string
  address: string
  imapHost: string
  imapPort: number
  smtpHost: string
  smtpPort: number
  secure: boolean
  authType: 'password' | 'app_password'
  isDefaultSender: boolean
}

interface MailMessageSummary {
  id: string
  accountId: string
  subject: string
  from: string
  to: string[]
  date: string
  unread: boolean
  hasAttachments: boolean
}
```

### 安全建议

- 密码或授权码不直接明文落盘
- 优先使用系统安全存储或加密后本地保存
- 日志中禁止输出完整账号凭证

## 3.5 设置模块

### 职责

- 统一维护所有用户偏好
- 提供默认值与配置校验
- 管理运行时热更新能力

### 建议配置结构

```ts
interface AppSettings {
  theme: 'light' | 'dark' | 'system'
  language: 'zh-CN' | 'en-US'
  autoLaunch: boolean
  noteRootPath: string
  backupRootPath: string
  backupRetentionDays: number
  autoBackupEnabled: boolean
  defaultAiProvider: string
  aiContextLength: number
  aiTimeoutMs: number
  mailPullIntervalMinutes: number
  attachmentSavePath: string
}
```

## 4. 前后端职责边界

## 4.1 Vue 前端负责

- 界面渲染
- 状态同步
- 编辑器交互
- 表单校验
- AI / 邮件操作发起

## 4.2 Tauri / Rust 负责

- 本地文件系统操作
- 系统路径获取
- 权限适配
- 备份任务与本地定时清理
- 邮件协议层调用与敏感信息处理

## 5. 关键技术方案

## 5.1 Markdown 编辑方案

建议编辑器方案优先级如下：

1. `CodeMirror 6` + `marked` + `highlight.js`
2. 若后续要增强富交互，可升级为 `Milkdown` 或 `Monaco + Markdown 扩展`

原因：

- 轻量
- 适合 Tauri 桌面场景
- 可控性高，便于插入段落备注与 AI 操作入口

## 5.2 AI 统一适配层

建议定义统一接口：

```ts
interface AiProvider {
  name: string
  chat(input: {
    model: string
    prompt: string
    context?: string
    temperature?: number
    maxTokens?: number
  }): Promise<string>
}
```

不同模型只在 provider 内处理请求头、鉴权与返回结构差异，业务层只消费统一结果。

## 5.3 邮件接入方案

建议邮件逻辑尽量放在 Tauri 后端，原因如下：

- 凭证更适合在本地安全环境处理
- 降低前端直接暴露协议细节
- 更利于定时任务与附件文件管理

## 5.4 配置存储建议

优先级建议：

1. 普通配置：Tauri Store / JSON 配置文件
2. 敏感配置：系统密钥链或加密存储
3. 大体量数据：文件系统目录化管理

## 6. 风险与应对

### 风险 1：跨平台路径差异

- 统一使用 Tauri Path API 获取系统目录
- 所有路径拼接在 Rust 侧处理

### 风险 2：段落备注与 AI 历史绑定不稳定

- 使用段落 hash + 索引 + 上下文匹配三重策略

### 风险 3：不同邮箱协议兼容问题

- 预置常见邮箱模板
- 为失败连接提供详细错误提示与重试机制

### 风险 4：AI 上下文过长导致失败

- 提供上下文长度限制
- 自动截断并提示用户

## 7. MVP 建议范围

为控制复杂度，首个可交付版本建议只包含：

- 单窗口桌面应用
- 多目录 Markdown 笔记管理
- 自动保存与自动备份
- 单模型 AI 段落润色
- 基础设置页面

邮件模块建议作为第二阶段能力接入。
