# PRD：Rust 函数级复杂度退化裁决

## 要解决的问题

作者使用 AI 或手动修改 Rust 代码后，需要在 commit 前判断：

> 这次改动是否让某些函数变得明显更复杂？

人工读 diff 不稳定，AI 自评容易迎合，传统复杂度工具主要报告当前复杂函数，不回答“这次改动是否变糟”。`decay` 的产品切口是把 baseline 和当前工作区放在一起比较，只报告可解释的函数级退化。

## 目标用户和场景

目标用户只有作者本人。

典型流程：

1. 修改前或阶段开始时保存 baseline。
2. AI 或人完成一波修改。
3. commit 前运行 `decay diff <baseline>`。
4. 如果出现退化，作者决定返工、追加 review、忽略或判定为噪音。

v0.1.0 不承诺外部稳定接口。

## 命令语义

当前命令模型：

```bash
decay doctor
decay baseline <version>
decay diff <version>
decay diff <from> <to>
```

| 命令 | 作用 | 产品定位 |
|---|---|---|
| `decay` | 展示简洁命令列表，退出码 0，不扫描、不写 DB | 命令入口，不是体检，也不是 gate |
| `decay --help` | 展示完整用法、参数和子命令说明 | 详细帮助 |
| `decay doctor` | 显式查看当前代码风险 | 辅助体检，不是 gate |
| `decay baseline <version>` | 保存当前代码为命名 baseline | 为 diff 提供对比点 |
| `decay diff <version>` | 比较当前工作区相对 baseline 是否退化 | 核心价值路径 |
| `decay diff <from> <to>` | 比较两个命名 baseline 是否退化 | 核心价值路径 |

`doctor` 只能说明“当前哪里复杂”，不能说明“这次改动是否让它变复杂”。因此产品验证必须看 `diff`，不能只看 `doctor`。

v0.1.1 的 `decay diff <git-ref>` 已进入 fact evidence 路径：直接输出 `Complexity impact`、`Stability impact`、`Quality impact`、`Semantic impact`、`History impact`，并把 changeset、file changes 和 evidence 写入本地 SQLite。该行为不新增 `--evidence` flag。两个命名 baseline 之间的 `decay diff <from> <to>` 仍走 baseline snapshot 对比路径。

v0.2.0 的方向是在 fact evidence 之上增加 semantic selection layer。事实层继续负责客观扫描，语义层负责把事实组合成可复核的 review 证据：

- 强证据进入 top review evidence，例如默认值/配置边界变化、显式 unsafe/panic、可确认的 public API surface 变化、明确协议扫描 pattern。
- 中弱证据默认作为 supporting context，例如缺测试、touched old risky function、history hotspot。
- `mixed_concerns`、`blast_radius`、history、纯 touched old debt 不得单独作为高价值 Top risk；只有与直接 diff evidence 绑定时才提升。
- 每个 semantic finding 必须包含 claim、supporting facts、source refs、confidence reason、limitation 和 reviewer action。
- semantic layer 不做合入裁决，不声明 bug，只说明为什么这一组事实值得 reviewer 优先看。

## Active metrics

v0.1.0 只有 6 个 active metric：

| Metric | 阈值 | 含义 |
|---|---:|---|
| `nesting` | 4 | 最大控制流嵌套深度 |
| `cyclomatic` | 10 | 分支路径复杂度 |
| `cognitive` | 15 | 更贴近阅读负担的分支复杂度 |
| `params` | 5 | 函数参数数量 |
| `statement_count` | 25 | 函数体可执行语句数量 |
| `max_condition_ops` | 4 | 单个条件表达式中的布尔操作符最大数量 |

阈值语义统一为：

```text
value > threshold => breach
```

`mutable_bindings` 不是 v0.1.0 active metric。它曾出现在旧文档里，但没有实现和验证，已经从产品承诺中移除。

## 扫描范围

默认 scope 是 `prod`。

`prod` 排除：

- `tests/`
- `examples/`
- `benches/`
- `fixtures/`
- `target/`
- `.git/`
- `testutil.rs`
- `#[test]`
- `#[cfg(test)]`
- `mod tests` 内函数

可以用 `--scope all` 查看完整 Rust 文件视图。项目根目录 `.gitignore` 会生效，也支持 `--exclude <pattern>` 做局部排除。

## Diff 报告规则

命名 baseline diff 只报告退化：

| 类型 | 条件 |
|---|---|
| Added | 新增函数，且至少一个 active metric 超阈值 |
| CrossedThreshold | 同一函数从未超阈值变为超阈值 |
| Worsened | 同一函数原本已超阈值，且继续变大 |

不报告：

- 删除函数。
- 指标下降。
- 指标不变。
- 仍在阈值内的小幅上升。

这个策略故意不做“当前复杂函数排行榜”，因为 v0.1.0 的目标是判断本次改动是否退化。

git-ref diff 的 fact evidence 输出额外报告：

- `complexity`: 本次 touched function 的 worsened / improved / new high risk / touched existing risky evidence。
- `stability`: FFI/public API、storage/migration、protocol/serialization、unsafe、panic/unwrap/expect 等边界证据。
- `quality`: test gap、sensitive test gap、blast radius、mixed concerns、dependency/build/CI 变更证据。
- `history`: touched production files 的 churn hotspot、recently bugfixed area、frequent rework 证据。

这些 fact evidence 是事实层，不是合入风险评分；输出不得声明 `risk=high` 或“bug”。0.2.0 的 semantic layer 可以输出 top review evidence，但必须保留 source chain，不得把不可复查的主观判断当成事实。

## v0.2.0 Semantic Selection

### 目标

0.2.0 不再把更多静态扫描命中等同于更高产品价值。目标是利用事实 evidence 和可选外部上下文，筛出更少但更可信的 review 证据。

### 输入

- fact evidence：complexity / stability / quality / scanned semantic pattern / history。
- diff context：changed files、hunks、function ranges、added/deleted lines。
- local DB：changeset、file changes、evidence items、历史 judgement。
- opt-in external context：PR title/body/review comments、linked issue、CI failure、decision docs。

### 输出

每条 semantic finding 使用以下字段：

```text
claim:
supporting_facts:
source_refs:
confidence:
confidence_reason:
limitation:
review_action:
```

### 语义层必须解决的 0.1.1 限制

| 0.1.1 现状 | 问题 | 0.2.0 目标 |
|---|---|---|
| `mixed_concerns` / `blast_radius` 可进入 Top risk | 多数只是范围描述，行动价值弱 | 降级为 supporting context，除非绑定直接 diff evidence |
| `touched_existing_risky` 可进入 Top risk | 容易把旧债包装成本次风险 | 默认作为上下文，只在函数内有直接变化证据时提升 |
| API/protocol 行为类风险可能漏报 | 纯扫描不理解 reviewer 真正关心的行为边界 | 通过 API surface、protocol behavior、behavior gate 场景规则组合事实 |
| history evidence 容易显得吓人 | 历史不是本次 diff 的直接证据 | 只作为 context，不单独算 useful |
| pattern detector 易写死 | 小样本命中可能自我欺骗 | 每个 scenario 必须进入 bad-case regression 和 20-case benchmark |

### 验证目标

以 v0.1.1 的 10-case 结果为 baseline：

```text
precision_at_top3 = 65%
actionable_pr_rate = 60%
negative_control_silence_rate = 100%
missed_high_risk_count = 2
noise_top3_count = 3
```

0.2.0 第一轮 20-case 目标：

```text
precision_at_top3 >= 75%
actionable_pr_rate >= 70%
negative_control_silence_rate >= 90%
missed_high_risk_count <= 1
weak_only_top_risk <= 2
```

## 数据持久化

decay 使用 SQLite 保存 baseline 和本地 diff evidence。默认路径必须符合 XDG data 目录规范。

默认路径：

```text
$XDG_DATA_HOME/decay/snapshots.db
```

当 `XDG_DATA_HOME` 未设置时，回退到：

```text
$HOME/.local/share/decay/snapshots.db
```

测试可通过 `DECAY_DB_PATH` 覆盖。

Baseline 保存的信息：

- project id。
- scope。
- version。
- created / updated 时间。
- partial scan 状态。
- scan diagnostic 数量。
- 函数元数据。
- active metric 值。

v0.1.1 git-ref diff 额外保存：

- changeset：project key、base ref、base commit、head commit。
- file changes：status、old/new file、added/removed lines。
- evidence items：category、kind、file、subject、source。

v0.2.0 计划继续扩展：

- semantic findings：claim、confidence、limitation、review action。
- fact-to-claim links：每条 semantic finding 关联的 evidence item id 和外部 source ref。
- judgement feedback：useful / plausible / noise / missed、是否进入 top risk、是否改变 review action。

同一 changeset 重复运行会保持幂等，不重复堆积 evidence。当前 schema 仍只承诺本地 dogfood 使用，不承诺外部稳定接口。

## 验收标准

功能验收：

- `cargo test` 通过。
- `cargo clippy -- -D warnings` 通过。
- `decay` 无子命令展示简洁命令列表，退出码 0，不扫描、不写 DB。
- `decay --help` 展示完整用法、参数和子命令说明。
- `decay doctor` 能输出当前代码风险。
- `decay baseline <version>` 能保存命名 baseline。
- `decay diff <version>` 能识别受控退化，并在发现退化时返回非 0。
- parse 失败不会中断整体扫描，但结果必须标记 partial。

产品验收：

- dogfood 中至少 1 次 `decay diff` 捕捉到作者原本没注意、且作者认同应该返工的退化。
- 每个 dogfood 命中都必须归类为 `fixed`、`reviewed`、`ignored` 或 `noise`。

如果没有真实返工案例，v0.1.0 只能算功能 PoC，不能算产品假设成立。
