# AI NoteBook

基于 `Tauri + Vue3` 的多端笔记与邮件管理工具，聚焦两个核心场景：

- 多目录 Markdown 笔记管理
- 多账号邮件统一收发

项目目标是在 `Windows`、`macOS`、`Linux` 上提供一致、轻量、可扩展的桌面体验，同时兼顾 AI 辅助编辑、自动备份与本地数据可控。

## 核心能力

### 1. 笔记管理

- 多级目录管理，支持目录和笔记的增删改查
- Markdown 编辑 / 预览双模式
- 实时保存、撤销重做、图片插入、搜索排序
- 段落级备注与 AI 辅助润色 / 扩写 / 精简
- AI 历史记录与版本回溯

### 2. 备份管理

- 保存时自动生成备份
- 备份目录与保留时长可配置
- 支持手动备份、清理与恢复
- 默认执行过期备份自动清理

### 3. AI 交互

- 统一封装多模型接口
- 支持 OpenAI / 文心一言 / 通义千问等扩展接入
- 基于 Markdown 标题提取大纲，拼接最近内容作为上下文
- 支持模型参数、超时、上下文长度配置

### 4. 邮件管理

- 多邮箱账号配置
- IMAP 收信、SMTP 发信
- 定时拉取、统一列表、详情查看
- 回复、转发、删除、附件下载

### 5. 系统设置

- 主题、语言、窗口大小、开机自启
- 默认笔记目录、备份目录、附件目录
- AI 模型参数与邮件拉取频率设置

## 技术栈

### 前端

- `Vue 3` + `Composition API`
- `Pinia`
- `Vue Router`
- `Naive UI` 或 `Element Plus`
- `marked`
- `highlight.js`
- `axios`
- `js-cookie`
- `dayjs`

### 桌面端 / 后端

- `Tauri`
- `Rust`（Tauri Command、本地文件与系统能力）
- `Tauri FS / Path / Store` 相关能力

### 第三方能力

- 大模型 API：OpenAI / 文心一言 / 通义千问等
- 邮件：`imapflow`、`nodemailer`

## 推荐目录结构

```text
ai-markdown/
  src/
    app/
    components/
    layouts/
    pages/
    router/
    stores/
    services/
      ai/
      backup/
      mail/
      note/
    composables/
    utils/
    types/
  src-tauri/
    src/
      commands/
      mail/
      note/
      backup/
      utils/
    capabilities/
  docs/
    architecture.md
    roadmap.md
```

## 模块优先级

1. 多目录笔记管理
2. 自动备份与恢复
3. AI 段落编辑
4. 多账号邮件管理
5. 全局系统设置

## 里程碑规划

### 阶段 1：基础搭建与笔记核心

- 初始化 `Tauri + Vue3` 工程
- 完成目录 / 笔记基础 CRUD
- 完成 Markdown 编辑与预览

### 阶段 2：备份与 AI 基础

- 自动备份与过期清理
- 段落备注
- 基础 AI 段落修改

### 阶段 3：AI 增强与邮件功能

- AI 上下文增强
- AI 历史记录
- 多邮箱配置与收发

### 阶段 4：设置与多端适配

- 完成系统设置页面
- 适配三端打包与权限差异
- UI 与交互优化

### 阶段 5：测试与发布

- 功能测试、兼容测试、性能测试
- 打包安装包
- 编写使用文档与发布说明

## 当前文档

- [架构设计](D:/项目源码/ai-markdown/docs/architecture.md)
- [开发路线图](D:/项目源码/ai-markdown/docs/roadmap.md)

## 下一步建议

建议优先落地以下最小可用版本（MVP）：

1. 初始化 `Tauri + Vue3 + Pinia + Router`
2. 完成本地笔记目录与 Markdown 编辑
3. 接入自动保存与自动备份
4. 再逐步接入 AI 能力与邮件模块
