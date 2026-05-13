AI Notebook：AI 文案生成 + 智能笔记 + 邮件管理一体化工具
[![GitHub Stars](https://img.shields.io/github/stars/cuo9958/ai-notebook?style=flat-square&color=ff69b4)](https://github.com/cuo9958/ai-notebook)
[![GitHub Forks](https://img.shields.io/github/forks/cuo9958/ai-notebook?style=flat-square&color=00ced1)](https://github.com/cuo9958/ai-notebook)
[![GitHub Issues](https://img.shields.io/github/issues/cuo9958/ai-notebook?style=flat-square&color=f08080)](https://github.com/cuo9958/ai-notebook/issues)
[![GitHub License](https://img.shields.io/github/license/cuo9958/ai-notebook?style=flat-square&color=6a5acd)](https://github.com/cuo9958/ai-notebook/blob/main/LICENSE)
[![Last Commit](https://img.shields.io/github/last-commit/cuo9958/ai-notebook?style=flat-square&color=32cd32)](https://github.com/cuo9958/ai-notebook/commits/main)

---
📖 项目简介
AI Notebook 是一款面向内容创作者、职场人、自媒体运营的一站式AI工具，集多平台文案智能生成、沉浸式笔记记录、高效邮件管理于一体，打破创作、记录、办公的工具壁垒。项目支持灵活自定义大模型接入、自定义图床接入，兼顾易用性与扩展性，本地部署更保障数据隐私，打造专属的AI创作与知识管理工作台。
📍 项目地址：https://github.com/cuo9958/ai-notebook

---
✨ 核心功能
1. 多平台文案一键生成
- 覆盖小红书、公众号、抖音、知乎、微博、B站、短视频脚本等全场景文案模板
- 支持标题优化、正文扩写、文案润色、标签生成、文风切换（温柔/干练/搞笑/正式等）
- 保留创作历史，支持文案二次编辑、导出、直接同步至笔记
2. 沉浸式笔记管理
- 无干扰沉浸式编辑模式，专注创作与记录，支持Markdown语法、富文本编辑
- 笔记分类、标签、文件夹管理，支持全文检索、关键词筛选、笔记置顶
- 创作内容自动归档，文案生成结果一键存入笔记，打通创作-记录闭环
3. 高效邮件管理
- AI快速生成工作邮件、商务函件、请假申请、客户沟通等邮件模板
- 邮件内容润色、语气优化、语法纠错，适配职场正式场景
- 邮件草稿保存、分类归档，支持关联笔记素材，提升办公效率
4. 自定义扩展能力（核心亮点）
- 自定义大模型接入：兼容主流大模型API，支持切换国内外大模型、本地部署大模型，自定义温度、生成长度、上下文窗口等参数
- 自定义图床接入：支持GitHub图床、公共图床、私有图床、本地存储，图片一键上传、自动插入，适配文案/笔记配图需求
- 配置可视化，无需修改源码，后台页面即可完成参数配置，新手友好
5. 沉浸式创作体验
- AI流式输出，实时查看生成过程，支持中途暂停、修改指令、重新生成
- 暗黑/亮色模式切换，护眼排版，适配长时间创作
- 本地缓存，断电/刷新不丢失内容

---
🚀 快速开始（部署&使用）
环境要求
- Node.js ≥ 16.x（前端项目）/ Python ≥ 3.8（后端服务，根据技术栈适配）
- 操作系统：Windows 10+/macOS/Linux（支持本地部署、服务器部署）
- 可选：自备大模型API Key、图床令牌（如需使用自定义扩展功能）
本地部署步骤
# 1. 克隆项目到本地
git clone https://github.com/cuo9958/ai-notebook.git
cd ai-notebook

# 2. 安装依赖（前端示例，后端根据技术栈替换命令）
npm install
# 若为Python后端：pip install -r requirements.txt

# 3. 配置环境变量
# 复制.env.example为.env，填写大模型、图床相关配置（详见配置说明）
cp .env.example .env

# 4. 启动项目
# 前端开发模式
npm run dev
# 生产构建
npm run build

# 5. 访问使用
# 浏览器打开：http://localhost:3000（默认端口）
快速上手使用
1. 进入系统后，先在【设置】页面完成大模型、图床配置（不配置则使用默认演示模式）
2. 【文案生成】：选择平台、输入需求，一键生成优质文案，可直接编辑或存入笔记
3. 【笔记管理】：新建笔记，支持富文本/Markdown编辑，配图自动上传图床
4. 【邮件管理】：选择邮件场景，AI生成初稿，优化后保存草稿或复制使用
5. 开启沉浸式模式，专注创作，告别多工具切换繁琐

---
⚙️ 配置说明
自定义大模型配置
在项目.env文件或系统后台【模型设置】中填写：
- 模型接口地址、API Key、模型名称（如gpt-3.5-turbo、通义千问、Llama等）
- 可调参数：生成温度、最大token数、上下文记忆长度、请求超时时间
- 支持多模型切换，按需选择生成速度、效果、成本
自定义图床配置
在项目.env文件或系统后台【图床设置】中填写：
- 图床类型：GitHub图床/自定义API图床/本地存储
- GitHub图床：仓库地址、分支、Token、存储路径
- 自定义图床：上传接口、请求头、返回格式解析规则

---
📂 项目结构（参考）
ai-notebook/
├── public/            # 静态资源、图标、首页素材
├── src/               # 核心源码
│   ├── assets/        # 样式、图片资源
│   ├── components/    # 公共组件（文案生成、笔记编辑器、邮件模块）
│   ├── pages/         # 功能页面
│   ├── service/       # API接口（大模型、图床、邮件）
│   ├── utils/         # 工具函数、配置解析
│   └── App.js         # 入口文件（前端）
├── .env.example       # 环境变量模板
├── package.json       # 依赖管理
├── README.md          # 项目说明文档
└── LICENSE            # 开源协议

---
🤝 贡献指南
欢迎所有开发者参与项目贡献，一起完善功能、修复Bug、优化体验！
1. Fork 本项目到自己的GitHub仓库
2. 新建功能分支：git checkout -b feature/新功能
3. 提交代码：git commit -m "feat: 新增XX功能/fix: 修复XX问题"
4. 推送分支：git push origin feature/新功能
5. 提交Pull Request，等待审核合并
💡 建议：提交前先运行代码校验，保持代码风格统一；新增功能请补充相关说明文档。

---
📌 版本规划
- v1.0：基础功能上线（文案生成、笔记管理、邮件草稿、基础大模型接入）
- v1.5：优化沉浸式体验、新增多图床适配、完善邮件管理功能
- v2.0：支持本地大模型部署、团队协作、数据导出备份、更多文案模板

---
❓ 常见问题（FAQ）
Q1：不配置自定义大模型可以使用吗？

    A：可以，默认开启演示模式，生成内容有限制，配置专属API Key后可解锁完整功能。
Q2：数据存储在哪里，会不会泄露隐私？

    A：默认本地存储，所有笔记、文案、邮件草稿仅保存在本地/部署服务器，大模型请求仅发送至你配置的接口，绝不上传第三方无关服务器。
Q3：支持本地离线使用吗？

    A：基础笔记功能支持离线，文案生成、AI邮件润色需联网（接入本地大模型后可完全离线）。
Q4：图床上传失败怎么办？

    A：检查令牌权限、仓库配置、网络状态，参考配置说明核对参数，也可切换本地存储模式。

---
📄 开源许可证
本项目基于 MIT License 开源，可自由使用、修改、分发，详情查看 LICENSE 文件。

---
⭐ 支持项目
如果这个项目对你有帮助，欢迎点个 Star 支持，也可以分享给更多需要的小伙伴～

    如有问题、功能建议，欢迎提交 Issues。