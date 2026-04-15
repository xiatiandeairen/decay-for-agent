# 问题分类引擎 — 将 55 种 issue 标记为 A-H 类别

## 1. 问题

所有 issue 一视同仁地输出，用户和 agent 无法区分哪些是简单修复、哪些需要架构决策、哪些是误报。

## 2. 方案

- IssueCategory 枚举（8 类：MechanicalFix / PatternProblem / ArchitecturalDecision / SecurityCritical / ConventionDrift / ChronicDecay / ContextualException / Prevention）
- Issue.classification 字段
- classify.rs 规则引擎：基于 (dimension, message pattern, action_type, level) 映射
- JSON/terminal/markdown 输出包含分类标签

## 3. 验收标准

- 每种 issue 通过分类器后有唯一分类
- 9 个分类测试覆盖全部 8 类
- `cargo test` 全部通过
