# 维度相关性 技术方案

## 1. 背景

PRD 要求对所有维度对计算 Pearson 相关系数，发现跨维度联动模式。

## 2. 方案

### 2.1 类型 (trend.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum CorrelationStrength {
    Strong,   // |r| > 0.6
    Moderate, // |r| > 0.4
}

#[derive(Debug, Clone, Serialize)]
pub struct Correlation {
    pub dim_a: String,
    pub dim_b: String,
    pub coefficient: f64,
    pub strength: CorrelationStrength,
    pub data_points: usize,
}
```

### 2.2 pearson_correlation (trend.rs)

```rust
fn pearson_correlation(xs: &[i32], ys: &[i32]) -> Option<f64>
```

r = Σ((xi-x̄)(yi-ȳ)) / sqrt(Σ(xi-x̄)² × Σ(yi-ȳ)²)
- < 2 个点 → None
- 任一方差为 0 → Some(0.0)

### 2.3 analyze_correlations (trend.rs)

```rust
pub fn analyze_correlations(
    snapshots: &[crate::db::SnapshotScores],
) -> Vec<Correlation>
```

算法：
1. 收集所有维度名称，排序
2. 对每个有序维度对 (a, b)，a < b
3. 提取两维度在同一快照中都有值的分数对
4. < 5 个共同点 → 跳过
5. 计算 Pearson r
6. |r| ≤ 0.4 → 跳过
7. 按 |r| 降序排序

### 2.4 Report + 渲染

Report.correlations, terminal/markdown 展示。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/trend.rs` | Correlation + CorrelationStrength + pearson_correlation + analyze_correlations + tests |
| modify | `src/run.rs` | Report.correlations + 调用 |
| modify | `src/render.rs` | terminal + markdown 相关性展示 |

## 4. 迭代记录

- 2026-04-15: 初始方案
