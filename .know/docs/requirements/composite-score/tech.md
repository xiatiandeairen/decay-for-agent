# composite 合成 技术方案

## 1. 背景

PRD 要求将三个维度加权合成为 composite 分数。某维度 N/A 时不参与合成。

## 2. 方案

等权平均。三个维度都有值时 composite = (s + c + f) / 3。某维度 N/A 时权重重分配到其余维度。

### API

`score::composite(structural: i32, complexity: i32, fragility: Option<i32>) -> i32`

### 输出格式

```
Health: 75/100 (structural: 85, complexity: 70, fragility: 70)
```

fragility N/A 时：
```
Health: 78/100 (structural: 85, complexity: 70, fragility: N/A)
```

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 加权方式 | 等权 | PRD 排除可配置权重 |
| N/A 处理 | 不参与合成 | 不因缺失数据惩罚 |

## 4. 迭代记录

- 2026-04-14: 初始方案
