# CLI 框架 技术方案

## 1. 背景

PRD 要求建立 CLI 命令行入口，用户通过 `decay --help` 发现用法，后续功能通过统一 CLI 框架暴露。验收标准：`--help` 输出帮助信息 + `--version` 输出版本号 + 无参数不 panic。

## 2. 方案

clap derive macro 定义 Cli struct，anyhow 处理错误。无参数时显示 help。不定义子命令，为后续里程碑预留扩展点。

### 文件结构

| Action | File | Responsibility |
|--------|------|---------------|
| modify | `Cargo.toml` | 加 clap (features=derive) + anyhow |
| modify | `src/main.rs` | Cli struct + main 入口 |

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| CLI 库 | clap derive | 声明式，最少代码量，社区标准 |
| 无参数行为 | 显示 help | 与 --help 一致，用户立即知道怎么用 |
| 错误处理 | anyhow | 便捷错误传播，适合 CLI 应用 |

## 4. 迭代记录

- 2026-04-13: 初始方案，clap derive + anyhow
