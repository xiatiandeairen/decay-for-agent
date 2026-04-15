# 轨迹报告 技术方案

## 1. 背景

PRD 要求将 M2-M5 分散的趋势数据聚合为统一的 Trajectory 结构。

## 2. 方案

### 2.1 Trajectory 类型 (trend.rs)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Trajectory {
    pub overall_direction: Direction,
    pub snapshot_count: usize,
    pub velocities: Vec<Velocity>,
    pub regressions: Vec<Regression>,
    pub forecasts: Vec<Forecast>,
    pub correlations: Vec<Correlation>,
}
```

### 2.2 build_trajectory (trend.rs)

```rust
pub fn build_trajectory(
    snapshots: &[crate::db::SnapshotScores],
    regression_k: f64,
    forecast_threshold: i32,
) -> Trajectory
```

- 调用 calculate_velocities, detect_regressions, forecast_breaches, analyze_correlations
- overall_direction: 从 velocities 中找 composite 维度的 direction，无则 Stable

### 2.3 Report 集成 (run.rs)

```rust
pub struct Report {
    // ...existing fields retained for backward compat...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
}
```

trajectory 仅在 snapshot_count ≥ 3 时填充。

### 2.4 渲染集成

**Markdown**: 合并 Regressions/Forecasts/Correlations 为 "Health Trajectory" 段落。velocity 已在 Scores 表中。

**Terminal**: 轨迹摘要行: `Trajectory: {direction} | {N} velocities, {N} regressions, {N} forecasts, {N} correlations`

### 2.5 MCP 集成

检查 MCP server 是否需要更新以返回 trajectory。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/trend.rs` | Trajectory + build_trajectory + 3 tests |
| modify | `src/run.rs` | Report.trajectory + 替换分散调用 |
| modify | `src/render.rs` | 统一 trajectory 渲染 |
| check  | MCP server | 确认 JSON 输出已包含 trajectory |

## 4. 迭代记录

- 2026-04-15: 初始方案
