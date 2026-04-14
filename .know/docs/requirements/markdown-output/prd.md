# Markdown 输出

## 1. 问题

JSON 输出面向程序，terminal 输出面向人但无法嵌入文档。需要一种可嵌入对话和文档的格式化报告。

## 2. 目标用户

使用 `/decay` skill 的用户和 AI agent。获取可直接粘贴到 PR、issue 或对话中的健康报告。

## 3. 核心假设

**`--markdown` 输出格式化报告 → 用户和 agent 可直接引用，无需手动整理。**

验证方式：`decay --markdown` 输出合法 Markdown，包含评分、趋势、问题。

## 4. 方案

- **Before**: 只有 terminal 纯文本和 JSON → **After**: `--markdown` 输出表格驱动的格式化报告

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| Markdown 输出 tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `decay --markdown` → 输出包含 Scores 表格、Scan 摘要、Issues 列表的 Markdown
- 输出可被 GitHub/CommonMark 正确渲染
- 无 issues 时 → 输出 "No issues found."

## 6. 排除项

- 不包含 HTML 输出
- 不包含自定义模板路径（使用内置模板）
