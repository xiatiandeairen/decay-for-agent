# ripgrep 真实验证记录

> 日期: 2026-05-04
> 目标仓库: `/private/tmp/decay-test/ripgrep`
> ripgrep commit: `4519153`

## 1. 目标

验证两件事:

1. 用 `decay` 扫描完整 ripgrep 项目时, 是否能发现**有价值**的问题
2. 如果 `decay` 本身有问题, 找到**根因**并记录

## 2. 实测结果

### 2.1 基线扫描

在 ripgrep 根目录运行 `decay`:

```text
decay v0.1.0
Scanned 83 files, 2742 functions in 1.38s
Snapshot #1 saved [first snapshot — run `decay diff` after next change]
75 functions exceed threshold
```

补充验证:

- 用 `cargo run --example find_dupes -- /private/tmp/decay-test/ripgrep` 检查指纹冲突
- 结果: `colliding hash groups: 0`

结论:

- `decay` 已能在 ripgrep 这种真实 Rust 项目上完成全量扫描和落库
- `cfg` 相关指纹冲突已经被消除, 不再阻塞真实项目扫描

### 2.2 无改动 diff

连续扫描两次后运行 `decay diff`:

```text
decay v0.1.0
Diff: snapshot #2 vs #1 (0 minutes ago)
✓ No functions degraded since last snapshot.
```

结论:

- 同一份代码的重复扫描不会误报 regression
- snapshot 对齐和 diff 基本链路工作正常

### 2.3 受控回归注入

为了验证 `diff` 的核心价值, 在 ripgrep 的 `build.rs:1 fn main()` 中临时注入 5 层嵌套 `if`, 再扫描并运行 `decay diff`。

输出:

```text
decay v0.1.0
Diff: snapshot #4 vs #3 (0 minutes ago)

1 functions degraded:

  build.rs:1  main
    nesting: 2→6  (+4) ⚠ crossed (>4)
    cyclomatic: 3→7  (+4)
    cognitive: 5→21  (+16) ⚠ crossed (>15)
```

测试后已恢复 ripgrep 工作树, 当前 `git status --short` 为空。

结论:

- `decay diff` 在真实仓库上能抓到“这次改动让函数跨阈值/变坏”的变化
- 这一点符合产品的核心承诺: **看 delta, 不只看绝对值**

## 3. 有价值的发现

### 3.1 高价值热点确实能被扫出来

基线扫描中排在前列的函数大多不是噪音, 而是肉眼可见的复杂控制流:

- `crates/core/flags/hiargs.rs:113 from_low_args` — `cognitive 61`, `cyclomatic 38`
- `crates/ignore/src/dir.rs:431 matched_ignore` — `cognitive 58`, `cyclomatic 28`
- `crates/searcher/src/searcher/core.rs:385 match_by_line_fast` — `cognitive 41`, `cyclomatic 20`, `nesting 5`

抽样阅读源码后, 这些函数普遍具备以下特征:

- 大量 mode/flag 分支
- 多层 guard / early return / nested control flow
- 在一处函数内承担多段职责

这类结果对“发现真实复杂热点”是有价值的, 不是明显的误报。

### 3.2 真正有产品价值的是 diff, 不是第一次基线列表

对 ripgrep 这种成熟项目, 第一次扫描直接给出 `75 functions exceed threshold`。

这能说明项目里有哪些历史热点, 但它对“刚才这次 AI 改动有没有把代码变坏”这个核心问题帮助有限。真正有价值的是 2.3 那类受控回归验证结果: 改完再跑 `decay diff`, 立即告诉你这次改动让哪个函数跨阈值了。

结论:

- **存在真实价值**
- 但价值主要集中在 `decay diff`
- `decay` 首次扫描的热点列表更像辅助视图, 不是核心杀手特性

## 4. 暴露出的问题与根因

以下问题是这次 ripgrep 实测中暴露出来的, 按严重度排序。

### P1. cognitive complexity 对 `else if` 链明显高估

现象:

- `crates/ignore/src/pathutil.rs:113 file_name` 被报为 `cognitive: 16`
- 该函数源码本质上只是一个 guard-style 的 `if / else if / else if / else if` 链, 复杂度不应高到 16

根因:

- [`src/metric/cognitive.rs`](/Users/taoxia/Workspace/self/skills/decay/src/metric/cognitive.rs) 的 `score_if` 把 `alternative` 统一按 `nesting + 1` 递归
- 当 `else` 分支本身又是 `if_expression` 时, 当前实现把 `else if` 当成“更深一层的嵌套 if”
- 结果是 `else if` 链按 `1 + 2 + 3 + ...` 递增计分, 被错误放大

为什么这是 bug 而不是口味问题:

- `else if` 更接近平铺分支链, 不是语义上的更深嵌套
- 当前实现会系统性抬高大量 guard-chain / parser-style 代码的 `cognitive` 分数
- 这会直接污染阈值判断, 降低告警可信度

建议:

- 单独修 `else if` 记分规则
- `else if` 应记 branch increment, 但不应继续叠加更深 nesting penalty

### P1. 首次扫描噪音偏大, 与产品“看 delta”定位存在张力

现象:

- ripgrep 首次扫描直接输出 75 个超阈值函数
- 对成熟项目来说, 这张列表大部分是“历史债务”, 不是“这次改动造成的问题”

根因:

- [`src/cli/scan.rs`](/Users/taoxia/Workspace/self/skills/decay/src/cli/scan.rs) 的默认命令在保存 snapshot 后总是打印全部超阈值函数
- 这是一种“绝对值热点视图”, 不是“退化视图”
- 与产品主张的 “delta over absolute” 有轻微冲突

影响:

- 初次接入大项目时, 用户首先看到的是长列表噪音, 不是“这次改动的判断”
- 容易让用户把 decay 理解成又一个复杂度 lint, 而不是 regression detector

建议:

- 保留当前输出, 但弱化其主地位
- 后续可以考虑把首次扫描改成“摘要 + top N”, 或把热点列表变为显式子命令

### P2. examples / 非核心代码被一并扫描, 会稀释结果

现象:

- ripgrep 输出中包含 `crates/ignore/examples/walk.rs:5 main`
- 这类示例代码通常不是用户最关心的“核心维护面”

根因:

- walker 当前只排除了 `target/` 和 `.git/`
- 不读 `.gitignore`, 也不支持 include/exclude 配置

影响:

- 结果会混入 examples / fixtures / demo code
- 在大型仓库里会稀释信号密度

建议:

- 后续支持排除配置, 至少允许忽略 `examples/`、`tests/`、`benches/` 一类目录

### P3. `params > 5` 的告警在真实项目上信号偏弱

现象:

- ripgrep 中有多条告警只因为参数数达到 6 或 7, 例如:
  - `crates/printer/src/util.rs:557 replace_with_captures_in_context`
  - `crates/printer/src/util.rs:51 replace_all`
  - `crates/matcher/src/lib.rs:948 replace_with_captures_at`

根因:

- 参数阈值是全局硬编码的 `5`
- 不区分 public API / internal helper / builder-style utility

影响:

- 会产生一些“ technically true, but low actionability ”的结果
- 与 cognitive / nesting / cyclomatic 相比, params 更容易变成低信噪比指标

建议:

- 先不要删这个 metric
- 但后续应重新校准阈值, 或允许按 metric 单独关闭/调高

## 5. 结论

### 5.1 产品价值判断

结论是 **有价值, 但价值边界很清楚**:

- 作为“真实 Rust 项目的函数级复杂热点发现器”, `decay` 能产出一批基本可信的结果
- 作为“改动后立即判断这次是否变坏”的工具, `decay diff` 在 ripgrep 上通过了真实验证
- 它的核心价值确实成立, 但主要成立在 **diff 场景**, 不在首次全量热点列表

### 5.2 当前最值得修的问题

如果只按 ripgrep 这次验证结果排优先级, 建议顺序是:

1. 修 `cognitive` 的 `else if` 过度计分
2. 降低首次扫描的噪音, 让产品更贴近 “delta detector” 定位
3. 支持扫描排除配置, 避免 examples/tests 稀释结果
4. 重新校准 `params` metric 的默认阈值

## 6. 2026-05-04 第二轮修复后复测

本轮完成了三项修复:

- 入口改为方案 B: `init / check / diff / hotspots`
- `cognitive` 修正 `else if` 链不再持续叠加 nesting penalty
- 扫描新增 `--exclude` 能力

### 6.1 新入口在 ripgrep 上的表现

运行:

```text
decay init
```

输出:

```text
decay v0.1.0
Scanned 83 files, 2742 functions in 1.40s

Baseline snapshot #1 saved.
71 functions currently exceed threshold.
Run `decay hotspots` to inspect them.
Run `decay check` after your next change.
```

结论:

- 默认基线建立不再直接倾倒完整热点列表
- 产品入口已从“绝对值报告”切向“先建基线, 后做检查”

### 6.2 `else if` 修复后的热点变化

运行:

```text
decay hotspots --exclude examples
```

输出摘要:

- `80` files
- `2735` functions
- `69` functions exceed threshold

与修复前对比:

- 修复前基线热点数: `75`
- 修复后排除 `examples` 后热点数: `69`
- 之前可疑的 `crates/ignore/src/pathutil.rs:113 file_name` 已不再出现在超阈值列表中

结论:

- `else if` 计分修复确实消除了至少一类已确认的误报
- 这不是理论修复, 在 ripgrep 真实数据上已经反映到输出

### 6.3 `check` 主入口复测

运行:

```text
decay check --exclude examples
```

输出:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 1.39s

Check: current tree vs snapshot #1

✓ No functions degraded compared to the latest baseline.
```

结论:

- 新主入口已经符合“改完代码后做裁决”的目标语义
- `check` 比旧的默认 `scan + 热点列表` 更贴合产品定位

## 7. 2026-05-04 完整矩阵复测

本轮按 [docs/validation-matrix.md](/Users/taoxia/Workspace/self/skills/decay/docs/validation-matrix.md) 执行一套完整测试，使用独立 DB:

- DB: `/private/tmp/decay-test/ripgrep-full.db`
- 仓库: `/private/tmp/decay-test/ripgrep`
- ripgrep 工作树在测试结束后已恢复干净, `git status --short` 为空

### 7.1 `init`

运行:

```text
decay init
```

输出:

```text
decay v0.1.0
Scanned 83 files, 2742 functions in 1.36s

Baseline snapshot #1 saved.
71 functions currently exceed threshold.
Run `decay hotspots` to inspect them.
Run `decay check` after your next change.
```

判断:

- 基线建立成功
- 首次入口文案符合“先建 baseline, 再做检查”的方案 B 目标

### 7.2 `hotspots --exclude examples`

运行:

```text
decay hotspots --exclude examples
```

输出摘要:

- `80` files
- `2735` functions
- `69` functions exceed threshold

前列热点依旧是可解释的复杂函数:

- `crates/ignore/src/dir.rs:431 matched_ignore`
- `crates/core/flags/hiargs.rs:113 from_low_args`
- `crates/core/main.rs:160 search_parallel`

判断:

- 热点结果仍然有价值
- `examples` 噪音可被用户主动压掉
- `else if` 修复后, `file_name` 类误报已消失

### 7.3 `check` clean

运行:

```text
decay check --exclude examples
```

输出:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 1.36s

Check: current tree vs snapshot #1

✓ No functions degraded compared to the latest baseline.
```

判断:

- 无改动场景 clean
- `check` 已可作为日常主入口

### 7.4 受控回归注入

在 `build.rs:1 fn main()` 中临时加入 5 层嵌套 `if` 后，运行:

```text
decay check --exclude examples
```

输出:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 1.39s

Check: current tree vs snapshot #1

1 functions degraded:

  build.rs:1  main
    nesting: 0→6  (+6) ⚠ crossed (>4)
    cyclomatic: 1→7  (+6)
    cognitive: 0→21  (+21) ⚠ crossed (>15)
```

随后运行:

```text
decay init
decay diff
```

`diff` 输出:

```text
decay v0.1.0
Diff: snapshot #2 vs #1 (0 minutes ago)

1 functions degraded:

  build.rs:1  main
    nesting: 0→6  (+6) ⚠ crossed (>4)
    cyclomatic: 1→7  (+6)
    cognitive: 0→21  (+21) ⚠ crossed (>15)
```

判断:

- `check` 能抓当前工作树相对 baseline 的退化
- `diff` 能抓两次快照之间的同一退化
- 这两条链路都在真实 ripgrep 仓库上通过

### 7.5 最终结论

按完整矩阵复测后, 当前版本可以认为已经满足:

- **实用性**: 能在真实 Rust 大项目上稳定建基线、看热点、做检查、做 diff
- **价值性**: 输出中既有可信热点, 也能对受控回归给出直接可行动的裁决
- **可用性**: `init/check/diff/hotspots` 分工清晰, 首次使用与日常使用路径明确
