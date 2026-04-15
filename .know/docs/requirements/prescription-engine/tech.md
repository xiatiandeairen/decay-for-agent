# Prescription 引擎重构 技术方案

## 1. 背景

PRD 要求将 8 个维度的文本处方迁移为 Action 生成器，统一使用 Issue 构造函数。

## 2. 方案

### 2.1 Issue 构造函数（新增 src/diagnose.rs）

```rust
impl Issue {
    pub fn new(level, category, message, prescription) -> Self  // actions: vec![]
    pub fn with_actions(level, category, message, prescription, actions) -> Self
}
```

### 2.2 维度 → Action 映射

| 维度 | Issue 类型 | ActionType | Priority | Effort |
|------|-----------|-----------|----------|--------|
| structural | file_count > CRIT | Split | Critical | Large |
| structural | file_count > WARN | Refactor | High | Medium |
| structural | depth > WARN | Move | Medium | Medium |
| complexity | size > 50KB | Split | Critical | Large |
| complexity | size > 15KB | Extract | High | Medium |
| fragility | concentration > WARN | Refactor | High | Large |
| fragility | high churn | Split | Critical | Medium |
| maintainability | duplicates | Extract | High | Medium |
| maintainability | long file | Split | Critical/High | Medium |
| maintainability | long function | Extract | High | Small |
| observability | unwrap/panic | Replace | High | Medium |
| observability | no logging | Add | High | Medium |
| observability | empty catches | Replace | High | Small |
| quality | no tests | Add | Critical | Large |
| quality | low test ratio | Add | High | Large |
| quality | low line ratio | Add | High | Medium |
| reliability | unsafe code | Replace | High | Medium |
| reliability | injection | Replace | Critical | Small |
| reliability | secrets | Replace | Critical | Small |
| reliability | excess deps | Remove | Medium | Small |
| performance | nested loops | Extract | Critical/High | Small |
| performance | excess clones | Refactor | Medium | Medium |

Info 级 issue 不附带 action（纯信息性，无明确操作）。

### 2.3 统一原则

- `Issue::new()` 用于 Info 级或无明确操作的 issue
- `Issue::with_actions()` 用于有结构化操作的 issue
- reason 字段包含具体数值和操作建议
- target.file 指向具体文件路径（有路径时），或 "." 表示项目级

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/diagnose.rs` | 新增 `new()` + `with_actions()` |
| modify | `src/dimension/structural.rs` | struct literal → 构造函数 |
| modify | `src/dimension/complexity.rs` | 添加 Action + 构造函数 |
| modify | `src/dimension/fragility.rs` | 添加 Action + 构造函数 |
| modify | `src/dimension/maintainability.rs` | 添加 Action + 构造函数 |
| modify | `src/dimension/observability.rs` | 添加 Action + 构造函数 |
| modify | `src/dimension/quality.rs` | 添加 Action + 构造函数 |
| modify | `src/dimension/reliability.rs` | 添加 Action + 构造函数 |
| modify | `src/dimension/performance.rs` | 添加 Action + 构造函数 |

## 4. 迭代记录

- 2026-04-15: 初始方案
