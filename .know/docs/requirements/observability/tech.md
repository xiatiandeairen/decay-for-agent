# 可观测性与恢复能力 技术方案

## 评分逻辑（扣分制，100 分起）
| 指标 | 阈值 | 扣分 |
|------|------|------|
| unwrap/panic 密度 | >5 per 1K lines | -15 |
| unwrap/panic 密度 | >15 per 1K lines | -30（替代） |
| 无日志框架引用 | 源文件中无 log/logger 调用 | -20 |
| 错误吞没比例 | catch/except 块中无处理 >20% | -15 |
| 硬编码配置 | >5 处 | -10 |

## 文件变更
| Action | File |
|--------|------|
| create | src/dimension/observability.rs |
| modify | src/dimension/mod.rs |
