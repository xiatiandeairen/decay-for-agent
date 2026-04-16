# Scoring Model: 扣分制

## 背景

decay 需要一个评分模型将代码健康问题量化为 0-100 分的分数。分数需要直观、可解释、跨项目可比较。需要处理多维度加权和部分维度不可用的情况。

## 选项

| 方案 | 优势 | 劣势 |
|------|------|------|
| 扣分制（100 起步向下扣） | 直觉："满分 = 无问题"、问题与扣分直接对应、新项目自然高分 | 分数有底部压缩风险（扣完就是 0） |
| 加分制（0 起步向上加） | 可精确衡量做到了什么 | 不直觉：无法区分"未检测"和"未达标"、新空项目 = 0 分 |
| 百分位制（与同类项目对比排名） | 相对排名、避免绝对分数争议 | 需要基准数据库、无法独立运行、无法回答"我的代码好不好" |

## 决策

选择扣分制。从 100 分起步，每检测到一个问题扣固定分数。理由：

1. **直觉性** — "85 分 = 很好、65 分 = 需要关注、40 分 = 严重问题"，任何人都能理解
2. **问题可追溯** — 每次扣分都对应一个具体的 Issue，用户可以精确知道"为什么丢了这 15 分"
3. **新项目友好** — 空项目或小项目自然得高分（没有问题 = 100 分），不需要刻意"做什么才能加分"
4. **底部保护** — `score.max(0)` 防止负分

### 扣分幅度

```rust
// 典型扣分矩阵
// Critical 级别：30-45 分
score -= 40;  // structural: 文件数 > 1000
score -= 45;  // complexity: 大文件比例 > 40%
score -= 45;  // fragility: churn 集中度 > 70%

// Warning 级别：15-25 分
score -= 20;  // structural: 文件数 > 500
score -= 25;  // complexity: 大文件比例 > 20%
score -= 25;  // fragility: churn 集中度 > 50%

// 次要扣分：10-15 分
score -= 15;  // complexity: 平均文件 > 10KB
score -= 10;  // complexity: 最大文件 > 50KB
score -= 15;  // structural: 顶层目录 > 15
score -= 15;  // fragility: 存在高 churn 文件
```

## Per-ProjectType 权重分配

`ScoreProfile` 为 6 种项目类型定义不同的维度权重：

| 维度 | Cli | WebService | Library | MobileApp | Monorepo | Generic |
|------|-----|-----------|---------|-----------|----------|---------|
| structural | 0.12 | 0.10 | 0.12 | 0.10 | 0.18 | 0.125 |
| complexity | 0.18 | 0.12 | 0.15 | 0.15 | 0.12 | 0.125 |
| fragility | 0.12 | 0.15 | 0.10 | 0.12 | 0.12 | 0.125 |
| maintainability | 0.15 | 0.12 | 0.20 | 0.13 | 0.15 | 0.125 |
| observability | 0.08 | 0.18 | 0.05 | 0.12 | 0.08 | 0.125 |
| quality_assurance | 0.15 | 0.13 | 0.18 | 0.13 | 0.15 | 0.125 |
| reliability | 0.10 | 0.12 | 0.12 | 0.10 | 0.10 | 0.125 |
| performance | 0.10 | 0.08 | 0.08 | 0.15 | 0.10 | 0.125 |

权重设计逻辑：
- **WebService** 重 observability（0.18）— 服务端的日志和错误处理至关重要
- **Library** 重 maintainability（0.20）和 quality_assurance（0.18）— 库的 API 稳定性和测试覆盖更重要
- **MobileApp** 重 performance（0.15）— 移动端性能敏感
- **Monorepo** 重 structural（0.18）— 大仓库的目录结构是核心问题
- **Generic** 等权（0.125 x 8 = 1.0）— 无法推断项目特性时公平对待

## Composite 计算与 N/A 处理

```rust
pub fn weighted_composite(&self, scores: &HashMap<String, Option<i32>>) -> i32 {
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;

    for (name, score) in scores {
        if name == "composite" { continue; }  // 跳过已有的 composite
        if let Some(s) = score {
            let w = self.weights.get(name).copied().unwrap_or(1.0);
            weighted_sum += *s as f64 * w;
            weight_sum += w;
        }
        // None 分数被跳过，不参与计算
    }

    if weight_sum == 0.0 { return 0; }
    (weighted_sum / weight_sum).round() as i32
}
```

关键行为：
- **N/A 自动排除** — `score: None` 的维度不参与加权，权重在有效维度间自动重新归一化
- **无需预设 fallback** — 不用给 N/A 维度假设默认分数，避免失真
- **示例**: 如果 fragility 是 None（无 git 历史），structural=80, complexity=80 的项目在 Generic 配置下得 80 分（而非 80 * (0.125+0.125) / (0.125*3) = 53 分）

## 后果

- 分数在 [0, 100] 范围内，直观易理解
- 同一项目的分数可跨时间对比（同样的问题 = 同样的扣分）
- 不同 ProjectType 的分数不直接可比（权重不同）
- N/A 维度的自动排除意味着：有 git 历史的项目比没有的项目多了一个评估维度，但这不影响分数范围
- 扣分阈值是硬编码的常量，不同规模的项目可能需要不同的阈值（当前未实现配置化）

## 状态

已确认 — 项目初始
