# Trend Analysis Architecture

## 概述

趋势分析子系统基于 SQLite 中存储的历史 dimension_scores 时间序列数据，提供四种分析能力：velocity（线性回归斜率）、regression detection（均值 +/- k*sigma 异常检测）、threshold forecast（线性外推预测）、dimension correlation（Pearson 相关系数）。所有分析结果聚合到 Trajectory 结构体中统一输出。

## 架构图

```
dimension_scores (SQLite)
  │
  │  db::get_dimension_time_series()
  │  → Vec<SnapshotScores>  (最近 20 个快照)
  ▼
┌────────────────────────────────────────────────┐
│              build_trajectory()                 │
│                                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │ velocity │  │regression│  │ forecast │     │
│  │ 线性回归  │  │ σ 异常   │  │ 线性外推  │     │
│  │ slope    │  │ 检测     │  │ R²>0.7   │     │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘     │
│       │             │             │            │
│       │  ┌──────────┴──────────┐  │            │
│       │  │    correlation      │  │            │
│       │  │  Pearson 相关系数    │  │            │
│       │  └─────────┬──────────┘  │            │
│       └────────────┼─────────────┘            │
│                    ▼                           │
│             Trajectory                         │
│  { overall_direction, velocities,              │
│    regressions, forecasts, correlations }       │
└────────────────────────────────────────────────┘
```

## 核心抽象

### 时间序列存储

```sql
-- dimension_scores 表 (key-value 格式)
CREATE TABLE dimension_scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    dimension TEXT NOT NULL,
    score INTEGER  -- NULL 表示该维度不可用
);
```

查询接口：`db::get_dimension_time_series(conn, project_path, limit)` 返回最近 N 个快照的所有维度分数，按时间从旧到新排序。默认 limit=20。

### 共享数学工具 (`trend/math.rs`)

```rust
// 最小二乘线性回归斜率（x 轴使用顺序索引 0,1,2,...）
fn linear_regression_slope(points: &[(i64, i32)]) -> Option<f64>

// 总体标准差
fn std_dev(values: &[f64]) -> f64

// 提取单维度时间序列，跳过 None 值
pub fn dimension_series(snapshots: &[SnapshotScores], dimension: &str) -> Vec<(i64, i32)>
```

### Velocity (`trend/velocity.rs`)

用线性回归斜率衡量各维度的变化速度。

- **最少数据点**: 3
- **方向判定**: slope > 1.0 → Improving, slope < -1.0 → Declining, 否则 Stable
- **输出**: `Vec<Velocity>` 按维度名排序

```rust
pub struct Velocity {
    pub dimension: String,
    pub slope: f64,          // 每快照的分数变化量
    pub direction: Direction, // Improving / Declining / Stable
    pub data_points: usize,
}
```

### Regression Detection (`trend/regression.rs`)

检测最近一次分数下降是否异常（相对于历史波动）。

- **最少数据点**: 3
- **算法**: 计算相邻快照的分数差序列，取最后一次 diff 与历史 diff 的标准差比较
- **触发条件**: `|last_diff| > k * sigma`（默认 k=2.0）或 sigma=0（无历史波动时任何下降都是异常）
- **严重度**: `|last_diff| > 2k * sigma` → Severe，否则 Moderate

```rust
pub struct Regression {
    pub dimension: String,
    pub previous_score: i32,
    pub current_score: i32,
    pub drop: i32,
    pub threshold: f64,        // k * sigma
    pub severity: RegressionSeverity, // Moderate / Severe
}
```

### Threshold Forecast (`trend/forecast.rs`)

预测维度分数何时会跌破健康阈值。

- **最少数据点**: 5
- **前提条件**: slope < 0（正在下降）且 R^2 > 0.7（趋势可靠）且当前分数 > 阈值（尚未突破）
- **预测公式**: `snapshots_until_breach = ceil((current - threshold) / |slope|)`
- **默认阈值**: 60 分
- **排序**: 按 snapshots_until_breach 升序（最紧急的排前面）

```rust
pub struct Forecast {
    pub dimension: String,
    pub current_score: i32,
    pub slope: f64,
    pub r_squared: f64,
    pub threshold: i32,
    pub snapshots_until_breach: u32,
}
```

### Dimension Correlation (`trend/correlation.rs`)

分析维度间的 Pearson 相关系数，揭示维度间的联动关系。

- **最少数据点**: 5（per pair）
- **过滤**: |r| > 0.4
- **强度分级**: |r| > 0.6 → Strong，否则 Moderate
- **排序**: 按 |r| 降序

```rust
pub struct Correlation {
    pub dim_a: String,
    pub dim_b: String,
    pub coefficient: f64,        // Pearson r
    pub strength: CorrelationStrength, // Strong / Moderate
    pub data_points: usize,
}
```

### Trajectory (`trend/trajectory.rs`)

聚合所有趋势分析结果的统一结构体。

```rust
pub struct Trajectory {
    pub overall_direction: Direction, // 取 composite 维度的 velocity direction
    pub snapshot_count: usize,
    pub velocities: Vec<Velocity>,
    pub regressions: Vec<Regression>,
    pub forecasts: Vec<Forecast>,
    pub correlations: Vec<Correlation>,
}
```

`build_trajectory()` 调用入口：要求至少 3 个快照。在 `run.rs` 中以 `k=2.0, threshold=60` 调用。

## 数据流

1. `run.rs` 调用 `db::get_dimension_time_series()` 获取最近 20 个快照的维度分数
2. 检查快照数量 >= 3，满足则调用 `build_trajectory()`
3. `build_trajectory()` 并行调用四个分析模块：
   - `calculate_velocities()` — 每维度线性回归
   - `detect_regressions()` — 最近一次 diff 异常检测
   - `forecast_breaches()` — 线性外推预测
   - `analyze_correlations()` — 全维度对 Pearson 系数
4. 从 composite 维度的 velocity 提取 `overall_direction`
5. 结果写入 Report 的 `trajectory`, `velocities`, `regressions`, `forecasts`, `correlations` 字段
6. 输出层根据格式渲染趋势信息

## 设计约束

- **顺序索引** — 线性回归用顺序索引（0,1,2,...）作为 x 轴，而非 snapshot_id 或时间戳，避免 ID 间隙影响
- **最小数据点** — velocity/regression 至少 3 个，forecast/correlation 至少 5 个，确保统计意义
- **R^2 门槛** — forecast 要求 R^2 > 0.7，过滤掉噪声大、趋势不明显的维度
- **单快照粒度** — 所有预测以"快照数"为单位，不做时间推算（快照频率由用户决定）

## 扩展点

- **新分析模块** — 在 `trend/` 下添加新模块，在 `build_trajectory()` 中调用并将结果加入 Trajectory
- **自定义参数** — regression_k 和 forecast_threshold 可通过 CLI 参数或配置暴露
- **时间轴** — 当前用顺序索引；如需按时间回归，可使用 `SnapshotScores::created_at` 字段计算天数间隔
