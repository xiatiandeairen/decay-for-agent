# 评分体系

## 1. 问题

### 痛点

采集到的原始数据（文件结构、git 变更）无法直接回答"项目健康吗"。需要评分体系将数据转化为可操作的洞察，同时评分架构需要支持灵活扩展和按项目类型差异化。

- **原始数据不可操作**: 文件数、目录深度、churn 等指标无法直接判断好坏
- **维度硬编码**: 3 个维度的评分和诊断逻辑分散在 score.rs / diagnose.rs 中，新增维度需改 4+ 文件
- **一刀切评分**: 所有项目使用相同阈值和权重，CLI 工具和 Web 服务标准完全不同

### 影响范围

影响所有使用 decay 的项目。评分是从数据到洞察的关键一步，维度扩展和差异化评分是 v3 的核心目标。

### 为什么现在做

评分是让用户从原始数据获得可操作洞察的关键步骤。维度抽象是 v3 扩展 5 个新维度的前置条件。分层打分在 8 维度下比 3 维度更重要。

## 2. 目标用户

| 角色 | 场景 | Before | After |
|------|------|--------|-------|
| AI agent | 运行 `decay` 评估项目结构 | 有文件结构数据但不知道好坏 | 输出 structural 0-100 分，分数越高越健康 |
| AI agent | 运行 `decay` 评估项目复杂度 | 知道哪些文件大但不知道整体复杂度水平 | 获得 complexity 0-100 分，判断是否需要拆分大文件 |
| AI agent | 运行 `decay` 评估项目脆弱度 | 知道哪些文件改动频繁但不知道整体脆弱度 | 获得 fragility 0-100 分，判断哪些模块最脆弱 |
| AI agent | 运行 `decay` 快速评估项目 | 三个独立分数需要分别解读 | 一个 composite 分 + 多维度明细，一眼判断是否需要介入 |
| decay 开发者 | 新增一个评分维度 | 修改 score.rs / diagnose.rs / run.rs / db.rs 多处硬编码 | 新增 1 个文件 + 注册 1 行 |
| 开发者 | 用 decay 分析不同类型项目 | 所有项目一样的评分标准，误报或漏报 | 自动识别项目类型，差异化权重 |

## 3. 核心假设

- **假设**: 基于采集数据计算各维度 0-100 分数 → 用户能量化项目健康程度
- **验证方式**: 运行 `decay`，输出包含各维度分数和 composite 合成分
- **假设**: 将维度抽象为统一 trait + 注册表 → 新增维度成本从"改 4+ 文件"降为"新增 1 个文件 + 注册 1 行"
- **验证方式**: 重构后现有维度通过 trait 注册表统一调度，输出与重构前一致
- **假设**: 自动检测项目类型 + 按类型适配权重 → 减少误报/漏报
- **验证方式**: 对 CLI 工具和 Web 服务分别运行 decay，观测到不同维度的权重差异

## 4. 方案

### structural 评分
- **Before**: 有文件结构数据但不知道好坏 → **After**: 输出 structural 0-100 分，分数越高越健康

### complexity 评分
- **Before**: 知道哪些文件大但不知道整体复杂度水平 → **After**: 输出 complexity 0-100 分，量化复杂度健康程度

### fragility 评分
- **Before**: 知道哪些文件改动频繁但不知道整体脆弱度 → **After**: 输出 fragility 0-100 分，量化变更风险

### composite 合成评分
- **Before**: 三个独立分数需要分别解读 → **After**: 一个 composite 分 + 多维度明细，一目了然

### Dimension trait 统一注册
- **Before**: 3 个维度硬编码在 score.rs / diagnose.rs / run.rs / db.rs 中 → **After**: 每个维度是独立模块，实现 Dimension trait，通过注册表统一调度

### 分层打分系统
- **Before**: 所有项目一套固定阈值和等权平均 → **After**: 自动检测项目类型 → 加载 ScoreProfile → 各维度用该 profile 的权重聚合 composite score

### 任务追踪

| 任务 | Tech | 状态 | 备注 |
|------|------|------|------|
| structural 评分 | [tech](tech.md) | 已完成 | — |
| complexity 评分 | [tech](tech.md) | 已完成 | — |
| fragility 评分 | [tech](tech.md) | 已完成 | — |
| composite 合成评分 | [tech](tech.md) | 已完成 | — |
| Dimension trait 统一注册 | [tech](tech.md) | 已完成 | — |
| 分层打分系统 | [tech](tech.md) | 已完成 | — |

## 5. 验收标准

- 用户运行 `decay` → 应看到 structural、complexity、fragility 分数（各 0-100）
- 目录深度越深、文件数越多 → structural 分数越低
- 大文件占比高、平均文件大 → complexity 分数偏低
- 变更集中在少数文件、churn 高 → fragility 分数偏低
- 无 git 历史 → fragility: N/A（不报错）
- 用户运行 `decay` → 应看到 composite 分数和所有维度明细
- N/A 维度不参与合成，权重重新分配
- `cargo test` 全部通过，重构后 `decay` 输出与重构前一致
- 新增维度只需：创建 `src/dimension/xxx.rs`、实现 Dimension trait、在 `all_dimensions()` 中注册
- 项目类型自动检测正确识别 CLI / Web / Library / Generic
- 不同项目类型的 ScoreProfile 包含差异化的维度权重
- `decay --json` 输出包含检测到的项目类型

## 6. 排除项

- 文件内容分析如圈复杂度（→ complexity 维度后续增强）
- 评分等级映射如 A/B/C/D（→ 输出格式）
- 问题诊断和重构建议（→ 处方引擎）
- 历史对比和趋势（→ 趋势引擎）
- 作者维度分析如 bus factor（→ 后续版本）
- 用户手动指定项目类型（v3 scope 是自动检测）
- 用户自定义权重 UI（先用合理默认值验证假设）
