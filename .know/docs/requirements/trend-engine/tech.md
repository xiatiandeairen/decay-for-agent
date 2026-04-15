# 趋势引擎 技术方案

## 1. 背景

PRD 要求新增时间序列查询 API，为 v5 M2-M6 提供数据基础。

## 2. 方案

### 2.1 SnapshotScores 类型

```rust
pub struct SnapshotScores {
    pub snapshot_id: i64,
    pub created_at: String,
    pub scores: HashMap<String, Option<i32>>,
}
```

### 2.2 get_dimension_time_series (db.rs)

```sql
-- 查询最近 N 个快照
SELECT id, created_at FROM snapshots WHERE project_path = ?1 ORDER BY id DESC LIMIT ?2
-- 每个快照查维度分数
SELECT dimension, score FROM dimension_scores WHERE snapshot_id = ?1
```

结果按 snapshot_id 升序返回（oldest → newest）。

### 2.3 dimension_series (trend.rs)

从 SnapshotScores 序列提取单维度的 (snapshot_id, score) 对，跳过 None。

### 2.4 Report 集成

```rust
pub struct Report {
    // ...existing...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub time_series: Vec<SnapshotScores>,
}
```

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/db.rs` | SnapshotScores + get_dimension_time_series() + 3 tests |
| modify | `src/trend.rs` | dimension_series() + 1 test |
| modify | `src/run.rs` | Report.time_series + 查询集成 |

## 4. 迭代记录

- 2026-04-15: 初始方案
