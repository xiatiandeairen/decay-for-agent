# 数据采集 技术方案

## 1. 背景

PRD 要求自动扫描文件树、分析 git 历史并采集指标写入 SQLite 快照，同时将采集器抽象为统一 trait 实现插件化。

### 技术约束

- 文件扫描性能: 1000+ 文件项目 <5 秒
- 文件排除: .git/target/node_modules 不计入
- git 分析性能: 500+ commits 项目 <10 秒
- git 分析范围: 只分析当前分支，默认最近 90 天
- Collector trait: 必须 `Send + Sync`，支持并发
- adapter 模式: 不迁移 scan.rs / git.rs 内部逻辑，仅包装
- DB schema: 各 collector 通过 `ensure_schema()` 自行建表

### 前置依赖

- snapshot-store — 已完成
- dimension-trait（M1） — 已完成（collector-plugin 依赖）

## 2. 方案

### 文件/模块结构

**文件结构扫描:**

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 walkdir |
| create | `src/scan.rs` | collect 函数 + ScanSummary |
| modify | `src/db.rs` | 新增 files 表建表语句 |

**git 历史分析:**

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 git2 |
| create | `src/git.rs` | collect 函数 + GitSummary |
| modify | `src/db.rs` | 新增 git_changes 表建表语句 |

**采集层插件化:**

```
src/collector/
├── mod.rs              # trait 定义 + CollectorSummary + all_collectors()
├── file_scan.rs        # FileScan adapter，调用 scan::collect
└── git_history.rs      # GitHistory adapter，调用 git::collect
```

| 文件 | 职责 |
|------|------|
| `src/collector/mod.rs` | Collector trait + CollectorSummary + all_collectors() |
| `src/collector/file_scan.rs` | FileScan adapter，调用现有 scan 逻辑 |
| `src/collector/git_history.rs` | GitHistory adapter，调用现有 git 逻辑 |
| `src/run.rs` | 用 collector 注册表替代手动调用 |
| `src/db.rs` | init() 移除 files/git_changes 表创建 |

不修改：`src/scan.rs`、`src/git.rs`、`src/filter.rs`、`src/dimension/*`

### 核心流程

**文件结构扫描:**
1. walkdir → 递归遍历项目文件树 → 过滤后的文件条目流
2. collect() → 聚合路径/大小/深度 → 批量写入 files 表
3. ScanSummary → 统计 file_count/dir_count/max_depth → 返回扫描摘要

**git 历史分析:**
1. git2 → 打开项目 .git 仓库 → Repository 实例
2. revwalk → 遍历最近 90 天 commits → 每个 commit 的 diff
3. collect() → 按文件聚合 change_count/lines_added/lines_deleted → 写入 git_changes 表

**采集层插件化:**
1. `run.rs` 调用 `all_collectors()` 获取注册的采集器列表
2. 遍历采集器，先 `ensure_schema()` 建表 → 再 `available()` 检查是否可运行
3. 调用 `collect()` 采集数据写入 DB → 产出 `CollectorSummary`；失败时 log 错误，不阻塞其他采集器

### 数据结构

**files 表:**

| 字段 | 类型 | 用途 |
|------|------|------|
| snapshot_id | INTEGER FK | 关联快照 |
| path | TEXT | 文件相对路径 |
| size_bytes | INTEGER | 文件大小 |
| depth | INTEGER | 目录嵌套深度 |

**git_changes 表:**

| 字段 | 类型 | 用途 |
|------|------|------|
| snapshot_id | INTEGER FK | 关联快照 |
| path | TEXT | 文件路径 |
| change_count | INTEGER | 变更次数 |
| lines_added | INTEGER | 新增行数 |
| lines_deleted | INTEGER | 删除行数 |
| last_modified | TEXT | 最后修改时间 |

**CollectorSummary:**

| 字段 | 类型 | 用途 |
|------|------|------|
| name | String | 采集器名称 |
| stats | HashMap\<String, String\> | 统计信息，如 {"files": "42", "commits": "15"} |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 遍历库 | walkdir | 成熟库，支持过滤和深度控制；备选 std::fs 需手写递归 |
| 排除策略 | 硬编码排除列表 | PRD 排除自定义规则；备选 .gitignore 解析增加复杂度 |
| 存储粒度 | 每文件一行 | 支持后续按文件维度评分；备选聚合存储丢失文件级细节 |
| git 库 | git2 | 纯 Rust，不依赖系统 git；备选 shell 调用 git CLI 需处理解析和跨平台 |
| 时间范围 | 最近 90 天 | 平衡覆盖范围和性能；备选全量分析在大仓库性能差 |
| 分析粒度 | 每文件一行（聚合） | 支持后续按文件维度评分；备选 per-commit 存储数据量过大 |
| 内部逻辑迁移 | 不迁移，adapter 包装 | 最小风险，现有测试不受影响；备选"完整迁移"风险高且无额外收益 |
| DB schema 管理 | 各 collector 自建表 | 解耦，新 collector 不修改 db.rs；备选"集中建表"违反插件化原则 |
| available() 检查 | trait 方法 | git 需要 repo，未来 content 分析需要特定文件类型；备选"外部配置"过度设计 |
| 错误隔离 | collect 失败不阻塞其他 | 与现有 git 失败行为一致；备选"任一失败全部终止"体验差 |
| JSON 输出格式 | 保持不变 | 向后兼容 MCP server；备选"统一为 collectors 数组"破坏兼容性 |

## 4. 迭代记录

### 2026-04-14

- 采集层插件化方案
- git 历史分析方案，git2 + git_changes 表
- 文件结构扫描方案，walkdir + files 表
