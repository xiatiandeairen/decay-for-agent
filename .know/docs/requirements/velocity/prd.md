# 衰退速度 — 维度分数变化率计算

## 1. 问题

M1 建立了时间序列查询能力，但只有原始分数序列，没有变化率。用户和 agent 无法判断某个维度是在快速恶化还是缓慢改善，只能人工对比数字。

## 2. 目标用户

- AI agent：基于 velocity 判断哪些维度需要优先干预
- 开发者：通过方向标签快速了解健康趋势
- v5 后续里程碑（M3-M6）：回归检测和阈值预警依赖 velocity 数据

## 3. 核心假设

**线性回归斜率 + 方向标签 → agent 和用户都能快速判断趋势方向和速度。**

验证条件：≥3 个快照时输出 velocity，方向标签准确反映分数走势。

## 4. 方案

- 复用 M1 的 `dimension_series()` 提取单维度分数序列
- 对每个维度计算线性回归斜率（最小二乘法），作为 velocity
- 斜率映射为方向标签：Improving(↑) / Declining(↓) / Stable(→)
- Report JSON 新增 `velocities` 字段
- terminal 和 markdown 渲染展示 velocity 信息

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| velocity tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `linear_regression_slope()` 对 ≥2 个数据点返回斜率，<2 返回 None
- `calculate_velocities()` 对所有维度计算 velocity，<3 个快照的维度返回 None
- 方向标签阈值：斜率 > 1.0 → Improving，< -1.0 → Declining，其余 → Stable
- `--json` 输出包含 `velocities` 数组（有数据时）
- terminal 输出在分数旁展示方向标签
- markdown 输出包含 velocity 列
- 新增测试：线性回归精度、方向标签映射、空序列、不足快照降级
- `cargo test` 全部通过

## 6. 排除项

- 不做回归检测（M3 负责）
- 不做阈值预警（M4 负责）
- 不做加速度计算（二阶导数，如有需要 v6+ 考虑）
- 不做滑动窗口（全量数据线性回归，窗口化留给 M3+）
