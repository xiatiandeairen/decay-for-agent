# 测试加固 技术方案

## 1. 背景

补充边界测试 + `--debug` flag。

## 2. 方案

log + env_logger。`--debug` 设置 `RUST_LOG=debug`，`env_logger::init()`。关键路径加 `log::debug!`。

### --debug 行为

- `decay --debug` → 输出 debug 级别日志到 stderr
- `decay` → 无额外日志
- 日志覆盖：db init、scan 进度、git 分析、评分计算、诊断规则

### 补充测试

- score: 空快照、满分、零分边界
- diagnose: 无文件、无 git、全健康
- trend: 无历史、有历史、分数不变

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 日志框架 | log + env_logger | 轻量适合 CLI |
| debug 输出 | stderr | 不污染 stdout（尤其 --json 模式）|

## 4. 迭代记录

- 2026-04-14: 初始方案
