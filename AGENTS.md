## 项目介绍

`decay` 是一个 Rust 函数级复杂度退化检测 CLI，面向 AI 协作编程场景。

当前核心问题是：相对某个 baseline，本次改动是否让某个函数在局部结构上明显变糟。v0.1.0 是功能 PoC，产品价值尚未被真实 dogfood 证明；`diff` 是核心价值路径，`doctor` 只是辅助体检。

## 目录结构

- `src/cli/`: CLI 入口和 `doctor` / `baseline` / `diff` 命令实现。
- `src/metric/`: active metric registry 和各 metric 计算器。
- `src/parser.rs`: Rust 函数提取和函数上下文解析。
- `src/pipeline.rs`: walk、parse、metric、fingerprint 的扫描编排。
- `src/store.rs`: SQLite baseline 持久化。
- `src/diff.rs`: baseline/current 函数级退化分类。
- `tests/`: 单元测试和 E2E integration 测试。
- `docs/`: 按 know write 规范维护的项目文档。

## 安装使用命令

- 安装当前工作区版本：`cargo install --path .`
- 本地开发测试：`cargo test`
- 静态检查：`cargo clippy -- -D warnings`
- 格式检查：`cargo fmt -- --check`
- 查看简洁命令：`decay`
- 查看详细帮助：`decay --help`
- 当前风险体检：`decay doctor`
- 保存 baseline：`decay baseline <version>`
- 当前工作区相对 baseline 的退化裁决：`decay diff <version>`
- 两个 baseline 之间的退化裁决：`decay diff <from> <to>`

本机已启用 local-only `pre-push` hook：`.git/hooks/pre-push` 会在 push 前执行 `cargo install --path .`。不要把 hook 设计同步到远端。

## 项目约束

- when: entering this repository or updating project docs
  must: treat v0.1.0 as a functional PoC whose product value is still unproven — tests passing does not mean the commit-decision hypothesis is validated
  how: use docs/roadmap.md and docs/ops.md as the source of truth; record dogfood cases as fixed / ignored / noise before claiming product success
  until: dogfood records show at least one real fixed case and the roadmap updates v0.1.0 status

- when: adding, removing, or renaming complexity metrics
  must: update the central metric registry first and keep README, PRD, architecture, diff, doctor, store, and tests aligned
  how: active metrics live in src/metric/mod.rs; product commitments are documented in README.md, README.zh.md, docs/arch/decay.md, and docs/requirements/function-complexity-detection/prd.md
  until: metric storage moves to a key/value schema with generated docs

- when: using know in this project
  prefer: use /know learn for reusable rules and /know write for structured docs; reject low-entropy notes instead of growing CLAUDE.md
  how: project docs are intentionally limited to roadmap, PRD, architecture, ops, decision, and milestone docs in know-standard paths
  until: know write path rules change

## 参考文档

- roadmap: docs/roadmap.md
- prd/function-complexity-detection: docs/requirements/function-complexity-detection/prd.md
- arch/decay: docs/arch/decay.md
- ops: docs/ops.md
- decision/v0.1.0-closeout: docs/decision/v0.1.0-closeout.md
- milestone/m1: docs/milestones/m1.md
- milestone/m2: docs/milestones/m2.md

读取原则：加载能回答当前问题的最小文档。改产品方向先读 roadmap；改命令语义、scan scope、active metrics、diff reporting 先读 PRD；改模块边界先读 arch；记录 dogfood 证据先读 ops；追溯 v0.1.0 收尾判断先读 decision；核对里程碑交付状态先读 milestone。
