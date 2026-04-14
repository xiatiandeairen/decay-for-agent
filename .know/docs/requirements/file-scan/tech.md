# 文件结构扫描 技术方案

## 1. 背景

PRD 要求自动扫描文件树并采集结构指标，写入 SQLite 快照。验收标准：SQLite 中有文件数、目录深度、文件大小数据，1000+ 文件项目 <5 秒。

## 2. 方案

walkdir 遍历项目文件树，排除 .git/target/node_modules，将每个文件的路径、大小、深度写入 files 表。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 walkdir |
| create | `src/scan.rs` | collect 函数 + ScanSummary |
| modify | `src/db.rs` | 新增 files 表建表语句 |

### Schema

```sql
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    depth INTEGER NOT NULL
);
```

### API

- `scan::collect(conn, snapshot_id, project_path) -> Result<ScanSummary>`
- `ScanSummary { file_count: usize, dir_count: usize, max_depth: usize }`
- 排除目录：`.git`, `target`, `node_modules`

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 遍历库 | walkdir | 成熟库，支持过滤和深度控制 |
| 排除策略 | 硬编码排除列表 | PRD 排除自定义规则 |
| 存储粒度 | 每文件一行 | 支持后续按文件维度评分 |

## 4. 迭代记录

- 2026-04-14: 初始方案，walkdir + files 表
