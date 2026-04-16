# 输出集成

## 1. 问题

### 痛点

- **程序无法消费**: 当前 decay 只有 terminal 人可读输出，AI agent 和自动化工具需要手动解析文本才能提取数据。
- **无法嵌入文档**: JSON 面向程序，terminal 面向人但无法嵌入 PR、issue 或对话中。缺少格式化报告。
- **脚本集成繁琐**: 外部工具（CI/git hook）需要简洁的健康状态判断，当前只有完整输出和 JSON 两种模式，脚本集成需要额外解析。
- **agent 无法自动调用**: v1 的 decay CLI 只能手动运行，AI agent 无法程序化调用。每次健康检查都需要用户主动切换终端。
- **打断工作流**: 用户需要切换到终端手动运行 `decay` 命令，打断 Claude Code 工作流。
- **协议层未暴露 actions**: CLI JSON 中已包含结构化 actions 数组，但 MCP tool description 未提及，skill 不展示，markdown 无 actions 段落。

### 影响范围

所有需要程序化集成 decay 的场景（MCP server、skill 入口、CI 脚本、PR 描述、团队报告），以及所有通过协议层消费 decay 的 agent 和开发者。

### 为什么现在做

后续 MCP server 和 skill 入口都依赖机器可读的结构化输出。多种输出模式（JSON/Markdown/quiet）补全输出矩阵，MCP + Skill 让 agent 能自动化调用，协议层更新让 v4 的 actions 在所有输出通道可见。

## 2. 目标用户

| 角色 | 场景 | Before | After |
|------|------|--------|-------|
| AI agent | 自动健康检查 | 无法解析 terminal 输出 | `--json` 获取结构化数据，直接消费 |
| AI agent | 通过 MCP 调用 decay | 无法调用 decay | 通过 MCP 协议发现并调用 decay_check tool |
| AI agent | 通过 MCP 消费 actions | tool description 只提 prescriptions | description 明确说明 structured actions |
| 自动化脚本 | CI/CD 集成 | 需要正则解析文本输出 | JSON 字段直接访问 |
| CI 脚本 | pipeline 健康门禁 | 需要解析 JSON 提取状态 | exit code 直接判断，0=健康/1=critical |
| `/decay` skill 用户 | 对话中查看报告 | 看到 terminal 纯文本，格式杂乱 | Markdown 表格驱动的格式化报告 |
| 开发者 | PR/issue 中引用 | 需要手动整理输出格式 | `--markdown` 输出可直接粘贴 |
| 开发者 | 通过 `/decay` 或 `--markdown` 查看 actions | 无 Actions 段落 | 看到排序后的 Actions 表格 |
| Claude Code 用户 | 对话中检查健康 | 需要切换终端运行 `decay` | `/decay` 直接在对话中输出 |
| Claude Code 用户 | 配置 MCP server | 无 MCP 集成 | settings.json 配置后 agent 自动发现 tool |

## 3. 核心假设

- **假设**: `--json` 输出结构化 JSON → 程序可直接解析，为 MCP/skill 集成铺路
- **验证方式**: `decay --json` 输出合法 JSON，下游 MCP server 可直接透传
- **假设**: `--markdown` 输出格式化报告 → 用户和 agent 可直接引用
- **验证方式**: 输出合法 Markdown，可被 GitHub/CommonMark 正确渲染
- **假设**: `--quiet` + 语义化 exit code → 任何工具都能零解析集成 decay 健康检查
- **验证方式**: exit code 0=健康/1=有 critical
- **假设**: 通过 MCP 暴露 decay_check tool → agent 能在任意时机自动调用健康检查
- **验证方式**: Claude Code 配置后 agent 能调用 `decay_check` 并获取完整 JSON
- **假设**: `/decay` slash command → 用户零学习成本触发健康检查
- **验证方式**: 在 Claude Code 中输入 `/decay` 触发检查并看到结果
- **假设**: 更新协议层描述 + 输出格式 → agent 知道 actions 的存在并直接消费
- **验证方式**: MCP description 包含 "structured actions"，`--markdown` 包含 Actions 表格

## 4. 方案

- **Before**: 只有人可读 terminal 输出 → **After**: `--json` 输出 scores/issues/trend/scan/git 的完整 JSON
- **Before**: 只有 terminal 和 JSON → **After**: `--markdown` 输出表格驱动的格式化报告
- **Before**: 脚本需要解析 → **After**: `--quiet` 一行摘要 + 语义化 exit code
- **Before**: agent 无法调用 decay → **After**: MCP stdio 协议直接调用 decay_check
- **Before**: 需要切换终端 → **After**: `/decay` 直接在对话中输出评分+诊断+处方
- **Before**: MCP 描述只提 prescriptions → **After**: MCP/Skill/Markdown 全部暴露 structured actions

### 任务追踪

| 任务 | Tech | 状态 | 备注 |
|------|------|------|------|
| JSON 输出 | [tech](tech.md) | 已完成 | serde_json 序列化 Report |
| Markdown 输出 | [tech](tech.md) | 已完成 | 模板 + str::replace 填充 |
| Quiet 模式 | [tech](tech.md) | 已完成 | exit code 语义化 + 一行摘要 |
| MCP Server | [tech](tech.md) | 已完成 | TypeScript + @modelcontextprotocol/sdk |
| Skill 入口 | [tech](tech.md) | 已完成 | SKILL.md + plugin.json |
| Agent 协议集成 | [tech](tech.md) | 已完成 | MCP/Skill/Markdown 暴露 actions |

## 5. 验收标准

- 用户执行 `decay --json` → 应看到合法 JSON，包含 scores/issues/trend/scan/git/actions 字段
- 用户执行 `decay --markdown` → 应看到 Scores 表格、Scan 摘要、Issues 列表的 Markdown，可被 GitHub 渲染
- 用户执行 `decay --quiet` → 应看到一行输出 `Health: 81/100 (0 critical)`，exit code 0/1
- 用户启动 MCP server → 应正常响应 initialize 和 tools/list，返回 decay_check tool
- agent 调用 `decay_check` → 应获取完整 JSON 结果
- 用户在 Claude Code 输入 `/decay` → 应看到健康检查结果
- 用户查看 MCP tool description → 应看到提及 structured actions
- 用户运行 `decay --markdown` 有 action 时 → 应看到 Actions 表格

## 6. 排除项

- 不包含 YAML/XML 等其他格式 — 当前只有 JSON 消费场景
- 不包含 HTML 输出 — Markdown 已覆盖文档嵌入需求
- 不包含自定义 exit code 阈值 — critical/非 critical 二分已满足
- 不包含多 MCP tool 拆分 — 单 tool 够用
- 不包含 HTTP/SSE 传输 — stdio 满足 Claude Code 场景
- 不包含 skill 参数传递 — 当前只需基本触发能力
- 不自动执行 action — decay 只输出
