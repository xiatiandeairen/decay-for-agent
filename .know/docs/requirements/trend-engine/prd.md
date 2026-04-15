# 趋势引擎 — 快照序列存储 + 时间序列查询

## 1. 问题

现有趋势系统只对比当前快照和上一个快照（2 点对比），无法查询完整历史分数序列。v5 的衰退速度、回归检测、阈值预警都需要时间序列数据作为基础。

## 2. 目标用户

- v5 后续里程碑（M2-M6）：消费时间序列数据进行趋势分析
- AI agent：JSON 输出中包含历史分数序列

## 3. 核心假设

**复用现有 dimension_scores 表 + 新增时间序列查询 API → M2-M6 可直接消费时序数据。**

## 4. 方案

- 复用 `dimension_scores` 和 `snapshots` 表，不新建表
- 新增 `get_dimension_time_series()` 查询 API
- 新增 `dimension_series()` 从快照序列提取单维度分数序列
- Report JSON 输出新增 `time_series` 字段

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| trend-engine tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `get_dimension_time_series()` 返回按时间排序的快照分数序列
- 支持 `limit` 参数控制最大快照数
- `dimension_series()` 从序列提取单维度数据，跳过 None 值
- `--json` 输出包含 `time_series`（有数据时）
- 新增 4 个测试：空序列、多快照、limit、dimension_series
- `cargo test` 81 tests 全部通过

## 6. 排除项

- 不计算衰退速度（M2 负责）
- 不做回归检测（M3 负责）
- 不做阈值预警（M4 负责）
- 不做维度相关性分析（M5 负责）
