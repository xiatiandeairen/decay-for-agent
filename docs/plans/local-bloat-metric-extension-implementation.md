# 局部膨胀指标扩展 Implementation Plan

> 日期: 2026-05-04
> 目的: 把 [docs/decision/local-bloat-metric-extension-spec.md](/Users/taoxia/Workspace/self/skills/decay/docs/decision/local-bloat-metric-extension-spec.md) 落成可执行实现计划。

## 1. 范围

本计划只覆盖 3 个新增方向中的前两波:

### Wave 1

- `statement_count`
- `max_condition_ops`

### Wave 2

- `mutable_bindings`

目标不是一次性把新指标变成主裁决器，而是:

- 接入 snapshot + diff
- 加基础测试
- 在 `hotspots/check/diff` 中以辅助 signal 方式展示
- 通过 dogfood 决定后续是否升格

## 2. 现有代码基线

当前主链路:

- walker: [src/walk.rs](/Users/taoxia/Workspace/self/skills/decay/src/walk.rs)
- parser: [src/parser.rs](/Users/taoxia/Workspace/self/skills/decay/src/parser.rs)
- metrics pipeline: [src/pipeline.rs](/Users/taoxia/Workspace/self/skills/decay/src/pipeline.rs)
- metric modules:
  - [src/metric/nesting.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/nesting.rs)
  - [src/metric/cyclomatic.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/cyclomatic.rs)
  - [src/metric/cognitive.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/cognitive.rs)
  - [src/metric/params.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/params.rs)
- metrics struct: [src/types.rs](/Users/taoxia/Workspace/self/skills/decay/src/types.rs)
- threshold config: [src/config.rs](/Users/taoxia/Workspace/self/skills/decay/src/config.rs)
- output and breach collection: [src/cli/common.rs](/Users/taoxia/Workspace/self/skills/decay/src/cli/common.rs)
- diff logic: [src/diff.rs](/Users/taoxia/Workspace/self/skills/decay/src/diff.rs)
- persistence: [src/store.rs](/Users/taoxia/Workspace/self/skills/decay/src/store.rs)

## 3. 数据结构变更

### 3.1 `Metrics`

文件: [src/types.rs](/Users/taoxia/Workspace/self/skills/decay/src/types.rs)

新增字段顺序建议:

```rust
pub struct Metrics {
    pub nesting: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub params: u32,
    pub statement_count: u32,
    pub max_condition_ops: u32,
    pub mutable_bindings: u32,
}
```

说明:

- 即使 `mutable_bindings` 放在 Wave 2，也建议一次性把结构位留好
- 这样可以减少后续 schema 和输出改动次数

### 3.2 `Thresholds`

文件: [src/config.rs](/Users/taoxia/Workspace/self/skills/decay/src/config.rs)

建议新增:

```rust
pub struct Thresholds {
    pub nesting: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub params: u32,
    pub statement_count: u32,
    pub max_condition_ops: u32,
    pub mutable_bindings: u32,
}
```

初始建议:

- `statement_count = 25`
- `max_condition_ops = 4`
- `mutable_bindings = 5`

注意:

- 这三个阈值第一版更偏“展示/观察阈值”
- 不等于立刻作为强 gate

## 4. 存储变更

文件: [src/store.rs](/Users/taoxia/Workspace/self/skills/decay/src/store.rs)

### 4.1 数据库 schema

`functions` 表需新增列:

- `statement_count`
- `max_condition_ops`
- `mutable_bindings`

### 4.2 兼容策略

当前项目还没有正式 migration 体系。两种选择:

#### 方案 A

- 直接调整 schema 创建逻辑
- dogfood 阶段接受旧 DB 不兼容，手工删库

#### 方案 B

- 在 `open_db` 时做最小增量 `ALTER TABLE`

建议:

- 如果你想继续快推进，用 **方案 A**
- 如果你希望后续反复 dogfood 更稳，用 **方案 B**

从当前状态看，我更建议 **方案 B**，因为这次之后指标迭代还会继续。

## 5. Metric 模块设计

### 5.1 `statement_count`

新增文件:

- [src/metric/statements.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/statements.rs)
- [tests/metric_statements_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/metric_statements_test.rs)

实现职责:

- 输入: `tree`, `source`, `body_range`
- 输出: `u32`
- 只统计函数体内 statement 节点数量

实现建议:

- 参照现有 metric 模块结构
- 沿用 tree-sitter 遍历
- 在 body range 内统计 statement 类节点
- block 节点本身不计数

测试建议:

1. 空函数
2. 单 statement
3. 顺序多个 statement
4. 嵌套 block 内 statement
5. `if/match` 分支体内 statement
6. 注释/空行不影响结果

### 5.2 `max_condition_ops`

新增文件:

- [src/metric/condition_ops.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/condition_ops.rs)
- [tests/metric_condition_ops_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/metric_condition_ops_test.rs)

实现职责:

- 找出函数体中的条件表达式
- 对每个条件统计 `&&` / `||`
- 返回函数内最大值

第一版统计范围:

- `if`
- `else if`
- `while`
- `match guard`

实现建议:

- 不从纯文本扫 token
- 尽量基于 AST 定位条件节点
- 在条件子树内递归数逻辑操作节点

测试建议:

1. `if a {} -> 0`
2. `if a && b {} -> 1`
3. `if a && b || c {} -> 2`
4. 多个条件取最大值
5. 非条件位置的布尔表达式不计

### 5.3 `mutable_bindings`

新增文件:

- [src/metric/mutable_bindings.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/mutable_bindings.rs)
- [tests/metric_mutable_bindings_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/metric_mutable_bindings_test.rs)

实现职责:

- 统计函数体内显式 `let mut` 绑定数量

第一版不要做:

- 全赋值点统计
- shadowing 复杂去重推理
- 状态复杂度总分

测试建议:

1. 无 `mut`
2. 单 `let mut`
3. 多个 `let mut`
4. 普通 `let` 不计
5. 简单 shadowing 场景不误算

## 6. Pipeline 接入

文件: [src/pipeline.rs](/Users/taoxia/Workspace/self/skills/decay/src/pipeline.rs)

Wave 1 接入:

```rust
statement_count: metric::statements::compute(...),
max_condition_ops: metric::condition_ops::compute(...),
mutable_bindings: 0,
```

Wave 2 接入:

```rust
mutable_bindings: metric::mutable_bindings::compute(...),
```

文件: [src/metric/mod.rs](/Users/taoxia/Workspace/self/skills/decay/src/metric/mod.rs)

需导出新增模块。

## 7. Diff / breach / 输出接入

### 7.1 breach 收集

文件: [src/cli/common.rs](/Users/taoxia/Workspace/self/skills/decay/src/cli/common.rs)

扩展 `collect_breaches`:

- 新增 `statement_count`
- 新增 `max_condition_ops`
- 新增 `mutable_bindings`

### 7.2 输出策略

第一版建议:

- `hotspots`: 允许显示新指标，但排序仍以现有高信号 metric 为主
- `check/diff`: 新指标 delta 可以展示
- 不建议第一版把 `mutable_bindings` 作为单独强退化触发器

更具体地:

#### Wave 1

- `statement_count`、`max_condition_ops` 进入 breach 收集
- 但若只单独触发、且没有其他高信号项，可考虑仅在详情展示，不提升到顶部

#### Wave 2

- `mutable_bindings` 默认只在 diff 明细里展示

## 8. Diff 逻辑影响

文件: [src/diff.rs](/Users/taoxia/Workspace/self/skills/decay/src/diff.rs)

需要确认:

- diff comparison 是否对 `Metrics` 全字段生效
- 新 metric 加入后排序是否会被弱信号项扰动

建议:

- 先让新字段参与 delta 计算
- 排序逻辑保持“最大 overage”优先
- 如果新 metric 导致排序明显失真，再单独加权

## 9. 测试计划

### 9.1 新增单测

新增:

- [tests/metric_statements_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/metric_statements_test.rs)
- [tests/metric_condition_ops_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/metric_condition_ops_test.rs)
- [tests/metric_mutable_bindings_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/metric_mutable_bindings_test.rs)

### 9.2 现有测试需更新

需更新:

- [tests/diff_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/diff_test.rs)
  - `Metrics` 构造器补新字段
- [tests/store_test.rs](/Users/taoxia/Workspace/self/skills/decay/tests/store_test.rs)
  - round-trip 校验补新字段
- [tests/integration.rs](/Users/taoxia/Workspace/self/skills/decay/tests/integration.rs)
  - 至少增加一个新指标出现在输出中的集成 case

### 9.3 可选新增集成样例

建议补两个 fixture case:

1. 条件表达式膨胀
2. statement 数显著增加但嵌套不深

## 10. 建议实施顺序

### Step 1

- 扩 `Metrics`
- 扩 `Thresholds`
- 扩 store schema

### Step 2

- 实现 `statement_count`
- 单测通过

### Step 3

- 实现 `max_condition_ops`
- 单测通过

### Step 4

- 接入 pipeline + diff + output
- 更新 store/diff/integration tests

### Step 5

- 跑全量 `cargo test`
- 在自测仓库和 `ripgrep` 上观察输出

### Step 6

- 若 Wave 1 信号好，再实现 `mutable_bindings`

## 11. Go / No-Go 标准

### Wave 1 Go

- 两个新 metric 单测全通过
- 全量 `cargo test` 通过
- 不引入明显热点噪音爆炸
- 在至少一个真实函数上提供增量解释价值

### Wave 2 Go

- Wave 1 已完成
- 已观察到“状态膨胀”是现实盲区
- `mutable_bindings` 不会明显拖垮输出信噪比

## 12. 最终建议

如果现在就开工，实现上最稳的路径是:

1. 先做 Wave 1
2. 用真实项目观察输出
3. 再决定 `mutable_bindings` 是否升到 Wave 2

一句话:

> **先补职责堆叠和条件膨胀，再谨慎补状态膨胀。**
