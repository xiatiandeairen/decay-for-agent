# 局部膨胀指标扩展 Mini Spec

> 日期: 2026-05-04
> 目的: 为 `statement_count`、`condition_complexity`、`local_state_complexity` 三个方向提供可直接开工的最小设计。

## 1. 设计目标

当前 4 个 metric 已覆盖:

- 嵌套变深
- 分支变多
- 控制流更难读
- 参数面变宽

但未充分覆盖:

- 职责块堆叠
- 条件表达式膨胀
- 局部状态膨胀

本 spec 的目标不是一次性补完局部膨胀，而是为这三个缺口增加 **简单、可解释、低噪音** 的 proxy metric。

约束:

- 必须适合 commit 前裁决
- 必须易解释
- 必须能进入现有 snapshot + diff 模型
- 第一版尽量不用复杂综合分数

## 2. 推荐落地顺序

### 第一优先级

1. `statement_count`
2. `max_condition_ops`

### 第二优先级

3. `mutable_bindings`

原因:

- `statement_count` 最稳
- `max_condition_ops` 最贴 AI 补条件行为
- `mutable_bindings` 有增量，但噪音风险更高

## 3. Metric 1: statement_count

### 3.1 要解决的问题

识别这类函数:

- 没有明显深嵌套
- 也不一定分支很多
- 但函数体里不断堆职责块
- 越来越像“杂物间”

### 3.2 定义

`statement_count` = 函数体内部 statement 节点数量。

目标不是数 token、数行，而是数“这段函数里到底塞了多少独立执行单元”。

### 3.3 统计口径

第一版建议:

- 统计函数 body 内所有 statement 节点
- 包括嵌套 block 里的 statement
- 不统计:
  - 空白
  - 注释
  - 分隔符
  - 纯 block 节点本身

Rust 侧实现上，优先基于 tree-sitter 的 statement 类节点计数，而不是按文本行猜。

### 3.4 为什么选它

优点:

- 稳定
- 易解释
- 比 body line span 更不受格式影响
- 很适合补“职责堆叠”盲区

### 3.5 风险

- 长函数不一定难维护
- 某些数据映射 / builder / test helper 可能 statement 多但不糟

所以第一版不建议把它当成强告警主裁决器。

### 3.6 阈值建议

建议先作为观察项，阈值保守:

- `statement_count > 20` 观察
- `statement_count > 25` 才考虑热点评级

更重要的是看 delta:

- `12 → 24`
- `18 → 29`

### 3.7 测试样例

至少覆盖:

1. 空函数 / 单返回
2. 顺序 5 条 statement
3. 嵌套 block 内 statement 也计数
4. 带注释和空行不影响
5. `match`/`if` 分支体内 statement 被统计

### 3.8 接入建议

- `hotspots`: 展示，但排在 `cognitive/cyclomatic` 后
- `diff`: 展示 delta
- `check`: 可作为辅助信息，不建议第一版单独触发 degraded

## 4. Metric 2: max_condition_ops

### 4.1 要解决的问题

识别这类 case:

- 分支没有明显变深
- 但单个条件表达式越来越难读
- AI 常通过“再补一个 && 条件”继续修补逻辑

### 4.2 定义

`max_condition_ops` = 函数内单个条件表达式里逻辑操作符数量的最大值。

逻辑操作符第一版只统计:

- `&&`
- `||`

不把所有比较运算都纳入，避免口径膨胀。

### 4.3 统计口径

第一版只统计这些上下文中的条件表达式:

- `if`
- `else if`
- `while`
- `match guard`

对于每个条件:

- 递归统计其中出现的 `&&` / `||` 数量
- 取函数内最大值

### 4.4 为什么选它

优点:

- 很贴近 AI 真实补丁行为
- 简单直观
- 和 `cyclomatic` 有关联，但不完全重复

它回答的是:

- “这次是不是把一个条件写得越来越像谜语”

### 4.5 风险

- 对某些合法复杂判断可能误伤
- 和 `cognitive` 有一定重叠
- 需要处理 Rust 条件 AST 的细节

### 4.6 阈值建议

保守初值:

- `max_condition_ops > 3` 观察
- `max_condition_ops > 4` 才考虑热点

delta 更重要:

- `1 → 4`
- `2 → 5`

### 4.7 可选二期扩展

后续如果第一版信号好，再考虑:

- `max_condition_depth`
- 统计 `!` 连用或括号嵌套层级

但不建议第一版就上。

### 4.8 测试样例

至少覆盖:

1. `if a {}` → `0`
2. `if a && b {}` → `1`
3. `if a && b || c {}` → `2`
4. 多个条件时取最大值
5. 非条件布尔表达式不计入

### 4.9 接入建议

- `hotspots`: 可展示，排序权重低于 `cognitive`
- `diff`: 强烈建议展示 delta
- `check`: 第一版不建议单独作为 degraded 判定主条件

## 5. Metric 3: mutable_bindings

### 5.1 要解决的问题

识别这类函数:

- 局部状态越来越重
- `let mut` 越来越多
- 越来越像小状态机

### 5.2 定义

`mutable_bindings` = 函数体内 `let mut` 绑定数量。

第一版只统计显式 mutable 绑定，不统计所有赋值点，也不统计 shadowing 的复杂情况。

### 5.3 为什么先只做这个

相比:

- `local_bindings`
- `assignments`

`mutable_bindings` 更容易解释，也更接近“状态在流动”的直觉。

### 5.4 风险

- 某些算法函数天然需要多个 `mut`
- Rust 风格里有时 `mut` 只是实现细节，不一定坏

所以它比前两个 metric 更应该谨慎使用。

### 5.5 阈值建议

只建议先看 delta，不建议强看绝对值。

观察参考:

- `0 → 3`
- `1 → 4`
- `2 → 5`

初始热点阈值可以非常保守:

- `mutable_bindings > 4`

但第一版更建议只做 diff 展示。

### 5.6 后续可演进方向

如果 signal 足够好，再扩到:

- `assignments`
- `state_updates`
- `local_bindings`

但不建议一步到位做“状态复杂度总分”。

### 5.7 测试样例

至少覆盖:

1. 无 `mut`
2. 单个 `let mut`
3. 多个 `let mut`
4. shadowing 不重复误算
5. 只读绑定不计数

### 5.8 接入建议

- `hotspots`: 第一版不建议主展示
- `diff`: 可展示 delta
- `check`: 仅作辅助 signal

## 6. 统一接入策略

### 6.1 Snapshot / diff

这三个 metric 如果落地，应遵循现有模型:

- 进入 snapshot
- 支持 diff
- 和现有 metric 一样按函数粒度比较 delta

### 6.2 `hotspots`

第一版展示策略建议:

- 继续以 `cognitive / cyclomatic / nesting` 为主
- 新 metric 只做补充，不抢主排序

原因:

- 避免把热点列表做成大报表
- 避免新指标初期放大噪音

### 6.3 `check`

第一版判定策略建议:

- `statement_count` 和 `max_condition_ops` 可参与 degraded 说明
- 但不建议一开始单独作为“只因这一项就判 degraded”的主触发条件
- `mutable_bindings` 仅作辅助，不单独触发

也就是:

- 先让它们解释“为什么更糟”
- 再观察是否值得升格为主裁决项

## 7. 推荐实施计划

### Wave 1

- 实现 `statement_count`
- 实现 `max_condition_ops`
- 加单测
- 进入 snapshot + diff
- `hotspots/check` 仅补充展示

### Wave 2

- dogfood 观察噪音和增量价值
- 记录:
  - 哪类函数被抓到
  - 哪类明显误报
  - 是否补到“职责堆叠 / 条件膨胀”盲区

### Wave 3

- 若 Wave 2 信号好，再实现 `mutable_bindings`

## 8. 最终建议

如果现在就进入实现，我建议:

1. 先做 `statement_count`
2. 再做 `max_condition_ops`
3. `mutable_bindings` 放下一波

理由:

- `statement_count` 最稳
- `max_condition_ops` 最贴 AI 修补行为
- `mutable_bindings` 最有潜力，但也最容易放大噪音

## 9. 一句话结论

这三个方向都值得做，但正确姿势不是“再加三个强告警指标”，而是:

> **先把它们作为低噪音、可解释的增量 signal 接进现有 diff 裁决链路，再通过 dogfood 决定哪些值得升格。**
