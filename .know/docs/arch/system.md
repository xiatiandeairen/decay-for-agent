# System Architecture

## 概述

decay 是一个面向 AI agent 的项目健康监控 CLI 工具。系统采用六层流水线架构：Collection -> Evaluation -> Diagnosis -> Prescription -> Trend -> Output，每层职责清晰、单向依赖。核心设计目标是让 AI agent 能快速理解一个代码库的健康状况并获得可执行的改进建议。

## 架构图

```
CLI (clap)
  │
  ▼
┌─────────────────────────────────────────────────────────────┐
│  run::run()  — 主编排函数                                     │
│                                                              │
│  1. Collection    Collector trait → FileScan, GitHistory      │
│     │             写入 DB: files, git_changes                 │
│     ▼                                                        │
│  2. Evaluation    Dimension trait × 8 → DimensionResult       │
│     │             DataStore (lazy-load) 提供数据              │
│     ▼                                                        │
│  3. Diagnosis     classify_issues() → IssueCategory (A-H)    │
│     │             impact::compute_impact() → Impact           │
│     ▼                                                        │
│  4. Prescription  Action (type/target/priority/effort)        │
│     │             aggregate, patch, prevention, plan          │
│     ▼                                                        │
│  5. Trend         Trajectory: velocity, regression,           │
│     │             forecast, correlation                       │
│     ▼                                                        │
│  6. Output        Terminal / JSON / Markdown / Quiet          │
└─────────────────────────────────────────────────────────────┘
     │
     ▼
  SQLite (snapshots.db)  — 持久化历史数据
```

## 核心抽象

### Collector trait (`collector/mod.rs`)

```rust
pub trait Collector: Send + Sync {
    fn name(&self) -> &'static str;
    fn ensure_schema(&self, conn: &Connection) -> Result<()>;
    fn available(&self, project_path: &Path) -> bool;
    fn collect(&self, conn: &Connection, snapshot_id: i64, project_path: &Path) -> Result<CollectorSummary>;
}
```

两个实现：`FileScan`（文件扫描，写 `files` 表）、`GitHistory`（git 历史，写 `git_changes` 表）。每个 collector 自管 schema，可独立跳过。

### Dimension trait (`dimension/mod.rs`)

```rust
pub trait Dimension: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult>;
}
```

8 个维度实现：structural, complexity, fragility, maintainability, observability, quality_assurance, reliability, performance。每个维度从 DataStore 拉取数据，一次 evaluate 同时产出分数和 issues。

### DataStore (`data_store.rs`)

惰性加载的数据缓存层。通过 `OnceCell` 实现按需加载、全局去重：
- `source_files()` — 从 DB 查路径，从磁盘读内容，缓存
- `dependencies()` — 解析 Cargo.toml / package.json，缓存
- `conn()` — 直接访问 SQLite 连接

### Report (`run.rs`)

最终输出的聚合结构体，包含所有分析结果：scores, issues, actions, trend data, time_series 等。通过 `serde::Serialize` 支持 JSON 输出。

## 数据流

1. **CLI 解析** — `main.rs` 解析 clap 参数（`--json`, `--markdown`, `--quiet`, `--compare`, `--debug`）
2. **初始化** — `db::init()` 打开/创建 SQLite，`db::create_snapshot()` 创建新快照
3. **数据收集** — 遍历 `all_collectors()`，每个 collector 写入对应 DB 表
4. **项目类型检测** — `profile::detect()` 根据文件系统信号识别 ProjectType（Cli/WebService/Library/MobileApp/Monorepo/Generic）
5. **DataStore 创建** — 持有 DB 连接和 snapshot_id，供维度按需拉取数据
6. **维度评估** — 遍历 `all_dimensions()`，每个维度调用 `evaluate(store)` 产出分数 + issues
7. **问题分类** — `classify::classify_issues()` 为每个 issue 分配 A-H 类别
8. **影响评估** — `impact::compute_impact()` 基于 coupling map 和行数计算开发影响
9. **处方生成** — `action::collect_sorted()` 收集排序去重，`aggregate`, `patch`, `prevention`, `plan` 生成高层建议
10. **趋势分析** — 从历史快照构建 Trajectory（velocity/regression/forecast/correlation）
11. **输出渲染** — 根据 CLI 参数选择 terminal/JSON/markdown/quiet 格式

## 设计约束

- **语言无关** — 不依赖任何特定语言的 AST parser，通过 grep 模式匹配实现跨语言检测
- **零配置** — 自动检测项目类型和主要语言，无需配置文件即可运行
- **增量历史** — 每次运行创建新 snapshot，不修改历史数据，支持趋势分析
- **惰性求值** — DataStore 按需加载数据，未使用的数据源零成本
- **单进程** — CLI 工具，不启动 server，一次运行完成全部分析

## 扩展点

- **新 Collector** — 实现 `Collector` trait，添加到 `all_collectors()` 注册表
- **新 Dimension** — 实现 `Dimension` trait，添加到 `all_dimensions()` 注册表，在 `profile.rs` 各 ProjectType 中加权重
- **新 ProjectType** — 在 `profile.rs` 中添加 enum variant、检测逻辑和权重配置
- **新输出格式** — 在 `run::output()` 中添加新分支
- **新 DataStore 数据源** — 添加 `OnceCell` 字段和 getter 方法
