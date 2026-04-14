# Dimension trait 统一注册

## 1. 问题

现有 3 个维度（structural / complexity / fragility）的评分和诊断逻辑分散在 `score.rs` 和 `diagnose.rs` 中，以独立函数形式硬编码。`run.rs` 中的 `Scores` struct、`db.rs` 中的 scores 表、`diagnose.rs` 中的 `Category` enum 都与 3 个维度强耦合。v3 需要新增 5 个维度，当前架构每加一个维度需要改 4+ 个文件的多处代码，扩展成本高且容易遗漏。

## 2. 目标用户

decay 开发者（当前即维护者自己）。统一维度抽象后，新增维度只需实现 trait + 注册，无需修改调度代码。

## 3. 核心假设

**将维度抽象为统一 trait + 注册表 → 新增维度的实现成本从"改 4+ 文件"降为"新增 1 个文件 + 注册 1 行"。**

验证方式：重构后现有 3 个维度通过 trait 注册表统一调度，输出与重构前完全一致。

## 4. 方案

- **Before**: 3 个维度硬编码在 score.rs / diagnose.rs / run.rs / db.rs 中，新增维度需改多处
- **After**: 每个维度是一个独立模块，实现 Dimension trait，通过注册表统一调度

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| dimension-trait tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `cargo test` 全部通过
- `decay` 输出与重构前一致（3 个维度分数相同）
- `decay --json` 输出的 scores 包含所有维度分数
- 新增维度只需：创建 `src/dimension/xxx.rs`、实现 Dimension trait、在 `all_dimensions()` 中注册
- Category 不再是硬编码 enum，支持动态维度名
- DB 支持存储和读取任意维度的分数

## 6. 排除项

- 不新增任何维度（v3 M4-M8 负责）
- 不改采集层架构（v3 M2 负责）
- 不改分层打分系统（v3 M3 负责）
- 不改 composite score 加权逻辑（v3 M9 负责）
