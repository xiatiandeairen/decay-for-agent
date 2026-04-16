# 耦合检测使用频率相似性代理，存在误报

## Symptoms

impact.rs 的 `build_coupling_map` 用 change_count 的 ±30% 相似性判断文件耦合。两个不相关但 churn 频率相近的文件会被误判为耦合。

## Root Cause

缺少 per-commit 粒度数据。git_changes 表只存每个文件的总 change_count，没有记录"哪些文件在同一个 commit 中一起改动"。只能用频率相似性作为代理指标。

关键参数：
- `change_count > 3`：过滤低频文件，减少噪声
- `ratio > 0.7`：30% 容差窗口，过宽会误报，过窄会漏报

## Lesson

- 当前实现是 v8 的 MVP 方案，已知局限性
- 如果要提升精度，需要在 git collector 层记录 per-commit file groups，成本是 DB 大小和扫描时间增加
- 误报方向偏保守：宁可多报耦合（用户忽略即可），不应漏报（真正的耦合看不到）
- review_burden 和 risk_level 也依赖此数据，误报会级联
