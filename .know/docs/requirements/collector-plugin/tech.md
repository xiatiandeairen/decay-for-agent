# 采集层插件化 技术方案

## 1. 背景

PRD 要求将采集器（scan、git）抽象为统一 trait，通过注册表调度。现有两个采集器写入不同 DB 表（files、git_changes），各维度直接查这些表。

## 2. 方案

### 2.1 Collector trait

```rust
// src/collector/mod.rs

pub trait Collector: Send + Sync {
    /// Collector name, used for logging and error reporting.
    fn name(&self) -> &'static str;

    /// Ensure required DB tables exist.
    fn ensure_schema(&self, conn: &Connection) -> Result<()>;

    /// Whether this collector can run (e.g. git needs a repo).
    fn available(&self, project_path: &Path) -> bool;

    /// Collect data and write to DB. Returns a summary for reporting.
    fn collect(
        &self,
        conn: &Connection,
        snapshot_id: i64,
        project_path: &Path,
    ) -> Result<CollectorSummary>;
}

pub struct CollectorSummary {
    pub name: String,
    pub stats: HashMap<String, String>,  // e.g. {"files": "42", "commits": "15"}
}
```

### 2.2 注册表

```rust
pub fn all_collectors() -> Vec<Box<dyn Collector>> {
    vec![
        Box::new(file_scan::FileScan),
        Box::new(git_history::GitHistory),
    ]
}
```

### 2.3 采集器模块结构

```
src/collector/
├── mod.rs              # trait 定义 + 注册表
├── file_scan.rs        # 包装 scan.rs（调用现有 filter + scan 逻辑）
└── git_history.rs      # 包装 git.rs（调用现有 git 逻辑）
```

关键设计：**不迁移 scan.rs 和 git.rs 的内部逻辑**。Collector 实现仅作为 adapter 层，调用现有函数。这样：
- 不破坏现有单元测试
- 不增加重构风险
- scan.rs / git.rs 保持 FILE_NOT_MODIFIED（M1 的约束仍然有效）

### 2.4 run.rs 调度变更

```rust
// Before:
let scan_summary = scan::collect(&conn, snapshot_id, &project_path)?;
let git_summary = match git::collect(&conn, snapshot_id, &project_path, 90) { ... };

// After:
let collectors = collector::all_collectors();
let mut summaries: Vec<CollectorSummary> = Vec::new();
for c in &collectors {
    c.ensure_schema(&conn)?;
    if !c.available(&project_path) {
        debug!("collector {} skipped: not available", c.name());
        continue;
    }
    match c.collect(&conn, snapshot_id, &project_path) {
        Ok(summary) => summaries.push(summary),
        Err(e) => {
            debug!("collector {} failed: {e}", c.name());
            if !json && !markdown && !quiet {
                eprintln!("{} skipped: {e}", c.name());
            }
        }
    }
}
```

### 2.5 DB schema 管理

每个采集器通过 `ensure_schema()` 管理自己的表。现有表（files、git_changes）的 CREATE TABLE IF NOT EXISTS 从 db.rs 迁移到各自的 Collector 实现中。

db.rs 的 `init()` 只创建基础表（snapshots、scores、dimension_scores）。

### 2.6 ScanSummary / GitSummary 兼容

现有 Report struct（JSON 输出）包含 `scan: ScanSummary` 和 `git: Option<GitSummary>` 固定字段。重构后：
- Report 改为 `collectors: Vec<CollectorSummary>` 或保持向后兼容
- 推荐：保持 `scan` 和 `git` 字段不变，collector adapter 内部转换 CollectorSummary → ScanSummary/GitSummary
- JSON 输出格式不变

### 2.7 Dimension 与 Collector 的关系

Dimension 不直接依赖 Collector。Dimension 通过 DB 表获取数据。Collector 负责采集写入 DB，Dimension 负责读取评分。这是松耦合：

```
Collector → writes → DB tables ← reads ← Dimension
```

新增维度时，如果需要新数据源，同时新增一个 Collector 和一个 Dimension。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| create | `src/collector/mod.rs` | Collector trait + CollectorSummary + all_collectors() |
| create | `src/collector/file_scan.rs` | FileScan adapter，调用 scan::collect |
| create | `src/collector/git_history.rs` | GitHistory adapter，调用 git::collect |
| modify | `src/main.rs` | 添加 `mod collector` |
| modify | `src/run.rs` | 用 collector 注册表替代手动调用 |
| modify | `src/db.rs` | init() 移除 files/git_changes 表创建（迁移到 collector） |

不修改：`src/scan.rs`、`src/git.rs`、`src/filter.rs`、`src/dimension/*`

## 4. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 内部逻辑迁移 | 不迁移，adapter 包装 | 最小风险，现有测试不受影响 |
| DB schema 管理 | 各 collector 自建表 | 解耦，新 collector 不修改 db.rs |
| available() 检查 | trait 方法 | git 需要 repo，未来 content 分析需要特定文件类型 |
| 错误隔离 | collect 失败不阻塞其他 collector | 与现有 git 失败行为一致 |
| JSON 输出格式 | 保持不变 | 向后兼容 MCP server |

## 5. 迭代记录

- 2026-04-14: 初始方案
