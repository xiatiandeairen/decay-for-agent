# 分层打分系统

## 1. 问题

当前所有项目使用相同的阈值和权重打分。一个 CLI 工具和一个 Web 服务在结构深度、测试覆盖率、可观测性等方面的合理标准完全不同。固定阈值导致误报（对 CLI 报"可观测性不足"）或漏报（对 Web 服务放过"无错误处理"）。

## 2. 目标用户

使用 decay 分析不同类型项目的开发者和 AI agent。分层打分后，不同项目类型得到差异化的评分标准。

## 3. 核心假设

**自动检测项目类型 + 按类型适配阈值和权重 → 减少误报/漏报，评分更贴合项目实际需求。**

验证方式：对 CLI 工具和 Web 服务分别运行 decay，观测到不同维度的权重差异。

## 4. 方案

- **Before**: 所有项目一套固定阈值和等权平均
- **After**: 自动检测项目类型 → 加载 ScoreProfile → 各维度用该 profile 的阈值评分，composite 用该 profile 的权重聚合

### 任务

| 任务 | 文档 | 进度 |
|------|------|------|
| score-profile tech | [tech](tech.md) | 0/0 |

## 5. 验收标准

- 项目类型自动检测正确识别 CLI / Web / Library / Generic
- 不同项目类型的 ScoreProfile 包含差异化的维度权重
- Dimension trait 的 score() 可访问 ScoreProfile 中的阈值（接口扩展）
- composite score 使用 ScoreProfile 权重加权
- 检测失败时 fallback 到 generic profile
- `decay --json` 输出包含检测到的项目类型

## 6. 排除项

- 用户手动指定项目类型（v3 scope 是自动检测）
- 用户自定义权重 UI
- 每个维度的阈值细调（各维度 M4-M8 自行负责）
