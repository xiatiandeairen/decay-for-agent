# decay 能力总览

## 产品定位

项目健康监控工具，面向 AI agent 和 Claude Code 用户。核心差异：不是静态 lint，而是跨快照的趋势追踪 + agent 可消费的重构处方。

## 能力栈


| 层   | 能力                                                                                                                            | 关键模块                                                 |
| --- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| 采集  | 8 维度扫描（structural / complexity / fragility / maintainability / observability / quality_assurance / reliability / performance） | `src/dimension/`, `src/collector/`                   |
| 评估  | 自适应打分 + 6 种项目类型 profile 校准 + filter pipeline 降噪                                                                               | `src/profile.rs`, `src/filter_pipeline.rs`           |
| 诊断  | 8 类问题分类(A~H) + 模式聚合 + 根因归类                                                                                                    | `src/classify.rs`, `src/aggregate.rs`                |
| 处方  | 结构化 Action + 精确位置 + 机械 patch 生成 + 预防配置                                                                                        | `src/action.rs`, `src/patch.rs`, `src/prevention.rs` |
| 趋势  | velocity / 回归检测 / 预警 / 相关性 / 轨迹分析                                                                                             | `src/trend/`                                         |
| 影响  | git 共变耦合度 + review 负担 + 变更风险量化                                                                                                | `src/impact.rs`                                      |
| 计划  | 分阶段改善路线图（Quick Wins → Pattern Fix → Structural）                                                                               | `src/plan.rs`                                        |
| 输出  | 叙事化报告 + 自动对比 + MCP 摘要优先                                                                                                       | `src/report.rs`, `src/summary.rs`, `src/compare.rs`  |


## 接入方式


| 方式    | 入口           | 适用场景              |
| ----- | ------------ | ----------------- |
| CLI   | `decay [scan | compare           |
| MCP   | nerve-mcp 集成 | AI agent 调用       |
| Skill | `/decay`     | Claude Code 会话内使用 |


## 解决的用户问题

1. **不知道项目哪里在变差** → 跨快照趋势追踪，回归自动检测并归因到 commit
2. **issue 太多不知道先修哪个** → 影响度量化，按开发影响排序
3. **修了不知道有没有用** → `--compare last` 自动对比前后分数变化
4. **只知道问题不知道怎么修** → 结构化 action + 机械 patch 可直接 apply
5. **每次都是散点建议** → 分阶段改善计划，Phase 1 控制在 2 小时内
6. **AI agent 拿到数据不知道怎么用** → MCP 摘要优先 + skill 一键入口

## 技术规格

- 语言：Rust (edition 2024)
- 存储：SQLite (rusqlite bundled)
- 代码规模：~10k 行，30 个源文件，173 个测试
- 核心依赖：clap 4, git2 0.20, serde, ignore

