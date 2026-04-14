# 问题诊断 技术方案

## 1. 背景

PRD 要求从评分和采集数据中自动识别具体问题并分级。输出分级问题列表。

## 2. 方案

src/diagnose.rs 单文件，硬编码约 10 条诊断规则。每条规则查询 files/git_changes 表，匹配条件则生成 Issue。处方作为 Issue 的 prescription 字段一起输出。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| create | `src/diagnose.rs` | 诊断规则 + 处方生成 |
| modify | `src/main.rs` | 集成诊断输出 |

### 数据结构

```rust
Issue { level: Level, category: Category, message: String, prescription: Option<String> }
```

### 规则集

structural: 文件数 >1000 crit / >500 warn；深度 >5 warn；顶层 >15 info
complexity: 文件 >50KB crit / >15KB warn；大文件占比 >20% info
fragility: churn >500 crit；集中度 >50% warn；变更 >10 次 info

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 规则驱动 | 硬编码 | v1 规则少，不需要注册表 |
| 输出结构 | Vec<Issue> | 不需要跨快照查询 |
| 诊断+处方 | 同文件 | 紧密耦合，分开冗余 |

## 4. 迭代记录

- 2026-04-14: 初始方案
