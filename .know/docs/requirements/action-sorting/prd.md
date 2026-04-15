# Action 优先级排序 — 按影响范围和修复成本排序

## 1. 问题

M3 完成后 actions 有位置精度，但顶层 actions 数组只按 Priority 单维排序，同优先级内无序。agent 在同优先级中无法判断先做哪个。且多个维度可能对同一文件产生重复 action。

## 2. 目标用户

- AI agent：消费排序后的 actions，先做高优先级+低成本的修复
- 开发者：快速看到"最值得先修"的 action

## 3. 核心假设

**按 Priority → Effort 双键排序 + 去重 → agent 按数组顺序执行即为最优修复路径。**

## 4. 方案

- 排序策略：Priority asc（Critical 在前）→ Effort asc（Small 在前）
- 去重策略：同 dimension + file + action_type 的重复 action 只保留第一个（更高严重度）

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| action-sorting tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- Effort 有 Ord derive，可参与排序
- 顶层 actions 按 priority → effort 双键排序
- 同 dimension+file+action_type 的重复 action 去重
- 新增排序和 effort ordering 测试
- `cargo test` 77 tests 全部通过

## 6. 排除项

- 不引入复合评分公式（priority × effort 加权）— 当前枚举排序足够
- 不跨维度去重（不同维度对同文件的不同 action_type 应保留）
