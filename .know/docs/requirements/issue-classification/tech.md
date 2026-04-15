# 问题分类引擎 技术方案

## 文件变更

| File | 变更 |
|------|------|
| `src/diagnose.rs` | IssueCategory 枚举 + Issue.classification 字段 |
| `src/classify.rs` | 新模块：classify_issues + classify 规则函数 + 9 tests |
| `src/main.rs` | mod classify 注册 |
| `src/run.rs` | 调用 classify_issues |
| `src/render.rs` | markdown 输出分类标签 |

## 分类规则优先级

1. D: SecurityCritical — injection/credential 关键词
2. A: MechanicalFix — per-file unwrap/catch/hardcoded
3. G: ContextualException — unsafe/eval
4. H: Prevention — dependencies/blocking
5. B: PatternProblem — duplicate/clone density
6. F: ChronicDecay — TODO/ratio/Info-level
7. E: ConventionDrift — no logging/no tests/low ratio
8. C: ArchitecturalDecision — Split/Refactor/structural
9. Default → C (most conservative)
