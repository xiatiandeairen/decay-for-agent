# 轨迹报告 — 统一的 health trajectory 视图

## 1. 问题

M2-M5 各自输出 velocity、regression、forecast、correlation，但信息分散在 Report 的不同字段中。用户和 agent 无法一眼看到"项目健康往哪走"的整体画面。

## 2. 目标用户

- AI agent：消费统一的 trajectory 结构，制定长期重构策略
- 开发者：一个段落看清健康轨迹全貌
- MCP server：返回结构化 trajectory 数据

## 3. 核心假设

**聚合 M2-M5 数据为 Trajectory 结构 + overall direction → agent 和用户都能快速判断项目健康走向。**

## 4. 方案

- 新增 `Trajectory` 结构体聚合所有趋势数据
- `build_trajectory()` 计算 overall direction（基于 composite velocity）
- Report 新增 `trajectory` 字段（向后兼容：保留原有零散字段）
- markdown 新增 "Health Trajectory" 段落替代分散的 Regressions/Forecasts/Correlations 段落
- terminal 新增轨迹摘要行
- MCP tool 返回 trajectory

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| trajectory-report tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- `build_trajectory()` 正确聚合所有趋势数据
- overall_direction 基于 composite 维度的 velocity slope 决定
- 无 velocity 数据时 overall_direction = Stable
- `--json` 输出包含 `trajectory` 字段
- markdown 输出有 "Health Trajectory" 段落，包含方向、velocity、回归、预警、相关性
- terminal 输出轨迹摘要
- MCP server 返回 trajectory
- 新增测试：build_trajectory 正常/空数据/无 composite
- `cargo test` 全部通过

## 6. 排除项

- 不删除原有零散字段（向后兼容）
- 不做轨迹可视化（CLI 文本足够）
