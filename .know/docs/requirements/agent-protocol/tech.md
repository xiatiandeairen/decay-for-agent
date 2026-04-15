# Agent 协议集成 技术方案

## 1. 背景

PRD 要求更新 MCP、Skill、Markdown 输出层以暴露 v4 的结构化 actions。

## 2. 方案

### 2.1 MCP tool description 更新

```typescript
// before
"Run project health check: scan files, analyze git history, score health, diagnose issues, and generate refactoring prescriptions"
// after
"Run project health check: scan files, analyze git history, score 8 dimensions, diagnose issues, and generate structured actions (type, target file+line, priority, effort). Returns JSON with top-level sorted actions array for direct consumption."
```

MCP 返回格式不变（完整 JSON），description 更新让 agent 知道 actions 数组的存在。

### 2.2 Skill SKILL.md 更新

新增 Actions 段落：
- 字段说明表格（action_type, target, priority, effort, reason）
- 建议使用 `--json` 获取完整 action 数据
- After Running 更新：引导用 actions 数组规划修复

### 2.3 Markdown 输出

MarkdownCtx 新增 `actions: &[Action]` 字段。
render_markdown 在 Issues 和分隔线之间插入 Actions 表格：

```markdown
## Actions

| Priority | Type | Target | Effort | Reason |
|----------|------|--------|--------|--------|
| CRITICAL | SPLIT | src/big.rs:10-50 | Large | ... |
```

actions 为空时不输出 Actions 段落。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `mcp/src/index.ts` | tool description 更新 |
| modify | `skills/decay/SKILL.md` | 新增 Actions 段落 + After Running 更新 |
| modify | `src/run.rs` | MarkdownCtx 加 actions，render_markdown 加 Actions 表格 |

## 4. 迭代记录

- 2026-04-15: 初始方案
