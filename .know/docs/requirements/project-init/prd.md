# 项目初始化

## 1. 问题

decay-for-agent v1 需要从 Python MVP 迁移到 Rust 实现。当前没有 Rust 项目结构，无法开始任何功能开发。所有后续里程碑（数据采集、评分、处方）都依赖一个可编译的 Rust 项目和可运行的 CI。这是 v1 的前置依赖，阻塞所有开发工作。

## 2. 目标用户

使用 Claude Code 的 AI agent，需要在项目上运行 `decay` 命令获取健康报告。当前无 Rust 项目存在，agent 无法执行任何 decay 操作。

## 3. 核心假设

**建立标准 Rust 项目结构 + CI → 后续功能开发可以直接在此基础上迭代，无需重新处理构建和集成问题。**

验证方式：`cargo build` 成功且 CI pipeline green。

## 4. 方案

- **Before**: 无 Rust 项目，无法开始 v1 开发 → **After**: `cargo build` 通过，CI 自动验证每次提交

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| 项目初始化 tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- 运行 `cargo build` → 编译成功，无 error
- 推送代码到仓库 → CI pipeline 自动触发并 green
- 项目结构遵循 Rust 社区惯例（src/main.rs 或 src/lib.rs 入口）

## 6. 排除项

- 不包含 CLI 参数解析和 --help 输出（→ CLI 框架 PRD）
- 不包含任何业务逻辑实现
- 不包含发布流程（cargo publish）
