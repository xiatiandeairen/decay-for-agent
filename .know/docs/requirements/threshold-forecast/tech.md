# 阈值预警 技术方案

## 1. 背景

PRD 要求基于线性外推预测维度分数何时跌破健康阈值，仅在趋势可靠时（R² > 0.7）输出预警。

## 2. 方案

### 2.1 类型 (trend.rs)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Forecast {
    pub dimension: String,
    pub current_score: i32,
    pub slope: f64,
    pub r_squared: f64,
    pub threshold: i32,
    pub snapshots_until_breach: u32,
}
```

### 2.2 R² 决定系数 (trend.rs)

```rust
fn r_squared(points: &[(i64, i32)]) -> Option<f64>
```

R² = 1 - SS_res / SS_tot
- SS_tot = Σ(yi - ȳ)²
- SS_res = Σ(yi - ŷi)²，ŷi = intercept + slope × i
- < 2 个点 → None
- SS_tot = 0（所有值相同）→ Some(1.0)

### 2.3 forecast_breaches (trend.rs)

```rust
pub fn forecast_breaches(
    snapshots: &[crate::db::SnapshotScores],
    threshold: i32,
) -> Vec<Forecast>
```

算法：
1. 对每个维度提取分数序列
2. < 5 个数据点 → 跳过
3. 计算 slope 和 R²
4. slope ≥ 0 → 跳过（不在恶化）
5. R² ≤ 0.7 → 跳过（趋势不可靠）
6. current_score ≤ threshold → 跳过（已跌破）
7. snapshots_until_breach = ceil((threshold - current_score) / slope).abs()
8. 结果按 snapshots_until_breach 升序（最紧迫的优先）

默认 threshold = 60。

### 2.4 Report 集成 (run.rs)

```rust
pub struct Report {
    // ...existing...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forecasts: Vec<trend::Forecast>,
}
```

### 2.5 渲染集成

**Terminal**: `⚡ FORECAST: {dimension} will breach {threshold} in ~{N} snapshots (R²={r²:.2})`

**Markdown**: Forecasts 段落

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/trend.rs` | Forecast + r_squared + forecast_breaches + 5 tests |
| modify | `src/run.rs` | Report.forecasts + 调用 |
| modify | `src/render.rs` | terminal + markdown 预警展示 |

## 4. 迭代记录

- 2026-04-15: 初始方案
