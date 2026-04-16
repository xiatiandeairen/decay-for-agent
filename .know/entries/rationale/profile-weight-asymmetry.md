# Profile 权重不对称设计

## Why

6 种项目类型对 8 个维度的权重不同，反映不同类型项目的核心关注点差异。

设计原则：**每种项目类型有 1-2 个"命门"维度（权重 ≥ 0.18），其余维度分摊剩余权重**。

| 项目类型 | 命门维度 | 权重 | 理由 |
|----------|----------|------|------|
| CLI | complexity | 0.18 | CLI 单入口多分支，复杂度是首要可维护性障碍 |
| WebService | observability | 0.18 | 线上服务出问题时日志/监控是唯一线索 |
| Library | maintainability | 0.20 | 库的 API 被外部依赖，可维护性 = 兼容性 |
| MobileApp | performance | 0.15 | 移动端资源受限，性能直接影响用户体验 |
| Monorepo | structural | 0.18 | 多模块项目结构混乱会导致构建/依赖灾难 |
| Generic | all | 0.125 | 无法判断类型时平均分配 |

## Constraints

- 所有权重和 = 1.0（代码中 renormalize 保证）
- 最低权重 0.05（Library 的 observability），不会完全忽略任何维度
- 权重数值来源：基于项目类型的典型故障模式经验判断，非统计推导
