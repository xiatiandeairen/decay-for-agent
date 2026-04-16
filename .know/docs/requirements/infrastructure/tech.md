# 基础设施 技术方案

## 1. 背景

PRD 要求建立 Rust 项目骨架、CLI 命令行入口和 SQLite 快照存储层，作为 v1 所有功能的基础。

### 技术约束

- Rust edition: 必须使用 2024（最新稳定版）
- CI: GitHub Actions，push 和 PR to main 触发
- CLI 库: 使用 clap derive macro，声明式定义
- 错误处理: 统一使用 anyhow
- 存储引擎: SQLite（嵌入式，免服务端）
- 存储路径: XDG data 目录，不污染项目目录

### 前置依赖

- 无（基础设施是最底层模块）

## 2. 方案

### 文件/模块结构

**项目初始化:**

| Action | File | Responsibility |
|--------|------|---------------|
| create | `Cargo.toml` | 项目元数据，package name = decay |
| create | `src/main.rs` | `fn main()` 占位入口 |
| create | `.github/workflows/ci.yml` | CI: checkout → Rust stable → build → test |
| create | `.gitignore` | 忽略 `target/` |

**CLI 框架:**

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 clap (features=derive) + anyhow |
| modify | `src/main.rs` | Cli struct + main 入口 |

**快照存储:**

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 rusqlite (bundled) + dirs |
| create | `src/db.rs` | init + create_snapshot + db_path |
| modify | `src/main.rs` | 引入 db 模块，运行时创建快照 |

### 核心流程

**项目初始化:**
1. `cargo init` → 生成 Cargo.toml + src/main.rs → 可编译项目骨架
2. 配置 CI workflow → 定义 checkout/toolchain/build/test/clippy/fmt steps → CI pipeline
3. `cargo build` + `cargo test` → 本地验证 + CI 验证 → 绿色构建

**CLI 框架:**
1. clap → 解析命令行参数 → Cli struct 实例
2. main → 匹配参数/无参数 → 显示 help 或执行对应逻辑
3. anyhow → 捕获所有 Result → 统一错误输出

**快照存储:**
1. db_path() → 定位 XDG data dir → 返回 decay/snapshots.db 路径
2. init() → 打开/创建数据库 + 执行建表 SQL → Connection 实例
3. create_snapshot() → 插入快照记录 → 返回 snapshot ID

### 数据结构

**快照表 schema:**

| 字段 | 类型 | 用途 |
|------|------|------|
| id | INTEGER PK | 快照唯一标识 |
| project_path | TEXT | 项目根路径 |
| created_at | TEXT | 创建时间 |
| version | TEXT | decay 版本号 |

## 3. 关键决策

| 决策 | 选择 | 为什么 |
|------|------|--------|
| 项目结构 | single crate | 当前只有一个 binary，workspace 增加配置复杂度且无收益 |
| CI 平台 | GitHub Actions | 项目托管在 GitHub，原生集成；备选 CircleCI 需额外配置 |
| 外部依赖 | 零依赖（初始化阶段） | clap 等属于 CLI 框架阶段，避免模块间耦合 |
| Rust edition | 2024 | 新项目使用最新稳定 edition；2021 缺少新语法特性 |
| CI 质量检查 | clippy + fmt + cache | coding rules 要求 lint 0 error 0 warning；不加 cache 则 CI 慢 |
| CLI 库 | clap derive | 声明式最少代码量，社区标准；备选 structopt 已合并入 clap |
| 无参数行为 | 显示 help | 与 --help 一致，用户立即知道怎么用；备选 panic 或空输出体验差 |
| 错误处理 | anyhow | 便捷错误传播，适合 CLI 应用；备选 thiserror 更适合库而非应用 |
| SQLite 库 | rusqlite (bundled) | 轻量同步 API，bundled 免安装系统 SQLite；备选 diesel 太重 |
| 存储路径 | XDG data dir | 集中管理，不污染项目目录；备选 .decay/ 在项目内会被 git 追踪 |
| 模块结构 | src/db.rs 单文件 | schema 简单，单文件足够；备选拆目录增加无意义复杂度 |

## 4. 迭代记录

### 2026-04-14

- 快照存储方案，rusqlite + dirs + XDG data dir

### 2026-04-13

- CLI 框架方案，clap derive + anyhow
- 修正项目初始化方案 — edition 升级到 2024，CI 加 clippy/fmt/cache
- 初始方案，single crate + GitHub Actions CI
