# Scoring Engine Architecture

## 概述

评分引擎是 decay 的核心子系统，负责将原始代码数据转化为量化的健康分数。系统采用 trait-based 插件架构：Collector 收集原始数据写入 DB，Dimension 从 DataStore 拉取数据计算分数，ScoreProfile 根据项目类型加权合成综合分。所有维度采用扣分制（100 分起步向下扣）。

## 架构图

```
              ┌─────────────┐     ┌─────────────┐
              │  FileScan    │     │  GitHistory  │
              │  Collector   │     │  Collector   │
              └──────┬───────┘     └──────┬───────┘
                     │ write              │ write
                     ▼                    ▼
              ┌──────────────────────────────────┐
              │         SQLite Tables             │
              │  files | git_changes              │
              └──────────────┬───────────────────┘
                             │ read (lazy)
                             ▼
              ┌──────────────────────────────────┐
              │          DataStore                │
              │  source_files() | conn()          │
              │  dependencies()                   │
              └──────────────┬───────────────────┘
                             │ pull
            ┌────────────────┼────────────────┐
            ▼                ▼                ▼
     ┌────────────┐  ┌────────────┐  ┌────────────┐
     │ structural │  │ complexity │  │ fragility  │  ... × 8
     │ Dimension  │  │ Dimension  │  │ Dimension  │
     └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
           │               │               │
           └───────────────┼───────────────┘
                           ▼
              ┌──────────────────────────────────┐
              │  ScoreProfile::weighted_composite │
              │  ProjectType → weights → 综合分   │
              └──────────────────────────────────┘
```

## 核心抽象

### Collector trait

```rust
pub trait Collector: Send + Sync {
    fn name(&self) -> &'static str;
    fn ensure_schema(&self, conn: &Connection) -> Result<()>;
    fn available(&self, project_path: &Path) -> bool;
    fn collect(&self, conn: &Connection, snapshot_id: i64, project_path: &Path) -> Result<CollectorSummary>;
}
```

注册表模式：`all_collectors()` 返回 `Vec<Box<dyn Collector>>`。当前实现：

| Collector | 写入表 | 可用条件 |
|-----------|--------|----------|
| FileScan | `files` (path, size_bytes, depth) | 始终可用 |
| GitHistory | `git_changes` (path, change_count, lines_added, lines_deleted, last_modified) | `.git` 目录存在 |

### Dimension trait

```rust
pub trait Dimension: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, store: &DataStore) -> Result<DimensionResult>;
}

pub struct DimensionResult {
    pub name: String,
    pub score: Option<i32>,  // None = 数据源不可用
    pub issues: Vec<Issue>,
}
```

8 个维度的评估策略：

| 维度 | 数据源 | 评估内容 |
|------|--------|----------|
| structural | DB (files) | 文件数量、目录深度、顶层目录数 |
| complexity | DB (files) | 大文件比例、平均文件大小、最大文件 |
| fragility | DB (git_changes) | churn 集中度、高 churn 文件、频繁修改文件 |
| maintainability | source_files | TODO/FIXME、重复代码块、长函数 |
| observability | source_files | unwrap/panic、空 catch、硬编码配置、日志框架 |
| quality_assurance | source_files + DB | 测试文件比例、测试覆盖、测试/源码行比 |
| reliability | source_files + deps | unsafe/eval、SQL 注入、依赖数量 |
| performance | source_files | 嵌套循环、阻塞调用、clone/copy |

### ScoreProfile

```rust
pub struct ScoreProfile {
    pub project_type: ProjectType,
    pub weights: HashMap<String, f64>,
}
```

6 种 ProjectType，各有不同权重分配。检测优先级：MobileApp > Monorepo > WebService > Library > Cli > Generic。

### 扣分制模型

每个维度从 100 分起步，根据检测到的问题扣分：

```rust
let mut score: i32 = 100;
// Critical 问题扣 30-45 分
// Warning 问题扣 15-25 分
// 最终 score = score.max(0)
```

扣分与 Issue 生成同步进行，一次遍历完成。

## 数据流

1. **Collector 阶段**
   - `run_collectors()` 遍历所有 collector
   - 每个 collector 先 `ensure_schema()` 建表，再检查 `available()`
   - `FileScan` 通过 filter pipeline（DirExclusion → FileTypeFilter → LanguageFilter）过滤后写入 `files` 表
   - `GitHistory` 收集 90 天内 git 变更，经 git pipeline 过滤后写入 `git_changes` 表

2. **Evaluation 阶段**
   - 创建 `DataStore`（持有 conn + snapshot_id）
   - 遍历 `all_dimensions()`，每个调用 `evaluate(store)`
   - DB-only 维度（structural, complexity, fragility）直接查 SQL
   - 文件分析维度通过 `store.source_files()` 获取缓存的文件内容
   - 每个维度返回 `DimensionResult { score, issues }`

3. **Composite 计算**
   - `ScoreProfile::weighted_composite()` 加权求和
   - `None` 分数的维度被跳过，权重在有效维度间重新归一化
   - 公式：`sum(score_i * weight_i) / sum(weight_i)` 四舍五入到整数

4. **持久化**
   - `db::insert_dimension_scores()` 将所有维度分数写入 `dimension_scores` 表（key-value 格式）
   - `db::insert_scores()` 将核心分数写入 `scores` 表（legacy 固定列格式）

## 设计约束

- **Pull 模型** — 维度从 DataStore 拉取数据，而非 Collector 推送给维度，解耦收集与评估
- **惰性加载** — 文件内容通过 `OnceCell` 按需加载，DB-only 维度不会触发磁盘 IO
- **单次遍历** — 每个维度的 `evaluate()` 在一次函数调用中同时计算分数和生成 issues，避免重复扫描
- **可选维度** — `score: Option<i32>` 允许某些维度在数据不可用时返回 N/A（如无 git 历史时 fragility 返回 None）

## 扩展点

- **新 Collector** — 实现 trait + 添加到注册表，新 collector 可为新数据源创建新表
- **新 Dimension** — 实现 trait + 添加到注册表 + 在 ScoreProfile 各 ProjectType 中配权重
- **新 Filter Stage** — 实现 `FilterStage` trait 添加到 `run_pipeline()` 的 stages 列表
- **DataStore 扩展** — 添加新的 `OnceCell` 字段和对应 getter，所有维度自动可用
