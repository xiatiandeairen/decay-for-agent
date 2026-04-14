# git 历史分析 技术方案

## 1. 背景

PRD 要求自动分析 git 历史并采集变更指标，写入 SQLite 快照。验收标准：SQLite 中有变更频率、热点文件、churn 数据，500+ commits 项目 <10 秒。

## 2. 方案

git2 crate 读取项目 git 仓库，遍历最近 90 天的 commits，统计每个文件的变更次数和行数变化，写入 git_changes 表。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 git2 |
| create | `src/git.rs` | collect 函数 + GitSummary |
| modify | `src/db.rs` | 新增 git_changes 表建表语句 |

### Schema

```sql
CREATE TABLE IF NOT EXISTS git_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    path TEXT NOT NULL,
    change_count INTEGER NOT NULL,
    lines_added INTEGER NOT NULL,
    lines_deleted INTEGER NOT NULL,
    last_modified TEXT NOT NULL
);
```

### API

- `git::collect(conn, snapshot_id, project_path, days: u32) -> Result<GitSummary>`
- `GitSummary { files_analyzed: usize, total_commits: usize }`
- 默认分析最近 90 天，只看当前分支

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| git 库 | git2 | 纯 Rust，不依赖系统 git |
| 时间范围 | 最近 90 天 | 平衡覆盖范围和性能 |
| 分析粒度 | 每文件一行（聚合） | 支持后续按文件维度评分 |

## 4. 迭代记录

- 2026-04-14: 初始方案，git2 + git_changes 表
