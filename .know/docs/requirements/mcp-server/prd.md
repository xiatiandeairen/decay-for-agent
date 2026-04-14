# MCP server

## 1. 问题

v1 的 decay CLI 只能手动运行，AI agent 无法程序化调用。要让 agent 在工作流中自动触发健康检查，需要通过 MCP 协议暴露 decay 能力。

## 2. 目标用户

Claude Code 中的 AI agent。agent 通过 MCP 协议发现并调用 decay tool，无需用户手动运行命令。

## 3. 核心假设

**通过 MCP 暴露 decay_check tool → agent 能在任意时机自动调用健康检查，解锁自动化巡检场景。**

验证方式：Claude Code 配置 MCP server 后，agent 能调用 `decay_check` 并获取完整 JSON 结果。

## 4. 方案

- **Before**: agent 无法调用 decay，只能提示用户手动运行 → **After**: agent 通过 MCP 直接调用，获取结构化结果

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| MCP server tech | — | 0/0 |

## 5. 验收标准

- MCP server 以 stdio 模式启动，响应 `initialize` 和 `tools/list` 请求
- `tools/list` 返回 `decay_check` tool，包含参数定义（path: 可选）
- 调用 `decay_check` → 内部执行 `decay --json`，返回完整 JSON 结果
- 调用 `decay_check` 指定 path → 在指定目录执行
- 非 git 项目调用 → 返回结果（fragility: N/A），不报错
- Claude Code settings.json 配置后 → agent 能发现并调用 tool

## 6. 排除项

- 不包含多 tool（score/diagnose/trend 等独立 tool → 后续版本）
- 不包含 HTTP/SSE 传输（只做 stdio）
- 不包含认证/权限控制
- 不包含 tool 结果缓存
