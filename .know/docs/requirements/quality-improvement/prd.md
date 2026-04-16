# 质量提升

## 1. 问题

### 痛点

- **测试不充分**: 当前测试覆盖基本路径但缺少边界情况，`--debug` flag 不存在，调试时无法看到内部流程，排查问题靠猜。
- **检测误报**: observability 对 `#[cfg(test)]` 中的 unwrap 报警（测试代码使用 unwrap 是正常做法），reliability 对错误消息中的 format! + SQL 关键词误报为 SQL 注入。
- **issue 无分类**: 所有 issue 一视同仁地输出，用户和 agent 无法区分哪些是简单修复、哪些需要架构决策、哪些是误报。

### 影响范围

影响所有 decay 开发者和用户。测试不充分导致新功能回归风险高，误报降低工具可信度，无分类导致 agent 无法制定差异化修复策略。涉及 observability/reliability 评分准确性和全部 55 种 issue 的消费效率。

### 为什么现在做

v2 新增多个输出模式复杂度上升，测试不充分会导致回归。误报降低工具可信度，用户看到假阳性会忽略真正的问题。没有分类，agent 无法按类别制定差异化修复策略。

## 2. 目标用户

| 角色 | 场景 | Before | After |
|------|------|--------|-------|
| decay 开发者 | 新增功能/重构 | 测试不覆盖边界，回归靠手动验证 | 边界测试自动捕获回归 |
| 排查问题的用户 | 异常评分排查 | 无调试手段，靠猜 | `--debug` 输出采集进度、评分计算、诊断过程 |
| 开发者 | 查看 observability 评分 | 测试代码 unwrap 被误报，分数偏低 | 测试代码 unwrap 不计入，分数准确 |
| 开发者 | 查看 reliability 评分 | 错误消息中的 SQL 词被误报为注入 | 错误消息上下文被正确排除 |
| AI agent | 基于 issue 制定修复计划 | 需过滤误报，浪费 token | 只收到真正的 issue |
| AI agent | 按类别制定差异化修复策略 | 所有 issue 同质，需逐个分析 | 按 8 类分类，MechanicalFix 直接修复，ArchitecturalDecision 需确认 |
| 开发者 | 快速筛选可行动的 issue | 需逐个阅读判断类型 | 通过分类标签快速定位 |

## 3. 核心假设

- **假设**: 补充边界测试 + `--debug` flag → 代码质量有保障，问题可快速定位
- **验证方式**: `cargo test` 全通过且覆盖边界情况；`decay --debug` 输出详细日志
- **假设**: test block 检测 + 错误消息上下文排除 → 消除已知误报类型，不影响真正问题的检测
- **验证方式**: 改进后测试代码 unwrap 不计入评分，真正的 SQL 拼接仍被检测
- **假设**: 基于 (dimension, message pattern, action_type, level) 的规则映射 → 每种 issue 获得唯一准确分类
- **验证方式**: 9 个分类测试覆盖全部 8 类，无 issue 漏分类

## 4. 方案

- **Before**: 测试不完整、无调试手段 → **After**: 边界测试全覆盖，`--debug` 输出内部流程日志到 stderr
- **Before**: 测试代码 unwrap 被误报，错误消息被误报为 SQL 注入 → **After**: helpers 新增 test block 检测，observability 跳过测试代码，reliability 排除错误消息上下文
- **Before**: 所有 issue 无分类标签 → **After**: IssueCategory 枚举 8 类（MechanicalFix/PatternProblem/ArchitecturalDecision/SecurityCritical/ConventionDrift/ChronicDecay/ContextualException/Prevention），classify.rs 规则引擎

### 任务追踪

| 任务 | Tech | 状态 | 备注 |
|------|------|------|------|
| 测试加固 + debug flag | [tech](tech.md) | 已完成 | log + env_logger，边界测试补充 |
| 检测精度提升 | [tech](tech.md) | 已完成 | test mask + 错误消息过滤 |
| 问题分类引擎 | [tech](tech.md) | 已完成 | 8 类枚举 + 规则引擎 |

## 5. 验收标准

- 用户执行 `cargo test` → 全通过，覆盖边界情况（空快照、满分、零分、无文件、无 git、全健康）
- 用户执行 `decay --debug` → 应看到详细日志输出到 stderr
- 用户有 `#[cfg(test)]` 或 `#[test]` 内的 unwrap → 不应计入 observability 评分
- 用户有 `format!("DELETE failed...")` 等错误消息 → 不应触发 SQL injection 报警
- 用户有真正的 SQL 拼接 → 应仍被检测到
- 用户查看任意 issue → 应看到唯一的分类标签
- 用户执行 `--json` → 应看到 issue 包含 classification 字段
- 9 个分类测试覆盖全部 8 类

## 6. 排除项

- 不包含 CI 覆盖率报告 — 当前阶段手动验证足够
- 不包含集成测试（跨模块 E2E）— 单元测试优先
- 不做其他维度的 test block 过滤 — 当前只有 observability 有此问题
- 不做通用的误报反馈机制 — 当前规则足够，反馈系统留 v7+
- 不做分类自定义配置 — 当前规则覆盖全部 issue
- 不做分类统计聚合 — 消费方可自行聚合
