# decay 能力全景

<!-- 数据置信: 实测标来源 > 估算标依据 > 目标标"待验证" > 无数据标原因。禁止编造。 -->
<!-- 结构锁定: 章节顺序与字段结构不可变。只能在现有框架内填充内容。 -->

<!-- 核心问题: 产品现在能做什么？
     定位: 跨版本能力清单快照
     不属于本文档: 版本规划（→ roadmap）、单个需求细节（→ prd）、技术方案（→ tech） -->

## 1. 能力清单

| 能力 | 描述 | 状态 | 版本 |
|------|------|------|------|
| **8 维度健康评分** | 一条命令扫描项目，输出 structural/complexity/fragility/maintainability/observability/quality_assurance/reliability/performance 评分 | 可用 | v3 |
| **自适应项目类型识别** | 自动识别项目类型（CLI/WebService/Library/MobileApp/Monorepo/Generic），按类型差异化评分权重 | 可用 | v3 |
| **问题诊断 + 8 类分类** | 检测 55+ 种 issue，按严重度（critical/warning/info）和分类（A-H: MechanicalFix 到 Prevention）排序 | 可用 | v6 |
| **结构化处方** | 每个 issue 生成 Action（7 种类型 × 4 级优先级 × 3 级 effort），精确到文件/行/符号 | 可用 | v4 |
| **开发影响评估** | 每个 issue 附带影响量化（耦合文件数/review 负担/变更风险等级） | 可用 | v8 |
| **跨快照趋势追踪** | 时间序列分析：衰退速度、回归检测、阈值预警、维度相关性 | 可用 | v5 |
| **处方效果对比** | `--compare last` 自动对比上一快照，展示处方执行前后的分数变化 | 可用 | v8 |
| **改善计划** | 基于优先级和影响度生成 3 阶段改善路线图（Quick Wins → Pattern Fix → Structural） | 可用 | v8 |
| **叙事化报告** | 报告首段为自然语言摘要，替代纯指标表格 | 可用 | v8 |
| **多格式输出** | terminal 默认视图、--json 机器可读、--markdown 文档嵌入、--quiet 脚本集成 | 可用 | v2 |
| **MCP server 集成** | AI agent 通过 MCP 协议调用 decay_check tool，获取摘要 + 完整 JSON | 可用 | v2 |
| **Claude Code skill** | `/decay` 会话内一键健康检查，输出 summary + top actions + 改善趋势 | 可用 | v2 |

## 2. 覆盖范围

### 已知限制

- 检测规则以 Rust 项目为主，其他语言覆盖有限（非 Rust 项目仍可获得 structural/complexity/fragility 评分，但 observability/reliability 等基于 grep 模式的维度精度下降）
- 单项目范围，不支持跨项目聚合（组织级健康视图需要独立服务，当前架构为单进程 CLI）
- 影响评估基于启发式估算（"节省 X 分钟"为保守估算非精确测量，标注 estimated）
- git 分析窗口固定为 90 天（超出窗口的 churn 数据不纳入 fragility 评分）
- 线性趋势预测在非线性衰退模式下偏差较大（R²<0.7 时不生成预警，但用户可能误读 velocity 方向）

### 未覆盖场景

- 多项目健康聚合（当前架构为单进程 CLI，无聚合服务 → v9 方向）
- 自动执行修复（生成处方和计划，执行仍由用户/agent 控制 → v9 方向）
- 自定义检测规则（内置规则 + 影响评估已够用，自定义引擎复杂度高 → v9 方向）
- GUI / Dashboard（当前通过 CLI/MCP/skill 三种方式接入，无 Web UI）
- 运行时性能分析（仅静态代码分析，不采集运行时指标如内存/CPU）
