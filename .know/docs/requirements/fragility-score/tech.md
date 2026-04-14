# fragility 评分 技术方案

## 1. 背景

PRD 要求基于 git 变更模式计算 0-100 的 fragility 分数。数据来自 git_changes 表。无 git 数据时返回 N/A。

## 2. 方案

扣分制，100 分起。从 git_changes 表聚合 churn 集中度和最高 churn 文件，超过阈值扣分。

### 扣分规则

| 指标 | 阈值 | 扣分 |
|------|------|------|
| top 10% 文件承担 churn | >50% | -25 |
| top 10% 文件承担 churn | >70% | -45（替代） |
| 最高 churn 文件变更行数 | >500 | -15 |

### API

`score::fragility(conn, snapshot_id) -> Result<Option<i32>>`（无 git 数据 → None）

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 无 git 数据 | 返回 None | 不报错，composite 跳过该维度 |
| churn 定义 | lines_added + lines_deleted | 标准 churn 指标 |

## 4. 迭代记录

- 2026-04-14: 初始方案
