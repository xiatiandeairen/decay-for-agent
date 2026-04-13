# 项目初始化 技术方案

## 1. 背景

PRD 要求建立可编译的 Rust 项目结构和 CI，作为 v1 所有功能的基础。验收标准：`cargo build` 成功 + CI green。

## 2. 方案

Single crate Rust 项目，binary target name = `decay`，edition 2021，零外部依赖。GitHub Actions CI 在 push 和 PR to main 时触发 `cargo build` + `cargo test`。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| create | `Cargo.toml` | 项目元数据，package name = decay |
| create | `src/main.rs` | `fn main()` 占位入口 |
| create | `.github/workflows/ci.yml` | CI: checkout → Rust stable → build → test |
| create | `.gitignore` | 忽略 `target/` |

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 项目结构 | single crate | 当前只有一个 binary，workspace 过度设计 |
| CI 平台 | GitHub Actions | 项目托管在 GitHub |
| 外部依赖 | 零依赖 | clap 等属于 CLI 框架 PRD |
| Rust edition | 2024 | 新项目使用最新稳定 edition |
| CI 质量检查 | clippy + fmt + cache | coding rules 要求 lint 0 error 0 warning |

## 4. 迭代记录

- 2026-04-13: 修正方案 — edition 升级到 2024，CI 加 clippy/fmt/cache
- 2026-04-13: 初始方案，single crate + GitHub Actions CI
