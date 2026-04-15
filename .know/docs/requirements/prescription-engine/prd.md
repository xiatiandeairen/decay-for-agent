# Prescription 引擎重构 — 8 维度迁移为 Action 生成器

## 1. 问题

M1 定义了 Action Schema 并在 structural 维度做了 POC。剩余 7 个维度仍用 `actions: vec![]` 空数组，处方信息只在文本 `prescription` 字段中，agent 无法消费。

## 2. 目标用户

- AI agent：所有维度的处方均可结构化消费
- decay 维护者：所有维度统一使用 `Issue::new()` / `Issue::with_actions()` 构造函数

## 3. 核心假设

**将 8 个维度全部迁移为 Action 生成器 → JSON 输出的 actions 数组覆盖所有可操作的处方。**

验证方式：所有维度的 Warning/Critical 级 issue 都附带结构化 action，Info 级 issue 使用 `Issue::new()`（无 action）。

## 4. 方案

- **Before**: 7 个维度用 `actions: vec![]`，Issue 用 struct literal 构造
- **After**: 所有维度用 `Issue::with_actions()` 或 `Issue::new()`，Warning/Critical 附带 Action

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| prescription-engine tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- Issue 有 `new()` 和 `with_actions()` 构造函数
- 8 个维度全部使用构造函数，无 struct literal 构造 Issue
- Warning/Critical 级 issue 附带对应 ActionType 的 Action
- Info 级 issue 使用 `Issue::new()` 不附带 action
- `cargo test` 75 tests 全部通过
- CLI 文本输出格式不变

## 6. 排除项

- 不修改 Action Schema 定义（M1 已完成）
- 不移除 prescription 字段（M3 负责）
- 不实现位置精度提升（M3 负责）
- 不实现优先级排序优化（M4 负责）
