# 衰退速度 技术方案

## 1. 背景

PRD 要求基于时间序列数据计算每个维度的分数变化率（velocity），为 M3-M6 提供基础数据。

## 2. 方案

### 2.1 Velocity 类型 (trend.rs)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Velocity {
    pub dimension: String,
    pub slope: f64,           // 线性回归斜率（分/快照）
    pub direction: Direction, // 方向标签
    pub data_points: usize,   // 参与计算的数据点数
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Direction {
    Improving,  // slope > 1.0
    Declining,  // slope < -1.0
    Stable,     // -1.0 ≤ slope ≤ 1.0
}
```

Display: Improving → "↑", Declining → "↓", Stable → "→"

### 2.2 线性回归 (trend.rs)

最小二乘法，x 为序号（0, 1, 2, ...），y 为分数：

```rust
fn linear_regression_slope(points: &[(i64, i32)]) -> Option<f64>
```

- < 2 个点 → None
- 所有 x 相同 → Some(0.0)
- 公式：slope = Σ((xi - x̄)(yi - ȳ)) / Σ((xi - x̄)²)

使用序号而非 snapshot_id 作为 x，避免 ID 间隔不均匀导致斜率失真。

### 2.3 calculate_velocities (trend.rs)

```rust
pub fn calculate_velocities(
    snapshots: &[crate::db::SnapshotScores],
) -> Vec<Velocity>
```

- 收集所有维度名称
- 对每个维度调用 `dimension_series()` + `linear_regression_slope()`
- < 3 个数据点的维度跳过（不输出 velocity）
- 按维度名称字母序排序

### 2.4 Report 集成 (run.rs)

```rust
pub struct Report {
    // ...existing...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub velocities: Vec<trend::Velocity>,
}
```

在 `run()` 中：time_series 有 ≥3 个快照时调用 `calculate_velocities()`。

### 2.5 渲染集成

**Terminal** (render.rs): 分数行追加 velocity 标签，如 `structural: 85 ↑(+2.3/snap)`

**Markdown** (render.rs): 分数表格新增 Velocity 列

**JSON**: 自动通过 serde 序列化

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/trend.rs` | Velocity + Direction + linear_regression_slope + calculate_velocities + 4 tests |
| modify | `src/run.rs` | Report.velocities + 调用集成 |
| modify | `src/render.rs` | terminal + markdown velocity 展示 |

## 4. 迭代记录

- 2026-04-15: 初始方案
