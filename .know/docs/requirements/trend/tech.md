# 趋势追踪 技术方案

## 1. 背景

对比同项目的上一个快照分数，输出变化方向。

## 2. 方案

src/trend.rs + db.rs 加 get_previous_scores 查询。Trend struct 含四个 Delta。

### API

- `db::get_previous_scores(conn, project_path, current_snapshot_id) -> Result<Option<Scores>>`
- `trend::compare(current, previous) -> Trend`
- `Delta { Up(i32), Down(i32), Unchanged, NA }`

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 对比目标 | 同 project_path 上一个快照 | 自动，PRD 定义 |

## 4. 迭代记录

- 2026-04-14: 初始方案
