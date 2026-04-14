# 性能 技术方案

## 评分逻辑（扣分制，100 分起）
| 指标 | 阈值 | 扣分 |
|------|------|------|
| 深层嵌套循环（≥3层） | >3 occurrences | -15 |
| 深层嵌套循环（≥3层） | >10 occurrences | -30（替代） |
| clone/copy 密度 | >10 per 1K lines | -10 |
| clone/copy 密度 | >25 per 1K lines | -20（替代） |
| 同步阻塞调用 | >5 occurrences | -10 |
| 同步阻塞调用 | >15 occurrences | -20（替代） |

## 文件变更
| Action | File |
|--------|------|
| create | src/dimension/performance.rs |
| modify | src/dimension/mod.rs |
