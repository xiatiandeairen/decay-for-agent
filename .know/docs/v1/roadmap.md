# decay-for-agent 产品路线图

## 1. 产品愿景

| | |
|---|---|
| **解决什么问题** | 项目在持续迭代中悄无声息地积累结构性债务，等到问题显现时清理成本已经很高 |
| **给谁用** | 使用 Claude Code 的开发者和 AI agent |
| **核心差异** | 不是静态 lint，而是跨快照的趋势追踪 + agent 可消费的重构处方 |

## 2. 版本规划

### v1 — Rust CLI 核心闭环

| 维度 | 标准 |
|------|------|
| **功能** | 单命令完成完整健康检查：采集 → 三维度评分 → 诊断 → 处方，有历史快照时附带趋势 |
| **质量** | 采集层 + 分析层有单元测试，核心路径有集成测试，`--debug` 支持调试 |
| **用户** | `decay` 一个命令即可使用，无需理解内部概念 |

#### 里程碑

| # | 里程碑 | 验证点 | 进度 | 需求 |
|---|--------|--------|------|------|
| M1 | **搭建脚手架** — Rust 项目结构、CI、基础 CLI 框架 | `cargo build` 通过，`decay --help` 输出帮助信息 | 2/2 | [project-init](../requirements/project-init/prd.md), [cli-framework](../requirements/cli-framework/prd.md) |
| M2 | **采集数据** — 文件结构扫描 + git 历史分析 → SQLite | 对真实项目跑 `decay`，SQLite 中有完整快照数据 | 3/3 | [snapshot-store](../requirements/snapshot-store/prd.md), [file-scan](../requirements/file-scan/prd.md), [git-analysis](../requirements/git-analysis/prd.md) |
| M3 | **计算评分** — structural / complexity / fragility 独立评分 + 加权合成 | 输出包含三个维度的 0-100 分数和 composite 分数 | 4/4 | [structural-score](../requirements/structural-score/prd.md), [complexity-score](../requirements/complexity-score/prd.md), [fragility-score](../requirements/fragility-score/prd.md), [composite-score](../requirements/composite-score/prd.md) |
| M4 | **生成处方** — 从指标识别问题，生成分优先级的重构建议 | 输出包含分级问题列表和可操作的处方 | 0/2 | [diagnosis](../requirements/diagnosis/prd.md), [prescription](../requirements/prescription/prd.md) |
| M5 | **追踪趋势** — 跨快照对比，有历史时自动附带 | 第二次运行时输出包含与上次的对比和变化方向 | 0/0 | — |
| M6 | **格式化输出** — terminal 默认 + `--json` 机器可读输出 | `decay` 输出人可读格式，`decay --json` 输出合法 JSON | 0/0 | — |
| M7 | **测试加固** — 单元测试覆盖采集层和分析层，`--debug` flag | `cargo test` 全通过，`decay --debug` 输出详细日志 | 0/0 | — |

## 3. 当前版本

### 包含

| 能力 | 解决什么问题 |
|------|-------------|
| **单命令健康检查** | 多步操作认知负担高 → 一个命令完成全流程 |
| **三维度评分** | 不知道哪个方面在恶化 → structural / complexity / fragility 独立评分 |
| **问题诊断** | 知道分低但不知道为什么 → 自动识别循环依赖、大文件、高复杂度等问题 |
| **重构处方** | 知道问题但不知道怎么修 → 分优先级的可执行重构方案 |
| **趋势对比** | 不知道改好了还是改坏了 → 有历史时自动附带变化方向 |
| **JSON 输出** | 程序无法消费人可读格式 → `--json` 为后续 MCP 集成铺路 |
| **语言无关分析** | 项目不一定是 Python/Swift → 结构和变更维度不依赖语言解析 |
| **语言增强** | Python/Swift 项目需要更精确的复杂度分析 → 语言级采集器作为增强 |

### 排除

- MCP server / skill 层
- 规则引擎（策略检查）
- 新语言采集器（TypeScript / Go）
- Markdown 报告
- PostToolUse 自动快照 hook
- 重复代码检测
- 子命令暴露（内部分层但不作为用户承诺）

## 4. 风险与依赖

| 风险 | 影响 | 缓解 |
|------|------|------|
| Rust 重写工程量大于预期 | 交付延迟 | MVP 已验证逻辑，重写聚焦实现而非探索 |
| 语言级解析靠启发式，准确率有限 | 复杂度维度可能漏报/误报 | 语言分析定位为增强，核心价值不依赖语言解析 |
| 架构过度设计拖慢 v1 | 交付延迟 | 为扩展预留接口，但 v1 只实现当前需要的 |
