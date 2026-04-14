# 可靠性与安全 技术方案

## 评分逻辑（扣分制，100 分起）
| 指标 | 阈值 | 扣分 |
|------|------|------|
| unsafe/eval 密度 | >2 per 1K lines | -15 |
| unsafe/eval 密度 | >8 per 1K lines | -30（替代） |
| SQL/shell 注入模式 | any | -20 per occurrence (max -40) |
| 硬编码密钥/密码模式 | any | -15 per occurrence (max -30) |
| 依赖数量 | >50 直接依赖 | -10 |
| 依赖数量 | >100 直接依赖 | -20（替代） |

## 文件变更
| Action | File |
|--------|------|
| create | src/dimension/reliability.rs |
| modify | src/dimension/mod.rs |
