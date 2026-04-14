# 快照存储 技术方案

## 1. 背景

PRD 要求建立 SQLite 快照存储层，作为文件扫描和 git 分析的统一写入目标。验收标准：运行 decay 后数据库自动创建，包含快照表和正确 schema。

## 2. 方案

rusqlite (bundled feature) 提供 SQLite 访问，dirs crate 定位 XDG data 目录。src/db.rs 单文件模块封装所有数据库操作。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 rusqlite (bundled) + dirs |
| create | `src/db.rs` | init + create_snapshot + db_path |
| modify | `src/main.rs` | 引入 db 模块，运行时创建快照 |

### Schema

```sql
CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    version TEXT NOT NULL
);
```

### API

- `db_path() -> Result<PathBuf>` — XDG data dir + decay/snapshots.db
- `init() -> Result<Connection>` — 打开/创建数据库 + 建表
- `create_snapshot(conn, project_path) -> Result<i64>` — 插入快照，返回 ID

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| SQLite 库 | rusqlite (bundled) | 轻量同步 API，适合 CLI，bundled 免安装系统 SQLite |
| 存储路径 | XDG data dir | 集中管理，不污染项目目录 |
| 模块结构 | src/db.rs | schema 简单，单文件足够 |

## 4. 迭代记录

- 2026-04-14: 初始方案，rusqlite + dirs + XDG data dir
