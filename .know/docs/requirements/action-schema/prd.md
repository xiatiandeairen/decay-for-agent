# Action Schema — 结构化处方类型系统

## 1. 问题

现有处方（prescription）是纯文本字符串（`Option<String>`），如 "split into sub-modules by responsibility"。agent 无法解析文本建议来自动执行重构——它不知道要操作哪个文件、做什么类型的变更、优先级如何。用户也无法对处方排序或过滤。

## 2. 目标用户

- **AI agent**：读取 decay JSON 输出后，直接解析 action 生成重构计划，无需理解自由文本
- **开发者**：通过优先级和变更类型筛选最值得处理的问题

## 3. 核心假设

**将文本处方结构化为 Action（包含变更类型、目标文件、原因、优先级、工作量估算） → agent 可直接消费 decay 输出执行重构。**

验证方式：structural 维度输出的 Action 可序列化为 JSON，包含所有必要字段，agent 可直接解析。

## 4. 方案

- **Before**: `prescription: Option<String>` — "split into sub-modules by responsibility"
- **After**: `actions: Vec<Action>` — `{ action_type: "split", target: { file: "src/" }, priority: "high", effort: "large", reason: "1200 files exceed threshold" }`

Issue 同时保留 `prescription` 文本字段用于 CLI 人类可读输出，双写过渡期在 M2 完成迁移后由 M3 移除。

Report 顶层新增 `actions: Vec<Action>`，从所有 issue 收集、按 priority 排序，供 agent 直接消费。

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| action-schema tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- Action Schema 类型定义完整：ActionType (7 种)、Priority (4 级)、Effort (3 级)、Target、Action
- 所有类型可序列化为 JSON（`serde::Serialize`）且格式稳定
- Issue 新增 `actions: Vec<Action>` 字段，默认空，不破坏现有输出
- Report 新增顶层 `actions` 字段，JSON 输出包含结构化 action 数组
- structural 维度 POC：3 个 Issue 附带对应 Action
- `cargo test` 全部通过，包含 Action 序列化测试
- CLI 文本输出格式不变（action 不影响终端显示）

## 6. 排除项

- 不迁移其他 7 个维度（M2 负责）
- 不填充 target.line_range / target.symbol 精度（M3 负责）
- 不实现 action 去重和高级排序（M4 负责）
- 不修改 MCP/skill 集成（M5 负责）
- 不自动执行 action（decay 只输出，不执行）
