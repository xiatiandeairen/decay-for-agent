# 8 类问题分类体系 (A-H)

## Overview

classify.rs 使用 first-match-wins 规则表将每个 issue 映射到 8 个类别之一。类别按**可操作性成本**排列：

| 代号 | 类别 | 含义 | 操作成本 |
|------|------|------|----------|
| D | SecurityCritical | 注入、凭据泄露 | 必须立即修 |
| A | MechanicalFix | unwrap/empty catch 等可自动修复 | 分钟级 |
| E | ConventionDrift | 规范偏离（无日志等） | 小时级 |
| B | PatternProblem | 重复模式需提取/重构 | 小时级 |
| F | ChronicDecay | 长期积累的衰退（大文件、高复杂度） | 天级 |
| C | ArchitecturalDecision | 需要设计决策的结构问题 | 天级 |
| G | ContextualException | 上下文相关的例外（测试/FFI/Parser） | 可忽略 |
| H | Prevention | 预防性配置（cargo-deny 等） | 配置级 |

## Key Steps

1. 安全类优先匹配（D），因为安全问题无论维度/级别都应最先处理
2. 然后按维度+消息内容匹配 A/E/B/F/C
3. G 类通过 FileContext 在 classify 之后降级，不在规则表中
4. H 类由 prevention.rs 独立生成

## Boundaries

- 分类是**诊断层**，不影响处方生成（action.rs 独立）
- 规则表是静态数据，不做运行时学习
- 类别之间无层级关系，first-match 是唯一优先级机制
- 新增规则只需追加到 RULES 数组，位置决定优先级
