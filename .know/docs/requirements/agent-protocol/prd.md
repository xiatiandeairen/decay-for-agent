# Agent 协议集成 — MCP + Skill 输出结构化 Action

## 1. 问题

M1-M4 在 CLI JSON 输出中已包含结构化 actions 数组，但 MCP tool description 未提及 actions，skill 输出不展示 actions，markdown 输出无 actions 段落。agent 和用户看不到 v4 的核心产出。

## 2. 目标用户

- AI agent：通过 MCP 调用 decay 后直接消费 actions 数组生成重构计划
- 开发者：通过 `/decay` 或 `--markdown` 看到排序后的 actions 表格

## 3. 核心假设

**更新协议层描述 + 输出格式 → agent 知道 actions 的存在并直接消费。**

## 4. 方案

| 层 | Before | After |
|----|--------|-------|
| MCP tool description | "generate prescriptions" | "generate structured actions (type, target, priority, effort)" |
| Skill SKILL.md | 无 Actions 说明 | Actions 字段表格 + 使用指引 |
| Markdown output | 无 Actions 段落 | Actions 表格（priority/type/target/effort/reason） |
| JSON output | 已包含 actions | 无变更 |

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| agent-protocol tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- MCP tool description 提及 structured actions
- SKILL.md 包含 Actions 段落说明字段和用法
- `decay --markdown` 输出包含 Actions 表格（有 action 时）
- `decay --json` 输出包含 actions 数组（M1-M4 已实现）
- `cargo test` 77 tests 全部通过

## 6. 排除项

- 不修改 MCP 协议格式（仍返回完整 JSON）
- 不自动执行 action（decay 只输出）
- 不添加新 MCP tool（复用 decay_check）
