# 输出集成 技术方案

## 1. 背景

多种输出模式（JSON/Markdown/quiet）+ 协议集成（MCP/Skill）+ 协议层 actions 暴露，让 decay 输出可被人、程序、agent 在各种场景消费。

### 技术约束

- Cli struct: 已有 clap 参数解析框架，新增 flag 需保持一致性
- Report struct: 需要 `#[derive(Serialize)]` 支持 JSON 序列化
- Markdown 模板: 通过 `include_str!` 编译时嵌入，CommonMark 兼容
- exit code: 遵循 Unix 惯例（0=成功, 非0=失败），`--quiet` 输出到 stdout，`--debug` 日志到 stderr
- MCP 协议: stdio transport 规范，支持 initialize/tools/list/tools/call
- MCP 格式: 只更新 tool description 文本，不修改返回 JSON 格式
- Markdown 渲染: actions 为空时不输出 Actions 段落

### 前置依赖

- composite-score — 已完成
- diagnosis — 已完成
- cli-framework — 已完成
- json-output — 已完成（MCP server 依赖 `decay --json`）
- action-schema — 已完成（Agent 协议集成依赖）

## 2. 方案

### 文件/模块结构

| 文件 | 职责 |
|------|------|
| `src/cli.rs` | 新增 `--json` / `--markdown` / `--quiet` flag |
| `src/report.rs` | Report struct 加 `#[derive(Serialize)]`，JSON 序列化逻辑 |
| `src/output/markdown.rs` | Markdown 渲染逻辑，模板填充 |
| `src/output/quiet.rs` | quiet 模式输出逻辑 + exit code 决定 |
| `templates/health-report.md` | Markdown 模板，`include_str!` 嵌入 |
| `Cargo.toml` | 新增 serde_json 依赖 |
| `mcp/package.json` | npm 配置 + 依赖 |
| `mcp/tsconfig.json` | TypeScript 编译配置 |
| `mcp/src/index.ts` | MCP server 入口，tool 注册，CLI 调用逻辑，tool description |
| `skills/decay/SKILL.md` | skill 定义：触发条件、使用说明、Actions 字段说明 |
| `.claude-plugin/plugin.json` | plugin 元数据注册 |
| `src/run.rs` | MarkdownCtx 加 actions，render_markdown 加 Actions 表格 |

### 核心流程

**JSON 输出**:
1. 用户传入 `--json` → clap 解析得到 `json: true`
2. 引擎执行完整分析 → 产出 Report struct
3. Report → serde_json::to_string_pretty → stdout

**Markdown 输出**:
1. 用户传入 `--markdown` → clap 解析得到 `markdown: true`
2. Report 数据 → str::replace 填充模板占位符（`{{structural}}`/`{{issues_section}}` 等）→ 格式化 Markdown

**Quiet 模式**:
1. 用户传入 `--quiet` → clap 解析
2. 统计 critical issues 数量
3. 输出 `Health: {composite}/100 ({N} critical)` → exit code 0（无 critical）或 1（有 critical）

**MCP Server**:
1. Claude Code 启动 MCP server → stdio transport 建立连接
2. agent 发送 `tools/list` → server 返回 `decay_check` tool 定义（参数: path 可选）
3. agent 发送 `tools/call` → child process 执行 `decay --json --path {path}` → 返回 JSON

**Skill 入口**:
1. 用户输入 `/decay` → Claude Code 加载 SKILL.md
2. AI 按指示通过 Bash tool 执行 `decay` CLI
3. CLI 输出 terminal 结果 → AI 展示给用户

**Agent 协议集成**:
1. `mcp/src/index.ts` 更新 tool description → agent 发现 actions 数组
2. `SKILL.md` 新增 Actions 段落 → 开发者了解 action 结构
3. `src/run.rs` render_markdown 在 Issues 和分隔线之间插入 Actions 表格

### 数据结构

**JSON Report 字段**:

| 字段 | 类型 | 用途 |
|------|------|------|
| snapshot_id | u64 | 快照标识 |
| scores | Object | 各维度评分 |
| trend | Object | 各维度趋势变化 |
| issues | Array | 诊断问题列表 |
| actions | Array | 结构化处方数组 |
| scan | Object | 文件扫描统计 |
| git | Object | Git 分析统计 |

**Quiet 模式 exit code**:

| exit code | 含义 | 条件 |
|-----------|------|------|
| 0 | 健康 | 无 critical issues |
| 1 | 有问题 | 存在 critical issues |
| 2 | 执行错误 | 已由 anyhow 处理 |

**MCP CLI 路径发现**:

| 优先级 | 路径 | 场景 |
|--------|------|------|
| 1 | `../target/release/decay` | 同仓库 release build |
| 2 | `../target/debug/decay` | 同仓库 debug build |
| 3 | 系统 PATH `decay` | 全局安装 |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 序列化库 | serde_json | Rust 生态标准做法 |
| 输出结构 | 单个 Report 对象 | 一次序列化输出完整报告 |
| 模板引擎 | str::replace | 零依赖，占位符数量少（~10 个） |
| 模板嵌入方式 | include_str! | 编译时嵌入无运行时文件依赖 |
| exit code 语义 | 0=ok, 1=critical | 标准 Unix 惯例 |
| quiet 输出内容 | composite + critical 数 | 最小有用信息 |
| MCP 语言 | TypeScript | MCP 官方 SDK 最成熟 |
| MCP 位置 | mcp/ 目录 | 同仓库维护版本一致 |
| MCP CLI 发现 | 同仓库优先 → 回退 PATH | 开发和生产都能工作 |
| Skill 调用方式 | Bash tool 调用 CLI | 不依赖 MCP 配置 |
| Skill 输出格式 | terminal 原始输出 | v1 输出已可读 |
| MCP 更新范围 | 只改 description 文本 | JSON 已包含 actions，无需改格式 |
| Actions 表格位置 | Issues 和分隔线之间 | actions 是 issues 的结构化提炼，逻辑上紧跟 issues |
| 空 actions 处理 | 不输出 Actions 段落 | 保持输出简洁 |

## 4. 迭代记录

### 2026-04-14

- JSON 输出：Cli struct 加 `--json` flag，serde_json 序列化 Report
- Markdown 输出：templates/health-report.md 模板，include_str! 编译嵌入
- Quiet 模式：`--quiet` flag，一行摘要输出，exit code 0/1
- MCP Server：TypeScript + @modelcontextprotocol/sdk，stdio transport，单 tool decay_check
- Skill 入口：SKILL.md + plugin.json，Bash tool 调用 CLI

### 2026-04-15

- Agent 协议集成：MCP description 更新，SKILL.md 新增 Actions 段落，Markdown 加 Actions 表格
