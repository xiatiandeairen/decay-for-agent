# ripgrep scope/role 收敛验证

> 日期: 2026-05-04
> 仓库: `/private/tmp/decay-test/ripgrep`
> commit: `4519153e5e`
> 二进制: `/Users/taoxia/Workspace/self/skills/decay/target/debug/decay`
> 范围: `scope prod|all` + `role` 收敛后的真实仓库验证

## 1. 目标

这轮验证不再回答“实现是否可行”，而是回答两个更接近产品的问题:

1. `scope/role` 收敛后，`statement_count + max_condition_ops` 的价值链路是否明显变稳
2. 当前局部膨胀方案，是否已经足够支撑 v0.1，而不是立刻继续开 `Wave 2`

## 2. 当前实现口径

默认 `--scope prod` 当前会排除:

- 路径角色: `tests/`、`examples/`、`benches/`、`fixtures/`
- 文件角色: `testutil.rs`
- 语义角色:
  - `#[test]`
  - `#[cfg(test)]`
  - `mod tests { ... }` 内函数

`--scope all` 保留完整视图。

## 3. 执行结果

### 3.1 热点总量变化

| 场景 | 文件数 | 函数数 | 热点数 |
|---|---:|---:|---:|
| `hotspots --scope all` | 83 | 2742 | 89 |
| `hotspots` (`prod`) | 70 | 2120 | 74 |

结论:

- 默认 `prod` 相比 `all` 少了 `13 files / 622 functions / 15 hotspots`
- 这说明之前“新指标一加进来就变吵”的很大一部分，不是指标本身，而是扫描对象混入了非主维护面代码

### 3.2 被成功压掉的典型噪音

这轮从 `prod` 里消失的典型热点包括:

- `crates/searcher/src/testutil.rs:configs`
- `crates/searcher/src/testutil.rs:test`
- `crates/ignore/tests/gitignore_matched_path_or_any_parents_tests.rs:test_dirs_in_root`
- `crates/ignore/tests/gitignore_matched_path_or_any_parents_tests.rs:test_dirs_in_deep`
- `crates/ignore/examples/walk.rs:main`
- `crates/grep/examples/simplegrep.rs:search`
- `crates/core/flags/defs.rs:test_context`
- `crates/core/flags/defs.rs:test_file`
- `crates/core/flags/defs.rs:test_after_context`
- `crates/core/flags/defs.rs:test_before_context`
- `crates/core/flags/defs.rs:test_regexp`

这批样本很关键，因为它们正好覆盖了三类之前最难清掉的噪音:

1. 目录级测试代码
2. `src/` 下的测试辅助文件
3. 主源码文件里由 `#[cfg(test)]` 包起来的测试函数

说明这轮 role 收敛不是“又多几个路径排除”，而是真正开始识别“谁不是主维护面”。

### 3.3 主维护面信号是否被误伤

`prod` 模式下保留下来的头部热点仍然集中在真实主链路源码，比如:

- `crates/ignore/src/dir.rs:matched_ignore`
- `crates/core/flags/hiargs.rs:from_low_args`
- `crates/core/main.rs:search_parallel`
- `crates/searcher/src/searcher/core.rs:match_by_line_fast`
- `crates/searcher/src/searcher/glue.rs:run`

这些都是典型“值得用户关心”的主维护面函数。

结论:

- 这轮收敛显著降低了噪音
- 但没有把主链路热点一起扫掉

这比单纯调高阈值更有价值。

## 4. 对局部膨胀价值的影响

### 4.1 `statement_count`

在 scope/role 收敛之前，`statement_count` 最大的问题是:

- 有真实增量价值
- 但会把测试、样例、test helper 一起推上热点榜

收敛之后，这个问题明显改善了。

证据是:

- 热点数从上一轮 `prod` 的 `85` 下降到 `74`
- 被清掉的样本里，大量是 `statement_count` 特别容易击中的长测试函数

这说明:

- `statement_count` 不是“天然噪音指标”
- 它更像“对扫描对象极度敏感的指标”

也就是说，之前它的问题首先不是 metric 无价值，而是默认扫描对象不准。

### 4.2 `max_condition_ops`

这个指标在上一轮已经表现为:

- 自然命中少
- 噪音低
- 更偏解释项

scope/role 收敛后，这个判断没有变化。

所以当前最稳的结论仍然是:

- `statement_count`: 价值在 scope 收敛后变得更可信
- `max_condition_ops`: 继续保持为低噪音补充信号

## 5. 是否足够支撑 v0.1

### 5.1 可以说“够了”的部分

如果把 v0.1 的目标收窄成:

> 让用户在 AI 改完代码后，对主维护面函数的局部膨胀做一次 commit 前裁决

那么当前链路已经基本够了。

原因是:

1. `check / diff / hotspots` 主链路已完整可用
2. `statement_count` 补到了“函数变长 / 职责块堆叠”的盲区
3. `max_condition_ops` 补到了“条件表达式膨胀”的盲区
4. `scope/role` 已经把最显著的一批噪音压下去了

这意味着:

- 当前方案已经不是“只能跑的 PoC”
- 而是一个初步成立的 v0.1 局部膨胀裁决器

### 5.2 还不能说“完成了”的部分

这不等于 v0.1 产品验证已经完成。

还缺的不是实现，而是证据:

1. 真实 dogfood 返工案例
2. 真实“原本会漏掉，但被工具拦下”的案例
3. 长期使用下剩余噪音是否还能接受

也就是说，当前更准确的判断是:

- **v0.1 的局部膨胀主链路已经可用**
- **v0.1 的产品验证还没完成**

## 6. 为什么现在不该直接开 Wave 2

`Wave 2` 计划里的核心是 `mutable_bindings`。

现在不应该直接开它，原因不是它不重要，而是当前最关键的问题已经变了:

- 之前最大问题是“scope 不准，噪音大”
- 现在 scope/role 已经明显改善
- 接下来最有价值的，不是继续堆 metric
- 而是确认当前这套主链路，是否真的改变 commit 决策

如果此时直接开 `Wave 2`，风险是:

- 还没来得及确认 `Wave 1 + scope/role` 的真实上限
- 就把新的状态型指标噪音引进来
- 重新打乱价值判断

所以更合理的顺序是:

1. 先 dogfood 当前主链路
2. 收集真实返工 / 忽略 / 噪音案例
3. 再决定 `mutable_bindings` 是不是值得进入下一波

## 7. 最终结论

这轮 `ripgrep` scope/role 收敛验证后，可以下 3 个结论:

1. 默认扫描对象已经从“仓库里所有 Rust 代码”收敛到了更接近“主维护面源码”
2. `statement_count + max_condition_ops` 这条局部膨胀链路，在降噪后已经足够支撑 v0.1 的最小产品定位
3. 当前最该做的不是直接进入 `Wave 2`，而是先做真实 dogfood 价值验证

一句话说:

> **现在的阻塞点已经不是缺一个新 metric，而是缺真实使用证据。**
