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
4. 如果出现退化，作者决定返工、忽略或判定为噪音。

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

`decay diff` 只报告退化：

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

## 数据持久化

v0.1.0 使用 SQLite 保存 baseline。默认路径必须符合 XDG data 目录规范。

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

v0.1.0 不做 DB migration 兼容，因为当前没有外部用户和稳定 schema 承诺。

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
- 每个 dogfood 命中都必须归类为 `fixed`、`ignored` 或 `noise`。

如果没有真实返工案例，v0.1.0 只能算功能 PoC，不能算产品假设成立。
