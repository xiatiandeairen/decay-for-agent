## Know

### 文档索引

- [产品路线图](.know/docs/roadmap.md) | v8 当前版本
- [项目能力总览](.know/docs/capabilities.md) | 2026-04-16

#### Architecture

| 文档 | 内容 |
|------|------|
| [系统架构](.know/docs/arch/system.md) | 六层流水线、数据流、核心抽象 |
| [评分引擎](.know/docs/arch/scoring.md) | Dimension/Collector trait、ScoreProfile、扣分制模型 |
| [趋势分析](.know/docs/arch/trend.md) | 时间序列、velocity/regression/forecast/correlation |

#### Decision

| 文档 | 决策点 |
|------|--------|
| [存储选型](.know/docs/decision/storage.md) | SQLite vs JSONL vs 文件 |
| [检测策略](.know/docs/decision/detection.md) | grep 模式 vs AST vs 混合 |
| [评分模型](.know/docs/decision/scoring-model.md) | deduction-based vs additive vs percentile |

#### Requirements（8 对 prd+tech，按主题合并）

| 主题 | 包含内容 | 版本 |
|------|---------|------|
| [基础设施](.know/docs/requirements/infrastructure/) | 项目初始化、CLI 框架、快照存储 | v1 |
| [数据采集](.know/docs/requirements/data-collection/) | 文件扫描、git 分析、采集器插件 | v1-v3 |
| [评分体系](.know/docs/requirements/scoring-system/) | 3 基础维度、composite、Dimension trait、ScoreProfile | v1-v3 |
| [扩展维度](.know/docs/requirements/extended-dimensions/) | maintainability、observability、quality、reliability、performance | v3 |
| [诊断处方](.know/docs/requirements/diagnosis-actions/) | 诊断引擎、处方生成、Action schema、排序、位置精度 | v1-v4 |
| [趋势分析](.know/docs/requirements/trend-analysis/) | 趋势对比、时间序列、velocity、回归、预警、相关性、轨迹 | v1-v5 |
| [输出集成](.know/docs/requirements/output-integration/) | JSON/Markdown/Quiet 输出、MCP server、Skill、Agent protocol | v2-v4 |
| [质量提升](.know/docs/requirements/quality-improvement/) | 测试调试、检测精度、问题分类 | v2-v6 |
