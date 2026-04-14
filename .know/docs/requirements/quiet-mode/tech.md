# quiet 模式 技术方案

## 1. 背景

`--quiet` flag + 语义化 exit code，面向自动化集成。

## 2. 方案

Cli struct 加 `--quiet` bool flag。输出一行摘要，exit code 反映健康状态。

### exit code

- 0: 无 critical issues（健康或只有 warning/info）
- 1: 有 critical issues
- 2: 执行错误（已由 anyhow 处理）

### 输出

`Health: {composite}/100 ({N} critical)`

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| exit code 语义 | 0=ok, 1=critical | 标准 Unix 惯例 |
| 输出内容 | composite + critical 数 | 最小有用信息 |

## 4. 迭代记录

- 2026-04-14: 初始方案
