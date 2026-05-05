# ripgrep 真实验证记录

> 日期: 2026-05-04
> 仓库: `/private/tmp/decay-test/ripgrep`
> commit: `4519153`
> DB: `/private/tmp/decay-test/ripgrep-may04-full.db`
> 二进制: `/Users/taoxia/Workspace/self/skills/decay/target/debug/decay`

## 1. 目标

这轮验证不再只回答“能不能跑”，而要回答四件事:

1. `decay` 在大仓库上是否稳定可用
2. 当前默认扫描对象是否已经接近“主维护面源码”
3. `check / diff` 是否真的构成局部膨胀裁决链路
4. 当前版本最该修的噪音或误报在哪里

## 2. 执行步骤

按 [docs/validation-matrix.md](/Users/taoxia/Workspace/self/skills/decay/docs/validation-matrix.md) 的主流程执行:

1. `decay check --exclude examples`，验证无 baseline 引导
2. `decay diff`，验证无 baseline 引导
3. `decay init --exclude examples`，建立 baseline
4. `decay hotspots --exclude examples`，查看真实热点
5. `decay check --exclude examples`，验证 clean
6. 在 `build.rs:1 fn main()` 注入 6 层嵌套 `if`
7. 再跑 `decay check --exclude examples`
8. 执行 `decay init --exclude examples`
9. 执行 `decay diff`
10. 恢复 `build.rs`，确认工作树干净

## 3. 实测结果

### 3.1 无 baseline 提示

`check --exclude examples`:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 1.40s

No baseline snapshot for this project.
Run `decay init` to create one.
```

`diff`:

```text
decay v0.1.0
No previous snapshot for this project.
Run `decay init` to create a baseline snapshot.
```

判断:

- 首次误用路径可接受
- 文案明确，没有把用户扔进内部错误

### 3.2 baseline 建立

运行:

```text
decay init --exclude examples
```

输出:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 1.43s

Baseline snapshot #1 saved.
69 functions currently exceed threshold.
Run `decay hotspots` to inspect them.
Run `decay check` after your next change.
```

判断:

- 大仓库扫描稳定
- 首次入口已经符合“先建 baseline，再做 check”的主流程
- `--exclude examples` 能立即压掉一部分非核心噪音

### 3.3 热点可信度

运行:

```text
decay hotspots --exclude examples
```

输出摘要:

- `80` files
- `2735` functions
- `69` functions exceed threshold

前列热点:

- `crates/ignore/src/dir.rs:431 matched_ignore`
- `crates/core/flags/hiargs.rs:113 from_low_args`
- `crates/core/main.rs:160 search_parallel`
- `crates/printer/src/standard.rs:1290 write_exceeded_line`
- `crates/searcher/src/searcher/core.rs:385 match_by_line_fast`

人工判断:

- 这些函数普遍具有多层分支、早返回、模式分流、多职责混合等特征
- 前列热点不是明显噪音
- 热点本身有维护价值，但它们更多是“历史热点”，不是“本次改动裁决”

### 3.4 clean check

运行:

```text
decay check --exclude examples
```

输出:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 1.43s

Check: current tree vs snapshot #1

✓ No functions degraded compared to the latest baseline.
```

判断:

- 无改动时不误报
- `check` 可以承担日常主入口

### 3.5 受控退化

在 `build.rs:1 fn main()` 注入 6 层嵌套后执行:

```text
decay check --exclude examples
```

输出:

```text
decay v0.1.0
Scanned 80 files, 2735 functions in 2.10s

Check: current tree vs snapshot #1

1 functions degraded:

  build.rs:1  main
    nesting: 0→6  (+6) ⚠ crossed (>4)
    cyclomatic: 1→7  (+6)
    cognitive: 0→21  (+21) ⚠ crossed (>15)
```

随后执行:

```text
decay init --exclude examples
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

- `check` 和 `diff` 都精确命中同一退化函数
- 输出已经足够支持“这次改动需要重构或拆分”的决策
- 这是当前版本最核心、也最成立的产品价值

### 3.6 工作树恢复

测试后已恢复 `build.rs`，并确认:

```text
git status --short
```

输出为空。

## 4. 价值判断

### 4.1 已验证成立的价值

- `decay` 能在 `ripgrep` 这种成熟 Rust 项目上稳定运行
- `hotspots` 可以提供一批基本可信的复杂函数候选
- `check` / `diff` 能对受控复杂化给出直接、可行动的裁决

### 4.2 真正的核心价值

这次验证再次证明:

- **真正的核心价值是 `check` / `diff`**
- 首次 `hotspots` 列表只是辅助视图
- 如果只看第一次全量热点，`decay` 很容易被误解为另一个 complexity lint

### 4.3 这轮对“扫描对象”的真实结论

这轮验证不能得出“默认扫描对象已经正确”。

更准确的结论是:

- 默认扫描行为在工程上可用
- 但默认扫描对象仍然偏宽
- 需要依赖 `--exclude examples` 才更接近“主维护面源码”

所以这轮验证证明的是:

- **扫描对象问题已经被清楚暴露**
- **但默认扫描对象还没有被产品方案彻底解决**

## 5. 暴露出的现实问题

按当前这轮复测，仍然最值得关注的是:

1. `params` 告警信号偏弱
   - 例如 `replace_all`、`replace_with_captures_at` 一类函数
   - “参数多”在这些上下文里 often true，但不总是高 actionability
2. 大仓库仍需要排除策略
   - `examples` 不排除时会稀释结果
   - 未来很可能还需要 `tests` / `benches` / `fixtures`
3. 首次热点数量仍然较多
   - 即使排掉 `examples`，首次 baseline 后仍有 `69` 个热点
   - 这再次说明 baseline 结果不能代替 regression 判断

## 6. 结论

这轮 `ripgrep` 完整测试后，可以给出明确结论:

- **实用性成立**: 可在大型 Rust 仓库上稳定建基线、看热点、做检查、做 diff
- **价值性成立**: 对真实复杂热点有一定识别能力，对受控退化有强判断力
- **可用性基本成立**: `init / check / diff / hotspots` workflow 清晰，但依赖排除策略控噪

当前最合理的产品判断不是“已经完美”，而是:

- 可以继续 dogfood
- 可以继续拿真实仓库做回归验证
- 下一轮优化优先级应放在降噪，而不是扩更多新命令
