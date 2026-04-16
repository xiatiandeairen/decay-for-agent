# Storage: SQLite 选型

## 背景

decay 需要持久化每次运行的快照数据（文件列表、git 变更、维度分数），以支持趋势分析和快照对比。需要一个零运维、单文件、支持关系查询的存储方案。

## 选项

| 方案 | 优势 | 劣势 |
|------|------|------|
| SQLite | 零配置单文件、SQL 查询能力强、Rust 生态成熟（rusqlite）、跨平台 | 不支持并发写（CLI 工具不需要）、schema 迁移需手动 |
| JSONL（追加式日志） | 实现最简单、人类可读、无依赖 | 查询需全量扫描、关系查询困难、文件增长无限 |
| 纯文件目录 | 每快照一文件、直观 | 关系查询不可能、趋势分析需加载所有文件、文件数量爆炸 |

## 决策

选择 SQLite。CLI 工具单进程单用户的场景完美契合 SQLite 的设计。SQL 查询能力对于 collector 写入和 dimension 读取的模式尤为关键——结构化评分需要聚合查询（COUNT, AVG, MAX, SUM, ORDER BY ... LIMIT）和关联查询，这些在 JSONL 或纯文件方案中需要手写大量逻辑。

## Schema 设计

```sql
-- 快照元数据
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    version TEXT NOT NULL
);

-- FileScan collector 写入
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    depth INTEGER NOT NULL
);

-- GitHistory collector 写入
CREATE TABLE git_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    path TEXT NOT NULL,
    change_count INTEGER NOT NULL,
    lines_added INTEGER NOT NULL,
    lines_deleted INTEGER NOT NULL,
    last_modified TEXT NOT NULL
);

-- 维度分数（key-value 格式，可扩展）
CREATE TABLE dimension_scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    dimension TEXT NOT NULL,
    score INTEGER  -- NULL = 维度不可用
);

-- Legacy 固定列格式（向后兼容）
CREATE TABLE scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id),
    structural INTEGER NOT NULL,
    complexity INTEGER NOT NULL,
    fragility INTEGER,
    composite INTEGER NOT NULL
);
```

设计要点：
- **snapshot_id 关联** — 所有数据表通过 snapshot_id 关联到快照，一个快照 = 一次完整运行
- **collector 自管 schema** — 每个 collector 在 `ensure_schema()` 中用 `CREATE TABLE IF NOT EXISTS` 建表，新 collector 不需要改核心 schema
- **key-value 维度分数** — `dimension_scores` 用 (dimension, score) key-value 格式，新增维度无需改表结构
- **XDG 路径** — 数据库文件存储在 `~/.local/share/decay/snapshots.db`（Linux）或 `~/Library/Application Support/decay/snapshots.db`（macOS），跨项目共享

## 后果

- 趋势分析可直接用 SQL 查询历史数据（ORDER BY id DESC LIMIT N）
- 新 collector 只需添加新表，不影响现有 schema
- `get_previous_dimension_scores()` 同时支持新旧两种分数表格式的回退查询
- 单文件存储意味着所有项目的快照在同一个 DB 中，通过 project_path 区分
- 没有正式的 migration 机制，依赖 `CREATE TABLE IF NOT EXISTS` 做渐进式 schema 演进

## 状态

已确认 — 项目初始
