# 回归检测 技术方案

## 1. 背景

PRD 要求基于历史分数差值的统计特征，自动检测最新快照的显著分数下降。

## 2. 方案

### 2.1 类型 (trend.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum RegressionSeverity {
    Moderate,  // drop > k × σ
    Severe,    // drop > 2k × σ
}

#[derive(Debug, Clone, Serialize)]
pub struct Regression {
    pub dimension: String,
    pub previous_score: i32,
    pub current_score: i32,
    pub drop: i32,
    pub threshold: f64,
    pub severity: RegressionSeverity,
}
```

### 2.2 标准差 (trend.rs)

```rust
fn std_dev(values: &[f64]) -> f64
```

总体标准差（σ = √(Σ(xi - x̄)² / N)），空序列返回 0.0。

### 2.3 detect_regressions (trend.rs)

```rust
pub fn detect_regressions(
    snapshots: &[crate::db::SnapshotScores],
    k: f64,
) -> Vec<Regression>
```

算法：
1. 对每个维度调用 `dimension_series()` 获取分数序列
2. < 3 个数据点 → 跳过
3. 计算相邻差值序列：`diffs[i] = scores[i+1] - scores[i]`
4. 计算差值的标准差 σ
5. 最新差值（last diff）< 0 且 |last diff| > k × σ → 回归
6. |last diff| > 2k × σ → Severe，否则 Moderate
7. σ = 0（所有差值相同）时，任何下降都标记为回归

结果按维度名称字母序排序。

### 2.4 Report 集成 (run.rs)

```rust
pub struct Report {
    // ...existing...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regressions: Vec<trend::Regression>,
}
```

调用：`detect_regressions(&time_series, 2.0)`

### 2.5 渲染集成

**Terminal** (render.rs): 回归时在 issues 前打印警告行：`⚠ REGRESSION: {dimension} {prev} → {current} (−{drop}, threshold: {threshold:.1})`

**Markdown** (render.rs): Scores 表后新增 Regressions 段落

**JSON**: serde 自动序列化

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/trend.rs` | Regression + RegressionSeverity + std_dev + detect_regressions + 5 tests |
| modify | `src/run.rs` | Report.regressions + 调用集成 |
| modify | `src/render.rs` | terminal + markdown 回归警告 |

## 4. 迭代记录

- 2026-04-15: 初始方案
