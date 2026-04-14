# decay-for-agent 产品路线图

## 1. 产品愿景

| | |
|---|---|
| **解决什么问题** | 项目在持续迭代中悄无声息地积累结构性债务，等到问题显现时清理成本已经很高 |
| **给谁用** | 使用 Claude Code 的开发者和 AI agent |
| **核心差异** | 不是静态 lint，而是跨快照的趋势追踪 + agent 可消费的重构处方 |

## 2. 版本规划

### v1 — Rust CLI 核心闭环 ✅

| 维度 | 标准 |
|------|------|
| **功能** | 单命令完成完整健康检查：采集 → 三维度评分 → 诊断 → 处方，有历史快照时附带趋势 |
| **质量** | 采集层 + 分析层有单元测试，核心路径有集成测试，`--debug` 支持调试 |
| **用户** | `decay` 一个命令即可使用，无需理解内部概念 |

### v2 — Claude Code 集成层

| 维度 | 标准 |
|------|------|
| **功能** | agent 通过 MCP 自动调用 decay，用户通过 `/decay` 一键触发，sprint 完成后自动采集快照 |
| **质量** | MCP server 有协议测试，skill 有端到端验证 |
| **用户** | 安装 plugin 后 `/decay` 即可使用，无需手动运行 CLI |

#### 里程碑

| # | 里程碑 | 验证点 | 进度 | 需求 |
|---|--------|--------|------|------|
| M1 | **搭建 MCP server** — 独立进程包装 CLI `--json` 为 MCP tool | Claude Code agent 通过 MCP 调用 decay 并获取 JSON 结果 | 0/1 | [mcp-server](../requirements/mcp-server/prd.md) |
| M2 | **创建 skill 入口** — `/decay` slash command + SKILL.md | 用户在 Claude Code 中 `/decay` 触发健康检查并看到格式化结果 | 0/0 | — |
| M3 | **添加 Markdown 输出** — `--markdown` flag 生成人可读报告 | `decay --markdown` 输出包含评分、趋势、问题、处方的格式化报告 | 0/0 | — |
| M4 | **集成 sprint** — sprint insight 阶段自动调用 decay | sprint 结束时自动产生健康快照，趋势数据跟着开发节奏累积 | 0/0 | — |

## 3. 当前版本

### 包含

| 能力 | 解决什么问题 |
|------|-------------|
| **MCP server** | agent 无法程序化调用 decay → MCP 协议暴露 tool，解锁自动化场景 |
| **skill 入口** | 用户需要记命令和切换终端 → `/decay` 零学习成本触发 |
| **Markdown 报告** | JSON 对人不友好 → 生成可读报告供 skill 输出 |
| **sprint 集成** | 手动跑 decay 容易忘 → sprint 结束自动快照，趋势自然累积 |

### 排除

- 规则引擎（可配置策略检查 — 需求未验证）
- 新语言采集器（TypeScript / Go — 核心价值不依赖语言解析）
- PostToolUse hook（逐次文件修改快照 ROI 低，sprint 集成替代）
- 重复代码检测（实现复杂，与当前维度正交）

## 4. 风险与依赖

| 风险 | 影响 | 缓解 |
|------|------|------|
| MCP 协议变更 | server 需要适配新版本 | 用官方 SDK，关注 MCP spec 更新 |
| skill 与 MCP 职责边界不清 | 功能重复或用户困惑 | skill 是用户入口，MCP 是 agent 入口，明确分工 |
| sprint 集成需要 sprint skill 配合 | M4 依赖外部 skill 接口 | 先确认 sprint skill 的 hook 机制 |
