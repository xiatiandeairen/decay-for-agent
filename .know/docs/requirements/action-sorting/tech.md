# Action 优先级排序 技术方案

## 1. 背景

PRD 要求对顶层 actions 数组实现多级排序和去重。

## 2. 方案

### 2.1 Effort 可排序

```rust
// before
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
// after
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
```

枚举变体顺序 Small < Medium < Large，derive Ord 自动按定义顺序排。

### 2.2 多级排序

```rust
all_actions.sort_by(|a, b| {
    a.priority.cmp(&b.priority).then(a.effort.cmp(&b.effort))
});
```

效果：Critical+Small 在最前（紧急且低成本），Low+Large 在最后。

### 2.3 去重

```rust
all_actions.dedup_by(|b, a| {
    a.dimension == b.dimension
        && a.target.file == b.target.file
        && a.action_type == b.action_type
});
```

dedup_by 在排序前执行（issues 已按 level 排序），相邻的同类 action 去重保留第一个。

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| modify | `src/action.rs` | Effort 加 PartialOrd + Ord |
| modify | `src/run.rs` | dedup + 多级排序 |
| modify | `src/action.rs` | 新增 effort_ordering + sort 测试 |

## 4. 迭代记录

- 2026-04-15: 初始方案
