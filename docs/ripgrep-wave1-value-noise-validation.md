# ripgrep Wave 1 价值与噪音验证

> 日期: 2026-05-04
> 仓库: `/private/tmp/decay-test/ripgrep`
> commit: `4519153e5e`
> 二进制: `/Users/taoxia/Workspace/self/skills/decay/target/debug/decay`
> 本轮指标: `statement_count`、`max_condition_ops`

## 1. 目标

这轮验证只回答两个问题:

1. Wave 1 新指标是否真的补到了现有局部膨胀盲区
2. Wave 1 新指标是否明显放大了热点噪音

这里不再验证“命令能不能跑”，因为实现层测试已经由 `cargo test` 覆盖。

## 2. 测试用例设计

### A. 噪音验证

| 编号 | 用例 | 执行方式 | 目标 |
|---|---|---|---|
| N1 | 默认热点 | `hotspots` | 看新指标加入后热点总量和头部排序是否明显恶化 |
| N2 | 排除 `examples` | `hotspots --exclude examples` | 看非核心目录噪音是否下降 |
| N3 | 排除 `examples/tests` | `hotspots --exclude examples --exclude tests` | 看目录级测试噪音是否继续下降 |
| N4 | SQL 统计 statement-only 命中 | 查 snapshot DB | 看新命中里有多少只是 `statement_count` 单独触发 |
| N5 | SQL 统计 condition 命中 | 查 snapshot DB | 看 `max_condition_ops` 在真实仓库里是否产生自然信号 |

### B. 价值验证

| 编号 | 用例 | 执行方式 | 目标 |
|---|---|---|---|
| V1 | clean `check` | baseline 后执行 `check --exclude examples` | 确认新指标没有引入无改动误报 |
| V2 | 受控 statement case | 在 `build.rs:1 main` 注入大量顺序 statement | 验证 `statement_count` 能补到“职责堆叠 / 变长”盲区 |
| V3 | 受控 condition case | 在 `build.rs:1 main` 注入长 `&&` 条件 | 验证 `max_condition_ops` 能补到“条件表达式膨胀”盲区 |
| V4 | 输出链路核对 | 对比 `hotspots` 与 `check/diff` | 看新指标是否不仅检测到，而且能向用户解释出来 |

## 3. 执行结果

### 3.1 clean baseline

执行:

```text
decay init --exclude examples
decay check --exclude examples
```

结果:

- baseline 建立成功
- `check` clean，无误报

结论:

- 新指标没有破坏主链路稳定性

### 3.2 热点总量与噪音

执行结果:

| 场景 | 文件数 | 函数数 | 热点数 |
|---|---:|---:|---:|
| `hotspots` | 83 | 2742 | 89 |
| `hotspots --exclude examples` | 80 | 2735 | 87 |
| `hotspots --exclude examples --exclude tests` | 73 | 2657 | 85 |

结论:

- 排掉 `examples` 只减少 `2` 个热点
- 再排掉目录级 `tests` 也只再减少 `2` 个热点
- 说明噪音不只来自目录级样例代码，还来自:
  - `src/testutil.rs`
  - 内嵌在主源码里的测试辅助函数
  - 名称上像测试，但物理位置仍在主维护面内的函数

这进一步证明:

- **扫描对象问题不是简单加几个 `--exclude` 就能彻底解决**
- 后续仍需要 role/scope 方案

### 3.3 `statement_count` 的真实影响

SQL 统计:

- 总热点数: `87`
- `statement_count > 25` 的函数数: `44`
- 其中仅由 `statement_count` 单独触发、其余旧指标都未超阈值的函数数: `18`

这 18 个 `statement_count-only` 函数的头部样例:

- `crates/core/flags/defs.rs:test_context` — `66`
- `crates/ignore/tests/gitignore_matched_path_or_any_parents_tests.rs:test_dirs_in_root` — `66`
- `crates/ignore/tests/gitignore_matched_path_or_any_parents_tests.rs:test_dirs_in_deep` — `66`
- `crates/regex/src/ast.rs:various` — `45`
- `crates/core/flags/defs.rs:test_file` — `37`

结论:

- `statement_count` 确实新增了一批旧指标抓不到的长函数
- 但这批新增命中里，测试代码和测试辅助代码占比很高
- 所以它同时带来了:
  - **真实增量价值**
  - **明显噪音放大**

这是典型的“有信号，但默认阈值 / 默认扫描对象还不够稳”的状态。

### 3.4 `max_condition_ops` 的真实影响

SQL 统计:

- `max_condition_ops > 4` 的函数数: `0`

结论:

- 在干净的 `ripgrep` 基线里，`max_condition_ops` 没有自然触发任何热点
- 这说明它当前:
  - 噪音很低
  - 但自然命中也很少

所以它不像 `statement_count` 那样会立刻放大热点列表。

### 3.5 受控 `statement_count` case

用例:

- 在 `build.rs:1 main` 里临时加入大量顺序 statement
- 不额外增加分支或深嵌套

DB 中快照对比:

| snapshot | nesting | cyclomatic | cognitive | params | statement_count | max_condition_ops |
|---|---:|---:|---:|---:|---:|---:|
| baseline | 0 | 1 | 0 | 0 | 2 | 0 |
| modified | 0 | 1 | 0 | 0 | 25 | 0 |

结论:

- 新增退化完全来自 `statement_count`
- 这正是它想补的盲区:
  - 函数顺序变长
  - 职责块堆叠
  - 旧控制流指标不动

但同时暴露了一个产品问题:

- `check/diff` 报了 `build.rs:1 main degraded`
- 却没有把 `statement_count` 明细打印出来

也就是说:

- **检测链路已经接上**
- **解释链路还没接完整**

### 3.6 受控 `max_condition_ops` case

用例:

- 在 `build.rs:1 main` 注入 6 段 `&&` 链式条件

DB 中快照对比:

| snapshot | nesting | cyclomatic | cognitive | params | statement_count | max_condition_ops |
|---|---:|---:|---:|---:|---:|---:|
| baseline | 0 | 1 | 0 | 0 | 2 | 0 |
| modified | 1 | 7 | 2 | 0 | 4 | 5 |

`hotspots` 能明确显示:

```text
build.rs:1  main
  max_condition_ops: 5 ⚠ (>4)
```

结论:

- `max_condition_ops` 可以稳定补到“条件表达式膨胀”盲区
- 它和 `cyclomatic`、`cognitive` 有重叠，但提供了更直接的解释
- 在真实基线上它几乎不产生自然噪音，说明第一版口径比较克制

## 4. 价值判断

### 4.1 `statement_count`

增量价值:

- 能抓到旧控制流指标看不见的“函数变长 / 职责块堆叠”
- 在受控 case 中证明了这条链路成立

主要问题:

- 在真实仓库里噪音偏大
- 很多新增命中来自测试、测试辅助函数、长样例函数

判断:

- **有价值，但不能直接升为强主裁决指标**
- 更适合先作为辅助 signal

### 4.2 `max_condition_ops`

增量价值:

- 能补到“条件表达式越来越像谜语”这类盲区
- 解释性强

主要问题:

- 在 `ripgrep` 基线上自然命中很少
- 需要更多真实仓库继续确认是否太稀疏

判断:

- **信号干净，值得保留**
- 但当前更像“低噪音补充解释项”，不是热点主力

## 5. 关键发现

### 5.1 新指标已经揭示出一个产品缺口

当前存在一个不一致:

- `hotspots` 会展示 `statement_count` / `max_condition_ops`
- `check/diff` 会用它们参与退化判定
- 但 `check/diff` 的明细输出不会把它们打印出来

这会导致用户看到:

- “degraded 了”
- 但不知道新指标到底变坏在哪里

这是价值链路上的缺口，不只是 UI 小问题。

### 5.2 `statement_count` 的问题首先不是算法，而是 scope

从这轮结果看，`statement_count` 最大的问题不是“完全没价值”，而是:

- 它对扫描对象非常敏感
- 一旦主维护面和测试辅助代码混在一起，噪音会立刻放大

所以:

- **先修 scope / role**
- 比继续调它的阈值更重要

## 6. 最终结论

这轮 `ripgrep` Wave 1 价值 / 噪音验证后，可以得出:

1. `statement_count` 有真实增量价值，但当前噪音偏大
2. `max_condition_ops` 有清晰解释价值，且当前噪音很低
3. 默认扫描对象仍然是这批新指标最大的放大器
4. `check/diff` 的解释链路还没跟上新指标

所以当前最合理的产品动作不是“继续猛加新指标”，而是:

1. 保留 Wave 1 指标
2. 先把 `check/diff` 明细输出补完整
3. 优先推进扫描对象 `scope/role` 收敛
4. 再决定 `statement_count` 是否升格为更强信号
